//! 搜索：并行打分 + 历史加权 + Top-K。
//!
//! 空查询：按纯历史习惯分排序（用户刚呼出窗口时列出常用项）；无历史时给前 K 个程序。
//! 非空：rayon 并行对每个候选取最佳关键字分（不满足容错门槛的候选直接剔除），
//! 加历史加成后 `select_nth_unstable_by` 取 Top-K 再排序——不对全量结果排序。

use rayon::prelude::*;

use super::history::History;
use super::model::{Candidate, Hit, Kind};
use super::score;

/// 查询归一化：小写、连续空白折叠、去首尾空白。
pub fn normalize_query(query: &str) -> String {
    query
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn search(
    candidates: &[Candidate],
    history: &History,
    query: &str,
    top_k: usize,
    now: i64,
) -> Vec<Hit> {
    if candidates.is_empty() || top_k == 0 {
        return Vec::new();
    }
    if query.is_empty() {
        return empty_query(candidates, history, top_k, now);
    }

    // (总分, 候选下标, 命中关键字下标)
    let mut scored: Vec<(f64, usize, usize)> = candidates
        .par_iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            let (base, kw_index) = score::best_score(&candidate.keywords, query, candidate.bias)?;
            let total = base + history.boost(&candidate.path, query, base, now);
            Some((total, index, kw_index))
        })
        .collect();

    if scored.is_empty() {
        return Vec::new();
    }
    let k = top_k.min(scored.len());
    if scored.len() > k {
        // 第 k 名之前的都是更高分（降序）；只保证前 k 个是全局最高，内部无序
        scored.select_nth_unstable_by(k - 1, |a, b| b.0.total_cmp(&a.0));
        scored.truncate(k);
    }
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored
        .into_iter()
        .map(|(total, index, kw_index)| {
            let c = &candidates[index];
            Hit {
                id: c.id,
                kind: c.kind,
                name: c.name.clone(),
                path: c.path.clone(),
                score: total,
                matched_keyword: c.keywords.get(kw_index).cloned().unwrap_or_default(),
            }
        })
        .collect()
}

/// 空查询：有历史按习惯分，否则前 K 个程序（跳过内置命令）。
fn empty_query(candidates: &[Candidate], history: &History, top_k: usize, now: i64) -> Vec<Hit> {
    let mut ranked: Vec<(f64, usize)> = if history.is_empty() {
        Vec::new()
    } else {
        candidates
            .iter()
            .enumerate()
            .filter_map(|(index, c)| {
                let habit = history.habit_only(&c.path, now);
                (habit > 0.0).then_some((habit, index))
            })
            .collect()
    };
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    ranked.truncate(top_k);
    if ranked.is_empty() {
        ranked = candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c.kind, Kind::App | Kind::Uwp))
            .take(top_k)
            .map(|(index, _)| (0.0, index))
            .collect();
    }
    ranked
        .into_iter()
        .map(|(score, index)| {
            let c = &candidates[index];
            Hit {
                id: c.id,
                kind: c.kind,
                name: c.name.clone(),
                path: c.path.clone(),
                score,
                matched_keyword: String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::keyword;
    use super::*;

    fn candidate(id: u32, name: &str, path: &str, kind: Kind) -> Candidate {
        Candidate {
            id,
            kind,
            name: name.into(),
            path: path.into(),
            keywords: keyword::generate(name),
            bias: 0.0,
        }
    }

    fn fixture() -> Vec<Candidate> {
        vec![
            candidate(0, "微信", "C:/WeChat.exe", Kind::App),
            candidate(1, "微信 Uninstall", "C:/Uninstall.exe", Kind::App),
            candidate(2, "Visual Studio Code", "C:/Code.exe", Kind::App),
            candidate(3, "Notepad++ (64-bit)", "C:/notepad++.exe", Kind::App),
            candidate(4, "Downloads", "C:/Users/x/Downloads", Kind::Folder),
        ]
    }

    #[test]
    fn pinyin_initials_and_typos_find_wechat_first() {
        let c = fixture();
        let h = History::default();
        for q in ["wx", "weixin", "weixn", "微信"] {
            let hits = search(&c, &h, &normalize_query(q), 9, 0);
            assert_eq!(
                hits.first().map(|h| h.id),
                Some(0),
                "query={q} hits={hits:?}"
            );
        }
        let hits = search(&c, &h, "vsc", 9, 0);
        assert_eq!(hits.first().map(|h| h.id), Some(2));
        let hits = search(&c, &h, "notepad", 9, 0);
        assert_eq!(hits.first().map(|h| h.id), Some(3));
    }

    #[test]
    fn top_k_and_no_match() {
        let c = fixture();
        let h = History::default();
        // 查询长度需超过最长关键字 + 容错，才保证全部候选被长度门槛剔除。
        assert!(search(&c, &h, &"z".repeat(40), 9, 0).is_empty());
        assert_eq!(search(&c, &h, "e", 2, 0).len(), 2);
    }

    #[test]
    fn history_lifts_frequent_item_but_not_when_match_is_weak() {
        let c = fixture();
        let mut h = History::default();
        for _ in 0..30 {
            h.record("C:/Code.exe", "", "2026-09-08", 1000);
        }
        // 精确搜微信：VSCode 再常用也不能压过真正匹配的微信
        let hits = search(&c, &h, "weixin", 9, 1100);
        assert_eq!(hits.first().map(|h| h.id), Some(0));
        // 空查询：按习惯，VSCode 排第一
        let hits = search(&c, &h, "", 9, 1100);
        assert_eq!(hits.first().map(|h| h.id), Some(2));
    }

    #[test]
    fn empty_query_without_history_lists_apps() {
        let c = fixture();
        let hits = search(&c, &History::default(), "", 3, 0);
        assert_eq!(hits.len(), 3);
        assert!(hits.iter().all(|h| matches!(h.kind, Kind::App)));
    }
}
