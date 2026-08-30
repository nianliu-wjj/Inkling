//! 主窗口归档视图：使用共享 Store 的真实数据渲染。

use gpui::{
    div, prelude::*, px, ClickEvent, Context, Entity, FontWeight, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled,
};

use crate::{
    app::{DeleteTarget, InboxApp},
    store::{self, ClipItem, Note, Priority, TodoItem},
    text_input::TextInput,
    theme::Theme,
};

fn open_external_link(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

fn section_title(t: &Theme, s: &str) -> impl IntoElement {
    div()
        .text_size(px(15.))
        .font_weight(FontWeight::SEMIBOLD)
        .pb_2()
        .text_color(t.text())
        .child(s.to_string())
}

fn tag_chip(t: &Theme, label: &str) -> impl IntoElement {
    div()
        .px_1p5()
        .py_0p5()
        .rounded_sm()
        .text_size(px(11.))
        .bg(t.hover())
        .text_color(t.accent())
        .child(format!("#{label}"))
}

fn delete_controls(
    t: &Theme,
    target: DeleteTarget,
    confirmed: bool,
    cx: &mut Context<InboxApp>,
) -> impl IntoElement {
    let request_target = target.clone();
    let confirm_target = target.clone();
    let cancel_target = target.clone();
    let target_key = match &target {
        DeleteTarget::Note(id) => format!("note-{id}"),
        DeleteTarget::Clip(id) => format!("clip-{id}"),
        DeleteTarget::Todo(id) => format!("todo-{id}"),
    };
    let mut controls = div()
        .absolute()
        .right_2()
        .flex()
        .items_center()
        .gap_1()
        .text_size(px(10.));
    if confirmed {
        controls = controls
            .bottom_full()
            .mb_1()
            .child(
                div()
                    .id(SharedString::from(format!("delete-confirm-{target_key}")))
                    .px_1p5()
                    .py_0p5()
                    .rounded_sm()
                    .bg(t.red())
                    .text_color(t.bg())
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        let deleted = match &confirm_target {
                            DeleteTarget::Note(id) => store::delete_note(cx, id),
                            DeleteTarget::Clip(id) => store::delete_clip(cx, id),
                            DeleteTarget::Todo(id) => store::delete_todo(cx, id),
                        };
                        if deleted {
                            this.set_delete_target(None);
                            cx.notify();
                        }
                    }))
                    .child("确认删除"),
            )
            .child(
                div()
                    .id(SharedString::from(format!("delete-cancel-{target_key}")))
                    .px_1p5()
                    .py_0p5()
                    .rounded_sm()
                    .bg(t.hover())
                    .text_color(t.text_dim())
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if this.delete_target() == Some(cancel_target.clone()) {
                            this.set_delete_target(None);
                            cx.notify();
                        }
                    }))
                    .child("取消"),
            );
    } else {
        controls = controls.top_2().child(
            div()
                .id(SharedString::from(format!("delete-request-{target_key}")))
                .px_1p5()
                .py_0p5()
                .rounded_sm()
                .text_color(t.text_dim())
                .cursor_pointer()
                .hover(|s| s.bg(t.red()).text_color(t.text()))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.set_delete_target(Some(request_target.clone()));
                    cx.notify();
                }))
                .child("✕"),
        );
    }
    controls
}

