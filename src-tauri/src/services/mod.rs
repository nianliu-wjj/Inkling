//! 服务层：剪贴板轮询、提醒调度与导出，均运行在后台线程。

pub mod clipboard_watcher;
pub mod export;
pub mod hotzone_watcher;
pub mod reminder;
