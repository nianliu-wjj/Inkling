//! 全盘扫描的目录排除规则：系统目录、回收站、版本控制、构建产物、缓存等。
//!
//! 全盘 WalkDir 时对每个目录名判定是否整棵跳过——既省时间也避免把无意义的
//! 海量小文件（如 node_modules）灌进索引。内置集合 + 设置里的运行期追加。

/// 内置排除目录名（小写比较）。
const BUILTIN: &[&str] = &[
    "windows",
    "$recycle.bin",
    "system volume information",
    "$winreagent",
    "recovery",
    "perflogs",
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "target",
    ".cache",
    "__pycache__",
    ".gradle",
    ".nuget",
];

/// 目录名是否应被跳过（大小写不敏感）。`extra` 为设置里追加的目录名。
pub fn is_excluded_dir(name: &str, extra: &[String]) -> bool {
    let lower = name.to_ascii_lowercase();
    BUILTIN.contains(&lower.as_str()) || extra.iter().any(|e| e.eq_ignore_ascii_case(&lower))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_builtin_and_extra_case_insensitive() {
        assert!(is_excluded_dir("Windows", &[]));
        assert!(is_excluded_dir("node_modules", &[]));
        assert!(is_excluded_dir("$Recycle.Bin", &[]));
        assert!(is_excluded_dir(".git", &[]));
        assert!(!is_excluded_dir("Projects", &[]));
        // 大小写不敏感 + 运行期追加
        assert!(is_excluded_dir("WINDOWS", &[]));
        assert!(is_excluded_dir("MyCache", &["mycache".to_string()]));
    }
}
