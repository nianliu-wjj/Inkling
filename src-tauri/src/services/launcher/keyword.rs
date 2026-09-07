//! 关键字生成管道（参考 ZeroLaunch `keyword_optimizer/`）。
//!
//! 给一个候选名生成多组小写「可搜关键字」：归一化原名 → 去版本号 → 去符号 / 去空格
//! → 拼音（连续汉字空格分隔；多音字取前两种读音做有限组合）→ 拼音首字母 → 大写缩写。
//! 搜索时逐关键字打分取最高。纯函数，无 IO；单测覆盖中英混排、版本号、缩写、多音字。

use pinyin::{ToPinyin, ToPinyinMulti};

/// 多音字组合上限：超过就只取每字首读音，避免关键字数量爆炸。
const MAX_PINYIN_COMBOS: usize = 4;

/// 生成候选名的全部关键字（小写、去重、保持生成顺序）。
pub fn generate(name: &str) -> Vec<String> {
    let base = normalize_spaces(&name.trim().to_lowercase());
    if base.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<String> = Vec::new();
    let push = |s: String, out: &mut Vec<String>| {
        let s = s.trim().to_string();
        if !s.is_empty() && !out.contains(&s) {
            out.push(s);
        }
    };

    // 第一层：原名与去版本号，二者都再派生「去符号」「去空格」两种形态。
    let no_version = normalize_spaces(&remove_version_number(&base));
    let mut layer1 = vec![base.clone()];
    if no_version != base {
        layer1.push(no_version);
    }
    for kw in &layer1 {
        push(kw.clone(), &mut out);
        push(remove_symbols(kw), &mut out);
        push(remove_spaces(kw), &mut out);
        push(remove_spaces(&remove_symbols(kw)), &mut out);
    }

    // 第二层：拼音（只对含汉字的关键字）。「wei xin」与「weixin」都保留。
    let mut pinyin_outputs: Vec<String> = Vec::new();
    for kw in layer1.iter() {
        if !kw.chars().any(is_han) {
            continue;
        }
        for spaced in to_pinyin_variants(kw) {
            pinyin_outputs.push(spaced.clone());
            push(spaced.clone(), &mut out);
            push(remove_spaces(&remove_symbols(&spaced)), &mut out);
        }
    }

    // 第三层：首字母缩写——消费拼音输出（微信→wei xin→wx）或多词英文名（visual studio code→vsc）。
    for source in pinyin_outputs.iter().chain(layer1.iter()) {
        let letters = first_letters(source);
        if letters.chars().count() >= 2 && letters != *source {
            push(letters, &mut out);
        }
    }

    // 第四层：纯 ASCII 名字的大写字母缩写（VSCode→vsc）。基于原始大小写。
    let acronym = upper_case_acronym(name);
    if acronym.chars().count() >= 2 {
        push(acronym, &mut out);
    }
    out
}

/// 是否汉字（CJK 统一表意文字基本区与扩展 A）。
fn is_han(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}')
}

/// 连续空白折叠为单个空格并去首尾空白。
fn normalize_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 去掉全部非字母数字字符（保留汉字——它们是 alphanumeric）。
fn remove_symbols(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
}

