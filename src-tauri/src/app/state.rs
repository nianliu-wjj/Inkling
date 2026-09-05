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
    /// 思维导图窗口的打开参数：窗口 label → 笔记 id（空串表示新建）。
    ///
    /// 用 Map 而非单值：导图窗口可以同时开多个（每个笔记一个），
    /// 单值会被后开的窗口覆盖掉先开的那个。
    pub mindmap_payloads: Mutex<std::collections::HashMap<String, String>>,
}

impl AppState {
    pub fn with_store(store: Store) -> Self {
        Self {
            store: Mutex::new(store),
            echo: Mutex::new(None),
            editor_payload: Mutex::new(None),
            mindmap_payloads: Mutex::new(std::collections::HashMap::new()),
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

    pub fn set_mindmap_payload(&self, label: String, note_id: String) {
        if let Ok(mut map) = self.mindmap_payloads.lock() {
            map.insert(label, note_id);
        }
    }

    /// 读取但不清除：窗口热重载后会重新拉取。
    pub fn mindmap_payload(&self, label: &str) -> Option<String> {
        self.mindmap_payloads
            .lock()
            .ok()
            .and_then(|map| map.get(label).cloned())
    }

    /// 窗口关闭时清理，避免 label 长期堆积。
    pub fn take_mindmap_payload(&self, label: &str) -> Option<String> {
        self.mindmap_payloads
            .lock()
            .ok()
            .and_then(|mut map| map.remove(label))
    }
}
