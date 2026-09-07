//! 事件名常量：跨窗口通信的统一契约。

pub const NAVIGATE: &str = "inkling://navigate";
pub const PANEL_SHOWN: &str = "inkling://panel-shown";
pub const PANEL_HIDDEN: &str = "inkling://panel-hidden";
pub const NOTES_CHANGED: &str = "inkling://notes-changed";
pub const CLIPBOARD_CHANGED: &str = "inkling://clipboard-changed";
pub const TODOS_CHANGED: &str = "inkling://todos-changed";
pub const SETTINGS_CHANGED: &str = "inkling://settings-changed";
pub const STATS_CHANGED: &str = "inkling://stats-changed";
pub const REMINDER_FIRED: &str = "inkling://reminder-fired";
/// 光标进入 / 离开顶部感应区，payload 为 bool（仅发给 hotzone 窗口）。
pub const HOTZONE_HOVER: &str = "inkling://hotzone-hover";
/// 编辑窗口已关闭（广播）：面板据此恢复失焦收起计时。
pub const EDITOR_CLOSED: &str = "inkling://editor-closed";
/// 光标进入 / 离开灵动岛，payload 为 bool（仅发给 island 窗口；穿透模式下的悬停来源）。
pub const ISLAND_HOVER: &str = "inkling://island-hover";
/// 灵动岛被左键点击（仅发给 island 窗口；穿透模式下由左键边沿探测得出）。
pub const ISLAND_CLICK: &str = "inkling://island-click";
/// 请求面板切换到某个插件页，payload 为插件 id（广播，面板订阅）。
pub const PANEL_NAVIGATE: &str = "inkling://panel-navigate";