pub(crate) fn clip_preview(text: &str) -> String {
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

fn note_card(
    t: &Theme,
    note: &Note,
    delete_target: Option<DeleteTarget>,
    edit_input: Entity<TextInput>,
    edit_tags_input: Entity<TextInput>,
    edit_id: Option<String>,
    cx: &mut Context<InboxApp>,
) -> impl IntoElement {
    let mut metadata = div()
        .flex()
        .items_center()
        .gap_1()
        .text_size(px(10.))
        .text_color(t.text_dim())
        .child(format!(
            "归档 · {}",
            store::display_timestamp(&note.created_at())
        ));
    for tag in note.tags().iter().take(3) {
        metadata = metadata.child(tag_chip(t, tag));
    }
    let confirmed = delete_target == Some(DeleteTarget::Note(note.id().clone()));
    let note_id = note.id().clone();
    let editing = edit_id.as_deref() == Some(note.id().as_str());
    let edit_id_for_click = note.id().clone();
    let edit_content_for_click = note.content().clone();
    let edit_tags_for_click = note.tags().join(", ");
    let edit_input_for_click = edit_input.clone();
    let edit_tags_input_for_click = edit_tags_input.clone();
    let mut actions = div().flex().items_center().justify_end().gap_1();
    if !editing {
        actions = actions.child(
            div()
                .id(SharedString::from(format!("edit-note-{}", note.id())))
                .px_1p5()
                .py_0p5()
                .rounded_sm()
                .text_size(px(10.))
                .text_color(t.accent())
                .cursor_pointer()
                .hover(|s| s.bg(t.hover()))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    edit_input_for_click.update(cx, |input, cx| {
                        input.set_content(edit_content_for_click.clone(), cx)
                    });
                    edit_tags_input_for_click.update(cx, |input, cx| {
                        input.set_content(edit_tags_for_click.clone(), cx)
                    });
                    this.set_note_edit_id(Some(edit_id_for_click.clone()));
                    cx.notify();
                }))
                .child("✏️ 编辑"),
        );
    }
    let pin_id_for_click = note.id().clone();
    actions = actions.child(
        div()
            .id(SharedString::from(format!("pin-note-{}", note.id())))
            .px_1p5()
            .py_0p5()
            .rounded_sm()
            .text_size(px(10.))
            .text_color(t.accent())
            .cursor_pointer()
            .hover(|s| s.bg(t.hover()))
            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                crate::pin::show(cx, crate::pin::PinTarget::Note(pin_id_for_click.clone()));
            }))
            .child("📌 置顶"),
    );
    let content = if editing {
        let save_id = note_id.clone();
        let save_input = edit_input.clone();
        let save_tags_input = edit_tags_input.clone();
        let cancel_input = edit_input.clone();
        let cancel_tags_input = edit_tags_input.clone();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().min_h(px(72.)).child(edit_input))
            .child(div().h(px(30.)).child(edit_tags_input))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id(SharedString::from(format!("save-note-{}", note.id())))
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(t.accent())
                            .text_color(t.bg())
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                let content = save_input.read(cx).content();
                                let tags = save_tags_input
                                    .read(cx)
                                    .content()
                                    .split(',')
                                    .map(|value| value.trim().to_string())
                                    .filter(|value| !value.is_empty())
                                    .collect();
                                if store::update_note(cx, &save_id, content, tags) {
                                    this.set_note_edit_id(None);
                                    save_input.update(cx, |input, cx| input.clear(cx));
                                    save_tags_input.update(cx, |input, cx| input.clear(cx));
                                    cx.notify();
                                }
                            }))
                            .child("保存"),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("cancel-note-{}", note.id())))
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .text_color(t.text_dim())
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.set_note_edit_id(None);
                                cancel_input.update(cx, |input, cx| input.clear(cx));
                                cancel_tags_input.update(cx, |input, cx| input.clear(cx));
                                cx.notify();
                            }))
                            .child("取消"),
                    ),
            )
            .into_any_element()
    } else {
        div()
            .text_size(px(13.))
            .text_color(t.text())
            .child(render_markdown_lite(&note.content(), t))
            .into_any_element()
    };
    div()
        .relative()
        .flex()
        .flex_col()
        .gap_1p5()
        .p_3()
        .rounded_lg()
        .bg(t.card())
        .border_l_2()
        .border_color(t.accent())
        .child(delete_controls(
            t,
            DeleteTarget::Note(note.id().clone()),
            confirmed,
            cx,
        ))
        .child(content)
        .child(metadata)
        .child(actions)
}

fn render_markdown_lite(text: &str, t: &Theme) -> impl IntoElement {
    // GPUI 当前没有 DOM Markdown 渲染器；先提供安全的轻量展示：按行保留换行，
    // 并用次要色标识常见代码行，避免把原始内容误当 HTML。
    let mut wrapper = div().flex().flex_col().gap_1();
    for line in text.lines() {
        wrapper = wrapper.child(
            div()
                .when(line.trim_start().starts_with("```"), |el| {
                    el.text_color(t.accent()).font_weight(FontWeight::SEMIBOLD)
                })
                .child(line.to_string()),
        );
    }
    wrapper
}

