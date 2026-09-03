//! 事件名常量：跨窗口通信的统一契约。

pub const NAVIGATE: &str = "inkling://navigate";
pub const PANEL_SHOWN: &str = "inkling://panel-shown";
pub const PANEL_HIDDEN: &str = "inkling://panel-hidden";
pub const NOTES_CHANGED: &str = "inkling://notes-changed";
pub const CLIPBOARD_CHANGED: &str = "inkling://clipboard-changed";
pub const TODOS_CHANGED: &str = "inkling://todos-changed";
pub const SETTINGS_CHANGED: &str = "inkling://settings-changed";
pub const STATS_CHANGED: &str = "inkling://stats-changed";
pub const PIN_UPDATED: &str = "inkling://pin-updated";
pub const REMINDER_FIRED: &str = "inkling://reminder-fired";
/// 光标进入 / 离开顶部感应区，payload 为 bool（仅发给 hotzone 窗口）。
pub const HOTZONE_HOVER: &str = "inkling://hotzone-hover";
/// 编辑窗口已关闭（广播）：面板据此恢复失焦收起计时。
pub const EDITOR_CLOSED: &str = "inkling://editor-closed";
