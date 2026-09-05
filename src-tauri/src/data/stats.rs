//! 统计数据查询：热力图 / 月度趋势 / 全量摘要 / 日期详情。
//! 口径：笔记按归档时刻、剪贴板按捕获时刻、待办总数按计划完成日期、
//! 完成数按 completed_at、逾期数按查询时刻从业务数据推导。

use super::{db_err, local_day_end, local_day_start, Store};
use crate::domain::models::{DayActivity, DayDetailItem, MonthTrend, Note, StatsSummary, Todo};
use chrono::{Datelike, Days, Local, Utc};

impl Store {
    fn notes_archived_on(&self, date: &str) -> Result<i64, String> {
        self.db
            .query_row(
                "SELECT COUNT(*) FROM notes WHERE is_draft=0 AND archived_at >= ? AND archived_at < ?",
                rusqlite::params![local_day_start(date), local_day_end(date)],
                |r| r.get(0),
            )
            .map_err(db_err)
    }

    fn overdue_on(&self, date: &str, now: &str) -> Result<i64, String> {
        // 历史日期：该日计划完成且仍未完成的事项全部视为逾期；当天：完成时刻已过的部分。
        if date == super::local_date_key(Utc::now()) {
            self.db
                .query_row(
                    "SELECT COUNT(*) FROM todos WHERE status='open' AND due_at >= ? AND due_at < ? AND due_at < ?",
                    rusqlite::params![local_day_start(date), local_day_end(date), now],
                    |r| r.get(0),
                )
                .map_err(db_err)
        } else {
            self.db
                .query_row(
                    "SELECT COUNT(*) FROM todos WHERE status='open' AND due_at >= ? AND due_at < ?",
                    rusqlite::params![local_day_start(date), local_day_end(date)],
                    |r| r.get(0),
                )
                .map_err(db_err)
        }
    }

    /// 近 N 天（含今天）的每日活跃度。
    pub fn heatmap(&self, days: u32) -> Result<Vec<DayActivity>, String> {
        let today = Local::now().date_naive();
        let now = super::now();
        let mut result = Vec::new();
        for offset in (0..days).rev() {
            let date = today - Days::new(offset as u64);
            let date_str = date.to_string();
            let (todos, completed) = self.todos_on(&date_str)?;
            result.push(
                DayActivity::builder()
                    .notes(self.notes_archived_on(&date_str)?)
                    .clips(self.clips_captured_on(&date_str)?)
                    .todos(todos)
                    .completed(completed)
                    .overdue(self.overdue_on(&date_str, &now)?)
                    .date(date_str)
                    .build()?,
            );
        }
        Ok(result)
    }

    /// 近 6 个月的月度趋势。
    pub fn trend(&self) -> Result<Vec<MonthTrend>, String> {
        let mut result = Vec::new();
        let today = Local::now().date_naive();
        for offset in (0..6).rev() {
            let first = first_day_of_month_offset(today, offset);
            let month = first.format("%Y-%m").to_string();
            let start = local_day_start(&first.to_string());
            let last_day = last_day_of_month(first);
            let end = local_day_end(&last_day.to_string());
            let params = rusqlite::params![start, end];
            let notes: i64 = self
                .db
                .query_row(
                    "SELECT COUNT(*) FROM notes WHERE is_draft=0 AND archived_at >= ? AND archived_at < ?",
                    params,
                    |r| r.get(0),
                )
                .map_err(db_err)?;
            let clips: i64 = self
                .db
                .query_row(
                    "SELECT COUNT(*) FROM clipboard_entries WHERE copied_at >= ? AND copied_at < ?",
                    params,
                    |r| r.get(0),
                )
                .map_err(db_err)?;
            let todos: i64 = self
                .db
                .query_row(
                    "SELECT COUNT(*) FROM todos WHERE due_at >= ? AND due_at < ?",
                    params,
                    |r| r.get(0),
                )
                .map_err(db_err)?;
            let completed: i64 = self
                .db
                .query_row(
                    "SELECT COUNT(*) FROM todos WHERE completed_at >= ? AND completed_at < ?",
                    params,
                    |r| r.get(0),
                )
                .map_err(db_err)?;
            result.push(
                MonthTrend::builder()
                    .month(month)
                    .notes(notes)
                    .clips(clips)
                    .todos(todos)
                    .completed(completed)
                    .build()?,
            );
        }
        Ok(result)
    }

