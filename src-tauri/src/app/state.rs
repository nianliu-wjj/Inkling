//! 全局共享状态：数据库连接池 + 剪贴板回声抑制标记 + 编辑窗口打开参数。

use crate::data::Store;
use std::sync::Mutex;

pub struct AppState {
    pub store: Mutex<Store>,
    /// 应用自身写回剪贴板的内容哈希，下一次轮询命中即跳过（防自记录回声）。
    pub echo: Mutex<Option<String>>,
    /// 编辑窗口本次打开的参数（JSON 字符串）。
    ///
    /// 不走 URL 查询串：`WebviewUrl::App` 收的是相对路径，`?` 会被转义掉。
    /// 建窗前写入，编辑窗口前端挂载时来取，取完不清除——窗口关闭时统一清空。
    pub editor_payload: Mutex<Option<String>>,
}

impl AppState {
    pub fn with_store(store: Store) -> Self {
        Self {
            store: Mutex::new(store),
            echo: Mutex::new(None),
            editor_payload: Mutex::new(None),
        }
    }

    pub fn lock_store(&self) -> Result<std::sync::MutexGuard<'_, Store>, String> {
        self.store.lock().map_err(|_| "数据库锁已损坏".to_string())
    }

    pub fn set_echo(&self, hash: Option<String>) {
        if let Ok(mut echo) = self.echo.lock() {
            *echo = hash;
        }
    }

    pub fn take_echo(&self) -> Option<String> {
        self.echo.lock().ok().and_then(|mut x| x.take())
    }

    pub fn set_editor_payload(&self, payload: Option<String>) {
        if let Ok(mut slot) = self.editor_payload.lock() {
            *slot = payload;
        }
    }

    pub fn editor_payload(&self) -> Option<String> {
        self.editor_payload.lock().ok().and_then(|x| x.clone())
    }
}
