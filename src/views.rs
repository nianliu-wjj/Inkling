//! 三大视图（笔记 / 粘贴板 / 待办）与统计的静态示例内容。
//! 基础阶段先以主题令牌渲染示例卡片，数据层（SQLite）在后续里程碑接入。

use gpui::{div, prelude::*, px, FontWeight, IntoElement, ParentElement, Rgba, Styled};

use crate::store::{Note, TodoItem};
use crate::theme::Theme;

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

fn note_card(t: &Theme, text: &str, tags: &[&str]) -> impl IntoElement {
    let mut tags_row = div().flex().flex_row().gap_1();
    for tag in tags {
        tags_row = tags_row.child(tag_chip(t, tag));
    }
    div()
        .flex()
        .flex_col()
        .gap_1p5()
        .p_3()
        .rounded_lg()
        .bg(t.card())
        .border_l_2()
        .border_color(t.accent())
        .child(
            div()
                .text_size(px(13.))
                .text_color(t.text())
                .child(text.to_string()),
        )
        .child(tags_row)
}

pub fn notes(t: &Theme, notes: &[Note]) -> impl IntoElement {
    let mut list = div().flex().flex_col().gap_2();
    for note in notes {
        list = list.child(note_card(t, &note.content().clone(), &[]));
    }
    div()
        .flex()
        .flex_col()
        .p_4()
        .child(section_title(t, "📝 笔记"))
        .child(list)
}

fn clip_row(t: &Theme, kind: &str, kind_color: Rgba, text: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .p_3()
        .rounded_lg()
        .mb_2()
        .bg(t.card())
        .child(
            div()
                .px_1p5()
                .py_0p5()
                .rounded_sm()
                .text_size(px(11.))
                .bg(t.hover())
                .text_color(kind_color)
                .child(kind.to_string()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(13.))
                .text_color(t.text())
                .child(text.to_string()),
        )
}

pub fn clips(t: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .p_4()
        .child(section_title(t, "📋 粘贴板"))
        .child(clip_row(
            t,
            "文本",
            t.accent(),
            "把鼠标移到屏幕顶部中央试试 —— Inkling 的核心交互",
        ))
        .child(clip_row(
            t,
            "链接",
            t.green(),
            "https://tauri.app/zh-cn/v2/guides/",
        ))
        .child(clip_row(
            t,
            "代码",
            t.gold(),
            "fn top_center(size: &PhysicalSize<u32>)",
        ))
}

fn prio_badge(t: &Theme, label: &str, color: Rgba) -> impl IntoElement {
    div()
        .px_1p5()
        .rounded_sm()
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .bg(t.hover())
        .text_color(color)
        .child(label.to_string())
}

fn todo_row(t: &Theme, prio: (&str, Rgba), text: &str, done: bool, due: &str) -> impl IntoElement {
    let mut row = div()
        .flex()
        .items_center()
        .gap_2()
        .p_3()
        .rounded_lg()
        .mb_2()
        .bg(t.card());
    // 复选框
    row = row.child(
        div()
            .w(px(14.))
            .h(px(14.))
            .rounded_sm()
            .border_1()
            .border_color(t.text_dim())
            .when(done, |el| el.bg(t.green()).border_color(t.green())),
    );
    row = row.child(prio_badge(t, prio.0, prio.1));
    row = row.child(
        div()
            .flex_1()
            .min_w_0()
            .text_size(px(13.))
            .text_color(t.text())
            .when(done, |el| {
                el.text_color(t.text_dim())
                    .child(div().line_through().child(text.to_string()))
            })
            .when(!done, |el| el.child(text.to_string())),
    );
    row = row.child(
        div()
            .text_size(px(11.))
            .text_color(t.gold())
            .child(format!("📅 {due}")),
    );
    row
}

pub fn todos(t: &Theme, todos: &[TodoItem]) -> impl IntoElement {
    let mut list = div().flex().flex_col();
    for todo in todos {
        let color = if todo.done() { t.green() } else { t.gold() };
        let label = if todo.done() { "低" } else { "中" };
        list = list.child(todo_row(
            t,
            (label, color),
            todo.text().as_str(),
            todo.done(),
            "当日",
        ));
    }
    div()
        .flex()
        .flex_col()
        .p_4()
        .child(section_title(t, "✅ 待办（当日）"))
        .child(list)
}

#[allow(dead_code)]
pub fn stats(t: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .child(section_title(t, "📊 使用统计"))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .p_4()
                .rounded_lg()
                .bg(t.card())
                .text_color(t.text_dim())
                .text_size(px(13.))
                .child("每日活跃度热力图（基础阶段占位，后续接入 SQLite 统计数据）"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .p_4()
                .rounded_lg()
                .bg(t.card())
                .text_color(t.text_dim())
                .text_size(px(13.))
                .child("近 6 个月趋势折线图（占位）"),
        )
}
