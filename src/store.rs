//! 共享数据层：笔记 / 待办 / 剪贴板历史（呼出面板与主窗口共用）。
//! 基础阶段存于内存（Global），后续接入 SQLite。

use gpui::{App, Global};

crate::accessors! {
    #[derive(Clone, Debug)]
    pub struct Note {
        content: String,
    }
}

crate::accessors! {
    #[derive(Clone, Debug)]
    pub struct TodoItem {
        text: String,
        done: bool,
    }
}

#[derive(Default)]
pub struct Store {
    notes: Vec<Note>,
    todos: Vec<TodoItem>,
    clips: Vec<String>,
}

impl Global for Store {}

/// 初始化（写入示例数据）
pub fn init(cx: &mut App) {
    if cx.has_global::<Store>() {
        return;
    }
    cx.set_global(Store {
        notes: vec![
            Note {
                content: "Inkling 1 秒原则：从念头产生到文字落屏必须 < 1 秒，全程不切换当前应用。"
                    .into(),
            },
            Note {
                content: "桌面感应区方案：常驻透明窗口 > 鼠标轮询（零 CPU 开销）。".into(),
            },
            Note {
                content: "GSAP 物理弹性动效适合面板滑入（200ms 滑入 / 150ms 滑出）。".into(),
            },
        ],
        todos: vec![
            TodoItem {
                text: "回复设计组毛玻璃反馈".into(),
                done: false,
            },
            TodoItem {
                text: "准备版本发布清单".into(),
                done: false,
            },
            TodoItem {
                text: "给产品文档补充截图".into(),
                done: false,
            },
            TodoItem {
                text: "昨天已完成的旧任务".into(),
                done: true,
            },
        ],
        clips: vec![],
    });
}

fn store(cx: &mut App) -> &mut Store {
    if !cx.has_global::<Store>() {
        init(cx);
    }
    cx.global_mut::<Store>()
}

/// 全部笔记
pub fn notes(cx: &mut App) -> Vec<Note> {
    store(cx).notes.clone()
}

/// 全部待办
pub fn todos(cx: &mut App) -> Vec<TodoItem> {
    store(cx).todos.clone()
}

/// 剪贴板历史
pub fn clips(cx: &mut App) -> Vec<String> {
    store(cx).clips.clone()
}

/// 归档笔记（置顶插入）
pub fn add_note(cx: &mut App, content: String) {
    store(cx).notes.insert(0, Note { content });
}

/// 切换待办完成状态
pub fn toggle_todo(cx: &mut App, index: usize) {
    if let Some(item) = store(cx).todos.get_mut(index) {
        item.set_done(!item.done());
    }
}

/// 捕获剪贴板内容（置顶去重，最多保留 10 条）
pub fn push_clip(cx: &mut App, text: String) {
    let s = store(cx);
    if s.clips.first().map(|c| c == &text).unwrap_or(false) {
        return;
    }
    s.clips.insert(0, text);
    if s.clips.len() > 10 {
        s.clips.pop();
    }
}
