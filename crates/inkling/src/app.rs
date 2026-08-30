//! 主窗口：标题栏 + 侧边栏（三视图 / 设置 / 统计）+ 主内容区。
//! 对应原型 `doc/index.html` 的「Inkling 单窗口（左右结构）」布局。

use gpui::{
    actions, div, prelude::*, px, App, Context, FocusHandle, Focusable,
    FontWeight, InteractiveElement, IntoElement, KeyBinding, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Window, WindowControlArea,
};

use crate::theme::{Theme, THEMES};
use crate::views;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActiveView {
    Notes,
    Clips,
    Todos,
    Settings,
    Stats,
}

actions!(
    inkling,
    [SwitchNotes, SwitchClips, SwitchTodos, NextTheme, QuitApp]
);

pub fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("ctrl-1", SwitchNotes, None),
        KeyBinding::new("ctrl-2", SwitchClips, None),
        KeyBinding::new("ctrl-3", SwitchTodos, None),
        KeyBinding::new("ctrl-t", NextTheme, None),
    ]
}

pub struct InboxApp {
    active_view: ActiveView,
    theme_index: usize,
    focus: FocusHandle,
}

impl Focusable for InboxApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl InboxApp {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            active_view: ActiveView::Notes,
            theme_index: crate::theme::DEFAULT_THEME,
            focus: cx.focus_handle(),
        }
    }

    fn theme(&self) -> &'static Theme {
        &THEMES[self.theme_index]
    }

    fn handle_switch_notes(&mut self, _: &SwitchNotes, _: &mut Window, cx: &mut Context<Self>) {
        self.active_view = ActiveView::Notes;
        cx.notify();
    }

    fn handle_switch_clips(&mut self, _: &SwitchClips, _: &mut Window, cx: &mut Context<Self>) {
        self.active_view = ActiveView::Clips;
        cx.notify();
    }

    fn handle_switch_todos(&mut self, _: &SwitchTodos, _: &mut Window, cx: &mut Context<Self>) {
        self.active_view = ActiveView::Todos;
        cx.notify();
    }

    fn handle_next_theme(&mut self, _: &NextTheme, _: &mut Window, cx: &mut Context<Self>) {
        self.theme_index = (self.theme_index + 1) % THEMES.len();
        cx.notify();
    }

    fn handle_quit(&mut self, _: &QuitApp, _: &mut Window, cx: &mut Context<Self>) {
        cx.quit();
    }

    // ── 标题栏 ──────────────────────────────────
    fn render_titlebar(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .h(px(38.))
            .px_3()
            .bg(theme.sidebar)
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text)
                    .child("✒️ Inkling"),
            )
            // 中段空白作为窗口拖拽区（自定义无边框标题栏）
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .window_control_area(WindowControlArea::Drag),
            )
            .child(
                div()
                    .id("titlebar-close")
                    .px_2()
                    .rounded_md()
                    .cursor_pointer()
                    .text_color(theme.text_dim)
                    .hover(|s| s.bg(theme.red).text_color(theme.text))
                    .on_click(cx.listener(|_: &mut Self, _: &gpui::ClickEvent, _, cx| {
                        cx.quit();
                    }))
                    .child("✕"),
            )
    }

    // ── 侧边栏 ──────────────────────────────────
    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        div()
            .w(px(150.))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .bg(theme.sidebar)
            .border_r_1()
            .border_color(theme.border)
            .child(self.nav_item("nav-notes", "📝 笔记", ActiveView::Notes, cx))
            .child(self.nav_item("nav-clips", "📋 粘贴板", ActiveView::Clips, cx))
            .child(self.nav_item("nav-todos", "✅ 待办", ActiveView::Todos, cx))
            .child(div().flex_1())
            .child(self.nav_item("nav-settings", "⚙️ 偏好设置", ActiveView::Settings, cx))
            .child(self.nav_item("nav-stats", "📊 使用统计", ActiveView::Stats, cx))
            .child(
                div()
                    .pt_2()
                    .mt_1()
                    .border_t_1()
                    .border_color(theme.border)
                    .text_size(px(10.))
                    .text_color(theme.text_dim)
                    .child(format!("主题：{}（Ctrl+T 切换）", theme.name)),
            )
    }

    fn nav_item(
        &self,
        id: &'static str,
        label: &'static str,
        view: ActiveView,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.active_view == view;
        let theme = self.theme();
        div()
            .id(SharedString::from(id))
            .flex()
            .items_center()
            .px_2()
            .py_1()
            .rounded_md()
            .text_size(px(13.))
            .cursor_pointer()
            .when(active, |el| {
                el.bg(theme.hover)
                    .text_color(theme.text)
                    .border_1()
                    .border_color(theme.accent)
            })
            .when(!active, |el| {
                el.text_color(theme.text_dim)
                    .hover(|s| s.bg(theme.hover).text_color(theme.text))
            })
            .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                this.active_view = view;
                cx.notify();
            }))
            .child(label.to_string())
    }

    // ── 主内容区 ────────────────────────────────
    fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        div()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .child(match self.active_view {
                ActiveView::Notes => views::notes(theme).into_any_element(),
                ActiveView::Clips => views::clips(theme).into_any_element(),
                ActiveView::Todos => views::todos(theme).into_any_element(),
                ActiveView::Stats => views::stats(theme).into_any_element(),
                ActiveView::Settings => self.render_settings(cx).into_any_element(),
            })
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let mut list = div().flex().flex_col().gap_2();
        for (index, t) in THEMES.iter().enumerate() {
            let active = index == self.theme_index;
            list = list.child(
                div()
                    .id(SharedString::from(format!("theme-{index}")))
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .rounded_lg()
                    .w(px(240.))
                    .cursor_pointer()
                    .bg(theme.card)
                    .border_1()
                    .when(active, |el| el.border_color(theme.accent))
                    .when(!active, |el| el.border_color(theme.border))
                    .hover(|s| s.bg(theme.hover))
                    .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _, cx| {
                        this.theme_index = index;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .text_size(px(13.))
                            .text_color(if active { theme.accent } else { theme.text })
                            .child(t.name.to_string()),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text_dim)
                            .child(if active { "✓ 当前" } else { "" }),
                    ),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text)
                    .child("⚙️ 偏好设置"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.text_dim)
                    .child("主题（后续按原型补齐 30 套与持久化）"),
            )
            .child(list)
    }
}

impl Render for InboxApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        div()
            .id("root")
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::handle_switch_notes))
            .on_action(cx.listener(Self::handle_switch_clips))
            .on_action(cx.listener(Self::handle_switch_todos))
            .on_action(cx.listener(Self::handle_next_theme))
            .on_action(cx.listener(Self::handle_quit))
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.bg)
            .text_color(theme.text)
            .font_family("Segoe UI")
            .child(self.render_titlebar(theme, cx))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .min_h_0()
                    .child(self.render_sidebar(cx))
                    .child(self.render_content(cx)),
            )
    }
}