pub fn notes(
    t: &Theme,
    notes: &[Note],
    delete_target: Option<DeleteTarget>,
    edit_input: Entity<TextInput>,
    edit_tags_input: Entity<TextInput>,
    edit_id: Option<String>,
    cx: &mut Context<InboxApp>,
) -> impl IntoElement {
    let mut list = div().flex().flex_col().gap_2();
    if notes.is_empty() {
        list = list.child(
            div()
                .text_color(t.text_dim())
                .text_size(px(12.))
                .child("还没有归档笔记，请从顶部呼出面板开始记录。"),
        );
    }
    for note in notes {
        list = list.child(note_card(
            t,
            note,
            delete_target.clone(),
            edit_input.clone(),
            edit_tags_input.clone(),
            edit_id.clone(),
            cx,
        ));
    }
    div()
        .id("notes-view")
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .p_4()
        .child(section_title(t, "📝 笔记"))
        .child(list)
}

fn clip_row(
    t: &Theme,
    clip: &ClipItem,
    delete_target: Option<DeleteTarget>,
    edit_input: Entity<TextInput>,
    edit_id: Option<String>,
    cx: &mut Context<InboxApp>,
) -> impl IntoElement {
    let text = clip.content().clone();
    let clip_id = clip.id().clone();
    let favorite_id = clip.id().clone();
    let confirmed = delete_target == Some(DeleteTarget::Clip(clip.id().clone()));
    let favorite = clip.favorite();
    let editing = edit_id.as_deref() == Some(clip.id().as_str());
    let mut row = div()
        .id(SharedString::from(format!("clip-{clip_id}")))
        .relative()
        .flex()
        .items_center()
        .gap_2()
        .p_3()
        .rounded_lg()
        .mb_2()
        .bg(t.card())
        .cursor_pointer()
        .hover(|s| s.bg(t.hover()))
        .child(delete_controls(
            t,
            DeleteTarget::Clip(clip.id().clone()),
            confirmed,
            cx,
        ))
        .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.clone()));
            cx.notify();
        }))
        .child(
            div()
                .px_1p5()
                .py_0p5()
                .rounded_sm()
                .text_size(px(11.))
                .bg(t.hover())
                .text_color(t.accent())
                .child(clip.kind().clone()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(13.))
                .text_color(t.text())
                .child(clip_preview(&clip.content())),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(t.text_dim())
                .child(store::display_timestamp(&clip.captured_at())),
        );
    if !editing {
        let edit_id_for_click = clip.id().clone();
        let edit_content_for_click = clip.content().clone();
        let edit_input_for_click = edit_input.clone();
        row = row.child(
            div()
                .id(SharedString::from(format!("edit-clip-{}", clip.id())))
                .px_1()
                .py_0p5()
                .rounded_sm()
                .text_size(px(11.))
                .text_color(t.accent())
                .cursor_pointer()
                .hover(|s| s.bg(t.hover()))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    edit_input_for_click.update(cx, |input, cx| {
                        input.set_content(edit_content_for_click.clone(), cx)
                    });
                    this.set_clip_edit_id(Some(edit_id_for_click.clone()));
                    cx.notify();
                }))
                .child("✏️"),
        );
    }
    row = row.child(
        div()
            .id(SharedString::from(format!("favorite-clip-{}", favorite_id)))
            .px_1()
            .py_0p5()
            .rounded_sm()
            .text_size(px(13.))
            .text_color(if favorite { t.gold() } else { t.text_dim() })
            .cursor_pointer()
            .hover(|s| s.bg(t.hover()).text_color(t.gold()))
            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                store::set_clip_favorite(cx, &favorite_id, !favorite);
                cx.notify();
            }))
            .child(if favorite { "★" } else { "☆" }),
    );
    if clip.kind() == "link" {
        let url = clip.content().clone();
        row = row.child(
            div()
                .id(SharedString::from(format!("open-clip-{}", clip.id())))
                .px_1()
                .py_0p5()
                .rounded_sm()
                .text_size(px(11.))
                .text_color(t.accent())
                .cursor_pointer()
                .hover(|s| s.bg(t.hover()))
                .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                    let _ = open_external_link(&url);
                    cx.notify();
                }))
                .child("↗"),
        );
    }
    row = row.child(div().text_size(px(10.)).text_color(t.text_dim()).child(
        if clip.kind() == "link" {
            "可打开链接"
        } else {
            "点击复制"
        },
    ));
    if editing {
        let save_id = clip.id().clone();
        let save_input = edit_input.clone();
        let cancel_input = edit_input.clone();
        row = row.flex_col().items_start().child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().flex_1().min_w_0().h(px(32.)).child(edit_input))
                .child(
                    div()
                        .id(SharedString::from(format!("save-clip-{}", clip.id())))
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(t.accent())
                        .text_color(t.bg())
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            let content = save_input.read(cx).content();
                            if store::update_clip_content(cx, &save_id, content) {
                                this.set_clip_edit_id(None);
                                save_input.update(cx, |input, cx| input.clear(cx));
                                cx.notify();
                            }
                        }))
                        .child("保存"),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("cancel-clip-{}", clip.id())))
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .text_color(t.text_dim())
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.set_clip_edit_id(None);
                            cancel_input.update(cx, |input, cx| input.clear(cx));
                            cx.notify();
                        }))
                        .child("取消"),
                ),
        );
    }
    row
}

