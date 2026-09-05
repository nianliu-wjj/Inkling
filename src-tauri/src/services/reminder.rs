//! 提醒调度：每 30s 扫描到期提醒实例（幂等抢占），按渠道分别触发。
//!
//! 提醒时刻由「完成时间 - 用户选择的偏移」现算，另加一次到点兜底；
//! 弹窗与邮件各自记账，互不影响（邮件重试不会被弹窗的成功记录挡掉）。
//! `repeat_rule` 在提醒触发后按周期推进 `remind_at` 游标。

use chrono::{DateTime, Duration, Utc};
use std::time::Duration as StdDuration;

use crate::app::state::AppState;
use crate::app::windows;
use crate::domain::models::Todo;
use crate::domain::todo as logic;
use crate::events;
use crate::services::mailer::{self, MailRequest};
use tauri::{AppHandle, Emitter, Manager};

const TICK: StdDuration = StdDuration::from_secs(30);

pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("reminder-scheduler".into())
        .spawn(move || loop {
            std::thread::sleep(TICK);
            if let Err(error) = tick(&app) {
                eprintln!("[reminder] 调度失败: {error}");
            }
        })
        .expect("启动提醒调度线程失败");
}

fn tick(app: &AppHandle) -> Result<(), String> {
    let now = Utc::now();
    // 顺带兜住托盘提示的跨日刷新：日期未变时只是比较一个字符串，开销可忽略。
    crate::app::tray::refresh_tooltip_if_day_changed(app);
    let state = app.state::<AppState>();
    let todos = {
        let store = state.lock_store()?;
        store.list_open_remindable_todos()?
    };
    for todo in todos {
        let Some(due) = logic::parse_time(todo.due_at()) else {
            continue;
        };
        // 跳过未来 1 天以后的事项，降低无效计算；最大偏移正好是 1 天。
        if due > now + Duration::days(1) {
            continue;
        }

        // 一次性 / 重复的额外提醒（提醒卡片的「稍后提醒」写入 remind_at）。
        if let Some(extra) = todo.remind_at().as_deref().and_then(logic::parse_time) {
            if extra <= now {
                fire_channels(app, &todo, "snooze", extra)?;
                finish_or_repeat(app, &todo, extra)?;
            }
        }

        let mut due_fired = false;
        for (when, slot) in logic::reminder_slots(due, *todo.remind_offset_minutes()) {
            if when > now {
                continue;
            }
            fire_channels(app, &todo, slot, when)?;
            if slot == "due" {
                due_fired = true;
            }
        }

        if due_fired {
            finish_or_repeat(app, &todo, due)?;
        }
    }
    Ok(())
}

/// 按待办启用的渠道分别触发一次提醒。
fn fire_channels(
    app: &AppHandle,
    todo: &Todo,
    slot: &str,
    when: DateTime<Utc>,
) -> Result<(), String> {
    if *todo.remind_desktop() {
        fire_desktop(app, todo, slot, when)?;
    }
    if *todo.remind_email() {
        fire_email(app, todo, slot, when)?;
    }
    Ok(())
}

/// 抢占一次提醒实例；返回 true 表示本次由当前调用负责触发。
fn claim(
    app: &AppHandle,
    todo_id: &str,
    slot: &str,
    channel: &str,
    when: DateTime<Utc>,
) -> Result<bool, String> {
    let key = logic::reminder_instance_key(todo_id, slot, channel, when);
    let state = app.state::<AppState>();
    let store = state.lock_store()?;
    store.log_reminder(&key, todo_id)
}

fn fire_desktop(
    app: &AppHandle,
    todo: &Todo,
    slot: &str,
    when: DateTime<Utc>,
) -> Result<(), String> {
    if !claim(app, todo.id(), slot, "desktop", when)? {
        return Ok(());
    }
    eprintln!("[reminder] 弹窗提醒 todo={} slot={slot}", todo.id());
    windows::reminder_show(app, todo.id())?;
    let _ = app.emit(events::REMINDER_FIRED, todo.id().clone());
    Ok(())
}

fn fire_email(app: &AppHandle, todo: &Todo, slot: &str, when: DateTime<Utc>) -> Result<(), String> {
    if !claim(app, todo.id(), slot, "email", when)? {
        return Ok(());
    }
    eprintln!("[reminder] 邮件提醒入队 todo={} slot={slot}", todo.id());
    let heading = if slot == "due" {
        "待办已到完成时间"
    } else {
        "待办即将到期"
    };
    let remark_line = if todo.remark().is_empty() {
        String::new()
    } else {
        format!(
            "备注：{}
",
            todo.remark()
        )
    };
    mailer::enqueue(MailRequest {
        subject: format!("[Inkling] {heading}：{}", todo.content()),
        body: format!(
            "{heading}

内容：{}
完成时间：{}
优先级：{}
{remark_line}",
            todo.content(),
            todo.due_at(),
            todo.priority()
        ),
    });
    Ok(())
}

/// 提醒触发后的收尾：有重复规则则把 `remind_at` 推进到下一周期，
/// 否则清空一次性提醒并抑制后续扫描。
fn finish_or_repeat(app: &AppHandle, todo: &Todo, from: DateTime<Utc>) -> Result<(), String> {
    let state = app.state::<AppState>();
    let store = state.lock_store()?;
    match todo.repeat_rule().as_deref() {
        Some(rule) if logic::repeat_period(rule).is_some() => {
            store.advance_repeat(todo.id(), from, rule)?;
        }
        _ => {
            store.clear_remind_at(todo.id())?;
            store
                .db
                .execute("UPDATE todos SET remind_off=1 WHERE id=?", [&todo.id()])
                .map_err(|e| format!("数据库操作失败: {e}"))?;
        }
    }
    Ok(())
}
