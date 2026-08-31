//! 待办数据访问：校验、父子事务、完成联动、优先级/时间/提醒的窄更新与提醒实例登记。

use super::{db_err, local_date_key, now, Store};
use crate::domain::models::Todo;
use crate::domain::todo as logic;
use chrono::{DateTime, Duration, Utc};
use rusqlite::OptionalExtension;

/// 创建时间下限容差（秒），抵消时钟微差。
const CREATE_TIME_TOLERANCE_SECS: i64 = 60;

pub struct TodoInput {
    pub id: Option<String>,
    pub content: String,
    pub due_at: String,
    pub remind_at: Option<String>,
    pub repeat_rule: Option<String>,
    pub priority: String,
    pub remark: String,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    /// 历史日期补录时由前端置位，豁免“创建时间不得早于当前时刻”约束。
    pub allow_past: bool,
}

impl Store {
    fn todo_ids(&self, where_clause: &str, order_clause: &str) -> Result<Vec<String>, String> {
        let sql = format!("SELECT id FROM todos {where_clause} {order_clause}");
        let mut stmt = self.db.prepare(&sql).map_err(db_err)?;
        stmt.query_map([], |r| r.get::<_, String>(0))
            .map_err(db_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_err)
    }

    pub fn list_todos(&self) -> Result<Vec<Todo>, String> {
        self.todo_ids(
            "",
            "ORDER BY CASE status WHEN 'open' THEN 0 ELSE 1 END, due_at ASC, \
             CASE priority WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END, created_at ASC",
        )?
        .iter()
        .map(|id| self.todo(id))
        .collect()
    }

    pub fn list_open_remindable_todos(&self) -> Result<Vec<Todo>, String> {
        self.todo_ids("WHERE status='open' AND remind_off=0", "ORDER BY remind_at ASC")?
            .iter()
            .map(|id| self.todo(id))
            .collect()
    }

    fn child_count(&self, parent_id: &str) -> Result<i64, String> {
        self.db
            .query_row("SELECT COUNT(*) FROM todos WHERE parent_id=?", [parent_id], |r| r.get(0))
            .map_err(db_err)
    }

    fn child_ids(&self, parent_id: &str) -> Result<Vec<String>, String> {
        let sql = "SELECT id FROM todos WHERE parent_id=?";
        let mut stmt = self.db.prepare(sql).map_err(db_err)?;
        stmt.query_map([parent_id], |r| r.get::<_, String>(0))
            .map_err(db_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_err)
    }

    /// 新建或编辑待办。
    pub fn save_todo(&self, input: &TodoInput) -> Result<Todo, String> {
        logic::validate_fields(&input.content, &input.priority, &input.remark, &input.tags)?;
        if let Some(rule) = &input.repeat_rule {
            if !rule.is_empty() && !logic::is_valid_repeat_rule(rule) {
                return Err("无效的重复规则".into());
            }
        }
        let due = logic::parse_time(&input.due_at).ok_or("完成时间格式无效")?;
        let id = input.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let old = self.todo(&id).ok();
        if old.as_ref().is_some_and(|x| x.status == "done") {
            return Err("已完成待办不可编辑".into());
        }
        let is_new = old.is_none();
        if is_new && !input.allow_past {
            let lower_bound = Utc::now() - Duration::seconds(CREATE_TIME_TOLERANCE_SECS);
            if due < lower_bound {
                return Err("完成时间不能早于当前时刻".into());
            }
        }
        if let Some(parent_id) = &input.parent_id {
            let parent = self.todo(parent_id)?;
            logic::validate_parent(
                &parent.status,
                &parent.due_at,
                parent.parent_id.as_deref(),
                &input.due_at,
                self.child_count(parent_id)?,
                is_new,
            )?;
        }
        let timestamp = now();
        let created_at = old.as_ref().map(|x| x.created_at.clone()).unwrap_or_else(|| timestamp.clone());
        let remind_at = input.remind_at.as_deref().filter(|x| !x.is_empty());
        let repeat_rule = input.repeat_rule.as_deref().filter(|x| !x.is_empty());
        self.db
            .execute(
                "INSERT INTO todos(id, content, due_at, remind_at, repeat_rule, remind_off, priority, remark, parent_id, created_at, updated_at) \
                 VALUES(?,?,?,?,?,0,?,?,?,?,?) \
                 ON CONFLICT(id) DO UPDATE SET content=excluded.content, due_at=excluded.due_at, \
                   remind_at=excluded.remind_at, repeat_rule=excluded.repeat_rule, remind_off=0, \
                   priority=excluded.priority, remark=excluded.remark, parent_id=excluded.parent_id, \
                   updated_at=excluded.updated_at",
                rusqlite::params![
                    id,
                    input.content.trim(),
                    input.due_at,
                    remind_at,
                    repeat_rule,
                    input.priority,
                    input.remark,
                    input.parent_id,
                    created_at,
                    timestamp
                ],
            )
            .map_err(db_err)?;
        let tags = crate::domain::clipboard::validate_todo_tags(&input.tags)?;
        self.replace_tags("todo_tags", "todo_id", &id, &tags)?;
        if is_new {
            self.record_event("todo_created", &id, &timestamp)?;
        }
        self.todo(&id)
    }