pub fn clips(
    t: &Theme,
    clips: &[ClipItem],
    delete_target: Option<DeleteTarget>,
    edit_input: Entity<TextInput>,
    edit_id: Option<String>,
    cx: &mut Context<InboxApp>,
) -> impl IntoElement {
    let mut list = div().flex().flex_col();
    if clips.is_empty() {
        list = list.child(
            div()
                .text_color(t.text_dim())
                .text_size(px(12.))
                .child("暂无剪贴板历史，复制任意内容后会自动捕获。"),
        );
    }
    for clip in clips {
        list = list.child(clip_row(
            t,
            clip,
            delete_target.clone(),
            edit_input.clone(),
            edit_id.clone(),
            cx,
        ));
    }
    div()
        .id("clips-view")
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .p_4()
        .child(section_title(t, "📋 粘贴板"))
        .child(list)
}

fn priority_badge(t: &Theme, todo: &TodoItem, cx: &mut Context<InboxApp>) -> impl IntoElement {
    let priority = todo.priority();
    let id = todo.id().clone();
    let color = match priority {
        Priority::High => t.red(),
        Priority::Medium => t.gold(),
        Priority::Low => t.green(),
    };
    div()
        .id(SharedString::from(format!("priority-{}", todo.id())))
        .px_1p5()
        .py_0p5()
        .rounded_sm()
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .bg(t.hover())
        .text_color(color)
        .cursor_pointer()
        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
            if this.priority_menu_open() == Some(id.clone()) {
                this.set_priority_menu_open(None);
            } else {
                this.set_priority_menu_open(Some(id.clone()));
            }
            cx.notify();
        }))
        .child(format!(
            "{} {}",
            if matches!(priority, Priority::High) {
                "●"
            } else {
                "○"
            },
            priority.label()
        ))
}

fn priority_menu(t: &Theme, todo: &TodoItem, cx: &mut Context<InboxApp>) -> impl IntoElement {
    let id = todo.id().clone();
    let mut menu = div()
        .id(SharedString::from(format!("priority-menu-{}", todo.id())))
        .absolute()
        .top_full()
        .left_0()
        .mt_1()
        .flex()
        .gap_1()
        .p_1()
        .rounded_md()
        .bg(t.sidebar())
        .border_1()
        .border_color(t.border())
        .shadow_lg();
    for priority in [Priority::High, Priority::Medium, Priority::Low] {
        let selected = priority == todo.priority();
        let item_id = id.clone();
        menu = menu.child(
            div()
                .id(SharedString::from(format!(
                    "prio-{}-{}",
                    id,
                    priority.as_str()
                )))
                .px_2()
                .py_1()
                .rounded_sm()
                .text_size(px(11.))
                .cursor_pointer()
                .text_color(match priority {
                    Priority::High => t.red(),
                    Priority::Medium => t.gold(),
                    Priority::Low => t.green(),
                })
                .when(selected, |el| {
                    el.bg(t.hover()).font_weight(FontWeight::SEMIBOLD)
                })
                .hover(|s| s.bg(t.hover()))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    if store::set_priority(cx, &item_id, priority) {
                        this.set_priority_menu_open(None);
                        cx.notify();
                    }
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().w(px(7.)).h(px(7.)).rounded_full().bg(match priority {
                            Priority::High => t.red(),
                            Priority::Medium => t.gold(),
                            Priority::Low => t.green(),
                        }))
                        .child(priority.label())
                        .child(if selected { "✓" } else { "" }),
                ),
        );
    }
    menu
}

