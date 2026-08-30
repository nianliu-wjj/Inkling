//! 桌面置顶小浮窗：使用 GPUI PopUp 窗口显示被 pin 的笔记或待办。

use gpui::{
    div, prelude::*, px, App, AppContext, Bounds, ClickEvent, Context, FontWeight, IntoElement,
    ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window, WindowBackgroundAppearance, WindowBounds,
    WindowKind, WindowOptions,
};

use crate::{store, text_input::TextInput, theme::Theme};

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

crate::accessors! {
    pub struct PinnedApp {
        target: PinTarget,
        editing: bool,
        opacity: f32,
        edit_input: gpui::Entity<TextInput>,
    }
}

impl PinnedApp {
    fn new(target: PinTarget, cx: &mut Context<Self>) -> Self {
        let edit_input = cx.new(|cx| {
            TextInput::new(
                "双击后编辑内容…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            )
        });
        let initial_content = match &target {
            PinTarget::Note(id) => store::notes(cx)
                .into_iter()
                .find(|note| note.id() == *id)
                .map(|note| note.content().clone()),
            PinTarget::Todo(id) => store::todos(cx)
                .into_iter()
                .find(|todo| todo.id() == *id)
                .map(|todo| todo.text().clone()),
        };
        if let Some(content) = initial_content {
            edit_input.update(cx, |input, cx| input.set_content(content, cx));
        }
        Self {
            target,
            editing: false,
            opacity: 1.0,
            edit_input,
        }
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

    fn save_edit(&mut self, cx: &mut Context<Self>) {
        let content = self.edit_input.read(cx).content();
        let saved = match &self.target {
            PinTarget::Note(id) => {
                let tags = store::notes(cx)
                    .into_iter()
                    .find(|note| note.id() == *id)
                    .map(|note| note.tags().clone())
                    .unwrap_or_default();
                store::update_note(cx, id, content, tags)
            }
            PinTarget::Todo(id) => store::update_todo_text(cx, id, content),
        };
        if saved {
            self.set_editing(false);
            cx.notify();
        }
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
        let mut body = div().id("pinned-body").flex().flex_col().gap_2();
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
        let edit_input = self.edit_input.clone();
        let target_for_edit = self.target.clone();
        body = body
            .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                if event.click_count() >= 2 {
                    let content = match &target_for_edit {
                        PinTarget::Note(id) => store::notes(cx)
                            .into_iter()
                            .find(|note| note.id() == *id)
                            .map(|note| note.content().clone()),
                        PinTarget::Todo(id) => store::todos(cx)
                            .into_iter()
                            .find(|todo| todo.id() == *id && !todo.done())
                            .map(|todo| todo.text().clone()),
                    };
                    if let Some(content) = content {
                        edit_input.update(cx, |input, cx| input.set_content(content, cx));
                        this.set_editing(true);
                        cx.notify();
                    }
                }
            }))
            .when(self.editing(), |el| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().flex_1().min_w_0().h(px(34.)).child(self.edit_input.clone()))
                        .child(
                            div()
                                .id("save-pin-edit")
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .bg(t.accent())
                                .text_color(t.bg())
                                .text_size(px(10.))
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.save_edit(cx);
                                }))
                                .child("保存"),
                        )
                        .child(
                            div()
                                .id("cancel-pin-edit")
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .text_color(t.text_dim())
                                .text_size(px(10.))
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.set_editing(false);
                                    cx.notify();
                                }))
                                .child("取消"),
                        ),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_size(px(10.))
                    .text_color(t.text_dim())
                    .child("透明度")
                    .child({
                        let mut controls = div().flex().items_center().gap_1();
                        for value in [0.3_f32, 0.5, 0.7, 1.0] {
                            let label = format!("{}%", (value * 100.0) as u32);
                            let selected = (self.opacity() - value).abs() < f32::EPSILON;
                            controls = controls.child(
                                div()
                                    .id(SharedString::from(format!("pin-opacity-{}", label)))
                                    .px_1()
                                    .py_0p5()
                                    .rounded_sm()
                                    .text_size(px(9.))
                                    .when(selected, |el| el.bg(t.accent()).text_color(t.bg()))
                                    .when(!selected, |el| el.bg(t.hover()).text_color(t.text_dim()))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                        this.set_opacity(value);
                                        cx.notify();
                                    }))
                                    .child(label),
                            );
                        }
                        controls
                    }),
            )
            .child(self.close_button(t, cx));
        div()
            .id("pinned-window")
            .size_full()
            .p_3()
            .rounded_lg()
            .bg(t.card())
            .border_1()
            .border_color(t.accent())
            .opacity(self.opacity())
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
