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
    /// 各感应区窗口的**物理像素**矩形 (left, top, right, bottom)，label → rect。
    ///
    /// 由建窗 / 重定位时写入，hotzone_watcher 每轮读取。不回读窗口的 outer_position：
    /// 混合 DPI（主屏 1.5×、外接屏 1.0×）下 tao 回报的位置会在两种缩放间跳变，
    /// 用它判定进入/离开会闪烁；自己算出来的物理矩形与 cursor_position 同坐标系，稳定。
    pub hotzone_rects: Mutex<std::collections::HashMap<String, (f64, f64, f64, f64)>>,
    /// 灵动岛窗口的物理像素矩形 (left, top, right, bottom)；未显示时为 None。
    pub island_rect: Mutex<Option<(f64, f64, f64, f64)>>,
    /// 面板下一次显示时应切到的插件页（灵动岛点击写入，面板 panel-shown 后取走）。
    ///
    /// 不直接向隐藏的面板 emit：WebView2 在窗口 hide 后被挂起，此时投递的事件会丢。
    pub pending_panel_page: Mutex<Option<String>>,
}

impl AppState {
    pub fn with_store(store: Store) -> Self {
        Self {
            store: Mutex::new(store),
            echo: Mutex::new(None),
            editor_payload: Mutex::new(None),
            mindmap_payloads: Mutex::new(std::collections::HashMap::new()),
            hotzone_rects: Mutex::new(std::collections::HashMap::new()),
            island_rect: Mutex::new(None),
            pending_panel_page: Mutex::new(None),
        }
    }

    pub fn set_island_rect(&self, rect: Option<(f64, f64, f64, f64)>) {
        if let Ok(mut slot) = self.island_rect.lock() {
            *slot = rect;
        }
    }

    pub fn island_rect(&self) -> Option<(f64, f64, f64, f64)> {
        self.island_rect.lock().ok().and_then(|slot| *slot)
    }

    pub fn set_pending_panel_page(&self, page: Option<String>) {
        if let Ok(mut slot) = self.pending_panel_page.lock() {
            *slot = page;
        }
    }

    /// 取走并清空待切换的面板页。
    pub fn take_pending_panel_page(&self) -> Option<String> {
        self.pending_panel_page
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
    }

    pub fn set_hotzone_rect(&self, label: String, rect: (f64, f64, f64, f64)) {
        if let Ok(mut map) = self.hotzone_rects.lock() {
            map.insert(label, rect);
        }
    }

    /// 显示器拔掉后对应感应区窗口已关闭，同步移除其矩形，避免 watcher 继续对幽灵窗口发事件。
    pub fn remove_hotzone_rect(&self, label: &str) {
        if let Ok(mut map) = self.hotzone_rects.lock() {
            map.remove(label);
        }
    }

    pub fn hotzone_rects(&self) -> Vec<(String, (f64, f64, f64, f64))> {
        self.hotzone_rects
            .lock()
            .map(|map| map.iter().map(|(k, v)| (k.clone(), *v)).collect())
            .unwrap_or_default()
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