fn todo_meta_editor(
    t: &Theme,
    todo: &TodoItem,
    content_input: Entity<TextInput>,
    due_input: Entity<TextInput>,
    remind_input: Entity<TextInput>,
    repeat_input: Entity<TextInput>,
    tags_input: Entity<TextInput>,
    remark_input: Entity<TextInput>,
    cx: &mut Context<InboxApp>,
) -> impl IntoElement {
    let todo_id = todo.id().clone();
    let original_todo = todo.clone();
    let save_content = content_input.clone();
    let save_due = due_input.clone();
    let save_remind = remind_input.clone();
    let save_repeat = repeat_input.clone();
    let save_tags = tags_input.clone();
    let save_remark = remark_input.clone();
    let cancel_inputs = [
        content_input.clone(),
        due_input.clone(),
        remind_input.clone(),
        repeat_input.clone(),
        tags_input.clone(),
        remark_input.clone(),
    ];
    div()
        .mt_2()
        .p_2()
        .rounded_md()
        .bg(t.bg())
        .border_1()
        .border_color(t.border())
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t.text())
                .child("待办详情"),
        )
        .child(div().h(px(30.)).child(content_input))
        .child(div().h(px(30.)).child(due_input))
        .child(div().h(px(30.)).child(remind_input))
        .child(div().h(px(30.)).child(repeat_input))
        .child(div().h(px(30.)).child(tags_input))
        .child(div().h(px(30.)).child(remark_input))
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .id(SharedString::from(format!("save-meta-todo-{todo_id}")))
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(t.accent())
                        .text_color(t.bg())
                        .text_size(px(10.))
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            let mut candidate = original_todo.clone();
                            let content = save_content.read(cx).content().trim().to_string();
                            let due = save_due.read(cx).content().trim().to_string();
                            let remind = save_remind.read(cx).content().trim().to_string();
                            let repeat = save_repeat.read(cx).content().trim().to_lowercase();
                            let repeat = match repeat.as_str() {
                                "daily" | "weekly" => Some(repeat),
                                "" | "none" | "null" => None,
                                _ => return,
                            };
                            let tags = save_tags
                                .read(cx)
                                .content()
                                .split(',')
                                .map(|value| value.trim().to_string())
                                .filter(|value| !value.is_empty())
                                .collect();
                            let remark = save_remark.read(cx).content().chars().take(200).collect();
                            candidate.set_text(content);
                            candidate.set_due_at(due);
                            candidate.set_remind_at((!remind.is_empty()).then_some(remind));
                            candidate.set_repeat_rule(repeat);
                            candidate.set_tags(tags);
                            candidate.set_remark(remark);
                            if store::update_todo(cx, &candidate) {
                                this.set_todo_meta_id(None);
                                cx.notify();
                            }
                        }))
                        .child("保存详情"),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("cancel-meta-todo-{todo_id}")))
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .text_size(px(10.))
                        .text_color(t.text_dim())
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.set_todo_meta_id(None);
                            for input in cancel_inputs.iter() {
                                input.update(cx, |input, cx| input.clear(cx));
                            }
                            cx.notify();
                        }))
                        .child("取消"),
                ),
        )
}

