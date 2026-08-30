//! 统计视图的日期运算与 SQLite 统计数据适配。
//! 业务计数由 `store` 从持久化数据和幂等活动事件中聚合，本模块只负责日期网格与展示模型。

use chrono::Datelike;

crate::accessors! {
    /// 单日活跃度
    #[derive(Clone, Debug)]
    pub struct DayStat {
        /// YYYY-MM-DD
        date: String,
        notes: u32,
        clips: u32,
        todos: u32,
        done: u32,
        overdue: u32,
    }
}

impl DayStat {
    pub(crate) fn from_counts(
        date: impl Into<String>,
        notes: u32,
        clips: u32,
        todos: u32,
        done: u32,
        overdue: u32,
    ) -> Self {
        Self {
            date: date.into(),
            notes,
            clips,
            todos,
            done,
            overdue,
        }
    }

    pub fn total(&self) -> u32 {
        self.notes() + self.clips() + self.todos()
    }
}

/// 读取指定日期的真实业务统计。
pub fn day_stat(cx: &mut gpui::App, date: &str) -> DayStat {
    crate::store::daily_stat(cx, date)
}

/// 从今天往前（含今天）取 n 天的真实业务统计。
pub fn last_days(cx: &mut gpui::App, n: u32) -> Vec<DayStat> {
    let today = days_from_today();
    let dates: Vec<String> = (0..n)
        .rev()
        .map(|i| civil_from_days(today - i as i64))
        .collect();
    crate::store::daily_stats(cx, &dates)
}

/// 某年某月的日期网格（周一对齐），None 为前置空位
pub fn month_grid(year: i64, month: u32) -> (Vec<Option<String>>, u32) {
    let days_in_month = days_in_month(year, month);
    let first_days = days_from_civil(year, month, 1);
    let lead = (mod_i64(first_days + 3, 7)) as u32; // 1970-01-01 是周四
    let cells = (lead + days_in_month as u32 + 6) / 7 * 7;
    let mut grid = Vec::with_capacity(cells as usize);
    for i in 0..cells {
        let day = i as i64 - lead as i64 + 1;
        if day >= 1 && day <= days_in_month as i64 {
            grid.push(Some(civil_from_days(first_days + day - 1)));
        } else {
            grid.push(None);
        }
    }
    (grid, days_in_month)
}

/// 当月网格
/// 返回日期对应的中文星期名称。
pub fn weekday_name(date: &str) -> &'static str {
    let parts: Vec<u32> = date
        .split('-')
        .filter_map(|part| part.parse::<u32>().ok())
        .collect();
    if parts.len() != 3 {
        return "星期一";
    }
    // 1970-01-01 为星期四；余数 0..6 对应周一到周日。
    let monday_index = (days_from_civil(parts[0] as i64, parts[1], parts[2]) + 3)
        .rem_euclid(7);
    ["星期一", "星期二", "星期三", "星期四", "星期五", "星期六", "星期日"]
        [monday_index as usize]
}

pub fn current_month_grid() -> (Vec<Option<String>>, u32, i64, u32) {
    let today = days_from_today();
    let (y, m, _) = civil_ymd(today);
    let (grid, days) = month_grid(y, m);
    (grid, days, y, m)
}

// ── 日期运算（无第三方依赖） ─────────────────────

/// 今天距 1970-01-01 的天数
pub fn days_from_today() -> i64 {
    let today = chrono::Local::now().date_naive();
    days_from_civil(today.year() as i64, today.month(), today.day())
}

/// 天数 → YYYY-MM-DD（Howard Hinnant civil_from_days）
pub fn civil_from_days(z: i64) -> String {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// (年, 月) → 距 1970-01-01 的天数（civil_from_days 的逆）
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
            if leap {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn civil_ymd(days: i64) -> (i64, u32, u32) {
    let s = civil_from_days(days);
    let y: i64 = s[0..4].parse().unwrap_or(1970);
    let m: u32 = s[5..7].parse().unwrap_or(1);
    let d: u32 = s[8..10].parse().unwrap_or(1);
    (y, m, d)
}

pub fn today_str() -> String {
    civil_from_days(days_from_today())
}

fn mod_i64(a: i64, n: i64) -> i64 {
    a.rem_euclid(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_date_round_trip() {
        for (year, month, day) in [(1970, 1, 1), (2000, 2, 29), (2026, 8, 30)] {
            let days = days_from_civil(year, month, day);
            assert_eq!(
                civil_from_days(days),
                format!("{year:04}-{month:02}-{day:02}")
            );
        }
    }

    #[test]
    fn month_grid_is_week_aligned() {
        let (grid, days) = month_grid(2026, 8);
        assert_eq!(days, 31);
        assert_eq!(grid.len() % 7, 0);
        assert_eq!(grid.iter().flatten().count(), 31);
        assert_eq!(
            grid.iter().flatten().next().map(String::as_str),
            Some("2026-08-01")
        );
        assert_eq!(
            grid.iter().flatten().last().map(String::as_str),
            Some("2026-08-31")
        );
    }
}
