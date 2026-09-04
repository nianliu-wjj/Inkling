//! 领域模型：跨层共享的 serde 数据结构。
//!
//! 时间约定：所有时间均存储为带时区的 RFC3339 字符串（后端统一使用 UTC 写入），
//! 展示与日期归属由查询方转换为用户当前时区。

use serde::{Deserialize, Serialize};

/// 笔记。正文 ≤1MB 时直接存储；超出时落盘 `notes/` 并仅在 `file_path` 保留相对路径。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Note {
    pub id: String,
    pub content: String,
    /// text / mindmap。思维导图数据单独保存，避免污染 Markdown 正文。
    #[serde(default)]
    pub editor_mode: String,
    /// simple-mind-map 的 JSON 数据。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mindmap_data: Option<String>,
    pub tags: Vec<String>,
    pub is_draft: bool,
    pub pinned: bool,
    /// 成功归档时刻（草稿提升为正式笔记时写入；统计按该时刻计数）。
    pub archived_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 剪贴板条目。图片等大附件落盘 `clipboard/` 并以 `file_path` 引用。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClipboardEntry {
    pub id: String,
    /// text / link / code / image / richtext
    pub content_type: String,
    pub content: String,
    pub preview: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    pub pinned: bool,
    pub copied_at: String,
    pub modified_at: String,
}

/// 待办（父待办与一级子任务同构）。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Todo {
    pub id: String,
    pub content: String,
    /// 界面所称「完成时间」= 计划完成/截止时间。
    pub due_at: String,
    /// 实际勾选完成时刻。
    pub completed_at: Option<String>,
    /// open / done
    pub status: String,
    /// 重复提醒推进时的游标；**不再是用户设置的提醒时刻**。
    ///
    /// 用户设的是相对偏移（`remind_offset_minutes`），实际提醒时刻由
    /// `due_at - offset` 现算，这样改完成时间后提醒会自动跟随。
    pub remind_at: Option<String>,
    /// 提醒偏移分钟数（完成时间之前）；`None` = 不提醒。
    pub remind_offset_minutes: Option<i64>,
    /// 是否桌面弹窗提醒。
    pub remind_desktop: bool,
    /// 是否邮件提醒。
    pub remind_email: bool,
    /// daily / weekly / None
    pub repeat_rule: Option<String>,
    /// 用户在提醒卡片上点击「关闭」后置位，抑制后续提醒；编辑提醒时复位。
    pub remind_off: bool,
    /// high / medium / low
    pub priority: String,
    pub remark: String,
    pub parent_id: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// SMTP 默认端口：465（隐式 TLS），与多数邮箱服务商的默认一致。
fn default_smtp_port() -> i64 {
    465
}

fn default_true() -> bool {
    true
}

/// 面板唤出位置：四个方向均以屏幕中线为基准。
pub fn default_panel_position() -> String {
    #[cfg(target_os = "windows")]
    {
        return "bottom".into();
    }

    #[cfg(target_os = "macos")]
    {
        return "top".into();
    }

    "top".into()
}

/// 偏好设置。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    /// immediate / 3s / never
    pub collapse_policy: String,
    pub clipboard_retention_days: i64,
    pub start_on_boot: bool,
    pub shortcut: String,
    /// mixed / icon / text
    pub remark_style: String,
    pub theme: String,
    /// 归档主窗口是否启用毛玻璃（Windows Acrylic / macOS Vibrancy）。
    /// 关闭时窗口退化为不透明实色，见 styles/base.css 的 [data-acrylic="off"]。
    pub main_acrylic: bool,
    /// top / bottom / left / right；缺少该设置时使用当前平台默认值。
    #[serde(default = "default_panel_position")]
    pub panel_position: String,
    /// SMTP 服务器地址，如 smtp.qq.com；为空表示未配置邮件提醒。
    #[serde(default)]
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: i64,
    #[serde(default = "default_true")]
    pub smtp_tls: bool,
    #[serde(default)]
    pub smtp_username: String,
    /// 建议填邮箱的**应用专用密码**而非主账号密码。
    /// 读取时会被替换为掩码，避免真实值进入前端状态与日志。
    #[serde(default)]
    pub smtp_password: String,
    #[serde(default)]
    pub smtp_from: String,
    #[serde(default)]
    pub smtp_to: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            collapse_policy: "3s".into(),
            clipboard_retention_days: 30,
            start_on_boot: false,
            shortcut: "Ctrl+Shift+Space".into(),
            remark_style: "mixed".into(),
            theme: "dark".into(),
            main_acrylic: true,
            panel_position: default_panel_position(),
            smtp_host: String::new(),
            smtp_port: default_smtp_port(),
            smtp_tls: true,
            smtp_username: String::new(),
            smtp_password: String::new(),
            smtp_from: String::new(),
            smtp_to: String::new(),
        }
    }
}

/// 单日活跃度（真实业务数据派生，非聚合表推断）。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DayActivity {
    pub date: String,
    pub notes: i64,
    pub clips: i64,
    pub todos: i64,
    pub completed: i64,
    pub overdue: i64,
}

/// 月度趋势。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonthTrend {
    pub month: String,
    pub notes: i64,
    pub clips: i64,
    pub todos: i64,
    pub completed: i64,
}

/// 全量统计摘要。
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct StatsSummary {
    pub notes: i64,
    pub clips: i64,
    pub todos: i64,
    pub completed: i64,
    pub overdue: i64,
}

/// 日期详情混排条目：同一天内的笔记 / 剪贴板 / 待办。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DayDetailItem {
    /// note / clip / todo
    pub kind: String,
    pub time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Note>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip: Option<ClipboardEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todo: Option<Todo>,
}
