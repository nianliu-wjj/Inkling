//! 启动历史加权（参考 ZeroLaunch `history_booster.rs` 与 `query_affinity.rs`）。
//!
//! 三项习惯分 + 抑制因子 + 查询亲和度：
//! - 历史分 `ln(1 + 总启动次数)` × 0.8；
//! - 近期习惯：最近 7 天按天计数，越近权重越高（k = 1 - 天龄/7）× 1.5；
//! - 短期热度：`1 / (1 + Δt / (decay + 1))`，decay = 10800s × 0.5；
//! - 抑制因子 `(base_score / 15).clamp(0, 1)`：基础匹配分低时习惯加成打折，防高频程序挤占；
//! - 查询亲和度：记录「查询词 → 最终启动项」，同词再输时加 3.0 × 权重 × 时间衰减，15s 冷却防连点刷分。
//!
//! 纯函数 + JSON 落盘；日期以 `YYYY-MM-DD` 字符串传入，便于单测。

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

const HISTORY_WEIGHT: f64 = 0.8;
const RECENT_WEIGHT: f64 = 1.5;
const TEMPORAL_WEIGHT: f64 = 0.5;
const TEMPORAL_DECAY_SECS: f64 = 10800.0;
const AFFINITY_WEIGHT: f64 = 3.0;
/// 亲和度衰减尺度：7 天。
const AFFINITY_DECAY_SECS: f64 = 7.0 * 86400.0;
/// 同一（查询, 启动项）15 秒内重复启动不累计亲和度。
const AFFINITY_COOLDOWN_SECS: i64 = 15;
/// 近期习惯保留的天数。
const RECENT_DAYS: usize = 7;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct History {
    /// 启动项 → 总启动次数。
    counts: HashMap<String, u32>,
    /// 最近 7 天：(日期, 启动项 → 次数)，按日期升序，最多 7 条。
    daily: Vec<(String, HashMap<String, u32>)>,
    /// 启动项 → 最后一次启动的 Unix 秒。
    last_launch: HashMap<String, i64>,
    /// 查询词 → 启动项 → (权重, 最后记录时间)。
    affinity: HashMap<String, HashMap<String, (f64, i64)>>,
}

impl History {
    /// 记录一次启动。`today` 为本地日期键 `YYYY-MM-DD`，`now` 为 Unix 秒。
    pub fn record(&mut self, path: &str, query: &str, today: &str, now: i64) {
        *self.counts.entry(path.to_string()).or_insert(0) += 1;
        self.last_launch.insert(path.to_string(), now);

        // 按天滚动：今天没有条目则追加，超过 7 天丢最旧
        if self.daily.last().map(|(d, _)| d.as_str()) != Some(today) {
            self.daily.push((today.to_string(), HashMap::new()));
            while self.daily.len() > RECENT_DAYS {
                self.daily.remove(0);
            }
        }
        if let Some((_, map)) = self.daily.last_mut() {
            *map.entry(path.to_string()).or_insert(0) += 1;
        }

        // 查询亲和度（空查询不记）
        let query = query.trim().to_lowercase();
        if !query.is_empty() {
            let entry = self
                .affinity
                .entry(query)
                .or_default()
                .entry(path.to_string())
                .or_insert((0.0, 0));
            if now - entry.1 >= AFFINITY_COOLDOWN_SECS {
                entry.0 += 1.0;
                entry.1 = now;
            }
        }
    }

    /// 基础匹配分之上的习惯加成。
    pub fn boost(&self, path: &str, query: &str, base_score: f64, now: i64) -> f64 {
        let history = self
            .counts
            .get(path)
            .map(|c| (*c as f64).ln_1p())
            .unwrap_or(0.0);
        let recent = self.recent_score(path);
        let temporal = self
            .last_launch
            .get(path)
            .map(|last| 1.0 / (1.0 + (now - last).max(0) as f64 / (TEMPORAL_DECAY_SECS + 1.0)))
            .unwrap_or(0.0);
        let suppression = (base_score / 15.0).clamp(0.0, 1.0);
        let habit = suppression
            * (HISTORY_WEIGHT * history + RECENT_WEIGHT * recent + TEMPORAL_WEIGHT * temporal);

        let query = query.trim().to_lowercase();
        let affinity = self
            .affinity
            .get(&query)
            .and_then(|m| m.get(path))
            .map(|(weight, last)| {
                let age = (now - last).max(0) as f64;
                weight * (1.0 / (1.0 + age / AFFINITY_DECAY_SECS))
            })
            .unwrap_or(0.0);
        habit + AFFINITY_WEIGHT * affinity
    }

