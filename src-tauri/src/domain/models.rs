//! 领域模型：跨层共享的 serde 数据结构。
//!
//! 时间约定：所有时间均存储为带时区的 RFC3339 字符串（后端统一使用 UTC 写入），
//! 展示与日期归属由查询方转换为用户当前时区。

use serde::{Deserialize, Serialize};

crate::dto! {
    /// 笔记。正文 ≤1MB 时直接存储；超出时落盘 `notes/` 并仅在 `file_path` 保留相对路径。
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Note {
        id: String,
        content: String,
        /// text / mindmap。思维导图数据单独保存，避免污染 Markdown 正文。
        #[serde(default)]
        editor_mode: String,
        /// simple-mind-map 的 JSON 数据。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mindmap_data: Option<String>,
        tags: Vec<String>,
        is_draft: bool,
        pinned: bool,
        /// 成功归档时刻（草稿提升为正式笔记时写入；统计按该时刻计数）。
        archived_at: Option<String>,
        created_at: String,
        updated_at: String,
    }
}

crate::dto! {
    /// 剪贴板条目。图片等大附件落盘 `clipboard/` 并以 `file_path` 引用。
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct ClipboardEntry {
        id: String,
        /// text / link / code / image / richtext
        content_type: String,
        content: String,
        preview: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_path: Option<String>,
        /// 采集时的前台应用名（v6 起）；旧数据与采集失败时为 None，序列化时省略。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source_app: Option<String>,
        pinned: bool,
        copied_at: String,
        modified_at: String,
    }
}

crate::dto! {
    /// 待办（父待办与一级子任务同构）。
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Todo {
        id: String,
        content: String,
        /// 界面所称「完成时间」= 计划完成/截止时间。
        due_at: String,
        /// 实际勾选完成时刻。
        completed_at: Option<String>,
        /// open / done
        status: String,
        /// 重复提醒推进时的游标；**不再是用户设置的提醒时刻**。
        ///
        /// 用户设的是相对偏移（`remind_offset_minutes`），实际提醒时刻由
        /// `due_at - offset` 现算，这样改完成时间后提醒会自动跟随。
        remind_at: Option<String>,
        /// 提醒偏移分钟数（完成时间之前）；`None` = 不提醒。
        remind_offset_minutes: Option<i64>,
        /// 是否桌面弹窗提醒。
        remind_desktop: bool,
        /// 是否邮件提醒。
        remind_email: bool,
        /// daily / weekly / None
        repeat_rule: Option<String>,
        /// 用户在提醒卡片上点击「关闭」后置位，抑制后续提醒；编辑提醒时复位。
        remind_off: bool,
        /// high / medium / low
        priority: String,
        remark: String,
        parent_id: Option<String>,
        tags: Vec<String>,
        created_at: String,
        updated_at: String,
    }
}

/// 玻璃质感默认档：标准档，等于 tokens.css 里 :root 的既有值，升级后观感不变。
fn default_glass_level() -> String {
    "standard".into()
}

/// SMTP 默认端口：465（隐式 TLS），与多数邮箱服务商的默认一致。
fn default_smtp_port() -> i64 {
    465
}

fn default_true() -> bool {
    true
}

/// 面板唤出位置：四个方向均以屏幕中线为基准。
///
/// 用 `cfg!` 宏而非 `#[cfg]` 属性块：属性块里写 `return` 会让后面的兜底表达式
/// 在该平台上永远不可达，编译器据此报 unreachable_code。
/// `cfg!` 是编译期常量，未命中的分支会被优化掉，且不产生不可达代码。
/// 灵动岛默认宽度（逻辑像素）。
fn default_island_width() -> i64 {
    360
}

/// 灵动岛默认高度（逻辑像素）。
fn default_island_height() -> i64 {
    36
}

/// 灵动岛默认不透明度。
fn default_island_opacity() -> f64 {
    0.85
}

/// 灵动岛默认轮播间隔（秒）。
fn default_island_cycle_seconds() -> i64 {
    4
}

/// 灵动岛默认插件列表：只有「当日待办」。
fn default_island_plugins() -> String {
    "today-todos".into()
}

/// 启动器默认全局快捷键。
fn default_launcher_shortcut() -> String {
    "Alt+Space".into()
}

pub fn default_panel_position() -> String {
    // 需求 2.1「鼠标触顶」：感应区与面板默认都在屏幕顶部中央。
    // 曾按平台区分（Windows 默认底部），但感应区当时固定在顶部，
    // 用户在顶部悬停、面板却从底部弹出，被误认为「无法唤出」。
    "top".into()
}

