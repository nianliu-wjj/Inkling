//! 待办领域逻辑：字段校验、稳定排序、逾期判定与提醒槽位计算（零 Tauri 依赖，可单测）。

use chrono::{DateTime, Duration, Utc};

/// 优先级排序权重：高 → 中 → 低。
pub fn priority_weight(priority: &str) -> i32 {
    match priority {
        "high" => 0,
        "medium" => 1,
        "low" => 2,
        _ => 3,
    }
}

pub fn is_valid_priority(priority: &str) -> bool {
    matches!(priority, "high" | "medium" | "low")
}

pub fn is_valid_repeat_rule(rule: &str) -> bool {
    matches!(rule, "daily" | "weekly")
}

/// 解析 RFC3339 时间，兼容无时区的时间戳（按 UTC 兜底）。
pub fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

/// 判定未完成事项是否逾期：完成时刻已过即逾期。
pub fn is_overdue(due_at: &str, status: &str, now: DateTime<Utc>) -> bool {
    status == "open" && parse_time(due_at).is_some_and(|due| due < now)
}

/// 校验待办字段（内容 / 优先级 / 备注 / 标签）。
pub fn validate_fields(
    content: &str,
    priority: &str,
    remark: &str,
    tags: &[String],
) -> Result<(), String> {
    if content.trim().is_empty() {
        return Err("待办内容不能为空".into());
    }
    if !is_valid_priority(priority) {
        return Err("无效的优先级".into());
    }
    if remark.chars().count() > 200 {
        return Err("待办备注最多 200 个字".into());
    }
    crate::domain::clipboard::validate_todo_tags(tags)?;
    Ok(())
}

/// 校验父子关系约束：父级必须是顶级待办、子任务数量 ≤5、子任务完成时间不得晚于父级
/// （父级已完成时豁免——已完成父级新增子任务是唯一例外）。
pub fn validate_parent(
    parent_status: &str,
    parent_due_at: &str,
    parent_parent_id: Option<&str>,
    child_due_at: &str,
    existing_children: i64,
    is_new: bool,
) -> Result<(), String> {
    if parent_parent_id.is_some() {
        return Err("子任务不可继续嵌套".into());
    }
    if is_new && existing_children >= 5 {
        return Err("一个顶级待办最多只能有 5 个子任务".into());
    }
    let child_due = parse_time(child_due_at).ok_or("完成时间格式无效")?;
    let parent_due = parse_time(parent_due_at).ok_or("父待办完成时间无效")?;
    if parent_status == "open" && child_due > parent_due {
        return Err("子任务的完成时间不能晚于父待办".into());
    }
    Ok(())
}

/// 可选的提醒偏移档位（分钟）。
///
/// 与前端下拉框选项一一对应，前后端都以此为准：
/// 15 分钟 / 30 分钟 / 1 小时 / 3 小时 / 6 小时 / 1 天。
pub const REMIND_OFFSETS: [i64; 6] = [15, 30, 60, 180, 360, 1440];

/// 偏移值是否合法。`None`（不提醒）由调用方单独处理，不走这里。
pub fn is_valid_offset(minutes: i64) -> bool {
    REMIND_OFFSETS.contains(&minutes)
}

/// 某待办的提醒槽位：偏移提醒一次 + 到点兜底一次。
///
/// 兜底的存在是为了让「前 1 天」这类大偏移不至于错过到期本身。
/// `offset_minutes` 为 `None` 时表示不提醒，返回空列表。
pub fn reminder_slots(
    due_at: DateTime<Utc>,
    offset_minutes: Option<i64>,
) -> Vec<(DateTime<Utc>, &'static str)> {
    let Some(minutes) = offset_minutes else {
        return Vec::new();
    };
    vec![
        (due_at - Duration::minutes(minutes), "offset"),
        (due_at, "due"),
    ]
}

/// 重复提醒的周期。
pub fn repeat_period(rule: &str) -> Option<Duration> {
    match rule {
        "daily" => Some(Duration::days(1)),
        "weekly" => Some(Duration::weeks(1)),
        _ => None,
    }
}