fn remove_spaces(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// 词是否像版本号 / 架构标记：`1.2.3`、`v2`、`2024`、`x64`、`64-bit`、`(64-bit)`。
fn is_version_token(token: &str) -> bool {
    let t = token
        .trim_matches(|c: char| matches!(c, '(' | ')' | '（' | '）' | '[' | ']'))
        .trim_start_matches('v');
    if t.is_empty() {
        return false;
    }
    if matches!(
        t,
        "x64" | "x86" | "64-bit" | "32-bit" | "64位" | "32位" | "amd64" | "arm64"
    ) {
        return true;
    }
    // 纯数字与点：1 / 1.2 / 2024 / 1.2.3
    t.chars().all(|c| c.is_ascii_digit() || c == '.') && t.chars().any(|c| c.is_ascii_digit())
}

/// 去掉版本号词与括号内的版本/架构标记。
fn remove_version_number(s: &str) -> String {
    s.split_whitespace()
        .filter(|token| !is_version_token(token))
        .collect::<Vec<_>>()
        .join(" ")
}

/// 一个字符在拼音串里的片段：汉字给出 1–2 种读音，非汉字原样保留。
enum Piece {
    Han(Vec<String>),
    Other(char),
}

/// 汉字→无调拼音，连续汉字之间加空格，非汉字原样保留（前后补空格隔开）。
///
/// 多音字：取前两种读音；全部组合数 ≤ MAX_PINYIN_COMBOS 时枚举所有组合
/// （音乐 → yin yue / yin le 都能搜到），否则只取每字首读音。
fn to_pinyin_variants(s: &str) -> Vec<String> {
    let pieces: Vec<Piece> = s
        .chars()
        .map(|c| {
            if is_han(c) {
                let readings: Vec<String> = c
                    .to_pinyin_multi()
                    .map(|multi| {
                        multi
                            .into_iter()
                            .map(|p| p.plain().to_string())
                            .take(2)
                            .collect()
                    })
                    .unwrap_or_default();
                if readings.is_empty() {
                    // 字典没有的汉字原样保留
                    Piece::Other(c)
                } else {
                    Piece::Han(readings)
                }
            } else {
                Piece::Other(c)
            }
        })
        .collect();

    let combos: usize = pieces
        .iter()
        .map(|p| match p {
            Piece::Han(r) => r.len(),
            Piece::Other(_) => 1,
        })
        .product();
    let use_all = combos <= MAX_PINYIN_COMBOS;

    // 逐字展开组合（组合数很小，直接笛卡尔积）。
    let mut variants: Vec<String> = vec![String::new()];
    let mut prev_is_han = false;
    for piece in &pieces {
        match piece {
            Piece::Han(readings) => {
                let readings: Vec<&String> = if use_all {
                    readings.iter().collect()
                } else {
                    readings.iter().take(1).collect()
                };
                let mut next = Vec::with_capacity(variants.len() * readings.len());
                for v in &variants {
                    for r in &readings {
                        let mut s = v.clone();
                        if !s.is_empty() && !s.ends_with(' ') {
                            s.push(' ');
                        }
                        s.push_str(r);
                        s.push(' ');
                        next.push(s);
                    }
                }
                variants = next;
                prev_is_han = true;
            }
            Piece::Other(c) => {
                for v in variants.iter_mut() {
                    if prev_is_han && !v.ends_with(' ') {
                        v.push(' ');
                    }
                    v.push(*c);
                }
                prev_is_han = false;
            }
        }
    }
    variants.into_iter().map(|v| normalize_spaces(&v)).collect()
}

/// 单字首读音（供外部快速转换用，例如高亮匹配）。
#[allow(dead_code)]
pub fn first_pinyin(c: char) -> Option<&'static str> {
    c.to_pinyin().map(|p| p.plain())
}

/// 每个空格分隔词的首字符。
fn first_letters(s: &str) -> String {
    s.split_whitespace()
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_lowercase()
}

/// 纯 ASCII 名字的大写字母缩写（小写化）；含非 ASCII 字符则放弃。
fn upper_case_acronym(s: &str) -> String {
    if !s.is_ascii() {
        return String::new();
    }
    s.chars()
        .filter(|c| c.is_ascii_uppercase())
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_name_gets_pinyin_and_initials() {
        let kws = generate("微信");
        assert!(kws.contains(&"微信".to_string()));
        assert!(kws.contains(&"wei xin".to_string()));
        assert!(kws.contains(&"weixin".to_string()));
        assert!(kws.contains(&"wx".to_string()));
    }

    #[test]
    fn english_multiword_gets_initials_and_compact_form() {
        let kws = generate("Visual Studio Code");
        assert!(kws.contains(&"visual studio code".to_string()));
        assert!(kws.contains(&"visualstudiocode".to_string()));
        assert!(kws.contains(&"vsc".to_string()));
    }

    #[test]
    fn version_and_symbols_are_stripped() {
        let kws = generate("Notepad++ (64-bit)");
        assert!(kws.contains(&"notepad".to_string()), "{kws:?}");
        assert!(kws.contains(&"notepad++".to_string()));
        let kws = generate("Python 3.12");
        assert!(kws.contains(&"python".to_string()));
    }

    #[test]
    fn camel_case_acronym_from_uppercase_letters() {
        let kws = generate("VSCode");
        assert!(kws.contains(&"vsc".to_string()));
    }

    #[test]
    fn heteronyms_produce_both_readings() {
        let kws = generate("QQ音乐");
        // 乐：yue / le 两种读音都要能搜到
        assert!(kws.contains(&"qq yin yue".to_string()), "{kws:?}");
        assert!(kws.contains(&"qq yin le".to_string()), "{kws:?}");
        assert!(kws.contains(&"qqyinyue".to_string()));
        assert!(kws.contains(&"qyy".to_string()));
        assert!(kws.contains(&"qyl".to_string()));
    }

    #[test]
    fn too_many_heteronyms_fall_back_to_first_reading() {
        // 「重长行乐」四个多音字 → 16 种组合 > 上限，只取首读音，不会爆炸
        let kws = generate("重长行乐");
        let pinyin_count = kws.iter().filter(|k| k.contains(' ')).count();
        assert!(pinyin_count <= 2, "{kws:?}");
    }

    #[test]
    fn empty_and_whitespace_yield_nothing() {
        assert!(generate("").is_empty());
        assert!(generate("   ").is_empty());
    }
}
