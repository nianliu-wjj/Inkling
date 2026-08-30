//! 主题令牌系统 — 与原型 `doc/styles.css` 的 30 套主题令牌保持同源设计。
//! 基础阶段先移植 3 套（深色 / 浅色 / 霓虹未来），其余主题按同一结构补齐。

use std::sync::LazyLock;

use gpui::{rgba, Rgba};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Theme {
    pub name: &'static str,
    /// 窗口背景
    pub bg: Rgba,
    /// 侧边栏 / 面板底色
    pub sidebar: Rgba,
    /// 卡片底色
    pub card: Rgba,
    /// 悬浮 / 选中底色
    pub hover: Rgba,
    /// 边框
    pub border: Rgba,
    /// 主文字
    pub text: Rgba,
    /// 次要文字
    pub text_dim: Rgba,
    /// 强调色（选中态 / 链接）
    pub accent: Rgba,
    /// 提醒 / 金色标记
    pub gold: Rgba,
    /// 完成 / 低优先级
    pub green: Rgba,
    /// 逾期 / 删除
    pub red: Rgba,
}

pub static THEMES: LazyLock<[Theme; 3]> = LazyLock::new(|| [dark(), light(), neon()]);
pub const DEFAULT_THEME: usize = 0;

/// 深色（默认）
fn dark() -> Theme {
    Theme {
        name: "深色",
        bg: rgba(0x151826FF),
        sidebar: rgba(0x1B1F30FF),
        card: rgba(0x232735FF),
        hover: rgba(0x2C3046FF),
        border: rgba(0x32364AFF),
        text: rgba(0xEBEBF0FF),
        text_dim: rgba(0x8B8FA3FF),
        accent: rgba(0x6C8CFFFF),
        gold: rgba(0xFFD76EFF),
        green: rgba(0x7EE0A8FF),
        red: rgba(0xFF8A8AFF),
    }
}

/// 浅色
fn light() -> Theme {
    Theme {
        name: "浅色",
        bg: rgba(0xE9EDF8FF),
        sidebar: rgba(0xF4F6FBFF),
        card: rgba(0xFFFFFFFF),
        hover: rgba(0xE2E7F2FF),
        border: rgba(0xC9D0E0FF),
        text: rgba(0x151D2EFF),
        text_dim: rgba(0x5A6478FF),
        accent: rgba(0x4C68E0FF),
        gold: rgba(0x9A6A00FF),
        green: rgba(0x12805CFF),
        red: rgba(0xD64545FF),
    }
}

/// 霓虹未来
fn neon() -> Theme {
    Theme {
        name: "霓虹未来",
        bg: rgba(0x0D0720FF),
        sidebar: rgba(0x1A0E3CFF),
        card: rgba(0x241352FF),
        hover: rgba(0x2E1A62FF),
        border: rgba(0xA78BFA38),
        text: rgba(0xECE0FFFF),
        text_dim: rgba(0xA99BD0FF),
        accent: rgba(0x22D3EEFF),
        gold: rgba(0xFDE047FF),
        green: rgba(0x4ADE80FF),
        red: rgba(0xFB7185FF),
    }
}
