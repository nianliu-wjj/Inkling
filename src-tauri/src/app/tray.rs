//! 系统托盘：左键打开主窗口，右键弹出 Inkling 主菜单，悬浮提示当天未完成待办。

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use super::state::AppState;
use super::windows;

/// 托盘图标的固定标识，用于事后取回图标更新提示文本。
const TRAY_ID: &str = "main-tray";

/// 提示里最多列出的待办条数。
///
/// Windows 的托盘提示有长度上限（约 128 字符），超出会被系统直接截断成乱尾；
/// 列 5 条 + 一行汇总在中文下大致贴住这个上限，多出的用「还有 N 项」收口。
const TOOLTIP_MAX_ITEMS: usize = 5;

/// 单条待办在提示里的最大显示字数，超出以省略号收尾。
const TOOLTIP_ITEM_CHARS: usize = 14;

/// 按当天未完成待办拼出托盘提示文本。
///
/// 无待办时只显示应用名，不显示「0 项」——那反而像出了故障。
fn tooltip_text(app: &AppHandle) -> String {
    const TITLE: &str = "✒️ Inkling · 念头捕手";

    let today = crate::data::local_date_key(chrono::Utc::now());
    let todos = match app
        .state::<AppState>()
        .lock_store()
        .and_then(|store| store.list_today_open_todos(&today))
    {
        Ok(list) => list,
        Err(error) => {
            eprintln!("[tray] 读取当天待办失败，提示退回应用名: {error}");
            return TITLE.into();
        }
    };
    if todos.is_empty() {
        return format!(
            "{TITLE}
今天没有待办"
        );
    }

    let mut lines = vec![format!(
        "{TITLE}
今天还有 {} 项未完成：",
        todos.len()
    )];
    for todo in todos.iter().take(TOOLTIP_MAX_ITEMS) {
        // 逾期与高优先级各给一个标记，让用户在提示里就能分辨轻重。
        let overdue = crate::domain::todo::parse_time(todo.due_at())
            .map(|due| due < chrono::Utc::now())
            .unwrap_or(false);
        let mark = if overdue {
            "⚠️"
        } else if todo.priority() == "high" {
            "🔴"
        } else {
            "·"
        };
        // 按字符而非字节截断：中文一个字符占 3 字节，按字节切会切出半个字。
        let mut text: String = todo.content().chars().take(TOOLTIP_ITEM_CHARS).collect();
        if todo.content().chars().count() > TOOLTIP_ITEM_CHARS {
            text.push('…');
        }
        lines.push(format!("{mark} {text}"));
    }
    if todos.len() > TOOLTIP_MAX_ITEMS {
        lines.push(format!("还有 {} 项…", todos.len() - TOOLTIP_MAX_ITEMS));
    }
    lines.join(
        "
",
    )
}

/// 上一次刷新提示时的本地日期，用于跨日检测。
static LAST_TOOLTIP_DAY: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// 跨过午夜时刷新提示。
///
/// 提示内容是「今天」的未完成待办，日期一变清单就该变，而这种变化不伴随任何
/// 待办改动、不会走 TODOS_CHANGED。由提醒调度器每轮调用本函数兜住：
/// 只比较一个日期字符串，日期未变时不读数据库。
pub fn refresh_tooltip_if_day_changed(app: &AppHandle) {
    let today = crate::data::local_date_key(chrono::Utc::now());
    let Ok(mut last) = LAST_TOOLTIP_DAY.lock() else {
        return;
    };
    if *last == today {
        return;
    }
    *last = today;
    drop(last);
    eprintln!("[tray] 日期变更，刷新托盘提示");
    refresh_tooltip(app);
}

/// 刷新托盘提示。待办变化后调用，失败只记日志——提示文本不是关键路径。
pub fn refresh_tooltip(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        eprintln!("[tray] 托盘图标不存在，跳过提示刷新");
        return;
    };
    let text = tooltip_text(app);
    if let Err(error) = tray.set_tooltip(Some(&text)) {
        eprintln!("[tray] 更新托盘提示失败: {error}");
    }
}

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let history = MenuItem::with_id(app, "history", "📚 历史归档", true, None::<&str>)?;
    let stats = MenuItem::with_id(app, "stats", "📊 统计报表", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "⚙️ 偏好设置", true, None::<&str>)?;
    let quit = PredefinedMenuItem::quit(app, Some("⏻ 退出 Inkling"))?;
    let menu = Menu::with_items(app, &[&history, &stats, &settings, &quit])?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default icon".into()))?;

    // macOS 分支需要重新赋值，非 macOS 下 mut 不被使用，显式放行告警。
    #[allow(unused_mut)]
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip(tooltip_text(app))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "history" => {
                let _ = windows::show_main(app, "notes");
            }
            "stats" => {
                let _ = windows::show_main(app, "stats");
            }
            "settings" => {
                let _ = windows::show_main(app, "settings");
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle().clone();
                // 左键：主窗口可见则隐藏（toggle），否则打开默认笔记视图。
                let visible = app
                    .get_webview_window("main")
                    .map(|w| w.is_visible().unwrap_or(false))
                    .unwrap_or(false);
                if visible {
                    let _ = windows::hide_main(&app);
                } else {
                    let _ = windows::show_main(&app, "notes");
                }
            }
        });
    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }
    builder.build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    /// 提示文本的截断与标记逻辑用真实 Store 才能跑通，
    /// 这里只覆盖纯字符串部分：中文按字符截断而不是按字节。
    #[test]
    fn chinese_content_truncates_by_char_not_byte() {
        let content = "写完提醒改造的实机验证文档并推送远端";
        let taken: String = content.chars().take(super::TOOLTIP_ITEM_CHARS).collect();
        // 14 个字符 = 42 字节；按字节切会切出半个汉字。
        assert_eq!(taken.chars().count(), super::TOOLTIP_ITEM_CHARS);
        assert!(content.chars().count() > super::TOOLTIP_ITEM_CHARS);
        assert!(taken.is_char_boundary(taken.len()));
    }
}