    pub fn stats_summary(&self) -> Result<StatsSummary, String> {
        let now = super::now();
        // 先取各项计数再走建造者：build() 会校验字段是否齐全，
        // 新增字段时漏赋值会在构造时报出字段名，而不是静默留一个默认值。
        let count = |sql: &str| -> Result<i64, String> {
            self.db.query_row(sql, [], |r| r.get(0)).map_err(db_err)
        };
        let notes = count("SELECT COUNT(*) FROM notes WHERE is_draft=0")?;
        let clips = count("SELECT COUNT(*) FROM clipboard_entries")?;
        let todos = count("SELECT COUNT(*) FROM todos")?;
        let completed = count("SELECT COUNT(*) FROM todos WHERE status='done'")?;
        let overdue = self
            .db
            .query_row(
                "SELECT COUNT(*) FROM todos WHERE status='open' AND due_at < ?",
                [&now],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        StatsSummary::builder()
            .notes(notes)
            .clips(clips)
            .todos(todos)
            .completed(completed)
            .overdue(overdue)
            .build()
    }

    /// 某日全部记录（笔记按归档时刻、剪贴板按捕获时刻、待办按计划完成时刻），
    /// 按时间先后混排。
    pub fn day_detail(&self, date: &str) -> Result<Vec<DayDetailItem>, String> {
        let start = local_day_start(date);
        let end = local_day_end(date);
        let mut items: Vec<DayDetailItem> = Vec::new();

        let note_ids: Vec<String> = {
            let mut stmt = self
                .db
                .prepare(
                    "SELECT id FROM notes WHERE is_draft=0 \
                     AND COALESCE(archived_at, created_at) >= ? AND COALESCE(archived_at, created_at) < ?",
                )
                .map_err(db_err)?;
            let rows = stmt
                .query_map(rusqlite::params![start, end], |r| r.get(0))
                .map_err(db_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db_err)?;
            rows
        };
        for id in note_ids {
            let note: Note = self.note(&id)?;
            let time = note
                .archived_at()
                .clone()
                .unwrap_or_else(|| note.created_at().clone());
            items.push(
                DayDetailItem::builder()
                    .kind("note".into())
                    .time(time)
                    .note(Some(note))
                    .clip(None)
                    .todo(None)
                    .build()?,
            );
        }

        let clip_ids: Vec<String> = {
            let mut stmt = self
                .db
                .prepare("SELECT id FROM clipboard_entries WHERE copied_at >= ? AND copied_at < ?")
                .map_err(db_err)?;
            let rows = stmt
                .query_map(rusqlite::params![start, end], |r| r.get(0))
                .map_err(db_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db_err)?;
            rows
        };
        for id in clip_ids {
            if let Some(clip) = self.clipboard_entry(&id)? {
                let time = clip.copied_at().clone();
                items.push(
                    DayDetailItem::builder()
                        .kind("clip".into())
                        .time(time)
                        .clip(Some(clip))
                        .note(None)
                        .todo(None)
                        .build()?,
                );
            }
        }

        let todo_ids: Vec<String> = {
            let mut stmt = self
                .db
                .prepare("SELECT id FROM todos WHERE due_at >= ? AND due_at < ?")
                .map_err(db_err)?;
            let rows = stmt
                .query_map(rusqlite::params![start, end], |r| r.get(0))
                .map_err(db_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db_err)?;
            rows
        };
        for id in todo_ids {
            let todo: Todo = self.todo(&id)?;
            let time = todo.due_at.clone();
            items.push(
                DayDetailItem::builder()
                    .kind("todo".into())
                    .time(time)
                    .todo(Some(todo))
                    .note(None)
                    .clip(None)
                    .build()?,
            );
        }

        items.sort_by(|a, b| a.time().cmp(b.time()));
        Ok(items)
    }
}

/// 距今 months_back 个月份的当月第一天。
fn first_day_of_month_offset(today: chrono::NaiveDate, months_back: u32) -> chrono::NaiveDate {
    let total = today.year() * 12 + today.month0() as i32 - months_back as i32;
    let year = total.div_euclid(12);
    let month0 = total.rem_euclid(12);
    chrono::NaiveDate::from_ymd_opt(year, month0 as u32 + 1, 1).unwrap_or(today)
}

fn last_day_of_month(first: chrono::NaiveDate) -> chrono::NaiveDate {
    let (y, m) = (first.year(), first.month());
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    let next_first = chrono::NaiveDate::from_ymd_opt(ny, nm, 1).unwrap_or(first);
    next_first - Days::new(1)
}