fn todo_row(
    t: &Theme,
    todo: &TodoItem,
    menu_open: Option<String>,
    depth: usize,
    delete_target: Option<DeleteTarget>,
    edit_input: Entity<TextInput>,
    edit_id: Option<String>,
    meta_id: Option<String>,
    meta_due_input: Entity<TextInput>,
    meta_remind_input: Entity<TextInput>,
    meta_repeat_input: Entity<TextInput>,
    meta_tags_input: Entity<TextInput>,
    meta_remark_input: Entity<TextInput>,
    cx: &mut Context<InboxApp>,
) -> impl IntoElement {
    let done = todo.done();
    let overdue = store::is_overdue(todo);
    let item_id = todo.id().clone();
    let completion_id = item_id.clone();
    let confirmed = delete_target == Some(DeleteTarget::Todo(item_id.clone()));
    let mut tags = div().flex().gap_1();
    for tag in todo.tags().iter().take(3) {
        tags = tags.child(tag_chip(t, tag));
    }
    let mut bottom = div()
        .flex()
        .items_center()
        .gap_1()
        .child(tags)
        .child(
            div()
                .text_size(px(10.))
                .text_color(if overdue { t.red() } else { t.text_dim() })
                .child(format!("📅 {}", store::display_timestamp(&todo.due_at()))),
        )
        .when(todo.remind_at().is_some(), |el| {
            el.child(div().text_size(px(10.)).text_color(t.gold()).child("⏰"))
        })
        .when(todo.repeat_rule().is_some(), |el| {
            el.child(
                div()
                    .text_size(px(10.))
                    .text_color(t.accent())
                    .child(if todo.repeat_rule().as_deref() == Some("daily") {
                        "🔁 每天"
                    } else {
                        "🔁 每周"
                    }),
            )
        })
        .child(div().flex_1());
    if todo.parent_id().is_none() {
        let parent_id = todo.id().clone();
        bottom = bottom.child(
            div()
                .id(SharedString::from(format!("add-child-{}", todo.id())))
                .px_1p5()
                .py_0p5()
                .rounded_sm()
                .text_size(px(10.))
                .text_color(t.accent())
                .cursor_pointer()
                .hover(|s| s.bg(t.hover()))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.set_todo_parent_target(Some(parent_id.clone()));
                    cx.notify();
                }))
                .child("＋子任务"),
        );
    }
    if !done {
        let edit_id_for_click = item_id.clone();
        let edit_text_for_click = todo.text().clone();
        let edit_input_for_click = edit_input.clone();
        bottom = bottom.child(
            div()
                .id(SharedString::from(format!("edit-todo-{}", todo.id())))
                .px_1p5()
                .py_0p5()
                .rounded_sm()
                .text_size(px(10.))
                .text_color(t.accent())
                .cursor_pointer()
                .hover(|s| s.bg(t.hover()))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    edit_input_for_click.update(cx, |input, cx| {
                        input.set_content(edit_text_for_click.clone(), cx)
                    });
                    this.set_todo_edit_id(Some(edit_id_for_click.clone()));
                    cx.notify();
                }))
                .child("✏️ 编辑"),
        );
    }
    let pin_id_for_click = item_id.clone();
    bottom = bottom.child(
        div()
            .id(SharedString::from(format!("pin-todo-{}", todo.id())))
            .px_1p5()
            .py_0p5()
            .rounded_sm()
            .text_size(px(10.))
            .text_color(t.accent())
            .cursor_pointer()
            .hover(|s| s.bg(t.hover()))
            .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                crate::pin::show(cx, crate::pin::PinTarget::Todo(pin_id_for_click.clone()));
            }))
            .child("📌 置顶"),
    );
    if !done {
        let meta_id_for_click = item_id.clone();
        let meta_content = edit_input.clone();
        let meta_due = meta_due_input.clone();
        let meta_remind = meta_remind_input.clone();
        let meta_repeat = meta_repeat_input.clone();
        let meta_tags = meta_tags_input.clone();
        let meta_remark = meta_remark_input.clone();
        let current_text = todo.text().clone();
        let current_due = todo.due_at().clone();
        let current_remind = todo.remind_at().clone().unwrap_or_default();
        let current_repeat = todo.repeat_rule().clone().unwrap_or_default();
        let current_tags = todo.tags().join(", ");
        let current_remark = todo.remark().clone();
        bottom = bottom.child(
            div()
                .id(SharedString::from(format!("detail-todo-{}", todo.id())))
                .px_1p5()
                .py_0p5()
                .rounded_sm()
                .text_size(px(10.))
                .text_color(t.accent())
                .cursor_pointer()
                .hover(|s| s.bg(t.hover()))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    meta_content
                        .update(cx, |input, cx| input.set_content(current_text.clone(), cx));
                    meta_due.update(cx, |input, cx| input.set_content(current_due.clone(), cx));
                    meta_remind.update(cx, |input, cx| {
                        input.set_content(current_remind.clone(), cx)
                    });
                    meta_repeat.update(cx, |input, cx| {
                        input.set_content(current_repeat.clone(), cx)
                    });
                    meta_tags.update(cx, |input, cx| input.set_content(current_tags.clone(), cx));
                    meta_remark.update(cx, |input, cx| {
                        input.set_content(current_remark.clone(), cx)
                    });
                    this.set_todo_meta_id(Some(meta_id_for_click.clone()));
                    cx.notify();
                }))
                .child("⚙ 详情"),
        );
    }
    if let Some(remark) = (!todo.remark().is_empty()).then(|| todo.remark()) {
        bottom = bottom.child(div().text_size(px(10.)).text_color(t.text_dim()).child(
            if remark.len() > 100 {
                "📝".to_string()
            } else {
                remark
            },
        ));
    }
    let mut row = div()
        .id(SharedString::from(format!("todo-{}", todo.id())))
        .when(depth > 0, |el| el.ml(px((depth * 18) as f32)))
        .relative()
        .flex()
        .flex_col()
        .gap_1()
        .p_3()
        .rounded_lg()
        .mb_2()
        .bg(t.card())
        .when(overdue, |el| el.border_1().border_color(t.red()));
    if !done {
        row = row.child(delete_controls(
            t,
            DeleteTarget::Todo(item_id.clone()),
            confirmed,
            cx,
        ));
    }
    let priority_control = div()
        .relative()
        .child(if done {
            div()
                .px_1p5()
                .py_0p5()
                .rounded_sm()
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .bg(t.hover())
                .text_color(match todo.priority() {
                    Priority::High => t.red(),
                    Priority::Medium => t.gold(),
                    Priority::Low => t.green(),
                })
                .child(format!(
                    "{} {}",
                    if matches!(todo.priority(), Priority::High) {
                        "●"
                    } else {
                        "○"
                    },
                    todo.priority().label()
                ))
                .into_any_element()
        } else {
            priority_badge(t, todo, cx).into_any_element()
        })
        .when(
            !done && menu_open.as_deref() == Some(item_id.as_str()),
            |el| el.child(priority_menu(t, todo, cx)),
        );
    row = row.child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .id(SharedString::from(format!("check-{}", todo.id())))
                    .w(px(16.))
                    .h(px(16.))
                    .rounded_sm()
                    .border_1()
                    .border_color(if done { t.green() } else { t.text_dim() })
                    .when(done, |el| el.bg(t.green()).text_color(t.bg()).child("✓"))
                    .when(!done, |el| {
                        el.cursor_pointer().on_click(cx.listener(
                            move |_, _: &ClickEvent, _, cx| {
                                if store::complete_todo(cx, &completion_id) {
                                    cx.notify();
                                }
                            },
                        ))
                    }),
            )
            .child(priority_control)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(13.))
                    .text_color(if done { t.text_dim() } else { t.text() })
                    .when(done, |el| {
                        el.child(div().line_through().child(todo.text().clone()))
                    })
                    .when(!done, |el| el.child(todo.text().clone())),
            ),
    );
    row = row.child(bottom);
    if !done && edit_id.as_deref() == Some(item_id.as_str()) {
        let save_id = item_id.clone();
        let save_input = edit_input.clone();
        let cancel_input = edit_input.clone();
        row = row.child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().flex_1().h(px(30.)).child(edit_input.clone()))
                .child(
                    div()
                        .id(SharedString::from(format!("save-todo-{}", item_id)))
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(t.accent())
                        .text_color(t.bg())
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            let content = save_input.read(cx).content();
                            if store::update_todo_text(cx, &save_id, content) {
                                this.set_todo_edit_id(None);
                                save_input.update(cx, |input, cx| input.clear(cx));
                                cx.notify();
                            }
                        }))
                        .child("保存"),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("cancel-todo-{}", item_id)))
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .text_color(t.text_dim())
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.set_todo_edit_id(None);
                            cancel_input.update(cx, |input, cx| input.clear(cx));
                            cx.notify();
                        }))
                        .child("取消"),
                ),
        );
    }
    if !done && meta_id.as_deref() == Some(item_id.as_str()) {
        row = row.child(todo_meta_editor(
            t,
            todo,
            edit_input,
            meta_due_input,
            meta_remind_input,
            meta_repeat_input,
            meta_tags_input,
            meta_remark_input,
            cx,
        ));
    }
    row
}

