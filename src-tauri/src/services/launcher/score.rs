//! 标准检索引擎（参考 ZeroLaunch `standard_search_model.rs`，逐项移植）。
//!
//! 对候选的每个关键字计算得分，取最高者：
//! 1. 容错门槛：关键字 ≤2 字符严格匹配，否则允许输入比关键字多 1 字符；
//! 2. 编辑距离基础分：关键字任意后缀与输入的最短编辑距离 d，value = 1 - d/n，
//!    分 = 3·log2(n+1)·e^(3·value-2)；
//! 3. 长度比率调整（乘）：3·log2(min(1, len_in/len_kw) + 1)；
//! 4. 溢出惩罚（乘）：输入长于关键字时 max(0.7, 1 - 0.3·overflow_ratio)；
//! 5. 子集匹配分（加）：输入字符命中关键字字符多重集的个数；
//! 6. KMP 分（加）：公共前缀长度 + （关键字包含输入时）输入长度；
//! 7. 固定偏移（加）。
//!
//! 纯函数，无 IO。

/// 对单个关键字打分；不满足容错门槛返回 None。
pub fn score_keyword(keyword: &str, input: &str) -> Option<f64> {
    let input_len = input.chars().count();
    let target_len = keyword.chars().count();
    if input_len == 0 || target_len == 0 {
        return None;
    }
    let tolerance = if target_len <= 2 { 0 } else { 1 };
    if target_len + tolerance < input_len {
        return None;
    }

    // 1. 编辑距离基础分
    let mut score = shortest_edit_dis(keyword, input);

    // 2. 长度比率调整
    let ratio = if input_len > target_len {
        1.0
    } else {
        input_len as f64 / target_len as f64
    };
    score *= adjust_score_log2(ratio);

    // 3. 溢出惩罚
    if input_len > target_len {
        let overflow_ratio = (input_len as f64 - target_len as f64) / target_len as f64;
        score *= (1.0 - overflow_ratio * 0.3).max(0.7);
    }

    // 4. 子集匹配分
    score += subset_dis(keyword, input);

    // 5. KMP 首字符 + 子串匹配分
    score += kmp(keyword, input);

    Some(score)
}

/// 候选的最终引擎分：所有关键字取最高 + 固定偏移；全部不匹配时返回 None。
pub fn best_score(keywords: &[String], input: &str, bias: f64) -> Option<(f64, usize)> {
    let mut best: Option<(f64, usize)> = None;
    for (index, kw) in keywords.iter().enumerate() {
        if let Some(s) = score_keyword(kw, input) {
            let s = s + bias;
            if best.map(|(b, _)| s > b).unwrap_or(true) {
                best = Some((s, index));
            }
        }
    }
    best
}

/// log2 权重映射：3·log2(x+1)。
fn adjust_score_log2(origin: f64) -> f64 {
    3.0 * (origin + 1.0).log2()
}

/// 关键字任意后缀与输入的最短编辑距离，映射为分数（距离越小越高）。
fn shortest_edit_dis(compare: &str, input: &str) -> f64 {
    let compare_chars: Vec<char> = compare.chars().collect();
    let input_chars: Vec<char> = input.chars().collect();
    let m = compare_chars.len();
    let n = input_chars.len();
    if n == 0 {
        return 1.0;
    }
    let mut prev: Vec<i32> = (0..=n as i32).collect();
    let mut current = vec![0i32; n + 1];
    let mut min_operations = n as i32;
    for i in 1..=m {
        // current[0] = 0：允许从 compare 的任意位置开始匹配（即取后缀）。
        current[0] = 0;
        for j in 1..=n {
            let cost = if compare_chars[i - 1] == input_chars[j - 1] {
                0
            } else {
                1
            };
            current[j] = (prev[j - 1] + cost)
                .min(prev[j] + 1)
                .min(current[j - 1] + 1);
        }
        min_operations = min_operations.min(current[n]);
        std::mem::swap(&mut prev, &mut current);
    }
    let value = 1.0 - (min_operations as f64 / n as f64);
    adjust_score_log2(n as f64) * (3.0 * value - 2.0).exp()
}

/// 子集匹配：输入字符在关键字字符多重集中命中的个数。
fn subset_dis(compare: &str, input: &str) -> f64 {
    let mut counts = std::collections::HashMap::new();
    for c in compare.chars() {
        *counts.entry(c).or_insert(0i32) += 1;
    }
    let mut hits = 0;
    for c in input.chars() {
        if let Some(count) = counts.get_mut(&c) {
            if *count > 0 {
                hits += 1;
                *count -= 1;
            }
        }
    }
    hits as f64
}

/// 公共前缀长度 + 子串包含加成。
fn kmp(compare: &str, input: &str) -> f64 {
    let mut ret = 0.0;
    for (a, b) in compare.chars().zip(input.chars()) {
        if a == b {
            ret += 1.0;
        } else {
            break;
        }
    }
    if compare.contains(input) {
        ret += input.chars().count() as f64;
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match_beats_partial() {
        let exact = score_keyword("weixin", "weixin").unwrap();
        let partial = score_keyword("weixin", "wei").unwrap();
        assert!(exact > partial, "exact={exact} partial={partial}");
    }

    #[test]
    fn typo_within_tolerance_still_matches() {
        // 少打一个字母：weixn → weixin
        let typo = score_keyword("weixin", "weixn").unwrap();
        let wrong = score_keyword("weixin", "zzzzz").unwrap();
        assert!(typo > wrong);
        // 多打一个字符（容错 1）仍能匹配
        assert!(score_keyword("weixin", "weixinn").is_some());
        // 多打两个字符超出容错
        assert!(score_keyword("weixin", "weixinnn").is_none());
    }

    #[test]
    fn short_keyword_is_strict() {
        // 关键字 ≤2 字符不允许输入更长
        assert!(score_keyword("wx", "wxx").is_none());
        assert!(score_keyword("wx", "wx").is_some());
    }

    #[test]
    fn prefix_and_substring_get_bonus() {
        let prefix = score_keyword("notepad", "note").unwrap();
        let middle = score_keyword("notepad", "epad").unwrap();
        assert!(prefix > middle, "prefix={prefix} middle={middle}");
    }

    #[test]
    fn best_score_picks_highest_keyword_and_adds_bias() {
        let kws = vec![
            "weixin".to_string(),
            "wx".to_string(),
            "wei xin".to_string(),
        ];
        let (score, index) = best_score(&kws, "wx", 0.0).unwrap();
        assert_eq!(index, 1, "wx 应命中缩写关键字");
        let (biased, _) = best_score(&kws, "wx", 5.0).unwrap();
        assert!((biased - score - 5.0).abs() < 1e-9);
        assert!(best_score(&kws, "zzzzzzzzzzzz", 0.0).is_none());
    }

    #[test]
    fn uninstall_variant_ranks_below_main_program() {
        let main = best_score(&["weixin".to_string()], "weixin", 0.0)
            .unwrap()
            .0;
        let uninstall = best_score(&["weixin uninstall".to_string()], "weixin", 0.0)
            .unwrap()
            .0;
        assert!(main > uninstall);
    }
}
