//! 桌面置顶小浮窗：使用 GPUI PopUp 窗口显示被 pin 的笔记或待办。

use gpui::{
    div, prelude::*, px, App, AppContext, Bounds, ClickEvent, Context, FontWeight, IntoElement,
    ParentElement, Render, SharedString, Styled, Window, WindowBackgroundAppearance, WindowBounds,
    WindowKind, WindowOptions,
};

use crate::{store, theme::Theme};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PinTarget {
    Note(String),
    Todo(String),
}

impl PinTarget {
    fn key(&self) -> String {
        match self {
            Self::Note(id) => format!("note:{id}"),
            Self::Todo(id) => format!("todo:{id}"),
        }
    }
}

#[derive(Clone)]
struct PinWindow {
    key: String,
    handle: gpui::AnyWindowHandle,
}

#[derive(Default)]
struct PinWindows {
    windows: Vec<PinWindow>,
}

impl gpui::Global for PinWindows {}

pub struct PinnedApp {
    target: PinTarget,
}

impl PinnedApp {
    fn new(target: PinTarget, _cx: &mut Context<Self>) -> Self {
        Self { target }
    }

    fn close(&mut self, window: &mut Window, cx: &mut App) {
        let key = self.target.key();
        window.remove_window();
        if cx.has_global::<PinWindows>() {
            cx.update_global::<PinWindows, _>(|state, _| {
                state.windows.retain(|window| window.key != key);
            });
        }
    }

    fn theme(&self) -> &'static Theme {
        &crate::theme::THEMES[crate::theme::DEFAULT_THEME]
    }

    fn close_button(&self, t: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let key = self.target.key();
        div()
            .id(SharedString::from(format!("close-pin-{key}")))
            .px_2()
            .py_1()
            .rounded_sm()
            .text_size(px(10.))
            .text_color(t.text_dim())
            .cursor_pointer()
            .hover(|el| el.bg(t.hover()).text_color(t.red()))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.close(window, cx)))
            .child("关闭浮窗")
    }
}

impl Render for PinnedApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.theme();
        let mut body = div().flex().flex_col().gap_2();
        match &self.target {
            PinTarget::Note(id) => {
                if let Some(note) = store::notes(cx).into_iter().find(|note| note.id() == *id) {
                    body = body
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(t.text())
                                .child("📌 笔记"),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(t.text())
                                .child(crate::views::clip_preview(&note.content())),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(t.text_dim())
                                .child(store::display_timestamp(&note.updated_at())),
                        );
                } else {
                    body = body.child(div().text_color(t.text_dim()).child("该笔记已不存在"));
                }
            }
            PinTarget::Todo(id) => {
                if let Some(todo) = store::todos(cx).into_iter().find(|todo| todo.id() == *id) {
                    body = body
                        .child(
                            div()
                                .text_size(px(12.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(t.text())
                                .child(if todo.done() {
                                    "📌 已完成待办"
                                } else {
                                    "📌 待办"
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(if todo.done() { t.text_dim() } else { t.text() })
                                .when(todo.done(), |el| el.line_through())
                                .child(todo.text().clone()),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(if store::is_overdue(&todo) {
                                    t.red()
                                } else {
                                    t.text_dim()
                                })
                                .child(format!("📅 {}", store::display_timestamp(&todo.due_at()))),
                        );
                } else {
                    body = body.child(div().text_color(t.text_dim()).child("该待办已不存在"));
                }
            }
        }
        body = body.child(self.close_button(t, cx));
        div()
            .id("pinned-window")
            .size_full()
            .p_3()
            .rounded_lg()
            .bg(t.card())
            .border_1()
            .border_color(t.accent())
            .child(body)
    }
}

pub fn show(cx: &mut App, target: PinTarget) {
    let key = target.key();
    if !cx.has_global::<PinWindows>() {
        cx.set_global(PinWindows::default());
    }
    let already_open = cx
        .global::<PinWindows>()
        .windows
        .iter()
        .find(|window| window.key == key)
        .map(|window| window.handle);
    if let Some(handle) = already_open {
        let _ = handle.update(cx, |_, window, _| window.activate_window());
        return;
    }

    let display = cx
        .primary_display()
        .map(|display| display.bounds())
        .unwrap_or_else(|| {
            Bounds::new(
                gpui::point(px(0.), px(0.)),
                gpui::size(px(1920.), px(1080.)),
            )
        });
    let width = px(260.);
    let height = px(150.);
    let bounds = Bounds::new(
        gpui::point(
            display.right() - width - px(24.),
            display.bottom() - height - px(48.),
        ),
        gpui::size(width, height),
    );
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                focus: false,
                show: true,
                kind: WindowKind::PopUp,
                is_movable: true,
                is_resizable: false,
                window_background: WindowBackgroundAppearance::Blurred,
                app_id: Some("InklingPinned".into()),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| PinnedApp::new(target, cx)),
        )
        .ok();
    if let Some(handle) = handle {
        cx.update_global::<PinWindows, _>(|state, _| {
            state.windows.push(PinWindow {
                key,
                handle: handle.into(),
            });
        });
    }
}
