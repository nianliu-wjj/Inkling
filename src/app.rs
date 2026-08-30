//! 主窗口：标题栏 + 侧边栏（导航 / 当月热力图 / 设置统计入口）+ 主内容区。
//! 对应原型 `doc/index.html` 的「Inkling 单窗口（左右结构）」布局。

use gpui::{
    actions, div, prelude::*, px, App, ClickEvent, Context, Entity, FocusHandle, Focusable,
    FontWeight, InteractiveElement, IntoElement, KeyBinding, ParentElement, PathBuilder, Render,
    Rgba, SharedString, StatefulInteractiveElement, Styled, Window, WindowControlArea,
};

use crate::settings::{BlurClose, ClipRetention, RemarkStyle, Settings};
use crate::stats::{self, DayStat};
use crate::text_input::TextInput;
use crate::theme::{Theme, THEMES};
use crate::views;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeleteTarget {
    Note(String),
    Clip(String),
    Todo(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActiveView {
    Notes,
    Clips,
    Todos,
    Settings,
    Stats,
    Day,
}

actions!(
    inkling,
    [SwitchNotes, SwitchClips, SwitchTodos, NextTheme, QuitApp]
);

/// 步进器步进类型
#[derive(Clone, Copy)]
pub enum Step {
    /// 粘贴板自定义保留天数
    Retention(i32),
    /// 失焦延迟秒数
    BlurDelay(i32),
}

pub fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("ctrl-1", SwitchNotes, None),
        KeyBinding::new("ctrl-2", SwitchClips, None),
        KeyBinding::new("ctrl-3", SwitchTodos, None),
        KeyBinding::new("ctrl-t", NextTheme, None),
    ]
}

crate::accessors! {
    pub struct InboxApp {
        active_view: ActiveView,
        theme_index: usize,
        focus: FocusHandle,
        settings: Settings,
        theme_menu_open: bool,
        remark_menu_open: bool,
        day_detail_date: Option<String>,
        autostart_error: Option<String>,
        priority_menu_open: Option<String>,
        todo_input: Entity<TextInput>,
        todo_edit_input: Entity<TextInput>,
        todo_edit_id: Option<String>,
        todo_parent_target: Option<String>,
        search_input: Entity<TextInput>,
        search_query: String,
        delete_target: Option<DeleteTarget>,
    }
}

fn searchable_note(note: &crate::store::Note, query: &str) -> bool {
    note.content().to_lowercase().contains(query)
        || note
            .tags()
            .iter()
            .any(|tag| tag.to_lowercase().contains(query))
}

fn searchable_clip(clip: &crate::store::ClipItem, query: &str) -> bool {
    clip.content().to_lowercase().contains(query) || clip.kind().to_lowercase().contains(query)
}

fn searchable_todo(todo: &crate::store::TodoItem, query: &str) -> bool {
    todo.text().to_lowercase().contains(query)
        || todo.remark().to_lowercase().contains(query)
        || todo
            .tags()
            .iter()
            .any(|tag| tag.to_lowercase().contains(query))
}

