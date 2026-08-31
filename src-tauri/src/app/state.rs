//! 全局共享状态：数据库连接池 + 剪贴板回声抑制标记。

use crate::data::Store;
use std::sync::Mutex;

pub struct AppState {
    pub store: Mutex<Store>,
    /// 应用自身写回剪贴板的内容哈希，下一次轮询命中即跳过（防自记录回声）。
    pub echo: Mutex<Option<String>>,
}

impl AppState {
    pub fn with_store(store: Store) -> Self {
        Self { store: Mutex::new(store), echo: Mutex::new(None) }
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
}