/// 提醒实例的幂等键：待办 + 槽位 + 渠道 + 触发时刻毫秒。
///
/// 含渠道维度，是为了让弹窗与邮件各自独立记账：邮件发送失败重试时，
/// 若与弹窗共用一条记录，会被弹窗那次成功的记录挡掉而永不重发。
/// 后续接入新渠道（如 QQ 机器人）也不必再改键的结构。
pub fn reminder_instance_key(
    todo_id: &str,
    slot: &str,
    channel: &str,
    when: DateTime<Utc>,
) -> String {
    format!("{todo_id}|{slot}|{channel}|{}", when.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap()
    }

    #[test]
    fn overdue_depends_on_status_and_time() {
        let now = t(2026, 8, 30, 12, 0);
        assert!(is_overdue("2026-08-30T10:00:00+00:00", "open", now));
        assert!(!is_overdue("2026-08-30T10:00:00+00:00", "done", now));
        assert!(!is_overdue("2026-08-30T13:00:00+00:00", "open", now));
    }

    #[test]
    fn parent_validation_blocks_deep_nesting_and_late_children() {
        assert!(validate_parent(
            "open",
            "2026-08-30T18:00:00+00:00",
            Some("x"),
            "2026-08-30T19:00:00+00:00",
            0,
            true
        )
        .is_err());
        assert!(validate_parent(
            "open",
            "2026-08-30T18:00:00+00:00",
            None,
            "2026-08-30T19:00:00+00:00",
            0,
            true
        )
        .is_err());
        assert!(validate_parent(
            "open",
            "2026-08-30T18:00:00+00:00",
            None,
            "2026-08-30T17:00:00+00:00",
            0,
            true
        )
        .is_ok());
        // 已完成父级豁免“不得晚于父级”约束
        assert!(validate_parent(
            "done",
            "2026-08-30T10:00:00+00:00",
            None,
            "2026-08-30T19:00:00+00:00",
            0,
            true
        )
        .is_ok());
        assert!(validate_parent(
            "open",
            "2026-08-30T18:00:00+00:00",
            None,
            "2026-08-30T17:00:00+00:00",
            5,
            true
        )
        .is_err());
        assert!(validate_parent(
            "open",
            "2026-08-30T18:00:00+00:00",
            None,
            "2026-08-30T17:00:00+00:00",
            5,
            false
        )
        .is_ok());
    }

    #[test]
    fn reminder_slots_use_offset_plus_due() {
        let due = t(2026, 8, 30, 18, 0);
        let slots = reminder_slots(due, Some(15));
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].0, due - Duration::minutes(15));
        assert_eq!(slots[0].1, "offset");
        assert_eq!(slots[1].0, due);
        assert_eq!(slots[1].1, "due");
    }

    #[test]
    fn reminder_slots_empty_when_offset_missing() {
        let due = t(2026, 8, 30, 18, 0);
        assert!(reminder_slots(due, None).is_empty());
    }

    #[test]
    fn reminder_key_separates_channels() {
        let when = t(2026, 8, 30, 18, 0);
        let desktop = reminder_instance_key("t1", "due", "desktop", when);
        let email = reminder_instance_key("t1", "due", "email", when);
        assert_ne!(desktop, email);
        assert_eq!(desktop, reminder_instance_key("t1", "due", "desktop", when));
    }

    #[test]
    fn only_listed_offsets_are_valid() {
        assert!(is_valid_offset(15));
        assert!(is_valid_offset(1440));
        assert!(!is_valid_offset(20));
        assert!(!is_valid_offset(0));
    }

    #[test]
    fn repeat_period_matches_rules() {
        assert_eq!(repeat_period("daily"), Some(Duration::days(1)));
        assert_eq!(repeat_period("weekly"), Some(Duration::weeks(1)));
        assert_eq!(repeat_period(""), None);
    }
}