crate::dto! {
    /// 偏好设置。
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Settings {
        /// immediate / 3s / never
        collapse_policy: String,
        clipboard_retention_days: i64,
        start_on_boot: bool,
        shortcut: String,
        /// mixed / icon / text
        remark_style: String,
        theme: String,
        /// 归档主窗口是否启用毛玻璃（Windows Acrylic / macOS Vibrancy）。
        /// 关闭时窗口退化为不透明实色，见 styles/base.css 的 [data-acrylic="off"]。
        main_acrylic: bool,
        /// top / bottom / left / right；缺少该设置时使用当前平台默认值。
        #[serde(default = "default_panel_position")]
        panel_position: String,
        /// 启用的面板插件 id 有序列表（逗号分隔）。
        /// 在列表里 = 启用，列表次序 = 展示顺序与快捷键序号；为空则用全部内置插件的默认顺序。
        #[serde(default)]
        panel_plugins: String,
        /// 玻璃质感档位：minimal / standard / frosted。
        /// 与配色主题正交——同一套配色可轻可厚，见 styles/glass.css。
        #[serde(default = "default_glass_level")]
        glass_level: String,
        /// SMTP 服务器地址，如 smtp.qq.com；为空表示未配置邮件提醒。
        #[serde(default)]
        smtp_host: String,
        #[serde(default = "default_smtp_port")]
        smtp_port: i64,
        #[serde(default = "default_true")]
        smtp_tls: bool,
        #[serde(default)]
        smtp_username: String,
        /// 建议填邮箱的**应用专用密码**而非主账号密码。
        /// 读取时会被替换为掩码，避免真实值进入前端状态与日志。
        #[serde(default)]
        smtp_password: String,
        #[serde(default)]
        smtp_from: String,
        #[serde(default)]
        smtp_to: String,
        /// 灵动岛：是否显示（主屏顶部胶囊）。
        #[serde(default = "default_true")]
        island_enabled: bool,
        /// 灵动岛宽度（逻辑像素，200–800）。
        #[serde(default = "default_island_width")]
        island_width: i64,
        /// 灵动岛高度（逻辑像素，28–72）。
        #[serde(default = "default_island_height")]
        island_height: i64,
        /// 灵动岛背景不透明度（0.3–1.0）。
        #[serde(default = "default_island_opacity")]
        island_opacity: f64,
        /// 灵动岛鼠标穿透（零干扰）：开启后悬停/点击由 Rust 光标轮询探测。
        #[serde(default)]
        island_click_through: bool,
        /// 灵动岛轮播间隔秒数（2–30）。
        #[serde(default = "default_island_cycle_seconds")]
        island_cycle_seconds: i64,
        /// 启用的灵动岛插件 id 有序列表（逗号分隔）；为空则用全部内置插件。
        #[serde(default = "default_island_plugins")]
        island_plugins: String,
        /// 启动器全局快捷键。
        #[serde(default = "default_launcher_shortcut")]
        launcher_shortcut: String,
        /// 启动器文件扫描根目录（JSON 数组字符串）；为空则用默认用户目录。
        #[serde(default)]
        launcher_roots: String,
        /// 启动器全盘文件索引开关（默认开）；关闭时回退到 launcher_roots 根目录模式。
        #[serde(default = "default_true")]
        launcher_full_disk_index: bool,
        /// 全盘索引额外排除目录（逗号分隔目录名）。
        #[serde(default)]
        launcher_extra_excludes: String,
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            collapse_policy: "3s".into(),
            clipboard_retention_days: 30,
            start_on_boot: false,
            shortcut: "Ctrl+Shift+Space".into(),
            remark_style: "mixed".into(),
            theme: "typewriter".into(),
            main_acrylic: true,
            panel_position: default_panel_position(),
            panel_plugins: String::new(),
            glass_level: default_glass_level(),
            smtp_host: String::new(),
            smtp_port: default_smtp_port(),
            smtp_tls: true,
            smtp_username: String::new(),
            smtp_password: String::new(),
            smtp_from: String::new(),
            smtp_to: String::new(),
            island_enabled: true,
            island_width: default_island_width(),
            island_height: default_island_height(),
            island_opacity: default_island_opacity(),
            island_click_through: false,
            island_cycle_seconds: default_island_cycle_seconds(),
            island_plugins: default_island_plugins(),
            launcher_shortcut: default_launcher_shortcut(),
            launcher_roots: String::new(),
            launcher_full_disk_index: true,
            launcher_extra_excludes: String::new(),
        }
    }
}

crate::dto! {
    /// 单日活跃度（真实业务数据派生，非聚合表推断）。
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct DayActivity {
        date: String,
        notes: i64,
        clips: i64,
        todos: i64,
        completed: i64,
        overdue: i64,
    }
}

crate::dto! {
    /// 月度趋势。
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct MonthTrend {
        month: String,
        notes: i64,
        clips: i64,
        todos: i64,
        completed: i64,
    }
}

crate::dto! {
    /// 全量统计摘要。
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct StatsSummary {
        notes: i64,
        clips: i64,
        todos: i64,
        completed: i64,
        overdue: i64,
    }
}

crate::dto! {
    /// 日期详情混排条目：同一天内的笔记 / 剪贴板 / 待办。
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct DayDetailItem {
        /// note / clip / todo
        kind: String,
        time: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        note: Option<Note>,
        #[serde(skip_serializing_if = "Option::is_none")]
        clip: Option<ClipboardEntry>,
        #[serde(skip_serializing_if = "Option::is_none")]
        todo: Option<Todo>,
    }
}
