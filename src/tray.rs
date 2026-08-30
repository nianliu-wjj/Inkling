//! 系统托盘：提供无需打开主窗口即可访问 Inkling 核心入口的菜单。

use std::time::Duration;

use gpui::{App, Global};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

struct TrayGlobal {
    // TrayIcon 必须保持存活，否则系统托盘图标会被移除。
    _icon: TrayIcon,
}

impl Global for TrayGlobal {}

fn tray_icon() -> Result<Icon, String> {
    // 生成一个不依赖外部文件的 16×16 深色 Inkling 标记，避免安装后资源路径失效。
    let mut rgba = vec![0u8; 16 * 16 * 4];
    for y in 0..16 {
        for x in 0..16 {
            let inside = (x >= 3 && x <= 12 && y >= 2 && y <= 13)
                || (x >= 5 && x <= 10 && y >= 1 && y <= 14);
            let accent = (x == 7 || x == 8) && y >= 4 && y <= 11;
            let index = (y * 16 + x) * 4;
            if inside {
                rgba[index] = if accent { 0xf2 } else { 0x6f };
                rgba[index + 1] = if accent { 0xd2 } else { 0x54 };
                rgba[index + 2] = if accent { 0x72 } else { 0x9f };
                rgba[index + 3] = 0xff;
            }
        }
    }
    Icon::from_rgba(rgba, 16, 16).map_err(|error| error.to_string())
}

fn open_view(cx: &mut App, view: crate::app::ActiveView) {
    crate::app::show_main_window(cx, view);
}

/// 初始化系统托盘图标和右键菜单。
pub fn init(cx: &mut App) {
    let menu = Menu::with_items(&[
        &MenuItem::with_id("inkling-title", "Inkling", false, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id("inkling-archive", "历史归档", true, None),
        &MenuItem::with_id("inkling-stats", "数据统计", true, None),
        &MenuItem::with_id("inkling-settings", "设置", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id("inkling-quit", "退出 Inkling", true, None),
    ])
    .expect("创建系统托盘菜单失败");

    let icon = match tray_icon() {
        Ok(icon) => icon,
        Err(error) => {
            eprintln!("Inkling：生成系统托盘图标失败：{error}");
            return;
        }
    };
    let tray = match TrayIconBuilder::new()
        .with_id("inkling-tray")
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("Inkling")
        .build()
    {
        Ok(tray) => tray,
        Err(error) => {
            eprintln!("Inkling：初始化系统托盘失败：{error}");
            return;
        }
    };
    cx.set_global(TrayGlobal { _icon: tray });

    let receiver = MenuEvent::receiver();
    cx.spawn(async move |cx| loop {
        while let Ok(event) = receiver.try_recv() {
            match event.id().0.as_str() {
                "inkling-archive" => {
                    cx.update(|cx| open_view(cx, crate::app::ActiveView::Notes)).ok();
                }
                "inkling-stats" => {
                    cx.update(|cx| open_view(cx, crate::app::ActiveView::Stats)).ok();
                }
                "inkling-settings" => {
                    cx.update(|cx| open_view(cx, crate::app::ActiveView::Settings)).ok();
                }
                "inkling-quit" => {
                    cx.update(|cx| cx.quit()).ok();
                }
                _ => {}
            }
        }
        cx.background_executor().timer(Duration::from_millis(250)).await;
    })
    .detach();
}
