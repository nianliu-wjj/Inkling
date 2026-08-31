//! 提醒调度：每 30s 扫描到期提醒实例（幂等抢占），创建右上角提醒卡片窗口。
//!
//! 默认提醒计划（未设置 remind_at）：完成时间前 30 分钟 / 前 5 分钟 / 到点；
//! 设置了 remind_at：仅该时刻一次；repeat_rule 在提醒关闭或触发后按周期推进。

use chrono::{DateTime, Duration, Utc};
use std::time::Duration as StdDuration;

use crate::app::state::AppState;
use crate::app::windows;
use crate::domain::todo as logic;
use crate::events;
use tauri::{AppHandle, Emitter};

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
    let state = app.state::<AppState>();
    let todos = {
        let store = state.lock_store()?;
        store.list_open_remindable_todos()?
    };
    for todo in todos {
        let Some(due) = logic::parse_time(&todo.due_at) else { continue };
        // 跳过未来 1 天以后的事项，降低无效计算。
        if due > now + Duration::days(1) && todo.remind_at.is_none() {
            continue;
        }
        let mut due_fired = false;
        if let Some(remind_at) = todo.remind_at.as_deref().and_then(logic::parse_time) {
            if remind_at <= now {
                let key = logic::reminder_instance_key(&todo.id, "custom", remind_at);
                fire_or_repeat(app, &todo.id, &key, remind_at, todo.repeat_rule.as_deref())?;
            }
            continue;
        }
        for (when, slot) in logic::default_reminder_slots(due) {
            if when <= now {
                let key = logic::reminder_instance_key(&todo.id, slot, when);
                let store = state.lock_store()?;
                if store.reminder_fired(&key)? {
                    if slot == "due" {
                        due_fired = true;
                    }
                    continue;
                }
                drop(store);
                fire(app, &todo.id, &key)?;
                if slot == "due" {
                    due_fired = true;
                }
            }
        }
        if due_fired {
            // 到点提醒已触发：默认计划结束，标记 remind_off 抑制后续扫描。
            let store = state.lock_store()?;
            store
                .db
                .execute("UPDATE todos SET remind_off=1 WHERE id=?", [&todo.id])
                .map_err(|e| format!("数据库操作失败: {e}"))?;
        }
    }
    Ok(())
}

fn fire(app: &AppHandle, todo_id: &str, key: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let store = state.lock_store()?;
    if !store.log_reminder(key, todo_id)? {
        return Ok(());
    }
    drop(store);
    windows::reminder_show(app, todo_id)?;
    let _ = app.emit(events::REMINDER_FIRED, todo_id.to_string());
    Ok(())
}

/// 自定义提醒触发：重复规则按周期推进 remind_at；一次性提醒关闭抑制。
fn fire_or_repeat(
    app: &AppHandle,
    todo_id: &str,
    key: &str,
    from: DateTime<Utc>,
    rule: Option<&str>,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let store = state.lock_store()?;
        if store.reminder_fired(key)? {
            return Ok(());
        }
    }
    match rule {
        Some(rule) if logic::repeat_period(rule).is_some() => {
            // 重复提醒：先登记实例，再推进下一次时间，随后弹出本次提醒。
            fire(app, todo_id, key)?;
            let store = state.lock_store()?;
            store.advance_repeat(todo_id, from, rule)?;
        }
        _ => {
            fire(app, todo_id, key)?;
            let store = state.lock_store()?;
            store
                .db
                .execute("UPDATE todos SET remind_off=1 WHERE id=?", [todo_id])
                .map_err(|e| format!("数据库操作失败: {e}"))?;
        }
    }
    Ok(())
}