    /// 已完成父待办新增子任务（方案 B）：同一事务内插入子任务、父级重开、清空 completed_at，
    /// 父级 due_at 顺延为 max(原父级, 新子任务)。
    pub fn create_child_todo(&self, parent_id: &str, input: &TodoInput) -> Result<Todo, String> {
        logic::validate_fields(&input.content, &input.priority, &input.remark, &input.tags)?;
        let tags = crate::domain::clipboard::validate_todo_tags(&input.tags)?;
        let parent = self.todo(parent_id)?;
        logic::validate_parent(
            &parent.status,
            &parent.due_at,
            parent.parent_id.as_deref(),
            &input.due_at,
            self.child_count(parent_id)?,
            true,
        )?;
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = now();

        let tx = self.tx()?;
        tx.execute(
            "INSERT INTO todos(id, content, due_at, status, remind_off, priority, remark, parent_id, created_at, updated_at) \
             VALUES(?,?,?,'open',0,?,?,?,?)",
            rusqlite::params![
                id,
                input.content.trim(),
                input.due_at,
                input.priority,
                input.remark,
                parent_id,
                timestamp,
                timestamp
            ],
        )
        .map_err(db_err)?;
        let reopened = parent.status == "done";
        tx.execute(
            "UPDATE todos SET status='open', completed_at=NULL, \
               due_at=CASE WHEN due_at < ? THEN ? ELSE due_at END, updated_at=? WHERE id=?",
            rusqlite::params![input.due_at, input.due_at, timestamp, parent_id],
        )
        .map_err(db_err)?;
        let new_tags = tags.clone();
        for tag in &new_tags {
            let normalized = tag.to_lowercase();
            tx.execute(
                "INSERT OR IGNORE INTO tags(name, normalized) VALUES(?, ?)",
                rusqlite::params![tag, normalized],
            )
            .map_err(db_err)?;
            let tag_id: i64 = tx
                .query_row("SELECT id FROM tags WHERE normalized=?", [normalized], |r| r.get(0))
                .map_err(db_err)?;
            tx.execute(
                "INSERT OR IGNORE INTO todo_tags(todo_id, tag_id) VALUES(?, ?)",
                rusqlite::params![id, tag_id],
            )
            .map_err(db_err)?;
        }
        tx.commit().map_err(db_err)?;
        self.record_event("todo_created", &id, &timestamp)?;
        self.todo(&id)
    }

