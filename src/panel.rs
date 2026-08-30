//! 呼出面板：屏幕顶部中央滑入的三态快捕面板（笔记 / 粘贴板 / 待办）。
//!
//! - `WindowKind::PopUp` 置顶无边框窗口 + 失焦自动收起（按设置策略：立即 / 延迟 N 秒 / 永不）
//! - 滑入动画（200ms ease-out-quint + 淡入）
//! - 打开时自动捕获当前剪贴板（置顶去重）
//! - `Esc` 或失焦收起

use gpui::{
    actions, div, prelude::*, px, rgba, AnimationExt, App, ClickEvent, Context, Entity, Focusable,
    FontWeight, IntoElement, ParentElement, Render, Rgba, SharedString, Styled, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
};

use crate::settings::{BlurClose, Settings};
use crate::store;
use crate::text_input::TextInput;
use crate::theme::Theme;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PanelMode {
    Notes,
    Clips,
    Todos,
}

actions!(panel, [ClosePanel]);

pub struct PanelApp {
    mode: PanelMode,
    note_input: Entity<TextInput>,
    blur_at: Option<std::time::Instant>,
    settings: Settings,
}

impl Focusable for PanelApp {
    fn focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        self.note_input.focus_handle(cx)
    }
}

impl PanelApp {
    pub fn new(settings: Settings, cx: &mut Context<Self>) -> Self {
        let draft = store::draft(cx);
        // 打开面板：捕获当前剪贴板（置顶去重）
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            store::push_clip(cx, text);
        }
        let note_input = cx.new(|cx| {
            TextInput::new(
                "此刻在想什么？直接写下来…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            )
        });
        if !draft.is_empty() {
            note_input.update(cx, |input, cx| input.set_content(draft, cx));
        }
        Self {
            mode: PanelMode::Notes,
            note_input,
            blur_at: None,
            settings,
        }
    }

    fn close(&mut self, window: &mut Window, cx: &mut App) {
        let content = self.note_input.read(cx).content();
        if !content.trim().is_empty() {
            store::save_draft(cx, content);
        }
        crate::summon::close_panel(window, cx);
    }

    // ── 圆点导航 ────────────────────────────────
    fn mode_dot(
        &self,
        id: &'static str,
        icon: &'static str,
        mode: PanelMode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme();
        let active = self.mode == mode;
        let dot_color = match mode {
            PanelMode::Notes => gpui::rgb(0xFF6B81),
            PanelMode::Clips => gpui::rgb(0xFFD76E),
            PanelMode::Todos => gpui::rgb(0x7EE0A8),
        };
        div()
            .id(SharedString::from(id))
            .text_size(px(12.))
            .cursor_pointer()
            .px_1()
            .rounded_md()
            .when(active, |el| el.bg(theme.hover()).opacity(1.0))
            .when(!active, |el| el.opacity(0.45))
            .hover(|s| s.opacity(0.85))
            .text_color(dot_color)
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.mode = mode;
                cx.notify();
            }))
            .child(icon.to_string())
    }

    fn theme(&self) -> &'static Theme {
        let index = crate::theme::theme_index_by_id(&self.settings.theme_id())
            .unwrap_or(crate::theme::DEFAULT_THEME);
        &crate::theme::THEMES[index]
    }

    // ── 失焦自动收起（策略来自设置） ────────────
    fn check_blur_close(&mut self, active: bool, window: &mut Window, cx: &mut Context<Self>) {
        let now = std::time::Instant::now();
        match self.settings.blur_close() {
            BlurClose::Immediate => {
                if !active {
                    self.close(window, cx);
                }
            }
            BlurClose::Delay => {
                if !active {
                    match self.blur_at {
                        None => self.blur_at = Some(now),
                        Some(at) => {
                            if (now - at).as_secs() >= self.settings.blur_delay_secs() as u64 {
                                self.close(window, cx);
                            }
                        }
                    }
                } else {
                    self.blur_at = None;
                }
            }
            BlurClose::Never => {}
        }
    }
}

fn two_line_preview(text: &str) -> String {
    const MAX_CHARS_PER_LINE: usize = 96;
    let mut lines = text
        .lines()
        .take(3)
        .map(|line| {
            let mut value = line.chars().take(MAX_CHARS_PER_LINE).collect::<String>();
            if line.chars().count() > MAX_CHARS_PER_LINE {
                value.push('…');
            }
            value
        })
        .collect::<Vec<_>>();
    let truncated = lines.len() > 2;
    lines.truncate(2);
    if truncated {
        if let Some(last) = lines.last_mut() {
            last.push('…');
        }
    }
    if lines.is_empty() {
        "（空内容）".into()
    } else {
        lines.join("\n")
    }
}

