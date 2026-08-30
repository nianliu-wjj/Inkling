//! 统计数据：以日期字符串为种子的确定性伪随机日活跃度（笔记 / 粘贴板 / 待办）。
//! 数据层接入 SQLite 前，用稳定的示例数据驱动热力图与趋势图。

/// 单日活跃度
#[derive(Clone, Debug)]
pub struct DayStat {
    /// YYYY-MM-DD
    pub date: String,
    pub notes: u32,
    pub clips: u32,
    pub todos: u32,
    pub done: u32,
    pub overdue: u32,
}

impl DayStat {
    pub fn total(&self) -> u32 {
        self.notes + self.clips + self.todos
    }
}

/// FNV-1a 哈希：日期字符串 → 种子
fn fnv1a(s: &str) -> u32 {
    let mut hash: u32 = 0x811C9DC5;
    for b in s.bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

/// mulberry32 伪随机数
struct Rng(u32);

impl Rng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(0x6D2B79F5);
        let mut t = self.0;
        t = t.wrapping_mul(t ^ (t >> 15));
        t = t.wrapping_mul(t ^ (t >> 7));
        t ^= t >> 16;
        t
    }
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.next() % (hi - lo + 1)
    }
    fn chance(&mut self, pct: u32) -> bool {
        self.next() % 100 < pct
    }
}

/// 由日期（YYYY-MM-DD）生成当日活跃度（确定性）
pub fn day_stat(date: &str) -> DayStat {
    let mut rng = Rng(fnv1a(date));
    let weekend = is_weekend(date);
    let notes = if rng.chance(18) { 0 } else { rng.range(1, if weekend { 3 } else { 7 }) };
    let clips = if rng.chance(12) { 0 } else { rng.range(1, if weekend { 6 } else { 14 }) };
    let todos = if rng.chance(30) { 0 } else { rng.range(1, 5) };
    let open = if todos > 0 && rng.chance(38) { ((todos / 2).max(1)).min(todos) } else { 0 };
    let done = todos - open;
    let today = today_str();
    // 过去日期里未完成的部分记为逾期；当天不算逾期
    let overdue = if *date < *today { open } else { 0 };
    DayStat { date: date.into(), notes, clips, todos, done, overdue }
}

/// 从今天往前（含今天）取 n 天的活跃度
pub fn last_days(n: u32) -> Vec<DayStat> {
    let mut out = Vec::with_capacity(n as usize);
    let today = days_from_today();
    for i in (0..n).rev() {
        let date = civil_from_days(today - i as i64);
        out.push(day_stat(&date));
    }
    out
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
pub fn current_month_grid() -> (Vec<Option<String>>, u32, i64, u32) {
    let today = days_from_today();
    let (y, m, _) = civil_ymd(today);
    let (grid, days) = month_grid(y, m);
    (grid, days, y, m)
}

// ── 日期运算（无第三方依赖） ─────────────────────

/// 今天距 1970-01-01 的天数
pub fn days_from_today() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    secs.div_euclid(86400)
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
            if leap { 29 } else { 28 }
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

fn is_weekend(date: &str) -> bool {
    // 解析年月日 → 天数 → 星期
    let y: i64 = date[0..4].parse().unwrap_or(1970);
    let m: u32 = date[5..7].parse().unwrap_or(1);
    let d: u32 = date[8..10].parse().unwrap_or(1);
    let dow = mod_i64(days_from_civil(y, m, d) + 3, 7); // 0=周日
    dow == 0 || dow == 6
}

fn mod_i64(a: i64, n: i64) -> i64 {
    a.rem_euclid(n)
}
