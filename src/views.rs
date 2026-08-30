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
        .top_2()
        .right_2()
        .flex()
        .items_center()
        .gap_1()
        .text_size(px(10.));
    if confirmed {
        controls = controls
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
        controls = controls.child(
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

fn note_card(
    t: &Theme,
    note: &Note,
    delete_target: Option<DeleteTarget>,
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
        .child(
            div()
                .text_size(px(13.))
                .text_color(t.text())
                .child(render_markdown_lite(&note.content(), t)),
        )
        .child(metadata)
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
        list = list.child(note_card(t, note, delete_target.clone(), cx));
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
    cx: &mut Context<InboxApp>,
) -> impl IntoElement {
    let text = clip.content().clone();
    let confirmed = delete_target == Some(DeleteTarget::Clip(clip.id().clone()));
    div()
        .id(SharedString::from(format!("clip-{}", clip.id())))
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
                .child(two_line_preview(&clip.content())),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(t.text_dim())
                .child(store::display_timestamp(&clip.captured_at())),
        )
}

pub fn clips(
    t: &Theme,
    clips: &[ClipItem],
    delete_target: Option<DeleteTarget>,
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
        list = list.child(clip_row(t, clip, delete_target.clone(), cx));
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
                .child(format!(
                    "{} {}{}",
                    priority.label(),
                    if selected { "✓" } else { "" },
                    if selected { "" } else { "" }
                )),
        );
    }
    menu
}

fn todo_row(
    t: &Theme,
    todo: &TodoItem,
    menu_open: Option<String>,
    depth: usize,
    delete_target: Option<DeleteTarget>,
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
        .child(div().flex_1());
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
    row = row.child(delete_controls(
        t,
        DeleteTarget::Todo(item_id.clone()),
        confirmed,
        cx,
    ));
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
    if !done && menu_open.as_deref() == Some(item_id.as_str()) {
        row = row.child(priority_menu(t, todo, cx));
    }
    row
}

pub fn todos(
    t: &Theme,
    todos: &[TodoItem],
    menu_open: Option<String>,
    todo_input: Entity<TextInput>,
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
                cx,
            ));
        } else {
            normal_list = normal_list.child(todo_row(
                t,
                &todo,
                menu_open.clone(),
                depth,
                delete_target.clone(),
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
                    .unwrap_or_else(|| "默认明日到期".into()),
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
