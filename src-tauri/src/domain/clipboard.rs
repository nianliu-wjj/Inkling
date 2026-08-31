//! 剪贴板领域逻辑：内容哈希、类型分类与回声抑制策略（零 Tauri 依赖，可单测）。

use sha2::{Digest, Sha256};

/// 计算内容的 SHA-256 哈希（用于去重与回声抑制）。
pub fn hash_content(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

/// 内容分类结果。
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CaptureKind {
    Text,
    Link,
    Code,
    Image,
    RichText,
}

impl CaptureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CaptureKind::Text => "text",
            CaptureKind::Link => "link",
            CaptureKind::Code => "code",
            CaptureKind::Image => "image",
            CaptureKind::RichText => "richtext",
        }
    }
}

/// 简易 URL 判定：`scheme://非空白`。
fn looks_like_url(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.contains(char::is_whitespace) || trimmed.is_empty() {
        return false;
    }
    for prefix in ["http://", "https://"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.starts_with(|c: char| c.is_ascii_alphanumeric())
                && rest.chars().all(|c| !c.is_whitespace());
        }
    }
    false
}

/// 代码启发式判定：多行 + 常见语法特征。
fn looks_like_code(text: &str) -> bool {
    let trimmed = text.trim_end();
    if !trimmed.contains('\n') {
        return false;
    }
    const SIGNS: [&str; 10] = [
        "fn ", "let ", "const ", "function ", "class ", "import ", "export ", "#include", "=>",
        "&&",
    ];
    let has_sign = SIGNS.iter().any(|s| trimmed.contains(s));
    let has_block = trimmed.contains('{') && trimmed.contains('}');
    let has_semicolons = trimmed.matches(';').count() >= 2;
    let indented = trimmed.lines().filter(|l| l.starts_with("    ") || l.starts_with('\t')).count() >= 2;
    (has_sign && (has_block || has_semicolons)) || (has_block && has_semicolons) || indented && has_block
}

/// 根据文本与 HTML 特征分类内容。
pub fn classify_text(text: &str, has_html: bool) -> CaptureKind {
    if looks_like_url(text) {
        CaptureKind::Link
    } else if has_html {
        CaptureKind::RichText
    } else if looks_like_code(text) {
        CaptureKind::Code
    } else {
        CaptureKind::Text
    }
}

/// 生成条目预览（去换行、截断）。
pub fn build_preview(text: &str, limit: usize) -> String {
    let flat: String = text.chars().map(|c| if c == '\n' || c == '\r' || c == '\t' { ' ' } else { c }).collect();
    flat.chars().take(limit).collect()
}

/// 验证待办标签约束：去重、≤3 个、每个 ≤10 字。
pub fn validate_todo_tags(tags: &[String]) -> Result<Vec<String>, String> {
    let mut result: Vec<String> = Vec::new();
    for raw in tags {
        let tag = raw.trim();
        if tag.is_empty() {
            continue;
        }
        if tag.chars().count() > 10 {
            return Err("待办标签最多 10 个字".into());
        }
        if !result.iter().any(|item: &String| item.eq_ignore_ascii_case(tag)) {
            result.push(tag.to_string());
        }
    }
    if result.len() > 3 {
        return Err("待办最多只能有 3 个标签".into());
    }
    Ok(result)
}

/// 验证笔记标签约束：去重、≤3 个、每个 ≤5 字。
pub fn validate_note_tags(tags: &[String]) -> Result<Vec<String>, String> {
    let mut result: Vec<String> = Vec::new();
    for raw in tags {
        let tag = raw.trim();
        if tag.is_empty() {
            continue;
        }
        if tag.chars().count() > 5 {
            return Err("笔记标签最多 5 个字".into());
        }
        if !result.iter().any(|item: &String| item.eq_ignore_ascii_case(tag)) {
            result.push(tag.to_string());
        }
    }
    if result.len() > 3 {
        return Err("笔记最多只能有 3 个标签".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_case_sensitive() {
        assert_eq!(hash_content(b"Inkling"), hash_content(b"Inkling"));
        assert_ne!(hash_content(b"Inkling"), hash_content(b"inkling"));
    }

    #[test]
    fn classifies_links() {
        assert_eq!(classify_text("https://example.com/a?b=1", false), CaptureKind::Link);
        assert_eq!(classify_text("http://localhost:1420", false), CaptureKind::Link);
        assert_ne!(classify_text("https://a.com and more", false), CaptureKind::Link);
    }

    #[test]
    fn classifies_code() {
        assert_eq!(
            classify_text("const a = 1;\nconst b = 2;\nif (a && b) { log(); }", false),
            CaptureKind::Code
        );
        assert_ne!(classify_text("第一行\n第二行", false), CaptureKind::Code);
    }

    #[test]
    fn classifies_richtext_before_code() {
        assert_eq!(classify_text("const a = 1;\nconst b = 2;", true), CaptureKind::RichText);
    }

    #[test]
    fn preview_flattens_whitespace() {
        assert_eq!(build_preview("a\nb\tc", 10), "a b c");
        assert_eq!(build_preview("abcdef", 3), "abc");
    }

    #[test]
    fn todo_tags_validated() {
        assert_eq!(validate_todo_tags(&["  Rust ".into(), "rust".into()]).unwrap(), vec!["Rust"]);
        assert!(validate_todo_tags(&["a".into(), "b".into(), "c".into(), "d".into()]).is_err());
        assert!(validate_todo_tags(&["12345678901".into()]).is_err());
    }

    #[test]
    fn note_tags_validated() {
        assert_eq!(validate_note_tags(&["念头".into(), "想法".into()]).unwrap(), vec!["念头", "想法"]);
        assert!(validate_note_tags(&["123456".into()]).is_err());
        assert!(validate_note_tags(&["a".into(), "b".into(), "c".into(), "d".into()]).is_err());
    }
}
