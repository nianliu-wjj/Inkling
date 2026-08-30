//! 主窗口：标题栏 + 侧边栏（导航 / 当月热力图 / 设置统计入口）+ 主内容区。
//! 对应原型 `doc/index.html` 的「Inkling 单窗口（左右结构）」布局。

use gpui::{
    actions, div, prelude::*, px, AnyView, App, ClickEvent, Context, Entity, FocusHandle, Focusable,
    FontWeight, InteractiveElement, IntoElement, KeyBinding, KeyDownEvent, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder, Render,
    Rgba, SharedString, StatefulInteractiveElement, Styled, Window, WindowControlArea,
};

use crate::settings::{BlurClose, ClipRetention, RemarkStyle, Settings};
use crate::stats::{self, DayStat};
use crate::text_input::TextInput;
use crate::theme::{Theme, THEMES};
use crate::views;

#[derive(Copy, Clone)]
pub struct MainWindowGlobal {
    pub handle: gpui::AnyWindowHandle,
}

impl gpui::Global for MainWindowGlobal {}

struct HeatmapTooltip {
    text: SharedString,
    theme: Theme,
}

impl Render for HeatmapTooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_2()
            .rounded_md()
            .bg(self.theme.card())
            .border_1()
            .border_color(self.theme.border())
            .text_size(px(11.))
            .text_color(self.theme.text())
            .child(self.text.clone())
    }
}

fn heatmap_tooltip(cx: &mut App, theme: Theme, stat: &DayStat) -> AnyView {
    let text = format!(
        "{} {} · 总计 {} 条 · 笔记 {} · 复制项 {} · 待办 {} · 已完成 {} · 逾期 {}",
        stat.date(),
        stats::weekday_name(&stat.date()),
        stat.total(),
        stat.notes(),
        stat.clips(),
        stat.todos(),
        stat.done(),
        stat.overdue()
    );
    cx.new(|_| HeatmapTooltip {
        text: text.into(),
        theme,
    })
    .into()
}

/// 打开或激活唯一主窗口，并切换到指定视图。
pub fn show_main_window(cx: &mut App, view: ActiveView) {
    if let Some(handle) = cx
        .try_global::<MainWindowGlobal>()
        .and_then(|global| global.handle.downcast::<InboxApp>())
    {
        if handle
            .update(cx, |app, window, cx| {
                app.set_active_view(view);
                window.activate_window();
                window.focus(&app.focus_handle(cx));
                cx.notify();
            })
            .is_ok()
        {
            cx.activate(true);
            return;
        }
    }

    open_main_window(cx, Settings::load(), view);
}

