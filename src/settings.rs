//! 应用设置：读写 `%APPDATA%\inkling\settings.json`，并提供开机自启注册表维护。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 失焦自动收起策略
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlurClose {
    #[serde(rename = "immediate")]
    Immediate,
    #[serde(rename = "delay3s")]
    Delay3s,
    #[serde(rename = "never")]
    Never,
}

impl BlurClose {
    pub const ALL: [BlurClose; 3] = [BlurClose::Immediate, BlurClose::Delay3s, BlurClose::Never];
    pub fn label(&self) -> &'static str {
        match self {
            BlurClose::Immediate => "立即收起",
            BlurClose::Delay3s => "延迟 3 秒收起",
            BlurClose::Never => "固定不收起",
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    pub blur_close: BlurClose,
    pub clip_retention_days: u32,
    pub autostart: bool,
    pub remark_style: RemarkStyle,
    pub theme_id: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            blur_close: BlurClose::Delay3s,
            clip_retention_days: 30,
            autostart: false,
            remark_style: RemarkStyle::Mixed,
            theme_id: "dark".into(),
        }
    }
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

    /// 开机自启：写 / 删 HKCU Run 注册表项（仅 Windows 生效）
    pub fn set_autostart(enable: bool) -> Result<(), String> {
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
                key.set_value("Inkling", &exe).map_err(|e| e.to_string())?;
            } else {
                key.delete_value("Inkling").map_err(|e| e.to_string())?;
            }
        }
        #[cfg(not(windows))]
        {
            let _ = enable;
        }
        Ok(())
    }
}