impl Render for PanelApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 失焦收起检查（每帧）
        let active = window.is_window_active();
        self.check_blur_close(active, window, cx);

        let panel_h = 380.0f32;
        let theme = self.theme();

        let mut content = div().flex().flex_col().flex_1().min_h_0().gap_2();
        match self.mode {
            PanelMode::Notes => {
                content = content.child(self.note_input.clone()).child(
                    div().flex().justify_end().child(
                        div()
                            .id("archive-note")
                            .px_3()
                            .py_1p5()
                            .rounded_md()
                            .text_size(px(12.5))
                            .cursor_pointer()
                            .font_weight(FontWeight::SEMIBOLD)
                            .bg(theme.accent())
                            .text_color(gpui::rgb(0x151826FF))
                            .hover(|s| s.opacity(0.85))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                let content = this.note_input.read(cx).content();
                                if content.trim().is_empty() {
                                    return;
                                }
                                store::add_note(cx, content);
                                this.note_input.update(cx, |input, cx| input.clear(cx));
                                this.close(window, cx);
                            }))
                            .child("归档念头 ↵"),
                    ),
                );
            }
            PanelMode::Clips => {
                let clips = store::clips(cx);
                let mut list = div().flex().flex_col().gap_1p5();
                if clips.is_empty() {
                    list = list.child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.text_dim())
                            .child("暂无剪贴板历史 · 复制任意内容后面板会自动捕获"),
                    );
                }
                for (index, clip) in clips.iter().enumerate() {
                    let clip_text = clip.content().clone();
                    let clip = clip.content().clone();
                    list = list.child(
                        div()
                            .id(SharedString::from(format!("clip-{index}")))
                            .flex()
                            .items_center()
                            .p_2()
                            .rounded_lg()
                            .bg(theme.card())
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.hover()))
                            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                    clip_text.clone(),
                                ));
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .text_color(theme.text())
                                    .whitespace_normal()
                                    .child(two_line_preview(&clip)),
                            ),
                    );
                }
                content = content.child(
                    div()
                        .id("panel-clips")
                        .flex()
                        .flex_col()
                        .gap_2()
                        .max_h(px(260.))
                        .overflow_y_scroll()
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.text_dim())
                                .child("点击条目写回剪贴板 · 打开面板自动捕获最新内容"),
                        )
                        .child(list),
                );
            }
            PanelMode::Todos => {
                let todos = store::todos(cx);
                let mut list = div().flex().flex_col().gap_1p5();
                for todo in todos.iter() {
                    let todo = todo.clone();
                    let todo_id = todo.id().clone();
                    list = list.child(
                        div()
                            .id(SharedString::from(format!("pt-{}", todo.id())))
                            .flex()
                            .items_center()
                            .gap_2()
                            .p_2()
                            .rounded_lg()
                            .bg(theme.card())
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.hover()))
                            .when(!todo.done(), |el| {
                                el.on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                                    store::complete_todo(cx, &todo_id);
                                    cx.notify();
                                }))
                            })
                            .child(
                                div()
                                    .w(px(14.))
                                    .h(px(14.))
                                    .rounded_sm()
                                    .border_1()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(10.))
                                    .when(todo.done(), |el| {
                                        el.bg(theme.green())
                                            .border_color(theme.green())
                                            .text_color(gpui::rgb(0x151826FF))
                                            .child("✓".to_string())
                                    })
                                    .when(!todo.done(), |el| el.border_color(theme.text_dim())),
                            )
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .text_color(theme.text())
                                    .when(todo.done(), |el| {
                                        el.text_color(theme.text_dim())
                                            .line_through()
                                            .child(todo.text().clone())
                                    })
                                    .when(!todo.done(), |el| el.child(todo.text().clone())),
                            ),
                    );
                }
                content = content.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.text_dim())
                                .child("点击条目切换完成状态 · 新增待办在主窗口管理"),
                        )
                        .child(list),
                );
            }
        }

        div()
            .id("panel-root")
            .key_context("Panel")
            .on_action(cx.listener(|this, _: &ClosePanel, window, cx| {
                this.close(window, cx);
            }))
            .w(px(480.))
            .h(px(panel_h))
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .rounded_b_lg()
            .bg(rgba(0x1B1F30E6))
            .border_1()
            .border_color(rgba(0x32364AFF))
            .shadow_lg()
            .text_color(theme.text())
            .font_family("Segoe UI")
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(self.mode_dot("dot-notes", "🔴", PanelMode::Notes, cx))
                    .child(self.mode_dot("dot-clips", "🟡", PanelMode::Clips, cx))
                    .child(self.mode_dot("dot-todos", "🟢", PanelMode::Todos, cx))
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(10.))
                            .text_color(theme.text_dim())
                            .child("Esc 收起"),
                    ),
            )
            .child(content)
            .with_animation(
                "panel-slide",
                gpui::Animation::new(std::time::Duration::from_millis(200))
                    .with_easing(gpui::ease_out_quint()),
                move |el, delta| el.mt(px(-(1.0 - delta) * 28.0)).opacity(delta),
            )
    }
}
