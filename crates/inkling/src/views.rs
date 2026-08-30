//! 三大视图（笔记 / 粘贴板 / 待办）与统计的静态示例内容。
//! 基础阶段先以主题令牌渲染示例卡片，数据层（SQLite）在后续里程碑接入。

use gpui::{div, prelude::*, px, FontWeight, IntoElement, ParentElement, Rgba, Styled};

use crate::theme::Theme;

fn section_title(t: &Theme, s: &str) -> impl IntoElement {
    div()
        .text_size(px(15.))
        .font_weight(FontWeight::SEMIBOLD)
        .pb_2()
        .text_color(t.text)
        .child(s.to_string())
}

fn tag_chip(t: &Theme, label: &str) -> impl IntoElement {
    div()
        .px_1p5()
        .py_0p5()
        .rounded_sm()
        .text_size(px(11.))
        .bg(t.hover)
        .text_color(t.accent)
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
        .bg(t.card)
        .border_l_2()
        .border_color(t.accent)
        .child(
            div()
                .text_size(px(13.))
                .text_color(t.text)
                .child(text.to_string()),
        )
        .child(tags_row)
}

pub fn notes(t: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_4()
        .child(section_title(t, "📝 笔记"))
        .child(note_card(
            t,
            "Inkling 1 秒原则：从念头产生到文字落屏必须 < 1 秒，全程不切换当前应用。",
            &["产品", "核心原则"],
        ))
        .child(note_card(
            t,
            "桌面感应区方案：常驻透明窗口 > 鼠标轮询（零 CPU 开销）。",
            &["架构", "性能"],
        ))
        .child(note_card(
            t,
            "GSAP 物理弹性动效适合面板滑入（200ms 滑入 / 150ms 滑出）。",
            &["灵感"],
        ))
}

fn clip_row(t: &Theme, kind: &str, kind_color: Rgba, text: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .p_3()
        .rounded_lg()
        .mb_2()
        .bg(t.card)
        .child(
            div()
                .px_1p5()
                .py_0p5()
                .rounded_sm()
                .text_size(px(11.))
                .bg(t.hover)
                .text_color(kind_color)
                .child(kind.to_string()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(13.))
                .text_color(t.text)
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
            t.accent,
            "把鼠标移到屏幕顶部中央试试 —— Inkling 的核心交互",
        ))
        .child(clip_row(
            t,
            "链接",
            t.green,
            "https://tauri.app/zh-cn/v2/guides/",
        ))
        .child(clip_row(
            t,
            "代码",
            t.gold,
            "fn top_center(size: &PhysicalSize<u32>)",
        ))
}

fn prio_badge(t: &Theme, label: &str, color: Rgba) -> impl IntoElement {
    div()
        .px_1p5()
        .rounded_sm()
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .bg(t.hover)
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
        .bg(t.card);
    // 复选框
    row = row.child(
        div()
            .w(px(14.))
            .h(px(14.))
            .rounded_sm()
            .border_1()
            .border_color(t.text_dim)
            .when(done, |el| el.bg(t.green).border_color(t.green)),
    );
    row = row.child(prio_badge(t, prio.0, prio.1));
    row = row.child(
        div()
            .flex_1()
            .min_w_0()
            .text_size(px(13.))
            .text_color(t.text)
            .when(done, |el| {
                el.text_color(t.text_dim)
                    .child(div().line_through().child(text.to_string()))
            })
            .when(!done, |el| el.child(text.to_string())),
    );
    row = row.child(
        div()
            .text_size(px(11.))
            .text_color(t.gold)
            .child(format!("📅 {due}")),
    );
    row
}

pub fn todos(t: &Theme) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .p_4()
        .child(section_title(t, "✅ 待办（当日）"))
        .child(
            div()
                .px_2()
                .py_1()
                .mb_2()
                .rounded_md()
                .text_size(px(11.))
                .bg(t.hover)
                .text_color(t.red)
                .child("⚠️ 逾期事项 · 按完成时间与优先级置顶（示例）"),
        )
        .child(todo_row(
            t,
            ("低", t.green),
            "回复设计组毛玻璃反馈",
            false,
            "8/29 18:00",
        ))
        .child(todo_row(
            t,
            ("中", t.gold),
            "准备版本发布清单",
            false,
            "8/31 18:00",
        ))
        .child(todo_row(
            t,
            ("高", t.red),
            "给产品文档补充截图",
            false,
            "今天 12:00",
        ))
        .child(todo_row(
            t,
            ("低", t.green),
            "昨天已完成的旧任务",
            true,
            "昨天 20:00",
        ))
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
                .bg(t.card)
                .text_color(t.text_dim)
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
                .bg(t.card)
                .text_color(t.text_dim)
                .text_size(px(13.))
                .child("近 6 个月趋势折线图（占位）"),
        )
}