impl Focusable for InboxApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl InboxApp {
    pub fn new(settings: Settings, cx: &mut Context<Self>) -> Self {
        let theme_index = crate::theme::theme_index_by_id(&settings.theme_id())
            .unwrap_or(crate::theme::DEFAULT_THEME);
        let todo_input = cx.new(|cx| {
            TextInput::new(
                "新增待办，回车后点击添加…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            )
        });
        let todo_edit_input = cx.new(|cx| {
            TextInput::new(
                "修改待办内容…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            )
        });
        let search_input = cx.new(|cx| {
            TextInput::new(
                "搜索文本、标签或备注…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            )
        });
        let observed_search = search_input.clone();
        cx.observe(&observed_search, |this, input, cx| {
            this.set_search_query(input.read(cx).content());
            cx.notify();
        })
        .detach();
        Self {
            active_view: ActiveView::Notes,
            theme_index,
            focus: cx.focus_handle(),
            settings,
            theme_menu_open: false,
            remark_menu_open: false,
            day_detail_date: None,
            autostart_error: None,
            priority_menu_open: None,
            todo_input,
            todo_edit_input,
            todo_edit_id: None,
            todo_parent_target: None,
            search_input,
            search_query: String::new(),
            delete_target: None,
        }
    }

    fn theme(&self) -> &'static Theme {
        &THEMES[self.theme_index()]
    }

    fn set_theme(&mut self, index: usize, cx: &mut Context<Self>) {
        self.set_theme_index(index);
        self.settings.set_theme_id(THEMES[index].id().to_string());
        self.settings.save();
        self.theme_menu_open = false;
        cx.notify();
    }

    fn handle_switch_notes(&mut self, _: &SwitchNotes, _: &mut Window, cx: &mut Context<Self>) {
        self.set_active_view(ActiveView::Notes);
        cx.notify();
    }

    fn handle_switch_clips(&mut self, _: &SwitchClips, _: &mut Window, cx: &mut Context<Self>) {
        self.set_active_view(ActiveView::Clips);
        cx.notify();
    }

    fn handle_switch_todos(&mut self, _: &SwitchTodos, _: &mut Window, cx: &mut Context<Self>) {
        self.set_active_view(ActiveView::Todos);
        cx.notify();
    }

    fn handle_next_theme(&mut self, _: &NextTheme, _: &mut Window, cx: &mut Context<Self>) {
        let next = (self.theme_index() + 1) % THEMES.len();
        self.set_theme(next, cx);
    }

    fn handle_quit(&mut self, _: &QuitApp, _: &mut Window, cx: &mut Context<Self>) {
        cx.quit();
    }

    /// 热力图格子颜色：按活跃度分档的强调色透明度
    fn level_color(&self, level: u32) -> Rgba {
        let alpha = match level {
            1 => 0x33,
            2 => 0x5C,
            3 => 0x8C,
            4 => 0xD9,
            _ => 0x14,
        };
        gpui::rgba((self.theme().accent_rgb() << 8) | alpha)
    }

    fn intensity_level(total: u32) -> u32 {
        match total {
            0 => 0,
            1..=3 => 1,
            4..=6 => 2,
            7..=11 => 3,
            _ => 4,
        }
    }

    // ── 标题栏 ──────────────────────────────────
    fn render_titlebar(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .h(px(38.))
            .px_3()
            .bg(theme.sidebar())
            .border_b_1()
            .border_color(theme.border())
            .child(
                div()
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text())
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
                    .text_color(theme.text_dim())
                    .hover(|s| s.bg(theme.red()).text_color(theme.text()))
                    .on_click(cx.listener(|_: &mut Self, _: &ClickEvent, _, cx| {
                        cx.quit();
                    }))
                    .child("✕"),
            )
    }

    // ── 侧边栏 ──────────────────────────────────
    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        div()
            .w(px(160.))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .bg(theme.sidebar())
            .border_r_1()
            .border_color(theme.border())
            .child(
                div()
                    .id("summon-panel")
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_2()
                    .py_1p5()
                    .mb_1()
                    .rounded_md()
                    .text_size(px(12.5))
                    .cursor_pointer()
                    .bg(theme.card())
                    .border_1()
                    .border_color(theme.border())
                    .text_color(theme.text())
                    .hover(|s| s.border_color(theme.accent()))
                    .on_click(|_: &ClickEvent, _, cx| {
                        crate::summon::toggle_panel(cx);
                    })
                    .child("⚡ 呼出面板")
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(theme.text_dim())
                            .child("Ctrl+Shift+Space"),
                    ),
            )
            .child(self.nav_item("nav-notes", "📝 笔记", ActiveView::Notes, cx))
            .child(self.nav_item("nav-clips", "📋 粘贴板", ActiveView::Clips, cx))
            .child(self.nav_item("nav-todos", "✅ 待办", ActiveView::Todos, cx))
            .child(div().flex_1())
            .child(self.render_mini_heatmap(cx))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .pt_2()
                    .mt_1()
                    .border_t_1()
                    .border_color(theme.border())
                    .child(self.icon_button("sb-settings", "⚙️", ActiveView::Settings, cx))
                    .child(self.icon_button("sb-stats", "📊", ActiveView::Stats, cx)),
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
                el.bg(theme.hover())
                    .text_color(theme.text())
                    .border_1()
                    .border_color(theme.accent())
            })
            .when(!active, |el| {
                el.text_color(theme.text_dim())
                    .hover(|s| s.bg(theme.hover()).text_color(theme.text()))
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.set_active_view(view);
                cx.notify();
            }))
            .child(label.to_string())
    }

    fn icon_button(
        &self,
        id: &'static str,
        icon: &'static str,
        view: ActiveView,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active = self.active_view == view;
        let theme = self.theme();
        div()
            .id(SharedString::from(id))
            .w(px(30.))
            .h(px(28.))
            .rounded_md()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(14.))
            .cursor_pointer()
            .when(active, |el| {
                el.bg(theme.hover())
                    .text_color(theme.text())
                    .border_1()
                    .border_color(theme.accent())
            })
            .when(!active, |el| {
                el.text_color(theme.text_dim())
                    .hover(|s| s.bg(theme.hover()).text_color(theme.text()))
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.set_active_view(view);
                cx.notify();
            }))
            .child(icon.to_string())
    }

    /// 侧边栏当月迷你热力图：点击某日 → 日期详情视图
    fn render_mini_heatmap(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let (grid, _days, year, month) = stats::current_month_grid();
        let mut columns = div().flex().flex_col().gap(px(2.));
        for week in grid.chunks(7) {
            let mut row = div().flex().gap(px(2.));
            for cell in week {
                match cell {
                    Some(date) => {
                        let stat = stats::day_stat(cx, date);
                        let level = Self::intensity_level(stat.total());
                        let color = self.level_color(level);
                        let selected = self.day_detail_date().as_deref() == Some(date.as_str());
                        row = row.child(
                            div()
                                .id(SharedString::from(format!("mh-{date}")))
                                .w(px(11.))
                                .h(px(11.))
                                .rounded_sm()
                                .cursor_pointer()
                                .bg(color)
                                .when(stat.overdue() > 0, |el| {
                                    el.border_1().border_color(theme.red())
                                })
                                .when(selected, |el| el.border_1().border_color(theme.accent()))
                                .hover(|s| s.opacity(0.75))
                                .on_click(cx.listener({
                                    let date = date.clone();
                                    move |this, _: &ClickEvent, _, cx| {
                                        this.set_day_detail_date(Some(date.clone()));
                                        this.active_view = ActiveView::Day;
                                        cx.notify();
                                    }
                                })),
                        );
                    }
                    None => row = row.child(div().w(px(11.)).h(px(11.))),
                }
            }
            columns = columns.child(row);
        }
        div()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .rounded_lg()
            .bg(theme.card())
            .border_1()
            .border_color(theme.border())
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(theme.text_dim())
                    .child(format!("{month}月 {year} 活跃 · 点击查当日")),
            )
            .child(columns)
    }

    // ── 主内容区 ────────────────────────────────
    fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let query = self.search_query.trim().to_lowercase();
        let notes = crate::store::notes(cx)
            .into_iter()
            .filter(|note| query.is_empty() || searchable_note(note, &query))
            .collect::<Vec<_>>();
        let clips = crate::store::clips(cx)
            .into_iter()
            .filter(|clip| query.is_empty() || searchable_clip(clip, &query))
            .collect::<Vec<_>>();
        let todos = crate::store::todos(cx)
            .into_iter()
            .filter(|todo| query.is_empty() || searchable_todo(todo, &query))
            .collect::<Vec<_>>();
        let view = match self.active_view() {
            ActiveView::Notes => {
                views::notes(theme, &notes, self.delete_target(), cx).into_any_element()
            }
            ActiveView::Clips => {
                views::clips(theme, &clips, self.delete_target(), cx).into_any_element()
            }
            ActiveView::Todos => views::todos(
                theme,
                &todos,
                self.priority_menu_open(),
                self.todo_input.clone(),
                self.todo_edit_input.clone(),
                self.todo_edit_id(),
                self.todo_parent_target(),
                self.delete_target(),
                cx,
            )
            .into_any_element(),
            ActiveView::Stats => self.render_stats(cx).into_any_element(),
            ActiveView::Settings => self.render_settings(cx).into_any_element(),
            ActiveView::Day => self.render_day_detail(cx).into_any_element(),
        };
        let archive_view = matches!(
            self.active_view(),
            ActiveView::Notes | ActiveView::Clips | ActiveView::Todos
        );
        let mut container = div().flex().flex_col().flex_1().min_h_0().overflow_hidden();
        if archive_view {
            container = container.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .pt_3()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text_dim())
                            .child("搜索归档"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .h(px(30.))
                            .rounded_md()
                            .bg(theme.card())
                            .border_1()
                            .border_color(theme.border())
                            .child(self.search_input.clone()),
                    ),
            );
        }
        container.child(view)
    }

    // ── 设置页 ──────────────────────────────────
    fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let mut rows = div().flex().flex_col();

        rows = rows.child(
            self.setting_row("主题", cx)
                .child(self.render_theme_dropdown(theme, cx)),
        );
        rows = rows.child(
            self.setting_row("失焦自动收起", cx)
                .child(self.render_blur_close(cx)),
        );
        rows = rows.child(
            self.setting_row("粘贴板保留天数", cx)
                .child(self.render_retention(cx)),
        );

        let autostart_row = self
            .setting_row("开机静默自启动", cx)
            .child(self.render_autostart_toggle(cx));
        rows = rows.child(div().child(autostart_row).child(div().when_some(
            self.autostart_error().clone(),
            |el, msg| {
                el.child(
                    div()
                        .pl(px(140.))
                        .text_size(px(11.))
                        .text_color(theme.red())
                        .child(msg),
                )
            },
        )));

        rows = rows.child(
            self.setting_row("备注展示样式", cx)
                .child(self.render_remark_dropdown(cx)),
        );
        rows = rows.child(
            self.setting_row("全局快捷键", cx).child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(13.))
                    .text_color(theme.text())
                    .child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .text_size(px(11.))
                            .bg(theme.card())
                            .border_1()
                            .border_color(theme.border())
                            .child("Ctrl + Shift + Space"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.text_dim())
                            .child("呼出面板 · 录制功能后续接入"),
                    ),
            ),
        );

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text())
                    .child("⚙️ 偏好设置"),
            )
            .child(rows)
    }

    fn setting_row(&self, label: &str, _cx: &mut Context<Self>) -> gpui::Div {
        let theme = self.theme();
        div()
            .flex()
            .items_center()
            .gap_3()
            .py_2()
            .border_b_1()
            .border_color(theme.border())
            .child(
                div()
                    .w(px(140.))
                    .flex_shrink_0()
                    .text_size(px(13.))
                    .text_color(theme.text_dim())
                    .child(label.to_string()),
            )
    }

    fn render_theme_dropdown(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let trigger = div()
            .id("theme-dd-trigger")
            .flex()
            .items_center()
            .gap_2()
            .w(px(240.))
            .px_3()
            .py_2()
            .rounded_lg()
            .cursor_pointer()
            .text_size(px(13.))
            .bg(theme.card())
            .border_1()
            .when(self.theme_menu_open(), |el| el.border_color(theme.accent()))
            .when(!self.theme_menu_open(), |el| {
                el.border_color(theme.border())
            })
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                let next = !this.theme_menu_open();
                this.set_theme_menu_open(next);
                cx.notify();
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(theme.text())
                    .child(self.theme().name().to_string()),
            )
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(theme.text_dim())
                    .child(if self.theme_menu_open() { "▲" } else { "▼" }),
            );

        let mut container = div().relative().w(px(240.)).child(trigger);
        if self.theme_menu_open() {
            let mut menu = div()
                .id("theme-dd-menu")
                .absolute()
                .top_full()
                .left_0()
                .w_full()
                .mt_1()
                .max_h(px(280.))
                .overflow_y_scroll()
                .p_1()
                .rounded_lg()
                .bg(theme.sidebar())
                .border_1()
                .border_color(theme.border())
                .shadow_lg();
            for (index, t) in THEMES.iter().enumerate() {
                let active = index == self.theme_index();
                menu = menu.child(
                    div()
                        .id(SharedString::from(format!("theme-opt-{index}")))
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_2()
                        .py_1p5()
                        .rounded_md()
                        .cursor_pointer()
                        .text_size(px(12.5))
                        .text_color(theme.text())
                        .when(active, |el| el.bg(theme.hover()))
                        .hover(|s| s.bg(theme.hover()))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.set_theme(index, cx);
                        }))
                        .child(t.name().to_string())
                        .child(div().text_size(px(11.)).text_color(theme.accent()).child(
                            if active {
                                "✓".to_string()
                            } else {
                                "".to_string()
                            },
                        )),
                );
            }
            container = container.child(gpui::deferred(menu).with_priority(1));
        }
        container
    }

    fn render_blur_close(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let mut row = div().flex().items_center().gap_1();
        for option in BlurClose::ALL {
            let active = self.settings.blur_close() == option;
            row = row.child(
                div()
                    .id(SharedString::from(format!("blur-{:?}", option)))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .text_size(px(12.))
                    .cursor_pointer()
                    .when(active, |el| {
                        el.bg(theme.hover())
                            .text_color(theme.text())
                            .border_1()
                            .border_color(theme.accent())
                    })
                    .when(!active, |el| {
                        el.text_color(theme.text_dim())
                            .border_1()
                            .border_color(theme.border())
                            .hover(|s| s.text_color(theme.text()))
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.settings.set_blur_close(option);
                        this.settings.save();
                        cx.notify();
                    }))
                    .child(option.label().to_string()),
            );
        }
        // 延迟收起：秒数可配置（1 ~ 60）
        if self.settings.blur_close() == BlurClose::Delay {
            let secs = self.settings.blur_delay_secs();
            row = row.child(self.stepper_button("blur-dec", "−", Step::BlurDelay(-1), cx));
            row = row.child(
                div()
                    .w(px(56.))
                    .py_1()
                    .rounded_md()
                    .text_size(px(13.))
                    .text_color(theme.text())
                    .bg(theme.card())
                    .border_1()
                    .border_color(theme.border())
                    .flex()
                    .justify_center()
                    .child(format!("{secs} 秒")),
            );
            row = row.child(self.stepper_button("blur-inc", "+", Step::BlurDelay(1), cx));
        }
        row
    }

    /// 备注展示样式：下拉选择
    fn render_remark_dropdown(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let current = self.settings.remark_style();
        let trigger = div()
            .id("remark-dd-trigger")
            .flex()
            .items_center()
            .gap_2()
            .w(px(240.))
            .px_3()
            .py_2()
            .rounded_lg()
            .cursor_pointer()
            .text_size(px(13.))
            .bg(theme.card())
            .border_1()
            .when(self.remark_menu_open(), |el| {
                el.border_color(theme.accent())
            })
            .when(!self.remark_menu_open(), |el| {
                el.border_color(theme.border())
            })
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                let next = !this.remark_menu_open();
                this.set_remark_menu_open(next);
                cx.notify();
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(theme.text())
                    .child(current.label().to_string()),
            )
            .child(div().text_size(px(10.)).text_color(theme.text_dim()).child(
                if self.remark_menu_open() {
                    "▲"
                } else {
                    "▼"
                },
            ));

        let mut container = div().relative().w(px(240.)).child(trigger);
        if self.remark_menu_open() {
            let mut menu = div()
                .id("remark-dd-menu")
                .absolute()
                .top_full()
                .left_0()
                .w_full()
                .mt_1()
                .rounded_lg()
                .bg(theme.sidebar())
                .border_1()
                .border_color(theme.border())
                .shadow_lg()
                .p_1();
            for option in RemarkStyle::ALL {
                let active = self.settings.remark_style() == option;
                menu = menu.child(
                    div()
                        .id(SharedString::from(format!("remark-opt-{:?}", option)))
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_2()
                        .py_1p5()
                        .rounded_md()
                        .cursor_pointer()
                        .text_size(px(12.5))
                        .text_color(theme.text())
                        .when(active, |el| el.bg(theme.hover()))
                        .hover(|s| s.bg(theme.hover()))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.settings.set_remark_style(option);
                            this.settings.save();
                            this.set_remark_menu_open(false);
                            cx.notify();
                        }))
                        .child(option.label().to_string())
                        .child(div().text_size(px(11.)).text_color(theme.accent()).child(
                            if active {
                                "✓".to_string()
                            } else {
                                "".to_string()
                            },
                        )),
                );
            }
            container = container.child(gpui::deferred(menu).with_priority(1));
        }
        container
    }

    fn render_retention(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let mut row = div().flex().items_center().gap_1();
        for option in ClipRetention::OPTIONS {
            let active = self.settings.clip_retention() == option;
            row = row.child(
                div()
                    .id(SharedString::from(format!("ret-{}", option.label())))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .text_size(px(12.))
                    .cursor_pointer()
                    .when(active, |el| {
                        el.bg(theme.hover())
                            .text_color(theme.text())
                            .border_1()
                            .border_color(theme.accent())
                    })
                    .when(!active, |el| {
                        el.text_color(theme.text_dim())
                            .border_1()
                            .border_color(theme.border())
                            .hover(|s| s.text_color(theme.text()))
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        // 切到自定义时保留既有天数（无则 30）
                        let next = match option {
                            ClipRetention::Custom(_) => match this.settings.clip_retention() {
                                ClipRetention::Custom(d) => ClipRetention::Custom(d),
                                _ => ClipRetention::Custom(30),
                            },
                            other => other,
                        };
                        this.settings.set_clip_retention(next);
                        this.settings.save();
                        cx.notify();
                    }))
                    .child(option.label().to_string()),
            );
        }
        // 自定义模式：天数步进器，「天」为单位置于框外
        if let ClipRetention::Custom(days) = self.settings.clip_retention() {
            row = row.child(self.stepper_button("ret-dec", "−", Step::Retention(-1), cx));
            row = row.child(
                div()
                    .w(px(48.))
                    .py_1()
                    .rounded_md()
                    .text_size(px(13.))
                    .text_color(theme.text())
                    .bg(theme.card())
                    .border_1()
                    .border_color(theme.border())
                    .flex()
                    .justify_center()
                    .child(days.to_string()),
            );
            row = row.child(self.stepper_button("ret-inc", "+", Step::Retention(1), cx));
            row = row.child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.text_dim())
                    .child("天"),
            );
        }
        row
    }

    /// 通用步进按钮（保留天数 / 延迟秒数）
    fn stepper_button(
        &self,
        id: &'static str,
        label: &'static str,
        step: Step,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme();
        div()
            .id(SharedString::from(id))
            .w(px(26.))
            .h(px(26.))
            .rounded_md()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .text_size(px(14.))
            .text_color(theme.text())
            .bg(theme.card())
            .border_1()
            .border_color(theme.border())
            .hover(|s| s.bg(theme.hover()))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| match step {
                Step::Retention(delta) => {
                    if let ClipRetention::Custom(d) = this.settings.clip_retention() {
                        let next = (d as i32 + delta).clamp(1, 365);
                        this.settings
                            .set_clip_retention(ClipRetention::Custom(next as u32));
                        this.settings.save();
                        cx.notify();
                    }
                }
                Step::BlurDelay(delta) => {
                    let next = (this.settings.blur_delay_secs() as i32 + delta).clamp(1, 60);
                    this.settings.set_blur_delay_secs(next as u32);
                    this.settings.save();
                    cx.notify();
                }
            }))
            .child(label.to_string())
    }

    fn render_autostart_toggle(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let on = self.settings.autostart();
        div()
            .id("autostart-toggle")
            .w(px(38.))
            .h(px(20.))
            .rounded_full()
            .p(px(2.))
            .flex()
            .cursor_pointer()
            .when(on, |el| el.bg(theme.green()).justify_end())
            .when(!on, |el| el.bg(theme.border()).justify_start())
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                let next = !this.settings.autostart();
                match Settings::apply_autostart_registry(next) {
                    Ok(()) => {
                        this.settings.set_autostart(next);
                        this.settings.save();
                        this.autostart_error = None;
                    }
                    Err(msg) => this.set_autostart_error(Some(msg)),
                }
                cx.notify();
            }))
            .child(div().size(px(16.)).rounded_full().bg(theme.text()))
    }

    // ── 统计页 ──────────────────────────────────
    fn render_stats(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let days = stats::last_days(cx, 26 * 7);

        // 全量热力图：按周分列，月份标签放在每月首列上方
        let mut columns = div().flex().flex_row().gap(px(3.));
        let mut prev_month = 0u32;
        for week in days.chunks(7) {
            let mut col = div().flex().flex_col().gap(px(3.));
            let mut label = div()
                .h(px(14.))
                .text_size(px(10.))
                .text_color(theme.text_dim());
            if let Some(first) = week.first() {
                let month = first.date()[5..7].parse::<u32>().unwrap_or(0);
                if month != prev_month {
                    label = label.child(format!("{month}月"));
                    prev_month = month;
                } else {
                    label = label.child("".to_string());
                }
            }
            col = col.child(label);
            for day in week {
                let level = Self::intensity_level(day.total());
                let color = self.level_color(level);
                let selected = self.day_detail_date().as_deref() == Some(day.date().as_str());
                col = col.child(
                    div()
                        .id(SharedString::from(format!("st-{}", day.date())))
                        .w(px(13.))
                        .h(px(13.))
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(color)
                        .when(day.overdue() > 0, |el| {
                            el.border_1().border_color(theme.red())
                        })
                        .when(selected, |el| el.border_1().border_color(theme.accent()))
                        .hover(|s| s.opacity(0.75))
                        .on_click(cx.listener({
                            let date = day.date().clone();
                            move |this, _: &ClickEvent, _, cx| {
                                this.set_day_detail_date(Some(date.clone()));
                                cx.notify();
                            }
                        })),
                );
            }
            columns = columns.child(col);
        }

        // 点击日期的详情
        let detail = self
            .day_detail_date
            .as_ref()
            .map(|date| stats::day_stat(cx, date));

        let mut container = div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text())
                    .child("📊 使用统计"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.text_dim())
                    .child("每日活跃度热力图（近 26 周 · 点击查看当日明细 · 红框 = 存在逾期）"),
            )
            .child(
                div()
                    .p_3()
                    .rounded_lg()
                    .bg(theme.card())
                    .border_1()
                    .border_color(theme.border())
                    .id("stats-heatmap")
                    .overflow_x_scroll()
                    .child(columns),
            );

        if let Some(stat) = detail {
            container = container.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .rounded_lg()
                    .bg(theme.card())
                    .border_1()
                    .border_color(theme.border())
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text())
                            .child(format!("📌 {} 的记录", stat.date())),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(theme.text())
                            .child(format!(
                                "📝 笔记 {} 条 · 📋 复制项 {} 条",
                                stat.notes(),
                                stat.clips()
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .text_size(px(12.5))
                            .child(div().text_color(theme.text()).child(format!(
                                "✅ 待办 {} 条（已完成 {}",
                                stat.todos(),
                                stat.done()
                            )))
                            .child(
                                div()
                                    .text_color(if stat.overdue() > 0 {
                                        theme.red()
                                    } else {
                                        theme.text()
                                    })
                                    .child(if stat.overdue() > 0 {
                                        format!("· 逾期 {}）", stat.overdue())
                                    } else {
                                        "）".to_string()
                                    }),
                            ),
                    ),
            );
        }

        container
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.text_dim())
                    .child("近 6 个月趋势（各模块使用量折线）"),
            )
            .child(self.render_trend(theme, cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(11.))
                    .text_color(theme.text_dim())
                    .child("少".to_string())
                    .child(
                        div()
                            .w(px(11.))
                            .h(px(11.))
                            .rounded_sm()
                            .bg(self.level_color(1)),
                    )
                    .child(
                        div()
                            .w(px(11.))
                            .h(px(11.))
                            .rounded_sm()
                            .bg(self.level_color(2)),
                    )
                    .child(
                        div()
                            .w(px(11.))
                            .h(px(11.))
                            .rounded_sm()
                            .bg(self.level_color(3)),
                    )
                    .child(
                        div()
                            .w(px(11.))
                            .h(px(11.))
                            .rounded_sm()
                            .bg(self.level_color(4)),
                    )
                    .child("多".to_string())
                    .child(
                        div()
                            .w(px(11.))
                            .h(px(11.))
                            .rounded_sm()
                            .border_1()
                            .border_color(theme.red())
                            .bg(theme.card()),
                    )
                    .child("存在逾期".to_string()),
            )
    }

    /// 近 6 个月趋势折线图（canvas 自绘，颜色跟随主题）
    fn render_trend(&self, theme: &'static Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let days = stats::last_days(cx, 26 * 7);
        // 按月聚合（最多 6 个月）
        let mut months: Vec<(String, u32, u32, u32)> = Vec::new();
        for d in &days {
            let key = d.date()[0..7].to_string();
            match months.last_mut() {
                Some(last) if last.0 == key => {
                    last.1 += d.notes();
                    last.2 += d.clips();
                    last.3 += d.todos();
                }
                _ => months.push((key, d.notes(), d.clips(), d.todos())),
            }
        }
        if months.len() > 6 {
            months = months.split_off(months.len() - 6);
        }

        let max_v = months
            .iter()
            .flat_map(|m| [m.1, m.2, m.3])
            .max()
            .unwrap_or(10)
            .max(10) as f64;

        let w = 600.0f32;
        let h = 190.0f32;
        let pad_l = 34.0;
        let pad_b = 26.0;
        let inner_w = w - pad_l - 12.0;
        let inner_h = h - pad_b - 16.0;

        let month_count = months.len().max(1) as f32;
        let x_at = |i: usize| pad_l + i as f32 * inner_w / (month_count - 1.0).max(1.0);
        let y_at = |v: f64| 16.0f32 + inner_h - (v / max_v) as f32 * inner_h;

        let line_points: Vec<Vec<(f32, f32)>> = [1usize, 2, 3]
            .iter()
            .map(|&field| {
                months
                    .iter()
                    .enumerate()
                    .map(|(i, m)| {
                        let v = match field {
                            1 => m.1,
                            2 => m.2,
                            _ => m.3,
                        } as f64;
                        (x_at(i), y_at(v) as f32)
                    })
                    .collect()
            })
            .collect();
        let labels: Vec<(f32, String)> = months
            .iter()
            .enumerate()
            .map(|(i, m)| (x_at(i), format!("{}月", m.0[5..7].trim_start_matches('0'))))
            .collect();
        let colors = [theme.red(), theme.gold(), theme.green()];
        let accent_grid = theme.border();

        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .gap_3()
                    .justify_end()
                    .text_size(px(11.))
                    .text_color(theme.text_dim())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().w(px(10.)).h(px(10.)).rounded_sm().bg(theme.red()))
                            .child("笔记"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().w(px(10.)).h(px(10.)).rounded_sm().bg(theme.gold()))
                            .child("粘贴板"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().w(px(10.)).h(px(10.)).rounded_sm().bg(theme.green()))
                            .child("待办"),
                    ),
            )
            .child(
                div()
                    .p_3()
                    .rounded_lg()
                    .bg(theme.card())
                    .border_1()
                    .border_color(theme.border())
                    .overflow_hidden()
                    .child(
                        gpui::canvas(
                            move |bounds, _window, _cx| bounds.size,
                            move |bounds, _size, window, _| {
                                let origin = bounds.origin;
                                let at = |x: f32, y: f32| {
                                    gpui::point(origin.x + px(x), origin.y + px(y))
                                };
                                // 网格线（4 条）
                                for g in 0..=3u32 {
                                    let y = 16.0 + inner_h - (g as f32 / 3.0) * inner_h;
                                    window.paint_quad(gpui::fill(
                                        gpui::Bounds::new(
                                            at(pad_l, y),
                                            gpui::size(px(w - pad_l - 12.0), px(1.0)),
                                        ),
                                        accent_grid,
                                    ));
                                }
                                // 三条折线 + 数据点
                                for (pi, pts) in line_points.iter().enumerate() {
                                    let color = colors[pi];
                                    let mut builder = PathBuilder::stroke(px(2.0));
                                    if let Some(first) = pts.first() {
                                        builder.move_to(at(first.0, first.1));
                                        for p in &pts[1..] {
                                            builder.line_to(at(p.0, p.1));
                                        }
                                    }
                                    window.paint_path(
                                        builder.build().expect("path build 失败"),
                                        color,
                                    );
                                    for p in pts {
                                        let r = 3.0;
                                        window.paint_quad(gpui::fill(
                                            gpui::Bounds::new(
                                                at(p.0 - r, p.1 - r),
                                                gpui::size(px(r * 2.0), px(r * 2.0)),
                                            ),
                                            color,
                                        ));
                                    }
                                }
                            },
                        )
                        .w(px(w))
                        .h(px(h)),
                    )
                    // 月份标签行（与折线 x 位置近似对齐）
                    .child({
                        let mut row = div().flex().h(px(16.));
                        let mut cursor = pad_l;
                        for (x, label) in &labels {
                            row = row.child(
                                div()
                                    .w(px((*x - cursor).max(1.0)))
                                    .text_size(px(10.))
                                    .text_color(theme.text_dim())
                                    .child(label.clone()),
                            );
                            cursor = *x;
                        }
                        row.child(div().flex_1())
                    }),
            )
    }

    // ── 日期详情 ────────────────────────────────
    fn render_day_detail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let stat: DayStat = self
            .day_detail_date
            .as_ref()
            .map(|d| stats::day_stat(cx, d))
            .unwrap_or_else(|| {
                let today = stats::today_str();
                stats::day_stat(cx, &today)
            });

        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_4()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text())
                    .child(format!(
                        "📌 {} 的记录{}",
                        stat.date(),
                        if stat.date() == stats::today_str() {
                            " · 今天"
                        } else {
                            ""
                        }
                    )),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.text_dim())
                    .child("来自侧边栏当月热力图 · 当前数据来自 SQLite 真实聚合"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .rounded_lg()
                    .bg(theme.card())
                    .border_1()
                    .border_color(theme.border())
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(theme.text())
                            .child(format!("📝 笔记 {} 条", stat.notes())),
                    )
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(theme.text())
                            .child(format!("📋 复制项 {} 条", stat.clips())),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .text_size(px(12.5))
                            .child(div().text_color(theme.text()).child(format!(
                                "✅ 待办 {} 条（已完成 {}",
                                stat.todos(),
                                stat.done()
                            )))
                            .child(
                                div()
                                    .text_color(if stat.overdue() > 0 {
                                        theme.red()
                                    } else {
                                        theme.text()
                                    })
                                    .child(if stat.overdue() > 0 {
                                        format!("· 逾期 {}）", stat.overdue())
                                    } else {
                                        "）".to_string()
                                    }),
                            ),
                    ),
            )
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
            .bg(theme.bg())
            .text_color(theme.text())
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
