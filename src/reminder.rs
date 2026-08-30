//! 待办提醒调度与右上角自定义提醒卡片。

use gpui::{
    div, prelude::*, px, App, AppContext, Bounds, ClickEvent, Context, FontWeight, IntoElement,
    ParentElement, Render, SharedString, Styled, Window, WindowBackgroundAppearance, WindowBounds,
    WindowKind, WindowOptions,
};

use crate::store::{self, Reminder};
use crate::theme::THEMES;

#[derive(Clone)]
struct ReminderWindow {
    key: String,
    handle: gpui::AnyWindowHandle,
}

#[derive(Default)]
struct ReminderWindows {
    windows: Vec<ReminderWindow>,
}

impl gpui::Global for ReminderWindows {}

pub struct ReminderApp {
    reminder: Reminder,
}

impl ReminderApp {
    fn new(reminder: Reminder, _cx: &mut Context<Self>) -> Self {
        Self { reminder }
    }

    fn close(&mut self, window: &mut Window, cx: &mut App) {
        let key = self.reminder.id().clone();
        window.remove_window();
        if cx.has_global::<ReminderWindows>() {
            cx.update_global::<ReminderWindows, _>(|state, _| {
                state.windows.retain(|item| item.key != key);
            });
        }
    }

    fn snooze(&mut self, minutes: u64, window: &mut Window, cx: &mut App) {
        let next = store::reminder_after(minutes * 60);
        if store::set_todo_remind_at(cx, &self.reminder.todo_id(), Some(next)) {
            self.close(window, cx);
        }
    }
}

impl Render for ReminderApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = &THEMES[crate::theme::DEFAULT_THEME];
        let todo_id = self.reminder.todo_id().clone();
        let close_id = self.reminder.id().clone();
        div()
            .id(SharedString::from(format!(
                "reminder-{}",
                self.reminder.id()
            )))
            .size_full()
            .p_3()
            .rounded_lg()
            .bg(t.card())
            .border_1()
            .border_color(t.gold())
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.text())
                            .child("⏰ 待办提醒"),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("close-reminder-{close_id}")))
                            .px_1p5()
                            .py_0p5()
                            .text_size(px(10.))
                            .text_color(t.text_dim())
                            .cursor_pointer()
                            .hover(|el| el.text_color(t.red()).bg(t.hover()))
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.close(window, cx)
                            }))
                            .child("关闭"),
                    ),
            )
            .child(
                div()
                    .mt_2()
                    .text_size(px(13.))
                    .text_color(t.text())
                    .child(self.reminder.text().clone()),
            )
            .child(
                div()
                    .mt_1()
                    .text_size(px(10.))
                    .text_color(t.text_dim())
                    .child(format!(
                        "触发于 {}",
                        store::display_timestamp(&self.reminder.trigger_at())
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(t.text_dim())
                            .child(format!("待办：{}", todo_id)),
                    )
                    .child(div().flex_1())
                    .child(snooze_button("5 分钟", 5, t, cx))
                    .child(snooze_button("15 分钟", 15, t, cx))
                    .child(snooze_button("1 小时", 60, t, cx)),
            )
    }
}

fn snooze_button(
    label: &'static str,
    minutes: u64,
    t: &'static crate::theme::Theme,
    cx: &mut Context<ReminderApp>,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("snooze-{minutes}")))
        .px_1p5()
        .py_1()
        .rounded_md()
        .text_size(px(10.))
        .text_color(t.accent())
        .cursor_pointer()
        .hover(|el| el.bg(t.hover()))
        .on_click(
            cx.listener(move |this, _: &ClickEvent, window, cx| this.snooze(minutes, window, cx)),
        )
        .child(label)
}

fn show_reminder(cx: &mut App, reminder: Reminder) {
    if !cx.has_global::<ReminderWindows>() {
        cx.set_global(ReminderWindows::default());
    }
    if cx
        .global::<ReminderWindows>()
        .windows
        .iter()
        .any(|item| item.key == *reminder.id())
    {
        return;
    }
    let display = cx
        .primary_display()
        .map(|item| item.bounds())
        .unwrap_or_else(|| {
            Bounds::new(
                gpui::point(px(0.), px(0.)),
                gpui::size(px(1920.), px(1080.)),
            )
        });
    let width = px(360.);
    let height = px(170.);
    let bounds = Bounds::new(
        gpui::point(display.right() - width - px(24.), display.top() + px(24.)),
        gpui::size(width, height),
    );
    let Some(handle) = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                focus: true,
                show: true,
                kind: WindowKind::PopUp,
                is_movable: true,
                is_resizable: false,
                window_background: WindowBackgroundAppearance::Blurred,
                app_id: Some("InklingReminder".into()),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| ReminderApp::new(reminder.clone(), cx)),
        )
        .ok()
    else {
        return;
    };
    let key = reminder.id().clone();
    cx.update_global::<ReminderWindows, _>(|state, _| {
        state.windows.push(ReminderWindow {
            key,
            handle: handle.into(),
        });
    });
}

/// 启动提醒轮询。轮询间隔较短，触发实例由 SQLite 事件表幂等记录。
pub fn init(cx: &mut App) {
    cx.spawn(async move |cx| loop {
        cx.update(|cx| {
            for reminder in store::take_due_reminders(cx) {
                show_reminder(cx, reminder);
            }
        })
        .ok();
        cx.background_executor()
            .timer(std::time::Duration::from_secs(15))
            .await;
    })
    .detach();
}