pub fn todos(
    t: &Theme,
    todos: &[TodoItem],
    menu_open: Option<String>,
    todo_input: Entity<TextInput>,
    edit_input: Entity<TextInput>,
    edit_id: Option<String>,
    meta_id: Option<String>,
    meta_due_input: Entity<TextInput>,
    meta_remind_input: Entity<TextInput>,
    meta_repeat_input: Entity<TextInput>,
    meta_tags_input: Entity<TextInput>,
    meta_remark_input: Entity<TextInput>,
    parent_target: Option<String>,
    delete_target: Option<DeleteTarget>,
    cx: &mut Context<InboxApp>,
) -> impl IntoElement {
    let mut grouped_overdue = std::collections::HashSet::new();
    fn mark_overdue_group(
        id: &str,
        by_parent: &std::collections::HashMap<Option<String>, Vec<String>>,
        grouped: &mut std::collections::HashSet<String>,
    ) {
        if !grouped.insert(id.to_string()) {
            return;
        }
        if let Some(children) = by_parent.get(&Some(id.to_string())) {
            for child in children {
                mark_overdue_group(child, by_parent, grouped);
            }
        }
    }
    let mut by_parent: std::collections::HashMap<Option<String>, Vec<String>> =
        std::collections::HashMap::new();
    for todo in todos {
        by_parent
            .entry(todo.parent_id().clone())
            .or_default()
            .push(todo.id().clone());
    }
    for todo in todos.iter().filter(|todo| store::is_overdue(todo)) {
        let mut current = Some(todo.id().clone());
        while let Some(id) = current {
            mark_overdue_group(&id, &by_parent, &mut grouped_overdue);
            current = todos
                .iter()
                .find(|item| item.id() == id)
                .and_then(|item| item.parent_id().clone());
        }
    }
    let mut ordered = Vec::new();
    fn flatten(
        parent: Option<&str>,
        depth: usize,
        todos: &[TodoItem],
        grouped: &std::collections::HashSet<String>,
        ordered: &mut Vec<(TodoItem, usize, bool)>,
    ) {
        let mut children: Vec<TodoItem> = todos
            .iter()
            .filter(|todo| todo.parent_id().as_deref() == parent)
            .cloned()
            .collect();
        children.sort_by_key(|todo| {
            (
                if grouped.contains(&todo.id()) { 0 } else { 1 },
                todo.priority().rank(),
                todo.due_at().clone(),
                todo.created_at().clone(),
            )
        });
        for todo in children {
            let overdue_group = grouped.contains(&todo.id());
            ordered.push((todo.clone(), depth, overdue_group));
            let todo_id = todo.id().clone();
            flatten(Some(&todo_id), depth + 1, todos, grouped, ordered);
        }
    }
    flatten(None, 0, todos, &grouped_overdue, &mut ordered);
    let mut overdue_list = div().flex().flex_col();
    let mut normal_list = div().flex().flex_col();
    let mut has_overdue = false;
    for (todo, depth, overdue_group) in ordered {
        if overdue_group {
            has_overdue = true;
            overdue_list = overdue_list.child(todo_row(
                t,
                &todo,
                menu_open.clone(),
                depth,
                delete_target.clone(),
                edit_input.clone(),
                edit_id.clone(),
                meta_id.clone(),
                meta_due_input.clone(),
                meta_remind_input.clone(),
                meta_repeat_input.clone(),
                meta_tags_input.clone(),
                meta_remark_input.clone(),
                cx,
            ));
        } else {
            normal_list = normal_list.child(todo_row(
                t,
                &todo,
                menu_open.clone(),
                depth,
                delete_target.clone(),
                edit_input.clone(),
                edit_id.clone(),
                meta_id.clone(),
                meta_due_input.clone(),
                meta_remind_input.clone(),
                meta_repeat_input.clone(),
                meta_tags_input.clone(),
                meta_remark_input.clone(),
                cx,
            ));
        }
    }
    let add_input = todo_input.clone();
    let selected_parent = parent_target.clone();
    let target_label = parent_target
        .as_ref()
        .and_then(|parent| todos.iter().find(|todo| todo.id() == *parent))
        .map(|todo| todo.text());
    let add_bar = div()
        .flex()
        .items_center()
        .gap_2()
        .mb_3()
        .child(div().flex_1().min_w_0().child(todo_input))
        .child(
            div().text_size(px(10.)).text_color(t.accent()).child(
                target_label
                    .map(|label| format!("子任务 → {label}"))
                    .unwrap_or_else(|| "默认 1 小时后到期".into()),
            ),
        )
        .child(
            div()
                .id("add-todo")
                .px_3()
                .py_1p5()
                .rounded_md()
                .bg(t.accent())
                .text_color(t.bg())
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(|s| s.opacity(0.8))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    let text = add_input.read(cx).content();
                    if store::add_todo(cx, text, store::default_due_at(), selected_parent.clone())
                        .is_some()
                    {
                        add_input.update(cx, |input, cx| input.clear(cx));
                        this.set_todo_parent_target(None);
                        cx.notify();
                    }
                }))
                .child("＋新增待办"),
        );
    let mut body = div()
        .id("todos-view")
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .p_4()
        .child(section_title(t, "✅ 待办"))
        .child(add_bar);
    if has_overdue {
        body = body
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(t.red())
                    .child("逾期事项"),
            )
            .child(overdue_list);
    }
    body.child(
        div()
            .text_size(px(12.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(t.text_dim())
            .child("计划事项"),
    )
    .child(normal_list)
}