    /// 纯历史排序用（空查询时）：不带抑制因子，直接三项加权。
    pub fn habit_only(&self, path: &str, now: i64) -> f64 {
        let history = self
            .counts
            .get(path)
            .map(|c| (*c as f64).ln_1p())
            .unwrap_or(0.0);
        let temporal = self
            .last_launch
            .get(path)
            .map(|last| 1.0 / (1.0 + (now - last).max(0) as f64 / (TEMPORAL_DECAY_SECS + 1.0)))
            .unwrap_or(0.0);
        HISTORY_WEIGHT * history
            + RECENT_WEIGHT * self.recent_score(path)
            + TEMPORAL_WEIGHT * temporal
    }

    /// 是否有任何启动记录。
    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    /// 近 7 天加权次数：最新一天 k=1，每老一天权重线性下降 1/7。
    fn recent_score(&self, path: &str) -> f64 {
        let days = self.daily.len();
        self.daily
            .iter()
            .enumerate()
            .map(|(index, (_, map))| {
                let age = (days - 1 - index) as f64;
                let k = 1.0 - age / RECENT_DAYS as f64;
                map.get(path).copied().unwrap_or(0) as f64 * k
            })
            .sum()
    }

    pub fn load(file: &Path) -> Self {
        match std::fs::read_to_string(file) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|error| {
                eprintln!("[launcher] 历史文件损坏，重新开始: {error}");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// 原子写：先写 .tmp 再 rename。
    pub fn save(&self, file: &Path) -> Result<(), String> {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("创建历史目录失败: {e}"))?;
        }
        let tmp = file.with_extension("json.tmp");
        let text = serde_json::to_string(self).map_err(|e| format!("序列化历史失败: {e}"))?;
        std::fs::write(&tmp, text).map_err(|e| format!("写历史文件失败: {e}"))?;
        std::fs::rename(&tmp, file).map_err(|e| format!("提交历史文件失败: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suppression_factor_limits_habit_when_match_is_weak() {
        let mut h = History::default();
        for _ in 0..20 {
            h.record("C:/a.exe", "", "2026-09-08", 1_000_000);
        }
        let strong = h.boost("C:/a.exe", "", 30.0, 1_000_100);
        let weak = h.boost("C:/a.exe", "", 3.0, 1_000_100);
        assert!(strong > weak, "strong={strong} weak={weak}");
        // 抑制因子 = 3/15 = 0.2
        assert!((weak / strong - 0.2).abs() < 1e-9);
    }

    #[test]
    fn daily_window_keeps_only_seven_days() {
        let mut h = History::default();
        for day in 1..=10 {
            h.record("p", "", &format!("2026-09-{day:02}"), day as i64 * 86400);
        }
        assert_eq!(h.daily.len(), 7);
        assert_eq!(h.daily.first().unwrap().0, "2026-09-04");
        assert_eq!(h.daily.last().unwrap().0, "2026-09-10");
    }

    #[test]
    fn affinity_rewards_same_query_and_respects_cooldown() {
        let mut h = History::default();
        h.record("p", "wx", "2026-09-08", 1000);
        h.record("p", "wx", "2026-09-08", 1005); // 冷却期内，不累计
        h.record("p", "wx", "2026-09-08", 1100);
        let with = h.boost("p", "wx", 20.0, 1200);
        let without = h.boost("p", "other", 20.0, 1200);
        assert!(with > without);
        assert_eq!(h.affinity["wx"]["p"].0, 2.0);
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = std::env::temp_dir().join(format!("inkling-launcher-{}", std::process::id()));
        let file = dir.join("history.json");
        let mut h = History::default();
        h.record("p", "q", "2026-09-08", 1);
        h.save(&file).unwrap();
        let loaded = History::load(&file);
        assert_eq!(loaded.counts["p"], 1);
        std::fs::remove_dir_all(&dir).unwrap();
        // 不存在的文件 → 空历史
        assert!(History::load(&file).is_empty());
    }
}