    /// 完成 / 取消完成。
    /// 向下级联：完成父待办时，其未完成子任务在同一时刻一并完成；
    /// 向上联动：完成最后一个未完成子任务时，父待办自动完成。
    pub fn complete_todo(&self, id: &str, completed: bool) -> Result<Vec<Todo>, String> {
        let todo = self.todo(id)?;
        if !completed && todo.status == "done" {
            return Err("已完成待办不可取消完成".into());
        }
        if completed && todo.status == "done" {
            return Err("已完成待办不可重复完成".into());
        }
        let timestamp = now();
        let tx = self.tx()?;
        tx.execute(
            "UPDATE todos SET status='done', completed_at=?, remind_off=1, updated_at=? WHERE id=?",
            rusqlite::params![timestamp, timestamp, id],
        )
        .map_err(db_err)?;
        if todo.parent_id.is_none() {
            tx.execute(
                "UPDATE todos SET status='done', completed_at=?, remind_off=1, updated_at=? \
                 WHERE parent_id=? AND status='open'",
                rusqlite::params![timestamp, timestamp, id],
            )
            .map_err(db_err)?;
        } else if let Some(parent_id) = &todo.parent_id {
            let open_children: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM todos WHERE parent_id=? AND status='open'",
                    [parent_id],
                    |r| r.get(0),
                )
                .map_err(db_err)?;
            if open_children == 0 {
                tx.execute(
                    "UPDATE todos SET status='done', completed_at=?, remind_off=1, updated_at=? \
                     WHERE id=? AND status='open'",
                    rusqlite::params![timestamp, timestamp, parent_id],
                )
                .map_err(db_err)?;
            }
        }
        tx.commit().map_err(db_err)?;