/// 创建主窗口并登记句柄，供托盘和静默自启动模式复用。
pub fn open_main_window(cx: &mut App, settings: Settings, view: ActiveView) {
    let bounds = gpui::Bounds::centered(None, gpui::size(px(880.), px(680.)), cx);
    let handle = cx
        .open_window(
            gpui::WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Inkling".into()),
                    appears_transparent: true,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| InboxApp::new(settings.clone(), cx)),
        )
        .expect("打开主窗口失败");

    handle
        .update(cx, |app, window, cx| {
            app.set_active_view(view);
            window.focus(&app.focus_handle(cx));
            window.activate_window();
        })
        .ok();
    cx.set_global(MainWindowGlobal {
        handle: handle.into(),
    });
    cx.activate(true);
}

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
        day_filter: String,
        autostart_error: Option<String>,
        priority_menu_open: Option<String>,
        todo_input: Entity<TextInput>,
        todo_edit_input: Entity<TextInput>,
        todo_edit_id: Option<String>,
        todo_meta_id: Option<String>,
        todo_meta_due_input: Entity<TextInput>,
        todo_meta_remind_input: Entity<TextInput>,
        todo_meta_repeat_input: Entity<TextInput>,
        todo_meta_tags_input: Entity<TextInput>,
        todo_meta_remark_input: Entity<TextInput>,
        note_edit_input: Entity<TextInput>,
        note_edit_tags_input: Entity<TextInput>,
        note_edit_id: Option<String>,
        clip_edit_input: Entity<TextInput>,
        clip_edit_id: Option<String>,
        todo_parent_target: Option<String>,
        search_input: Entity<TextInput>,
        search_query: String,
        delete_target: Option<DeleteTarget>,
        sidebar_collapsed: bool,
        sidebar_width: f32,
        sidebar_dragging: bool,
        shortcut_input: Entity<TextInput>,
        shortcut_recording: bool,
        shortcut_error: Option<String>,
        export_status: Option<String>,
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
        let sidebar_width = settings.sidebar_width().clamp(110, 280) as f32;
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
        let todo_meta_due_input = cx.new(|cx| {
            TextInput::new(
                "计划完成时间（YYYY-MM-DD HH:MM）…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            )
        });
        let todo_meta_remind_input = cx.new(|cx| {
            TextInput::new(
                "提醒时间（YYYY-MM-DD HH:MM，可留空）…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            )
        });
        let todo_meta_repeat_input = cx.new(|cx| {
            TextInput::new(
                "重复提醒：daily / weekly，可留空…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            )
        });
        let todo_meta_tags_input = cx.new(|cx| {
            TextInput::new(
                "标签，用逗号分隔，最多 3 个…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            )
        });
        let todo_meta_remark_input = cx.new(|cx| {
            TextInput::new(
                "备注，最多 200 字…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            )
        });
        let note_edit_input = cx.new(|cx| {
            TextInput::new(
                "编辑笔记内容…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            )
        });
        let clip_edit_input = cx.new(|cx| {
            TextInput::new(
                "编辑剪贴板内容…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            )
        });
        let note_edit_tags_input = cx.new(|cx| {
            TextInput::new(
                "标签，用逗号分隔，最多 3 个…",
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
        let shortcut_input = cx.new(|cx| {
            let mut input = TextInput::new(
                "例如 Ctrl+Shift+Space…",
                gpui::hsla(0.0, 0.0, 1.0, 0.35),
                gpui::hsla(0.65, 0.08, 0.95, 1.0),
                cx,
            );
            input.set_content(settings.global_shortcut().clone(), cx);
            input
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
            day_filter: "all".into(),
            autostart_error: None,
            priority_menu_open: None,
            todo_input,
            todo_edit_input,
            todo_edit_id: None,
            todo_meta_id: None,
            todo_meta_due_input,
            todo_meta_remind_input,
            todo_meta_repeat_input,
            todo_meta_tags_input,
            todo_meta_remark_input,
            note_edit_input,
            note_edit_tags_input,
            note_edit_id: None,
            clip_edit_input,
            clip_edit_id: None,
            todo_parent_target: None,
            search_input,
            search_query: String::new(),
            delete_target: None,
            sidebar_collapsed: false,
            sidebar_width,
            sidebar_dragging: false,
            shortcut_input,
            shortcut_recording: false,
            shortcut_error: None,
            export_status: None,
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
    fn sidebar_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left {
            self.set_sidebar_dragging(true);
            if self.sidebar_collapsed() {
                self.set_sidebar_collapsed(false);
                self.set_sidebar_width(160.);
            }
            cx.notify();
        }
    }

    fn sidebar_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.sidebar_dragging() {
            return;
        }
        let width = f32::from(event.position.x);
        if width < 110.0 {
            self.set_sidebar_collapsed(true);
        } else {
            self.set_sidebar_collapsed(false);
            self.set_sidebar_width(width.clamp(110.0, 280.0));
        }
        cx.notify();
    }

    fn sidebar_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.sidebar_dragging() {
            self.set_sidebar_dragging(false);
            if !self.sidebar_collapsed() {
                self.settings.set_sidebar_width(self.sidebar_width().round() as u32);
                self.settings.save();
            }
            cx.notify();
        }
    }

    fn render_sidebar_resizer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        div()
            .id("sidebar-resizer")
            .w(px(6.))
            .h_full()
            .flex_shrink_0()
            .cursor_pointer()
            .bg(theme.bg())
            .hover(|el| el.bg(theme.accent()))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::sidebar_mouse_down))
            .on_mouse_move(cx.listener(Self::sidebar_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::sidebar_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::sidebar_mouse_up))
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let collapsed = self.sidebar_collapsed();
        let mut sidebar = div()
            .w(px(if collapsed { 52. } else { self.sidebar_width() }))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .bg(theme.sidebar())
            .border_r_1()
            .border_color(theme.border());
        let summon = div()
            .id("summon-panel")
            .flex()
            .items_center()
            .justify_center()
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
            .child(if collapsed { "⚡" } else { "⚡ 呼出面板" });
        sidebar = sidebar.child(summon);
        sidebar = sidebar
            .child(self.nav_item("nav-notes", "📝 笔记", ActiveView::Notes, cx))
            .child(self.nav_item("nav-clips", "📋 粘贴板", ActiveView::Clips, cx))
            .child(self.nav_item("nav-todos", "✅ 待办", ActiveView::Todos, cx))
            .child(div().flex_1());
        if !collapsed {
            sidebar = sidebar.child(self.render_mini_heatmap(cx));
        }
        let collapse_id = if collapsed {
            "sidebar-expand"
        } else {
            "sidebar-collapse"
        };
        let collapse_icon = if collapsed { "»" } else { "«" };
        sidebar.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .pt_2()
                .mt_1()
                .border_t_1()
                .border_color(theme.border())
                .child(self.icon_button("sb-settings", "⚙️", ActiveView::Settings, cx))
                .child(self.icon_button("sb-stats", "📊", ActiveView::Stats, cx))
                .child(
                    div()
                        .id(collapse_id)
                        .w(px(24.))
                        .h(px(24.))
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(16.))
                        .text_color(theme.text_dim())
                        .cursor_pointer()
                        .hover(|el| el.bg(theme.hover()).text_color(theme.text()))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.set_sidebar_collapsed(!this.sidebar_collapsed());
                            cx.notify();
                        }))
                        .child(collapse_icon),
                ),
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
        let mut item = div()
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
            }));
        item = item.child(if self.sidebar_collapsed() {
            label.chars().next().unwrap_or('•').to_string()
        } else {
            label.to_string()
        });
        item
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
                                .tooltip({
                                    let tooltip_theme = *theme;
                                    let stat = stat.clone();
                                    move |_, cx| heatmap_tooltip(cx, tooltip_theme, &stat)
                                })
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
            ActiveView::Notes => views::notes(
                theme,
                &notes,
                self.delete_target(),
                self.note_edit_input.clone(),
                self.note_edit_tags_input.clone(),
                self.note_edit_id(),
                cx,
            )
            .into_any_element(),
            ActiveView::Clips => views::clips(
                theme,
                &clips,
                self.delete_target(),
                self.clip_edit_input.clone(),
                self.clip_edit_id(),
                cx,
            )
            .into_any_element(),
            ActiveView::Todos => views::todos(
                theme,
                &todos,
                self.priority_menu_open(),
                self.todo_input.clone(),
                self.todo_edit_input.clone(),
                self.todo_edit_id(),
                self.todo_meta_id(),
                self.todo_meta_due_input.clone(),
                self.todo_meta_remind_input.clone(),
                self.todo_meta_repeat_input.clone(),
                self.todo_meta_tags_input.clone(),
                self.todo_meta_remark_input.clone(),
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

    fn record_shortcut(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.shortcut_recording() || event.keystroke.key.is_empty() {
            return;
        }
        let modifiers = &event.keystroke.modifiers;
        if !modifiers.modified() {
            return;
        }
        let mut parts = Vec::new();
        if modifiers.control { parts.push("Ctrl"); }
        if modifiers.alt { parts.push("Alt"); }
        if modifiers.shift { parts.push("Shift"); }
        if modifiers.platform { parts.push(if cfg!(target_os = "macos") { "Cmd" } else { "Super" }); }
        let key = match event.keystroke.key.as_str() {
            " " => "Space",
            value => value,
        };
        parts.push(key);
        let value = parts.join("+");
        self.shortcut_input.update(cx, |input, cx| input.set_content(value, cx));
        self.set_shortcut_recording(false);
        window.focus(&self.focus_handle(cx));
        cx.notify();
    }

    fn render_shortcut(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let recording = self.shortcut_recording();
        let input = self.shortcut_input.clone();
        let save_input = input.clone();
        div()
            .flex()
            .items_center()
            .gap_2()
            .on_key_down(cx.listener(Self::record_shortcut))
            .child(div().w(px(190.)).h(px(30.)).child(input))
            .child(
                div()
                    .id("shortcut-record")
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .cursor_pointer()
                    .text_size(px(11.))
                    .text_color(theme.text())
                    .bg(if recording { theme.accent() } else { theme.card() })
                    .border_1()
                    .border_color(theme.border())
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.set_shortcut_recording(true);
                        this.set_shortcut_error(None);
                        this.focus_handle(cx).focus(window);
                        cx.notify();
                    }))
                    .child(if recording { "请按组合键" } else { "录制" }),
            )
            .child(
                div()
                    .id("shortcut-apply")
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .cursor_pointer()
                    .text_size(px(11.))
                    .text_color(theme.text())
                    .bg(theme.card())
                    .border_1()
                    .border_color(theme.border())
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        let value = save_input.read(cx).content().trim().to_string();
                        match crate::summon::set_shortcut(cx, &value) {
                            Ok(normalized) => {
                                this.settings.set_global_shortcut(normalized);
                                this.settings.save();
                                this.set_shortcut_error(None);
                            }
                            Err(error) => this.set_shortcut_error(Some(error)),
                        }
                        cx.notify();
                    }))
                    .child("应用"),
            )
            .child(div().text_size(px(11.)).text_color(theme.text_dim()).child("格式：Ctrl+Shift+Space"))
            .when_some(self.shortcut_error().clone(), |el, error| {
                el.child(div().text_color(theme.red()).child(error))
            })
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
            self.setting_row("数据导出", cx)
                .child(self.render_export(cx)),
        );
        rows = rows.child(div().when_some(self.export_status().clone(), |el, status| {
            el.pl(px(140.))
                .text_size(px(11.))
                .text_color(theme.accent())
                .child(status)
        }));

        rows = rows.child(self.setting_row("全局快捷键", cx).child(self.render_shortcut(cx)));

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

    fn render_export(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let mut row = div().flex().items_center().gap_1();
        for format in ["md", "txt", "html"] {
            let label = match format {
                "md" => "Markdown",
                "txt" => "TXT",
                _ => "HTML",
            };
            row = row.child(
                div()
                    .id(SharedString::from(format!("export-{format}")))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .text_size(px(11.))
                    .text_color(theme.text_dim())
                    .border_1()
                    .border_color(theme.border())
                    .cursor_pointer()
                    .hover(|el| el.bg(theme.hover()).text_color(theme.text()))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        let status = match crate::store::export_archive(cx, format) {
                            Ok(path) => format!("已导出：{path}"),
                            Err(error) => format!("导出失败：{error}"),
                        };
                        this.set_export_status(Some(status));
                        cx.notify();
                    }))
                    .child(label),
            );
        }
        row
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
                        .tooltip({
                            let tooltip_theme = *theme;
                            let stat = day.clone();
                            move |_, cx| heatmap_tooltip(cx, tooltip_theme, &stat)
                        })
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
        let date = self
            .day_detail_date
            .clone()
            .unwrap_or_else(stats::today_str);
        let stat: DayStat = stats::day_stat(cx, &date);
        let query = self.search_query.trim().to_lowercase();
        enum DayItem {
            Note(crate::store::Note),
            Clip(crate::store::ClipItem),
            Todo(crate::store::TodoItem),
        }
        let mut items = Vec::<(String, DayItem)>::new();
        for note in crate::store::notes(cx) {
            if (self.day_filter == "all" || self.day_filter == "note")
                && crate::store::display_timestamp(&note.created_at()).starts_with(&date)
                && (query.is_empty() || searchable_note(&note, &query))
            {
                items.push((note.created_at().clone(), DayItem::Note(note)));
            }
        }
        for clip in crate::store::clips(cx) {
            if (self.day_filter == "all" || self.day_filter == "clip")
                && crate::store::display_timestamp(&clip.captured_at()).starts_with(&date)
                && (query.is_empty() || searchable_clip(&clip, &query))
            {
                items.push((clip.captured_at().clone(), DayItem::Clip(clip)));
            }
        }
        for todo in crate::store::todos(cx) {
            if (self.day_filter == "all" || self.day_filter == "todo")
                && crate::store::display_timestamp(&todo.due_at()).starts_with(&date)
                && (query.is_empty() || searchable_todo(&todo, &query))
            {
                items.push((todo.due_at().clone(), DayItem::Todo(todo)));
            }
        }
        items.sort_by(|a, b| a.0.cmp(&b.0));
        let mut mixed = div().flex().flex_col().gap_2();
        if items.is_empty() {
            mixed = mixed.child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.text_dim())
                    .child("当天没有匹配的归档记录"),
            );
        }
        for (_, item) in items {
            let row = match item {
                DayItem::Note(note) => {
                    let note_id = note.id().clone();
                    let edit_id = note.id().clone();
                    let edit_content = note.content().clone();
                    let edit_tags = note.tags().join(", ");
                    let edit_input = self.note_edit_input.clone();
                    let edit_tags_input = self.note_edit_tags_input.clone();
                    let actions = div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap_1()
                        .child(
                            div()
                                .id(SharedString::from(format!("day-edit-note-{}", note.id())))
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .text_size(px(10.))
                                .text_color(theme.accent())
                                .cursor_pointer()
                                .hover(|el| el.bg(theme.hover()))
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    edit_input.update(cx, |input, cx| {
                                        input.set_content(edit_content.clone(), cx)
                                    });
                                    edit_tags_input.update(cx, |input, cx| {
                                        input.set_content(edit_tags.clone(), cx)
                                    });
                                    this.set_note_edit_id(Some(edit_id.clone()));
                                    this.set_active_view(ActiveView::Notes);
                                    cx.notify();
                                }))
                                .child("✏️ 编辑"),
                        );
                    div()
                        .relative()
                        .border_l_2()
                        .border_color(theme.accent())
                        .p_3()
                        .rounded_md()
                        .bg(theme.card())
                        .child(crate::views::delete_controls(
                            theme,
                            DeleteTarget::Note(note_id),
                            self.delete_target() == Some(DeleteTarget::Note(note.id().clone())),
                            cx,
                        ))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.accent())
                                .child("📝 笔记"),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_color(theme.text())
                                .child(crate::views::render_markdown_lite(&note.content(), theme)),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_size(px(10.))
                                .text_color(theme.text_dim())
                                .child(if note.tags().is_empty() {
                                    "无标签".to_string()
                                } else {
                                    note.tags()
                                        .iter()
                                        .map(|tag| format!("#{tag}"))
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                }),
                        )
                        .child(actions)
                }
                DayItem::Clip(clip) => {
                    let clip_id = clip.id().clone();
                    let edit_id = clip.id().clone();
                    let edit_content = clip.content().clone();
                    let edit_input = self.clip_edit_input.clone();
                    let actions = div()
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap_1()
                        .child(
                            div()
                                .id(SharedString::from(format!("day-edit-clip-{}", clip.id())))
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .text_size(px(10.))
                                .text_color(theme.accent())
                                .cursor_pointer()
                                .hover(|el| el.bg(theme.hover()))
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    edit_input.update(cx, |input, cx| {
                                        input.set_content(edit_content.clone(), cx)
                                    });
                                    this.set_clip_edit_id(Some(edit_id.clone()));
                                    this.set_active_view(ActiveView::Clips);
                                    cx.notify();
                                }))
                                .child("✏️ 编辑"),
                        );
                    div()
                        .relative()
                        .border_l_2()
                        .border_color(theme.gold())
                        .p_3()
                        .rounded_md()
                        .bg(theme.card())
                        .child(crate::views::delete_controls(
                            theme,
                            DeleteTarget::Clip(clip_id),
                            self.delete_target() == Some(DeleteTarget::Clip(clip.id().clone())),
                            cx,
                        ))
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.gold())
                                .child(format!("📋 粘贴板 · {}", clip.kind())),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_color(theme.text())
                                .child(crate::views::clip_content_preview(theme, &clip)),
                        )
                        .child(actions)
                }
                DayItem::Todo(todo) => {
                    let todo_id = todo.id().clone();
                    let edit_id = todo.id().clone();
                    let edit_content = todo.text().clone();
                    let edit_due = todo.due_at().clone();
                    let edit_remind = todo.remind_at().clone().unwrap_or_default();
                    let edit_repeat = todo.repeat_rule().clone().unwrap_or_default();
                    let edit_tags = todo.tags().join(", ");
                    let edit_remark = todo.remark().clone();
                    let meta_content = self.todo_edit_input.clone();
                    let meta_due = self.todo_meta_due_input.clone();
                    let meta_remind = self.todo_meta_remind_input.clone();
                    let meta_repeat = self.todo_meta_repeat_input.clone();
                    let meta_tags = self.todo_meta_tags_input.clone();
                    let meta_remark = self.todo_meta_remark_input.clone();
                    let mut actions = div().flex().items_center().justify_end().gap_1();
                    if !todo.done() {
                        actions = actions.child(
                            div()
                                .id(SharedString::from(format!("day-edit-todo-{}", todo.id())))
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .text_size(px(10.))
                                .text_color(theme.accent())
                                .cursor_pointer()
                                .hover(|el| el.bg(theme.hover()))
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    meta_content.update(cx, |input, cx| input.set_content(edit_content.clone(), cx));
                                    meta_due.update(cx, |input, cx| input.set_content(edit_due.clone(), cx));
                                    meta_remind.update(cx, |input, cx| input.set_content(edit_remind.clone(), cx));
                                    meta_repeat.update(cx, |input, cx| input.set_content(edit_repeat.clone(), cx));
                                    meta_tags.update(cx, |input, cx| input.set_content(edit_tags.clone(), cx));
                                    meta_remark.update(cx, |input, cx| input.set_content(edit_remark.clone(), cx));
                                    this.set_todo_meta_id(Some(edit_id.clone()));
                                    this.set_active_view(ActiveView::Todos);
                                    cx.notify();
                                }))
                                .child("⚙ 详情"),
                        );
                    }
                    let delete_target = DeleteTarget::Todo(todo_id);
                    div()
                        .relative()
                        .border_l_2()
                        .border_color(theme.green())
                        .p_3()
                        .rounded_md()
                        .bg(theme.card())
                        .when(!todo.done(), |el| {
                            el.child(crate::views::delete_controls(
                                theme,
                                delete_target.clone(),
                                self.delete_target() == Some(DeleteTarget::Todo(todo.id().clone())),
                                cx,
                            ))
                        })
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme.green())
                                .child(format!("✅ 待办 · {}优先级", todo.priority().label())),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_color(if todo.done() { theme.text_dim() } else { theme.text() })
                                .when(todo.done(), |el| el.line_through())
                                .child(todo.text().clone()),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_size(px(10.))
                                .text_color(if crate::store::is_overdue(&todo) { theme.red() } else { theme.text_dim() })
                                .child(if crate::store::is_overdue(&todo) { "逾期" } else if todo.done() { "已完成" } else { "未完成" }),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_size(px(10.))
                                .text_color(theme.text_dim())
                                .child(format!(
                                    "📅 计划 {}{}{}",
                                    crate::store::display_timestamp(&todo.due_at()),
                                    todo.remind_at().as_ref().map(|value| format!(" · ⏰ {}", crate::store::display_timestamp(value))).unwrap_or_default(),
                                    todo.repeat_rule().as_deref().map(|rule| if rule == "daily" { " · 🔁 每天" } else { " · 🔁 每周" }).unwrap_or(""),
                                )),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_size(px(10.))
                                .text_color(theme.text_dim())
                                .child(if todo.tags().is_empty() { "无标签".to_string() } else { todo.tags().iter().map(|tag| format!("#{tag}")).collect::<Vec<_>>().join(" ") }),
                        )
                        .when(!todo.remark().is_empty(), |el| {
                            el.child(div().mt_1().text_size(px(10.)).text_color(theme.text_dim()).child(format!("📝 {}", todo.remark())))
                        })
                        .when(todo.parent_id().is_some(), |el| {
                            el.child(div().mt_1().text_size(px(10.)).text_color(theme.text_dim()).child(format!("子任务 · 父级 {}", todo.parent_id().as_deref().unwrap_or(""))))
                        })
                        .child(actions)
                }
            };
            mixed = mixed.child(row);
        }
        let mut filter_bar = div().flex().items_center().gap_1();
        for (key, label) in [("all", "全部"), ("note", "笔记"), ("clip", "粘贴板"), ("todo", "待办")] {
            let selected = self.day_filter == key;
            let filter_key = key.to_string();
            filter_bar = filter_bar.child(
                div()
                    .id(SharedString::from(format!("day-filter-{key}")))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .text_size(px(10.))
                    .when(selected, |el| el.bg(theme.accent()).text_color(theme.bg()))
                    .when(!selected, |el| el.bg(theme.hover()).text_color(theme.text_dim()))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.set_day_filter(filter_key.clone());
                        cx.notify();
                    }))
                    .child(label),
            );
        }

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
                        date,
                        if date == stats::today_str() {
                            " · 今天"
                        } else {
                            ""
                        }
                    )),
            )
            .child(filter_bar)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_size(px(11.)).text_color(theme.text_dim()).child("搜索详情"))
                    .child(div().flex_1().min_w_0().h(px(30.)).child(self.search_input.clone())),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(theme.text_dim())
                    .child("按时间混排笔记、剪贴板和待办；可使用归档搜索框过滤内容"),
            )
            .child(
                div()
                    .flex()
                    .gap_3()
                    .text_size(px(11.))
                    .text_color(theme.text_dim())
                    .child(format!("📝 {}", stat.notes()))
                    .child(format!("📋 {}", stat.clips()))
                    .child(format!("✅ {} / 已完成 {}", stat.todos(), stat.done()))
                    .child(format!("逾期 {}", stat.overdue())),
            )
            .child(mixed)
    }
}

impl Render for InboxApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        div()
            .id("root")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::record_shortcut))
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
                    .child(self.render_sidebar_resizer(cx))
                    .child(self.render_content(cx)),
            )
    }
}
