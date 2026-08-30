//! 应用设置：读写 `%APPDATA%\inkling\settings.json`，并提供开机自启注册表维护。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 失焦自动收起策略（延迟模式配合 `blur_delay_secs` 使用）
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlurClose {
    #[serde(rename = "immediate")]
    Immediate,
    #[serde(rename = "delay", alias = "delay3s")]
    Delay,
    #[serde(rename = "never")]
    Never,
}

impl Default for BlurClose {
    fn default() -> Self {
        BlurClose::Delay
    }
}

impl BlurClose {
    pub const ALL: [BlurClose; 3] = [BlurClose::Immediate, BlurClose::Delay, BlurClose::Never];

    pub fn label(&self) -> &'static str {
        match self {
            BlurClose::Immediate => "立即收起",
            BlurClose::Delay => "延迟收起",
            BlurClose::Never => "固定不收起",
        }
    }
}

/// 粘贴板保留策略
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClipRetention {
    /// 永不过期
    #[serde(rename = "never")]
    Never,
    /// 重启失效
    #[serde(rename = "session")]
    Session,
    /// 当天有效
    #[serde(rename = "today")]
    Today,
    /// 自定义天数
    #[serde(rename = "custom")]
    Custom(u32),
}

impl Default for ClipRetention {
    fn default() -> Self {
        ClipRetention::Custom(30)
    }
}

impl ClipRetention {
    /// 供分段选择器遍历的选项（自定义默认 30 天）
    pub const OPTIONS: [ClipRetention; 4] = [
        ClipRetention::Never,
        ClipRetention::Session,
        ClipRetention::Today,
        ClipRetention::Custom(30),
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ClipRetention::Never => "永不过期",
            ClipRetention::Session => "重启失效",
            ClipRetention::Today => "当天有效",
            ClipRetention::Custom(_) => "自定义",
        }
    }
}

/// 备注展示样式
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RemarkStyle {
    #[serde(rename = "mixed")]
    Mixed,
    #[serde(rename = "icon")]
    Icon,
    #[serde(rename = "line")]
    Line,
}

impl Default for RemarkStyle {
    fn default() -> Self {
        RemarkStyle::Mixed
    }
}

impl RemarkStyle {
    pub const ALL: [RemarkStyle; 3] = [RemarkStyle::Mixed, RemarkStyle::Icon, RemarkStyle::Line];

    pub fn label(&self) -> &'static str {
        match self {
            RemarkStyle::Mixed => "混合模式（超100字用图标）",
            RemarkStyle::Icon => "图标徽章 + 悬浮",
            RemarkStyle::Line => "置灰文本行",
        }
    }
}

crate::accessors! {
    /// 应用设置
    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Settings {
        /// 失焦自动收起策略
        #[serde(default)]
        blur_close: BlurClose,
        /// 延迟收起的秒数（1 ~ 60）
        #[serde(default = "default_delay_secs")]
        blur_delay_secs: u32,
        /// 粘贴板保留策略
        #[serde(default)]
        clip_retention: ClipRetention,
        /// 备注展示样式
        #[serde(default)]
        remark_style: RemarkStyle,
        /// 开机静默自启动
        #[serde(default)]
        autostart: bool,
        /// 当前主题 id
        #[serde(default)]
        theme_id: String,
        /// 侧边栏宽度（110 ~ 280，单位为逻辑像素）
        #[serde(default = "default_sidebar_width")]
        sidebar_width: u32,
        /// 全局快捷键，使用 global-hotkey 语法，例如 Ctrl+Shift+Space
        #[serde(default = "default_global_shortcut")]
        global_shortcut: String,
    }
}

fn default_delay_secs() -> u32 {
    3
}

fn default_sidebar_width() -> u32 {
    160
}

fn default_global_shortcut() -> String {
    "Ctrl+Shift+Space".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            blur_close: BlurClose::Delay,
            blur_delay_secs: 3,
            clip_retention: ClipRetention::Custom(30),
            remark_style: RemarkStyle::Mixed,
            autostart: false,
            theme_id: "dark".into(),
            sidebar_width: 160,
            global_shortcut: default_global_shortcut(),
        }
    }
}

#[cfg(target_os = "macos")]
fn macos_launchctl_domain() -> String {
    let uid = std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "0".into());
    format!("gui/{uid}")
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

impl Settings {
    fn config_path() -> PathBuf {
        let dir = std::env::var("APPDATA")
            .map(|d| PathBuf::from(d).join("inkling"))
            .unwrap_or_else(|_| PathBuf::from("."));
        std::fs::create_dir_all(&dir).ok();
        dir.join("settings.json")
    }

    /// 从磁盘加载（失败回退默认值）
    pub fn load() -> Self {
        std::fs::read_to_string(Self::config_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// 保存到磁盘
    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(Self::config_path(), json);
        }
    }

    /// 开机自启：Windows 使用 HKCU Run，macOS 使用当前用户 LaunchAgent。
    pub fn apply_autostart_registry(enable: bool) -> Result<(), String> {
        #[cfg(windows)]
        {
            use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};
            let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
            let key = hkcu
                .open_subkey_with_flags(
                    r"Software\Microsoft\Windows\CurrentVersion\Run",
                    KEY_SET_VALUE | KEY_QUERY_VALUE,
                )
                .map_err(|e| e.to_string())?;
            if enable {
                let exe = std::env::current_exe()
                    .map_err(|e| e.to_string())?
                    .display()
                    .to_string();
                let command = format!("\"{}\" --autostart", exe);
                key.set_value("Inkling", &command).map_err(|e| e.to_string())?;
            } else {
                if let Err(error) = key.delete_value("Inkling") {
                    if error.kind() != std::io::ErrorKind::NotFound {
                        return Err(error.to_string());
                    }
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or_else(|| "无法确定用户主目录".to_string())?;
            let agents_dir = home.join("Library/LaunchAgents");
            let plist_path = agents_dir.join("com.inkling.app.plist");
            if enable {
                std::fs::create_dir_all(&agents_dir).map_err(|e| e.to_string())?;
                let exe = std::env::current_exe().map_err(|e| e.to_string())?;
                let plist = format!(
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>com.inkling.app</string>
<key>ProgramArguments</key><array><string>{}</string><string>--autostart</string></array>
<key>RunAtLoad</key><true/>
</dict></plist>
"# ,
                    xml_escape(&exe.display().to_string())
                );
                std::fs::write(&plist_path, plist).map_err(|e| e.to_string())?;
                let domain = macos_launchctl_domain();
                let _ = std::process::Command::new("launchctl")
                    .args(["bootstrap", &domain])
                    .arg(&plist_path)
                    .status();
            } else {
                let domain = macos_launchctl_domain();
                let _ = std::process::Command::new("launchctl")
                    .args(["bootout", &domain])
                    .arg(&plist_path)
                    .status();
                match std::fs::remove_file(&plist_path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.to_string()),
                }
            }
        }
        #[cfg(all(not(windows), not(target_os = "macos")))]
        {
            let _ = enable;
        }
        Ok(())
    }
}