        // 幂等记录完成事件（父级自动完成同样计入）。
        let mut completed_ids = vec![id.to_string()];
        if todo.parent_id.is_none() {
            completed_ids.extend(self.child_ids(id)?);
        } else if let Some(parent_id) = &todo.parent_id {
            completed_ids.push(parent_id.clone());
        }
        for cid in completed_ids {
            self.record_event("todo_completed", &cid, &timestamp)?;
        }
        self.list_todos()
    }

    /// 优先级窄更新：只写 priority 与 updated_at，父子独立不级联。
    pub fn set_todo_priority(&self, id: &str, priority: &str) -> Result<Todo, String> {
        if !logic::is_valid_priority(priority) {
            return Err("无效的优先级".into());
        }
        let todo = self.todo(id)?;
        if todo.status == "done" {
            return Err("已完成待办不可变更优先级".into());
        }
        self.db
            .execute(
                "UPDATE todos SET priority=?, updated_at=? WHERE id=?",
                rusqlite::params![priority, now(), id],
            )
            .map_err(db_err)?;
        self.todo(id)
    }

    /// 完成时间窄更新（聚焦弹窗）。保持父子约束：子不得晚于父；父不得早于任何子。
    pub fn set_todo_due(&self, id: &str, due_at: &str) -> Result<Todo, String> {
        let todo = self.todo(id)?;
        if todo.status == "done" {
            return Err("已完成待办不可修改完成时间".into());
        }
        if logic::parse_time(due_at).is_none() {
            return Err("完成时间格式无效".into());
        }
        if let Some(parent_id) = &todo.parent_id {
            let parent = self.todo(parent_id)?;
            if parent.status == "open" && due_at > parent.due_at {
                return Err("子任务的完成时间不能晚于父待办".into());
            }
        } else {
            for child_id in self.child_ids(id)? {
                let child = self.todo(&child_id)?;
                if child.status == "open" && child.due_at > due_at {
                    return Err("存在完成时间晚于新时间的子任务，请先调整子任务".into());
                }
            }
        }
        self.db
            .execute(
                "UPDATE todos SET due_at=?, updated_at=? WHERE id=?",
                rusqlite::params![due_at, now(), id],
            )
            .map_err(db_err)?;
        self.todo(id)
    }

    /// 提醒窄更新：只改下一次提醒时间与重复规则；重新设置会解除关闭抑制。
    pub fn set_todo_reminder(
        &self,
        id: &str,
        remind_at: Option<&str>,
        repeat_rule: Option<&str>,
    ) -> Result<Todo, String> {
        let todo = self.todo(id)?;
        if todo.status == "done" {
            return Err("已完成待办不可修改提醒".into());
        }
        let remind_at = remind_at.filter(|x| !x.is_empty());
        if let Some(value) = remind_at {
            if logic::parse_time(value).is_none() {
                return Err("提醒时间格式无效".into());
            }
        }
        let repeat_rule = repeat_rule.filter(|x| !x.is_empty());
        if let Some(rule) = repeat_rule {
            if !logic::is_valid_repeat_rule(rule) {
                return Err("无效的重复规则".into());
            }
        }
        self.db
            .execute(
                "UPDATE todos SET remind_at=?, repeat_rule=?, remind_off=0, updated_at=? WHERE id=?",
                rusqlite::params![remind_at, repeat_rule, now(), id],
            )
            .map_err(db_err)?;
        self.todo(id)
    }

    pub fn delete_todo(&self, id: &str) -> Result<(), String> {
        let todo = self.todo(id)?;
        if todo.status == "done" {
            return Err("已完成待办不可删除".into());
        }
        self.db
            .execute("DELETE FROM todos WHERE id=?", [id])
            .map_err(db_err)?;
        Ok(())
    }

    /// 提醒实例是否已触发过（幂等防重复弹窗）。
    pub fn reminder_fired(&self, key: &str) -> Result<bool, String> {
        let exists: Option<i64> = self
            .db
            .query_row("SELECT 1 FROM reminder_log WHERE id=?", [key], |r| r.get(0))
            .optional()
            .map_err(db_err)?;
        Ok(exists.is_some())
    }

    /// 登记提醒实例，返回是否成功抢占（首次触发）。
    pub fn log_reminder(&self, key: &str, todo_id: &str) -> Result<bool, String> {
        let inserted = self
            .db
            .execute(
                "INSERT OR IGNORE INTO reminder_log(id, todo_id, fired_at) VALUES(?,?,?)",
                rusqlite::params![key, todo_id, now()],
            )
            .map_err(db_err)?;
        Ok(inserted > 0)
    }

    /// 推进重复提醒的下一次触发时间（只改 remind_at，不改计划完成时间）。
    pub fn advance_repeat(&self, todo_id: &str, from: DateTime<Utc>, rule: &str) -> Result<(), String> {
        let Some(period) = logic::repeat_period(rule) else {
            return Ok(());
        };
        let mut next = from + period;
        while next <= Utc::now() {
            next += period;
        }
        self.db
            .execute(
                "UPDATE todos SET remind_at=?, updated_at=? WHERE id=?",
                rusqlite::params![next.to_rfc3339(), now(), todo_id],
            )
            .map_err(db_err)?;
        Ok(())
    }

    /// 统计某本地日期键下的待办数量（按计划完成日期）与完成数量（按 completed_at）。
    pub fn todos_on(&self, date: &str) -> Result<(i64, i64), String> {
        let start = super::local_day_start(date);
        let end = super::local_day_end(date);
        let todos: i64 = self
            .db
            .query_row(
                "SELECT COUNT(*) FROM todos WHERE due_at >= ? AND due_at < ?",
                rusqlite::params![start, end],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        let completed: i64 = self
            .db
            .query_row(
                "SELECT COUNT(*) FROM todos WHERE completed_at >= ? AND completed_at < ?",
                rusqlite::params![start, end],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        Ok((todos, completed))
    }

    /// 创建事件日期键（诊断用）。
    pub fn todo_created_date(&self, id: &str) -> Result<String, String> {
        let created: String = self
            .db
            .query_row("SELECT created_at FROM todos WHERE id=?", [id], |r| r.get(0))
            .map_err(db_err)?;
        let dt = DateTime::parse_from_rfc3339(&created)
            .map(|x| x.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        Ok(local_date_key(dt))
    }
}
