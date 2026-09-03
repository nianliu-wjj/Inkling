# 待办提醒：相对偏移下拉 + 邮箱提醒 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把待办提醒从「两个日期时间输入设绝对时刻」改为「下拉框选相对完成时间的偏移」，并新增邮箱提醒方式（用户自配 SMTP）。

**Architecture:** 数据层新增三列（偏移分钟 / 是否弹窗 / 是否邮件）并迁移到 v3；`remind_at` 由用户设置降级为派生游标。调度器按 `due - offset` 与 `due` 两个槽位、按渠道分别记账触发。邮件经独立线程队列发送，避免 SMTP 握手阻塞持有数据库锁的调度扫描。

**Tech Stack:** Rust (Tauri 2, rusqlite, chrono, lettre), Vue 3 + TypeScript, SQLite

## Global Constraints

- 回答与提交信息用中文；提交格式 `<type>(<scope>): <中文描述>`。
- Rust 文件保持既有注释密度：模块头 `//!` 说明职责，关键分支与外部调用前后有注释。
- 关键节点必须有日志：Rust 侧用 `eprintln!("[scope] ...")`，前端用 `logger.info/error('scope', ...)`。全项目禁止裸 `console.*`。
- 前端提交前必须通过 `npx vue-tsc --noEmit` 与 `npx prettier --check`。
- Rust 提交前必须通过 `cargo fmt` 与 `cargo build`。
- 数据库迁移只增列、不删列，用既有的 `add_column_if_missing` 辅助方法。
- 六个偏移档位固定为：15 / 30 / 60 / 180 / 360 / 1440（分钟），`NULL` 表示不提醒。

---

### Task 1: 领域层 —— 偏移槽位与含渠道的幂等键

**Files:**
- Modify: `src-tauri/src/domain/todo.rs:81-101`（替换 `default_reminder_slots`，扩展 `reminder_instance_key`）
- Test: `src-tauri/src/domain/todo.rs`（文件内 `#[cfg(test)] mod tests`）

**Interfaces:**
- Consumes: 既有 `parse_time`、`repeat_period`
- Produces:
  - `pub const REMIND_OFFSETS: [i64; 6]`
  - `pub fn reminder_slots(due_at: DateTime<Utc>, offset_minutes: Option<i64>) -> Vec<(DateTime<Utc>, &'static str)>`
  - `pub fn reminder_instance_key(todo_id: &str, slot: &str, channel: &str, when: DateTime<Utc>) -> String`
  - `pub fn is_valid_offset(minutes: i64) -> bool`

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/domain/todo.rs` 的 `mod tests` 里，删除既有的 `default_reminder_slots` 测试（搜索 `fn default_reminder_slots_are_ordered` 或含 `slots[0].1, "due-30m"` 断言的那个测试函数整体删掉），追加：

```rust
    #[test]
    fn reminder_slots_use_offset_plus_due() {
        let due = t(2026, 8, 30, 18, 0);
        let slots = reminder_slots(due, Some(15));
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].0, due - Duration::minutes(15));
        assert_eq!(slots[0].1, "offset");
        assert_eq!(slots[1].0, due);
        assert_eq!(slots[1].1, "due");
    }

    #[test]
    fn reminder_slots_empty_when_offset_missing() {
        let due = t(2026, 8, 30, 18, 0);
        assert!(reminder_slots(due, None).is_empty());
    }

    #[test]
    fn reminder_key_separates_channels() {
        let when = t(2026, 8, 30, 18, 0);
        let desktop = reminder_instance_key("t1", "due", "desktop", when);
        let email = reminder_instance_key("t1", "due", "email", when);
        assert_ne!(desktop, email);
        assert_eq!(desktop, reminder_instance_key("t1", "due", "desktop", when));
    }

    #[test]
    fn only_listed_offsets_are_valid() {
        assert!(is_valid_offset(15));
        assert!(is_valid_offset(1440));
        assert!(!is_valid_offset(20));
        assert!(!is_valid_offset(0));
    }
```

同时把既有的 `reminder_instance_key` 测试改为四参数形式（搜索 `reminder_instance_key("t1", "due", t(`，把该测试函数整体替换为上面的 `reminder_key_separates_channels`）。

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test domain::todo 2>&1 | tail -20`
Expected: 编译失败，报 `cannot find function 'reminder_slots'` 与 `this function takes 3 arguments but 4 arguments were supplied`

- [ ] **Step 3: 实现**

在 `src-tauri/src/domain/todo.rs` 中，把 `default_reminder_slots` 整个函数（含其上方的文档注释）替换为：

```rust
/// 可选的提醒偏移档位（分钟）。
///
/// 与前端下拉框选项一一对应，前后端都以此为准：
/// 15 分钟 / 30 分钟 / 1 小时 / 3 小时 / 6 小时 / 1 天。
pub const REMIND_OFFSETS: [i64; 6] = [15, 30, 60, 180, 360, 1440];

/// 偏移值是否合法。`None`（不提醒）由调用方单独处理，不走这里。
pub fn is_valid_offset(minutes: i64) -> bool {
    REMIND_OFFSETS.contains(&minutes)
}

/// 某待办的提醒槽位：偏移提醒一次 + 到点兜底一次。
///
/// 兜底的存在是为了让「前 1 天」这类大偏移不至于错过到期本身。
/// `offset_minutes` 为 `None` 时表示不提醒，返回空列表。
pub fn reminder_slots(
    due_at: DateTime<Utc>,
    offset_minutes: Option<i64>,
) -> Vec<(DateTime<Utc>, &'static str)> {
    let Some(minutes) = offset_minutes else {
        return Vec::new();
    };
    vec![
        (due_at - Duration::minutes(minutes), "offset"),
        (due_at, "due"),
    ]
}
```

把 `reminder_instance_key` 替换为：

```rust
/// 提醒实例的幂等键：待办 + 槽位 + 渠道 + 触发时刻毫秒。
///
/// 含渠道维度，是为了让弹窗与邮件各自独立记账：邮件发送失败重试时，
/// 若与弹窗共用一条记录，会被弹窗那次成功的记录挡掉而永不重发。
/// 后续接入新渠道（如 QQ 机器人）也不必再改键的结构。
pub fn reminder_instance_key(
    todo_id: &str,
    slot: &str,
    channel: &str,
    when: DateTime<Utc>,
) -> String {
    format!("{todo_id}|{slot}|{channel}|{}", when.timestamp_millis())
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd src-tauri && cargo test domain::todo 2>&1 | tail -10`
Expected: `test result: ok.`，包含 `reminder_slots_use_offset_plus_due`、`reminder_slots_empty_when_offset_missing`、`reminder_key_separates_channels`、`only_listed_offsets_are_valid` 四项通过

- [ ] **Step 5: 提交**

```bash
cd src-tauri && cargo fmt && cd ..
git add src-tauri/src/domain/todo.rs
git commit -m "feat(todo): 提醒槽位改为相对偏移，幂等键加入渠道维度"
```

---

### Task 2: 数据层 —— 迁移到 v3 与 Todo 模型扩展

**Files:**
- Modify: `src-tauri/src/domain/models.rs:45-67`（`Todo` 加三个字段）
- Modify: `src-tauri/src/data/mod.rs:78-160`（`migrate` 加 v3 分支，新增 `with_v3`）
- Modify: `src-tauri/src/data/mod.rs:343-375`（`todo()` 行映射加三列）
- Test: `src-tauri/src/data/mod.rs`（新增 `#[cfg(test)] mod tests`）

**Interfaces:**
- Consumes: Task 1 的 `REMIND_OFFSETS`
- Produces: `Todo` 结构新增 `remind_offset_minutes: Option<i64>`、`remind_desktop: bool`、`remind_email: bool`

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/data/mod.rs` 文件末尾追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// 建一个 v2 状态的库：只有 todos 基础列，user_version=2。
    fn v2_db() -> rusqlite::Connection {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE todos (
               id TEXT PRIMARY KEY,
               content TEXT NOT NULL,
               due_at TEXT NOT NULL,
               remind_at TEXT,
               status TEXT NOT NULL DEFAULT 'open',
               priority TEXT NOT NULL DEFAULT 'medium',
               remark TEXT NOT NULL DEFAULT '',
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );
             INSERT INTO todos(id, content, due_at, remind_at, created_at, updated_at)
             VALUES('a','旧待办','2026-09-10T10:00:00+00:00','2026-09-10T09:40:00+00:00','x','x');
             INSERT INTO todos(id, content, due_at, remind_at, created_at, updated_at)
             VALUES('b','无提醒待办','2026-09-11T10:00:00+00:00',NULL,'x','x');",
        )
        .unwrap();
        db.pragma_update(None, "user_version", 2).unwrap();
        db
    }

    #[test]
    fn v3_migration_normalizes_existing_reminders() {
        let db = v2_db();
        let store = Store {
            db,
            data_dir: std::path::PathBuf::from("."),
        };
        store.with_v3().unwrap();

        // 存量一律归为「前 15 分钟 + 只弹窗」，remind_at 清空。
        let mut stmt = store
            .db
            .prepare("SELECT remind_offset_minutes, remind_desktop, remind_email, remind_at FROM todos ORDER BY id")
            .unwrap();
        let rows: Vec<(i64, i64, i64, Option<String>)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .map(|x| x.unwrap())
            .collect();
        assert_eq!(rows.len(), 2);
        for row in rows {
            assert_eq!(row.0, 15);
            assert_eq!(row.1, 1);
            assert_eq!(row.2, 0);
            assert_eq!(row.3, None);
        }
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test data::tests 2>&1 | tail -15`
Expected: 编译失败，报 `no method named 'with_v3' found`

- [ ] **Step 3: 实现**

在 `src-tauri/src/domain/models.rs` 的 `Todo` 结构里，把 `remind_at` 字段的文档注释与其后的 `repeat_rule` 之间插入新字段，即把：

```rust
    /// 下一次提醒时间；为空时按 `due_at` 应用默认提醒计划。
    pub remind_at: Option<String>,
```

替换为：

```rust
    /// 重复提醒推进时的游标；**不再是用户设置的提醒时刻**。
    ///
    /// 用户设的是相对偏移（`remind_offset_minutes`），实际提醒时刻由
    /// `due_at - offset` 现算，这样改完成时间后提醒会自动跟随。
    pub remind_at: Option<String>,
    /// 提醒偏移分钟数（完成时间之前）；`None` = 不提醒。
    pub remind_offset_minutes: Option<i64>,
    /// 是否桌面弹窗提醒。
    pub remind_desktop: bool,
    /// 是否邮件提醒。
    pub remind_email: bool,
```

在 `src-tauri/src/data/mod.rs` 的 `migrate` 方法里，把 `Ok(())` 之前的部分补上 v3 分支，即把：

```rust
        if version < 2 {
            self.with_v2()
                .map_err(|e| format!("数据库迁移到 v2 失败: {e}"))?;
            self.db
                .pragma_update(None, "user_version", 2)
                .map_err(db_err)?;
        }
        Ok(())
```

替换为：

```rust
        if version < 2 {
            self.with_v2()
                .map_err(|e| format!("数据库迁移到 v2 失败: {e}"))?;
            self.db
                .pragma_update(None, "user_version", 2)
                .map_err(db_err)?;
        }
        if version < 3 {
            self.with_v3()
                .map_err(|e| format!("数据库迁移到 v3 失败: {e}"))?;
            self.db
                .pragma_update(None, "user_version", 3)
                .map_err(db_err)?;
        }
        Ok(())
```

在 `with_v2` 方法之后插入：

```rust
    /// v3 增量：提醒偏移与提醒渠道。
    ///
    /// 存量数据统一归为「前 15 分钟 + 只弹窗」并清空 `remind_at`。
    /// 不按原 `remind_at` 与 `due_at` 的差值就近归类到六个档位——那会产生
    /// 「我设的 18:40 变成 18:45」这种静默偏移，比统一归默认值更难向用户解释。
    fn with_v3(&self) -> Result<(), String> {
        self.add_column_if_missing("todos", "remind_offset_minutes", "INTEGER")?;
        self.add_column_if_missing("todos", "remind_desktop", "INTEGER NOT NULL DEFAULT 1")?;
        self.add_column_if_missing("todos", "remind_email", "INTEGER NOT NULL DEFAULT 0")?;
        self.db
            .execute(
                "UPDATE todos SET remind_offset_minutes=15, remind_desktop=1, remind_email=0, remind_at=NULL",
                [],
            )
            .map_err(db_err)?;
        Ok(())
    }
```

在 `todo()` 方法里，把 SQL 与行映射一并改掉。把：

```rust
                "SELECT id, content, due_at, completed_at, status, remind_at, repeat_rule, remind_off, \
                 priority, remark, parent_id, created_at, updated_at FROM todos WHERE id=?",
```

替换为：

```rust
                "SELECT id, content, due_at, completed_at, status, remind_at, repeat_rule, remind_off, \
                 priority, remark, parent_id, created_at, updated_at, \
                 remind_offset_minutes, remind_desktop, remind_email FROM todos WHERE id=?",
```

并把闭包里的 `updated_at: r.get(12)?,` 之后补上三列，即把：

```rust
                        tags: Vec::new(),
                        created_at: r.get(11)?,
                        updated_at: r.get(12)?,
                    })
```

替换为：

```rust
                        tags: Vec::new(),
                        created_at: r.get(11)?,
                        updated_at: r.get(12)?,
                        remind_offset_minutes: r.get(13)?,
                        remind_desktop: r.get::<_, i64>(14)? != 0,
                        remind_email: r.get::<_, i64>(15)? != 0,
                    })
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd src-tauri && cargo test data::tests 2>&1 | tail -10`
Expected: `test result: ok. 1 passed`

注意此时 `cargo build` 仍会失败（`save_todo` 等处尚未适配新字段），这是预期的，由 Task 3 收口。

- [ ] **Step 5: 提交**

```bash
cd src-tauri && cargo fmt && cd ..
git add src-tauri/src/domain/models.rs src-tauri/src/data/mod.rs
git commit -m "feat(todo): 数据库迁移 v3，新增提醒偏移与渠道列"
```

---

### Task 3: 数据层 —— 写入路径适配

**Files:**
- Modify: `src-tauri/src/data/todos.rs:10-24`（`TodoInput` 加字段）
- Modify: `src-tauri/src/data/todos.rs:81-170`（`save_todo`）
- Modify: `src-tauri/src/data/todos.rs:190-235`（`create_child_todo`）
- Modify: `src-tauri/src/data/todos.rs:366-395`（`set_todo_reminder`）
- Modify: `src-tauri/src/data/todos.rs:49-57`（`list_open_remindable_todos` 排序）

**Interfaces:**
- Consumes: Task 1 的 `is_valid_offset`，Task 2 的 `Todo` 新字段
- Produces: `TodoInput` 新增 `remind_offset_minutes: Option<i64>`、`remind_desktop: bool`、`remind_email: bool`；`set_todo_reminder(id, offset_minutes, desktop, email, repeat_rule)` 新签名

- [ ] **Step 1: 写失败测试**

`data/todos.rs` 没有现成的测试模块，且写入路径依赖完整 `Store`。在 `src-tauri/src/data/todos.rs` 末尾追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        let dir = std::env::temp_dir().join(format!("inkling-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        Store::open(dir).unwrap()
    }

    fn input(offset: Option<i64>, email: bool) -> TodoInput {
        TodoInput {
            id: None,
            content: "测试待办".into(),
            due_at: "2099-01-01T10:00:00+00:00".into(),
            remind_at: None,
            remind_offset_minutes: offset,
            remind_desktop: true,
            remind_email: email,
            repeat_rule: None,
            priority: "medium".into(),
            remark: String::new(),
            tags: Vec::new(),
            parent_id: None,
            allow_past: false,
        }
    }

    #[test]
    fn save_todo_persists_offset_and_channels() {
        let store = store();
        let saved = store.save_todo(&input(Some(30), true)).unwrap();
        assert_eq!(saved.remind_offset_minutes, Some(30));
        assert!(saved.remind_desktop);
        assert!(saved.remind_email);
    }

    #[test]
    fn save_todo_rejects_unlisted_offset() {
        let store = store();
        let error = store.save_todo(&input(Some(20), false)).unwrap_err();
        assert!(error.contains("提醒偏移"), "实际错误: {error}");
    }

    #[test]
    fn save_todo_accepts_none_offset_as_no_reminder() {
        let store = store();
        let saved = store.save_todo(&input(None, false)).unwrap();
        assert_eq!(saved.remind_offset_minutes, None);
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test data::todos 2>&1 | tail -15`
Expected: 编译失败，报 `struct 'TodoInput' has no field named 'remind_offset_minutes'`

- [ ] **Step 3: 实现**

在 `src-tauri/src/data/todos.rs` 的 `TodoInput` 结构里，把：

```rust
    pub remind_at: Option<String>,
    pub repeat_rule: Option<String>,
```

替换为：

```rust
    /// 兼容字段，前端不再传；提醒时刻改由 `due_at - remind_offset_minutes` 派生。
    pub remind_at: Option<String>,
    /// 提醒偏移分钟数；`None` = 不提醒。取值须在 `domain::todo::REMIND_OFFSETS` 内。
    pub remind_offset_minutes: Option<i64>,
    /// 是否桌面弹窗提醒。
    pub remind_desktop: bool,
    /// 是否邮件提醒。
    pub remind_email: bool,
    pub repeat_rule: Option<String>,
```

在 `save_todo` 里，把校验段（`if let Some(rule) = &input.repeat_rule { ... }` 这一整块之后）补上偏移校验。即把：

```rust
        let due = logic::parse_time(&input.due_at).ok_or("完成时间格式无效")?;
```

替换为：

```rust
        // 偏移必须是下拉框列出的档位之一，防止前端传入任意值绕过 UI 约束。
        if let Some(minutes) = input.remind_offset_minutes {
            if !logic::is_valid_offset(minutes) {
                return Err("提醒偏移不在允许的档位内".into());
            }
        }
        let due = logic::parse_time(&input.due_at).ok_or("完成时间格式无效")?;
```

把「已完成待办仅允许修改备注」的比较逻辑里对 `remind_at` 的比较换成对偏移与渠道的比较。即把：

```rust
                && existing.remind_at.as_deref().unwrap_or("")
                    == input.remind_at.as_deref().unwrap_or("")
```

替换为：

```rust
                && existing.remind_offset_minutes == input.remind_offset_minutes
                && existing.remind_desktop == input.remind_desktop
                && existing.remind_email == input.remind_email
```

把 INSERT 语句与参数改为写入新列。把：

```rust
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
```

替换为：

```rust
        let repeat_rule = input.repeat_rule.as_deref().filter(|x| !x.is_empty());
        // remind_at 一律写 NULL：它只是重复提醒的推进游标，用户设的是偏移。
        self.db
            .execute(
                "INSERT INTO todos(id, content, due_at, remind_at, repeat_rule, remind_off, priority, remark, parent_id, created_at, updated_at, \
                   remind_offset_minutes, remind_desktop, remind_email) \
                 VALUES(?,?,?,NULL,?,0,?,?,?,?,?,?,?,?) \
                 ON CONFLICT(id) DO UPDATE SET content=excluded.content, due_at=excluded.due_at, \
                   remind_at=NULL, repeat_rule=excluded.repeat_rule, remind_off=0, \
                   priority=excluded.priority, remark=excluded.remark, parent_id=excluded.parent_id, \
                   updated_at=excluded.updated_at, \
                   remind_offset_minutes=excluded.remind_offset_minutes, \
                   remind_desktop=excluded.remind_desktop, remind_email=excluded.remind_email",
                rusqlite::params![
                    id,
                    input.content.trim(),
                    input.due_at,
                    repeat_rule,
                    input.priority,
                    input.remark,
                    input.parent_id,
                    created_at,
                    timestamp,
                    input.remind_offset_minutes,
                    input.remind_desktop as i64,
                    input.remind_email as i64
                ],
            )
            .map_err(db_err)?;
```

在 `create_child_todo` 里同样处理。把：

```rust
        tx.execute(
            "INSERT INTO todos(id, content, due_at, remind_at, repeat_rule, status, remind_off, priority, remark, parent_id, created_at, updated_at) \
             VALUES(?,?,?,?,?,'open',0,?,?,?,?,?)",
            rusqlite::params![
                id,
                input.content.trim(),
                input.due_at,
                input.remind_at.as_deref().filter(|value| !value.is_empty()),
                input.repeat_rule.as_deref().filter(|value| !value.is_empty()),
                input.priority,
                input.remark,
                parent_id,
                timestamp,
                timestamp
            ],
        )
        .map_err(db_err)?;
```

替换为：

```rust
        tx.execute(
            "INSERT INTO todos(id, content, due_at, remind_at, repeat_rule, status, remind_off, priority, remark, parent_id, created_at, updated_at, \
               remind_offset_minutes, remind_desktop, remind_email) \
             VALUES(?,?,?,NULL,?,'open',0,?,?,?,?,?,?,?,?)",
            rusqlite::params![
                id,
                input.content.trim(),
                input.due_at,
                input.repeat_rule.as_deref().filter(|value| !value.is_empty()),
                input.priority,
                input.remark,
                parent_id,
                timestamp,
                timestamp,
                input.remind_offset_minutes,
                input.remind_desktop as i64,
                input.remind_email as i64
            ],
        )
        .map_err(db_err)?;
```

把 `set_todo_reminder` 整个方法（从 `pub fn set_todo_reminder` 到其闭合 `}`）替换为：

```rust
    /// 快捷修改提醒设置（徽章点击入口）。
    ///
    /// 与 `save_todo` 一样只接受档位内的偏移；写入后复位 `remind_off`，
    /// 让此前被用户关掉的提醒重新参与调度。
    pub fn set_todo_reminder(
        &self,
        id: &str,
        offset_minutes: Option<i64>,
        desktop: bool,
        email: bool,
        repeat_rule: Option<&str>,
    ) -> Result<Todo, String> {
        let todo = self.todo(id)?;
        if todo.status == "done" {
            return Err("已完成待办不可修改提醒".into());
        }
        if let Some(minutes) = offset_minutes {
            if !logic::is_valid_offset(minutes) {
                return Err("提醒偏移不在允许的档位内".into());
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
                "UPDATE todos SET remind_offset_minutes=?, remind_desktop=?, remind_email=?, \
                   repeat_rule=?, remind_at=NULL, remind_off=0, updated_at=? WHERE id=?",
                rusqlite::params![
                    offset_minutes,
                    desktop as i64,
                    email as i64,
                    repeat_rule,
                    now(),
                    id
                ],
            )
            .map_err(db_err)?;
        self.todo(id)
    }
```

把 `list_open_remindable_todos` 的排序换掉（`remind_at` 现在恒为 NULL，按它排序无意义）。把：

```rust
        self.todo_ids(
            "WHERE status='open' AND remind_off=0",
            "ORDER BY remind_at ASC",
        )?
```

替换为：

```rust
        // remind_at 现在只是重复提醒的游标、通常为 NULL，按完成时间排序才有意义。
        self.todo_ids(
            "WHERE status='open' AND remind_off=0 AND remind_offset_minutes IS NOT NULL",
            "ORDER BY due_at ASC",
        )?
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd src-tauri && cargo test data::todos 2>&1 | tail -10`
Expected: `test result: ok. 3 passed`

- [ ] **Step 5: 提交**

```bash
cd src-tauri && cargo fmt && cd ..
git add src-tauri/src/data/todos.rs
git commit -m "feat(todo): 写入路径改用提醒偏移与渠道，校验档位合法性"
```

---

### Task 4: 设置层 —— SMTP 配置与密码掩码

**Files:**
- Modify: `src-tauri/src/domain/models.rs:86-116`（`Settings` 加七个字段与默认值）
- Modify: `src-tauri/src/data/settings.rs`（读写与掩码逻辑）
- Test: `src-tauri/src/data/settings.rs`（新增 `#[cfg(test)] mod tests`）

**Interfaces:**
- Produces: `Settings` 新增 `smtp_host: String`、`smtp_port: i64`、`smtp_tls: bool`、`smtp_username: String`、`smtp_password: String`、`smtp_from: String`、`smtp_to: String`；`pub const SMTP_PASSWORD_MASK: &str`；`Store::smtp_password_raw() -> Result<String, String>`

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/data/settings.rs` 末尾追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Store {
        let dir = std::env::temp_dir().join(format!("inkling-set-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        Store::open(dir).unwrap()
    }

    #[test]
    fn password_is_masked_on_read_and_preserved_on_save() {
        let store = store();
        let mut settings = store.get_settings().unwrap();
        settings.smtp_password = "secret-token".into();
        settings.smtp_host = "smtp.example.com".into();
        store.save_settings(&settings).unwrap();

        // 读出来是掩码，真实密码不外泄给前端。
        let read = store.get_settings().unwrap();
        assert_eq!(read.smtp_password, SMTP_PASSWORD_MASK);
        assert_eq!(read.smtp_host, "smtp.example.com");

        // 前端拿着掩码回存其他项，密码保持不变。
        let mut again = read.clone();
        again.smtp_host = "smtp.other.com".into();
        store.save_settings(&again).unwrap();
        assert_eq!(store.smtp_password_raw().unwrap(), "secret-token");
        assert_eq!(store.get_settings().unwrap().smtp_host, "smtp.other.com");
    }

    #[test]
    fn empty_password_is_not_masked() {
        let store = store();
        assert_eq!(store.get_settings().unwrap().smtp_password, "");
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test data::settings 2>&1 | tail -15`
Expected: 编译失败，报 `no field 'smtp_password' on type 'Settings'`

- [ ] **Step 3: 实现**

在 `src-tauri/src/domain/models.rs` 的 `Settings` 结构里，把 `panel_position` 字段之后的闭合 `}` 之前插入：

```rust
    /// SMTP 服务器地址，如 smtp.qq.com；为空表示未配置邮件提醒。
    #[serde(default)]
    pub smtp_host: String,
    #[serde(default = "default_smtp_port")]
    pub smtp_port: i64,
    #[serde(default = "default_true")]
    pub smtp_tls: bool,
    #[serde(default)]
    pub smtp_username: String,
    /// 建议填邮箱的**应用专用密码**而非主账号密码。
    /// 读取时会被替换为掩码，避免真实值进入前端状态与日志。
    #[serde(default)]
    pub smtp_password: String,
    #[serde(default)]
    pub smtp_from: String,
    #[serde(default)]
    pub smtp_to: String,
```

在 `Settings::default()` 的 `panel_position: default_panel_position(),` 之后插入：

```rust
            smtp_host: String::new(),
            smtp_port: default_smtp_port(),
            smtp_tls: true,
            smtp_username: String::new(),
            smtp_password: String::new(),
            smtp_from: String::new(),
            smtp_to: String::new(),
```

在 `default_panel_position` 函数之后追加：

```rust
/// SMTP 默认端口：465（隐式 TLS），与多数邮箱服务商的默认一致。
fn default_smtp_port() -> i64 {
    465
}

fn default_true() -> bool {
    true
}
```

把 `src-tauri/src/data/settings.rs` 整个文件替换为：

```rust
//! 偏好设置数据访问。

use super::{db_err, Store};
use crate::domain::models::Settings;

/// SMTP 密码对外的占位值。
///
/// `get_settings` 会把非空密码替换成它，真实密码不进入前端状态、日志与事件广播；
/// `save_settings` 收到该值时保留库中原值，这样前端不持有真实密码也能改其他设置项。
pub const SMTP_PASSWORD_MASK: &str = "********";

impl Store {
    pub fn get_settings(&self) -> Result<Settings, String> {
        let mut values = std::collections::HashMap::new();
        let mut stmt = self
            .db
            .prepare("SELECT key, value FROM settings")
            .map_err(db_err)?;
        for row in stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(db_err)?
        {
            let (key, value) = row.map_err(db_err)?;
            values.insert(key, value);
        }
        let defaults = Settings::default();
        let stored_password = values.get("smtp_password").cloned().unwrap_or_default();
        Ok(Settings {
            collapse_policy: values
                .get("collapse_policy")
                .cloned()
                .unwrap_or(defaults.collapse_policy),
            clipboard_retention_days: values
                .get("clipboard_retention_days")
                .and_then(|x| x.parse().ok())
                .unwrap_or(defaults.clipboard_retention_days),
            start_on_boot: values.get("start_on_boot").is_some_and(|x| x == "true"),
            shortcut: values.get("shortcut").cloned().unwrap_or(defaults.shortcut),
            remark_style: values
                .get("remark_style")
                .cloned()
                .unwrap_or(defaults.remark_style),
            theme: values.get("theme").cloned().unwrap_or(defaults.theme),
            // 缺省视为开启，与 Settings::default 一致。
            main_acrylic: values
                .get("main_acrylic")
                .map(|x| x == "true")
                .unwrap_or(defaults.main_acrylic),
            panel_position: values
                .get("panel_position")
                .cloned()
                .unwrap_or(defaults.panel_position),
            smtp_host: values.get("smtp_host").cloned().unwrap_or_default(),
            smtp_port: values
                .get("smtp_port")
                .and_then(|x| x.parse().ok())
                .unwrap_or(defaults.smtp_port),
            smtp_tls: values
                .get("smtp_tls")
                .map(|x| x == "true")
                .unwrap_or(defaults.smtp_tls),
            smtp_username: values.get("smtp_username").cloned().unwrap_or_default(),
            // 空密码不加掩码，前端据此判断「尚未配置」。
            smtp_password: if stored_password.is_empty() {
                String::new()
            } else {
                SMTP_PASSWORD_MASK.into()
            },
            smtp_from: values.get("smtp_from").cloned().unwrap_or_default(),
            smtp_to: values.get("smtp_to").cloned().unwrap_or_default(),
        })
    }

    /// 读取真实 SMTP 密码，仅供发信服务使用。
    pub fn smtp_password_raw(&self) -> Result<String, String> {
        Ok(self.setting_value("smtp_password")?.unwrap_or_default())
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        // 掩码代表「不改密码」：保留库中原值，否则前端一保存就会把真实密码冲成掩码串。
        let password = if settings.smtp_password == SMTP_PASSWORD_MASK {
            self.smtp_password_raw()?
        } else {
            settings.smtp_password.clone()
        };
        for (key, value) in [
            ("collapse_policy", settings.collapse_policy.clone()),
            (
                "clipboard_retention_days",
                settings.clipboard_retention_days.to_string(),
            ),
            ("start_on_boot", settings.start_on_boot.to_string()),
            ("shortcut", settings.shortcut.clone()),
            ("remark_style", settings.remark_style.clone()),
            ("theme", settings.theme.clone()),
            ("main_acrylic", settings.main_acrylic.to_string()),
            ("panel_position", settings.panel_position.clone()),
            ("smtp_host", settings.smtp_host.clone()),
            ("smtp_port", settings.smtp_port.to_string()),
            ("smtp_tls", settings.smtp_tls.to_string()),
            ("smtp_username", settings.smtp_username.clone()),
            ("smtp_password", password),
            ("smtp_from", settings.smtp_from.clone()),
            ("smtp_to", settings.smtp_to.clone()),
        ] {
            self.db
                .execute(
                    "INSERT INTO settings(key,value) VALUES(?,?) \
                     ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    rusqlite::params![key, value],
                )
                .map_err(db_err)?;
        }
        Ok(())
    }

    /// 读取单个设置项（清理调度使用）。
    pub fn setting_value(&self, key: &str) -> Result<Option<String>, String> {
        self.db
            .query_row("SELECT value FROM settings WHERE key=?", [key], |r| {
                r.get(0)
            })
            .map(Some)
            .or(Ok(None))
            .map_err(|e: rusqlite::Error| db_err(e))
    }
}
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd src-tauri && cargo test data::settings 2>&1 | tail -10`
Expected: `test result: ok. 2 passed`

- [ ] **Step 5: 提交**

```bash
cd src-tauri && cargo fmt && cd ..
git add src-tauri/src/domain/models.rs src-tauri/src/data/settings.rs
git commit -m "feat(settings): 新增 SMTP 配置项，密码读取加掩码"
```

---

### Task 5: 发信服务

**Files:**
- Modify: `src-tauri/Cargo.toml`（加 `lettre` 依赖）
- Create: `src-tauri/src/services/mailer.rs`
- Modify: `src-tauri/src/services/mod.rs`（注册模块）
- Modify: `src-tauri/src/main.rs`（启动发信线程）

**Interfaces:**
- Consumes: Task 4 的 `Settings` SMTP 字段与 `Store::smtp_password_raw`
- Produces:
  - `pub struct MailRequest { pub subject: String, pub body: String }`
  - `pub fn start(app: AppHandle)` —— 启动发信线程
  - `pub fn enqueue(request: MailRequest)` —— 投递到队列（配置在发信线程内读取，不需要 AppHandle）
  - `pub fn is_configured(app: &AppHandle) -> bool` —— 邮件提醒是否已配置好
  - `pub fn send_now(app: &AppHandle, request: &MailRequest) -> Result<(), String>` —— 同步发送，供测试邮件按钮用

- [ ] **Step 1: 加依赖并写失败测试**

在 `src-tauri/Cargo.toml` 的 `enigo = "0.6.1"` 之后插入：

```toml
# 邮件提醒：SMTP 客户端。关掉默认的 async 运行时，只用阻塞式发送——
# 发信在独立线程的队列里串行执行，不需要再引入一套异步栈。
lettre = { version = "0.11", default-features = false, features = [
  "smtp-transport",
  "rustls-tls",
  "builder",
] }
```

在 `src-tauri/src/services/mailer.rs`（新建）末尾预留测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_requires_host_and_recipient() {
        let mut config = MailConfig {
            host: String::new(),
            port: 465,
            tls: true,
            username: "u".into(),
            password: "p".into(),
            from: "a@b.com".into(),
            to: "c@d.com".into(),
        };
        assert!(config.validate().is_err());
        config.host = "smtp.example.com".into();
        assert!(config.validate().is_ok());
        config.to = String::new();
        assert!(config.validate().is_err());
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd src-tauri && cargo test services::mailer 2>&1 | tail -10`
Expected: 编译失败，报 `file not found for module 'mailer'` 或 `cannot find struct 'MailConfig'`

- [ ] **Step 3: 实现**

把 `src-tauri/src/services/mailer.rs` 写为（覆盖 Step 1 里的测试片段，保留其 `mod tests` 部分在文件末尾）：

```rust
//! 邮件提醒发送。
//!
//! 独立线程 + 通道队列，**不在调度器里同步发信**：`reminder::tick` 持着数据库锁，
//! 而 SMTP 握手可能耗数秒，同步发送会卡住整个提醒扫描。
//! 失败按指数退避重试 3 次，仍失败则记日志放弃，不阻塞后续提醒。

use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use std::time::Duration;

use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use tauri::{AppHandle, Manager};

use crate::app::state::AppState;

/// 单封邮件的内容。
pub struct MailRequest {
    pub subject: String,
    pub body: String,
}

/// 从偏好设置读出的发信配置。
pub struct MailConfig {
    pub host: String,
    pub port: i64,
    pub tls: bool,
    pub username: String,
    pub password: String,
    pub from: String,
    pub to: String,
}

impl MailConfig {
    /// 必填项校验：缺任意一项都发不出去，提前给出可读的错误。
    pub fn validate(&self) -> Result<(), String> {
        if self.host.trim().is_empty() {
            return Err("未配置 SMTP 服务器地址".into());
        }
        if self.from.trim().is_empty() {
            return Err("未配置发件人地址".into());
        }
        if self.to.trim().is_empty() {
            return Err("未配置收件人地址".into());
        }
        if self.password.is_empty() {
            return Err("未配置 SMTP 密码".into());
        }
        Ok(())
    }
}

/// 发信队列的发送端。进程内单例，`start` 时初始化。
static QUEUE: OnceLock<Sender<MailRequest>> = OnceLock::new();

/// 最大重试次数与退避基数。
const MAX_ATTEMPTS: u32 = 3;
const BACKOFF_BASE: Duration = Duration::from_secs(2);

/// 从当前偏好设置读取发信配置（密码取真实值，不是掩码）。
pub fn load_config(app: &AppHandle) -> Result<MailConfig, String> {
    let state = app.state::<AppState>();
    let store = state.lock_store()?;
    let settings = store.get_settings()?;
    let password = store.smtp_password_raw()?;
    Ok(MailConfig {
        host: settings.smtp_host,
        port: settings.smtp_port,
        tls: settings.smtp_tls,
        username: settings.smtp_username,
        password,
        from: settings.smtp_from,
        to: settings.smtp_to,
    })
}

/// 邮件提醒是否已具备发送条件，供保存待办时拦截使用。
pub fn is_configured(app: &AppHandle) -> bool {
    load_config(app).map(|c| c.validate().is_ok()).unwrap_or(false)
}

/// 同步发送一封邮件。供「发送测试邮件」按钮直接调用，错误原样回传给界面。
pub fn send_now(app: &AppHandle, request: &MailRequest) -> Result<(), String> {
    let config = load_config(app)?;
    config.validate()?;
    deliver(&config, request)
}

fn deliver(config: &MailConfig, request: &MailRequest) -> Result<(), String> {
    let email = Message::builder()
        .from(config.from.parse().map_err(|e| format!("发件人地址无效: {e}"))?)
        .to(config.to.parse().map_err(|e| format!("收件人地址无效: {e}"))?)
        .subject(request.subject.clone())
        .body(request.body.clone())
        .map_err(|e| format!("构造邮件失败: {e}"))?;

    // TLS 端口用隐式 TLS（relay），非 TLS 用明文连接（dangerous_* 仅用于内网自建服务）。
    let builder = if config.tls {
        SmtpTransport::relay(&config.host).map_err(|e| format!("连接 SMTP 失败: {e}"))?
    } else {
        SmtpTransport::builder_dangerous(&config.host)
    };
    let transport = builder
        .port(config.port as u16)
        .credentials(Credentials::new(
            config.username.clone(),
            config.password.clone(),
        ))
        .build();

    transport
        .send(&email)
        .map(|_| ())
        .map_err(|e| format!("发送邮件失败: {e}"))
}

/// 启动发信线程。
pub fn start(app: AppHandle) {
    let (tx, rx) = channel::<MailRequest>();
    if QUEUE.set(tx).is_err() {
        eprintln!("[mailer] 发信队列已初始化，跳过重复启动");
        return;
    }
    std::thread::Builder::new()
        .name("mailer".into())
        .spawn(move || {
            for request in rx {
                eprintln!("[mailer] 开始发送邮件：{}", request.subject);
                let mut attempt = 0;
                loop {
                    attempt += 1;
                    match send_now(&app, &request) {
                        Ok(()) => {
                            eprintln!("[mailer] 邮件已发送：{}", request.subject);
                            break;
                        }
                        Err(error) if attempt < MAX_ATTEMPTS => {
                            eprintln!("[mailer] 第 {attempt} 次发送失败，稍后重试：{error}");
                            std::thread::sleep(BACKOFF_BASE * attempt);
                        }
                        Err(error) => {
                            // 放弃，但不影响后续邮件与其他渠道的提醒。
                            eprintln!("[mailer] 发送失败，已重试 {attempt} 次放弃：{error}");
                            break;
                        }
                    }
                }
            }
        })
        .expect("启动发信线程失败");
}

/// 把一封邮件投递到队列，立即返回。
pub fn enqueue(request: MailRequest) {
    let Some(queue) = QUEUE.get() else {
        eprintln!("[mailer] 发信队列尚未启动，丢弃邮件");
        return;
    };
    if let Err(error) = queue.send(request) {
        eprintln!("[mailer] 投递邮件到队列失败：{error}");
    }
}
```

在 `src-tauri/src/services/mod.rs` 的 `pub mod hotzone_watcher;` 之后插入：

```rust
pub mod mailer;
```

在 `src-tauri/src/main.rs` 的 `services::hotzone_watcher::start(app.clone());` 之后插入：

```rust
            services::mailer::start(app.clone());
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd src-tauri && cargo test services::mailer 2>&1 | tail -10`
Expected: `test result: ok. 1 passed`

- [ ] **Step 5: 提交**

```bash
cd src-tauri && cargo fmt && cargo build 2>&1 | tail -2 && cd ..
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/services/mailer.rs src-tauri/src/services/mod.rs src-tauri/src/main.rs
git commit -m "feat(mailer): 新增 SMTP 发信服务，独立线程队列 + 失败重试"
```

---

### Task 6: 调度器改造

**Files:**
- Modify: `src-tauri/src/services/reminder.rs`（整体重写 `tick` 与触发逻辑）

**Interfaces:**
- Consumes: Task 1 的 `reminder_slots`、`reminder_instance_key`；Task 2 的 `Todo` 新字段；Task 5 的 `mailer::{enqueue, MailRequest}`
- Produces: 无对外新接口

- [ ] **Step 1: 写失败测试**

调度器依赖 `AppHandle` 无法单测，覆盖交由 Task 1 的槽位测试与 Task 9 的实机验证。本任务改为**先确认现有测试仍通过**，再重写。

Run: `cd src-tauri && cargo test 2>&1 | tail -5`
Expected: 全部通过（此时 `reminder.rs` 仍引用已删除的 `default_reminder_slots`，故编译失败——这就是本步要修的失败点）

- [ ] **Step 2: 确认失败原因**

Run: `cd src-tauri && cargo build 2>&1 | grep -E "^error" -A5 | head -20`
Expected: `cannot find function 'default_reminder_slots' in module 'logic'`

- [ ] **Step 3: 实现**

把 `src-tauri/src/services/reminder.rs` 的模块头注释与 `tick`、`fire`、`fire_or_repeat` 全部替换为：

```rust
//! 提醒调度：每 30s 扫描到期提醒实例（幂等抢占），按渠道分别触发。
//!
//! 提醒时刻由「完成时间 - 用户选择的偏移」现算，另加一次到点兜底；
//! 弹窗与邮件各自记账，互不影响（邮件重试不会被弹窗的成功记录挡掉）。
//! `repeat_rule` 在提醒触发后按周期推进 `remind_at` 游标。

use chrono::{Duration, Utc};
use std::time::Duration as StdDuration;

use crate::app::state::AppState;
use crate::app::windows;
use crate::domain::models::Todo;
use crate::domain::todo as logic;
use crate::events;
use crate::services::mailer::{self, MailRequest};
use tauri::{AppHandle, Emitter, Manager};

const TICK: StdDuration = StdDuration::from_secs(30);

pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("reminder-scheduler".into())
        .spawn(move || loop {
            std::thread::sleep(TICK);
            if let Err(error) = tick(&app) {
                eprintln!("[reminder] 调度失败: {error}");
            }
        })
        .expect("启动提醒调度线程失败");
}

fn tick(app: &AppHandle) -> Result<(), String> {
    let now = Utc::now();
    let state = app.state::<AppState>();
    let todos = {
        let store = state.lock_store()?;
        store.list_open_remindable_todos()?
    };
    for todo in todos {
        let Some(due) = logic::parse_time(&todo.due_at) else {
            continue;
        };
        // 跳过未来 1 天以后的事项，降低无效计算；最大偏移正好是 1 天。
        if due > now + Duration::days(1) {
            continue;
        }

        let mut due_fired = false;
        for (when, slot) in logic::reminder_slots(due, todo.remind_offset_minutes) {
            if when > now {
                continue;
            }
            if todo.remind_desktop {
                fire_desktop(app, &todo, slot, when)?;
            }
            if todo.remind_email {
                fire_email(app, &todo, slot, when)?;
            }
            if slot == "due" {
                due_fired = true;
            }
        }

        if due_fired {
            finish_or_repeat(app, &todo, due)?;
        }
    }
    Ok(())
}

/// 抢占一次提醒实例；返回 true 表示本次由当前调用负责触发。
fn claim(
    app: &AppHandle,
    todo_id: &str,
    slot: &str,
    channel: &str,
    when: chrono::DateTime<Utc>,
) -> Result<bool, String> {
    let key = logic::reminder_instance_key(todo_id, slot, channel, when);
    let state = app.state::<AppState>();
    let store = state.lock_store()?;
    store.log_reminder(&key, todo_id)
}

fn fire_desktop(
    app: &AppHandle,
    todo: &Todo,
    slot: &str,
    when: chrono::DateTime<Utc>,
) -> Result<(), String> {
    if !claim(app, &todo.id, slot, "desktop", when)? {
        return Ok(());
    }
    eprintln!("[reminder] 弹窗提醒 todo={} slot={slot}", todo.id);
    windows::reminder_show(app, &todo.id)?;
    let _ = app.emit(events::REMINDER_FIRED, todo.id.clone());
    Ok(())
}

fn fire_email(
    app: &AppHandle,
    todo: &Todo,
    slot: &str,
    when: chrono::DateTime<Utc>,
) -> Result<(), String> {
    if !claim(app, &todo.id, slot, "email", when)? {
        return Ok(());
    }
    eprintln!("[reminder] 邮件提醒入队 todo={} slot={slot}", todo.id);
    let heading = if slot == "due" {
        "待办已到完成时间"
    } else {
        "待办即将到期"
    };
    mailer::enqueue(MailRequest {
        subject: format!("[Inkling] {heading}：{}", todo.content),
        body: format!(
            "{heading}\n\n内容：{}\n完成时间：{}\n优先级：{}\n{}",
            todo.content,
            todo.due_at,
            todo.priority,
            if todo.remark.is_empty() {
                String::new()
            } else {
                format!("备注：{}\n", todo.remark)
            }
        ),
    });
    Ok(())
}

/// 到点提醒已触发后的收尾：有重复规则则推进，否则抑制后续扫描。
fn finish_or_repeat(
    app: &AppHandle,
    todo: &Todo,
    due: chrono::DateTime<Utc>,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let store = state.lock_store()?;
    match todo.repeat_rule.as_deref() {
        Some(rule) if logic::repeat_period(rule).is_some() => {
            store.advance_repeat(&todo.id, due, rule)?;
        }
        _ => {
            store
                .db
                .execute("UPDATE todos SET remind_off=1 WHERE id=?", [&todo.id])
                .map_err(|e| format!("数据库操作失败: {e}"))?;
        }
    }
    Ok(())
}
```

- [ ] **Step 4: 确认编译与测试通过**

Run: `cd src-tauri && cargo build 2>&1 | tail -2 && cargo test 2>&1 | tail -5`
Expected: `Finished` 且 `test result: ok.`

- [ ] **Step 5: 提交**

```bash
cd src-tauri && cargo fmt && cd ..
git add src-tauri/src/services/reminder.rs
git commit -m "feat(reminder): 调度改为偏移+到点两槽位，弹窗与邮件分渠道记账"
```

---

### Task 7: IPC 层

**Files:**
- Modify: `src-tauri/src/ipc.rs:445-465`（`todo_reminder` 新签名）
- Modify: `src-tauri/src/ipc.rs`（`todo_save` 增加邮件配置拦截，新增 `mail_test`）
- Modify: `src-tauri/src/main.rs`（注册 `mail_test`）

**Interfaces:**
- Consumes: Task 3 的 `set_todo_reminder` 新签名，Task 5 的 `mailer::{is_configured, send_now, MailRequest}`
- Produces: `todo_reminder(id, offset_minutes, desktop, email, repeat_rule)`、`mail_test()` 两个命令

- [ ] **Step 1: 确认当前编译失败点**

Run: `cd src-tauri && cargo build 2>&1 | grep -E "^error" -A5 | head -20`
Expected: `this method takes 5 arguments but 3 arguments were supplied`（`set_todo_reminder` 调用处）

- [ ] **Step 2: 实现**

在 `src-tauri/src/ipc.rs` 中，把 `todo_reminder` 命令整体替换为：

```rust
#[tauri::command]
pub fn todo_reminder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    offset_minutes: Option<i64>,
    desktop: bool,
    email: bool,
    repeat_rule: Option<String>,
) -> Result<Todo, String> {
    // 勾了邮件却没配好 SMTP 时直接拦截，避免用户以为已生效却收不到信。
    if email && !crate::services::mailer::is_configured(&app) {
        return Err("尚未配置邮件提醒，请先在设置页填写 SMTP 信息".into());
    }
    let todo = state.lock_store()?.set_todo_reminder(
        &id,
        offset_minutes,
        desktop,
        email,
        repeat_rule.as_deref(),
    )?;
    emit_all(&app, events::TODOS_CHANGED, id);
    Ok(todo)
}

/// 发送一封测试邮件，用于验证 SMTP 配置。同步执行，错误原样回传界面。
#[tauri::command]
pub fn mail_test(app: AppHandle) -> Result<(), String> {
    crate::services::mailer::send_now(
        &app,
        &crate::services::mailer::MailRequest {
            subject: "[Inkling] 测试邮件".into(),
            body: "这是一封来自 Inkling 的测试邮件。收到它说明邮件提醒已配置成功。".into(),
        },
    )
}
```

找到 `todo_save` 命令，在其函数体第一行插入邮件配置拦截。即把：

```rust
pub fn todo_save(
```

对应函数体的第一条语句之前插入（函数签名保持不变，只在体内加）：

```rust
    if input.remind_email && !crate::services::mailer::is_configured(&app) {
        return Err("尚未配置邮件提醒，请先在设置页填写 SMTP 信息".into());
    }
```

在 `src-tauri/src/main.rs` 的 `ipc::settings_save,` 之后插入：

```rust
            ipc::mail_test,
```

- [ ] **Step 3: 确认编译通过**

Run: `cd src-tauri && cargo build 2>&1 | tail -2 && cargo test 2>&1 | tail -5`
Expected: `Finished` 且 `test result: ok.`

- [ ] **Step 4: 提交**

```bash
cd src-tauri && cargo fmt && cd ..
git add src-tauri/src/ipc.rs src-tauri/src/main.rs
git commit -m "feat(ipc): 提醒命令改用偏移与渠道，新增测试邮件命令"
```

---

### Task 8: 前端 —— 类型、常量与编辑弹窗

**Files:**
- Modify: `src/typings/domain.ts:41-100`（`Todo`、`TodoInput`、`Settings`）
- Create: `src/constants/reminder.ts`
- Modify: `src/service/tauri.ts`（`todos.reminder` 签名、`mail.test`）
- Modify: `src/components/todo/TodoEditorModal.vue`（提醒区域）
- Modify: `src/components/todo/RemindBadge.vue`（文案与展示）
- Modify: `src/styles/components.css`（提醒行样式）

**Interfaces:**
- Consumes: Task 7 的命令签名
- Produces: `REMIND_OPTIONS: readonly { value: number | null; label: string }[]`、`DEFAULT_REMIND_OFFSET = 15`

- [ ] **Step 1: 新建常量文件**

创建 `src/constants/reminder.ts`：

```typescript
/**
 * 提醒偏移档位。
 *
 * 与后端 `domain::todo::REMIND_OFFSETS` 一一对应，两侧都以此为准；
 * 后端会校验传入值是否在档位内，前端改动此处必须同步改后端常量。
 */
export const REMIND_OPTIONS: readonly { value: number | null; label: string }[] = [
  { value: null, label: '不提醒' },
  { value: 15, label: '前 15 分钟' },
  { value: 30, label: '前 30 分钟' },
  { value: 60, label: '前 1 小时' },
  { value: 180, label: '前 3 小时' },
  { value: 360, label: '前 6 小时' },
  { value: 1440, label: '前 1 天' },
]

/** 新建待办时的默认偏移。 */
export const DEFAULT_REMIND_OFFSET = 15

/** 把偏移分钟数转成显示文案，未命中档位时回退为「不提醒」。 */
export function remindOffsetLabel(minutes: number | null): string {
  return REMIND_OPTIONS.find((option) => option.value === minutes)?.label ?? '不提醒'
}
```

- [ ] **Step 2: 改类型与 API**

在 `src/typings/domain.ts` 的 `Todo` 接口里，把 `remind_at: string | null` 那一行替换为：

```typescript
  /** 重复提醒的推进游标，不是用户设置的提醒时刻。 */
  remind_at: string | null
  /** 提醒偏移分钟数（完成时间之前）；null = 不提醒。 */
  remind_offset_minutes: number | null
  remind_desktop: boolean
  remind_email: boolean
```

在 `TodoInput` 接口里，把 `remindAt?: string | null` 那一行替换为：

```typescript
  remindOffsetMinutes: number | null
  remindDesktop: boolean
  remindEmail: boolean
```

在 `Settings` 接口的 `panel_position: PanelPosition` 之后插入：

```typescript
  smtp_host: string
  smtp_port: number
  smtp_tls: boolean
  smtp_username: string
  /** 读取时后端返回掩码；原样回存表示不修改密码。 */
  smtp_password: string
  smtp_from: string
  smtp_to: string
```

在 `src/service/tauri.ts` 中，把 `todos` 下的 `reminder` 方法替换为：

```typescript
    reminder: (
      id: string,
      offsetMinutes: number | null,
      desktop: boolean,
      email: boolean,
      repeatRule: string | null,
    ) => invoke<Todo>('todo_reminder', { id, offsetMinutes, desktop, email, repeatRule }),
```

在 `api` 对象末尾（`data_dir` 所在分组之后）追加：

```typescript
  mail: {
    /** 发送测试邮件验证 SMTP 配置，失败时错误信息原样抛出。 */
    test: () => invoke<void>('mail_test'),
  },
```

- [ ] **Step 3: 改编辑弹窗**

在 `src/components/todo/TodoEditorModal.vue` 的 `<script setup>` 中：

把 `import { fromDateAndTimeInputs, toDateAndTimeInputs, todayKey } from '@/utils/datetime'` 替换为：

```typescript
import { DEFAULT_REMIND_OFFSET, REMIND_OPTIONS } from '@/constants/reminder'
import { fromDateAndTimeInputs, toDateAndTimeInputs, todayKey } from '@/utils/datetime'
```

把这三行状态：

```typescript
const remindDate = ref(initialRemind.date)
const remindTime = ref(initialRemind.time)
```

替换为：

```typescript
const remindOffset = ref<number | null>(
  props.todo ? props.todo.remind_offset_minutes : DEFAULT_REMIND_OFFSET,
)
const remindDesktop = ref(props.todo ? props.todo.remind_desktop : true)
const remindEmail = ref(props.todo ? props.todo.remind_email : false)
/** 选「不提醒」时渠道勾选无意义，禁用并置灰。 */
const channelsDisabled = computed(() => fieldsDisabled.value || remindOffset.value === null)
```

删除 `const initialRemind = toDateAndTimeInputs(props.todo?.remind_at ?? null)` 这一行，以及 `const remindTimeInput = ref<HTMLInputElement | null>(null)` 这一行。

把 `onMounted` 里的 `else if (props.focus === 'remind') remindTimeInput.value?.focus()` 替换为：

```typescript
    else if (props.focus === 'remind') remindSelect.value?.focus()
```

并在 `const dueTimeInput = ref<HTMLInputElement | null>(null)` 之后插入：

```typescript
const remindSelect = ref<HTMLSelectElement | null>(null)
```

在 `save()` 函数中，把提醒校验与 `input` 构造里的提醒字段替换。即把：

```typescript
  // 提醒时间：日期与时刻必须同时填写或同时留空。
  const remindAt = fromDateAndTimeInputs(remindDate.value, remindTime.value)
  if ((remindDate.value || remindTime.value) && !remindAt) {
    toast('请填写完整的提醒日期与时间，或全部留空')
    return
  }
```

替换为：

```typescript
  // 选了提醒时间却一个渠道都没勾，等于不会提醒，提前拦下避免误以为已生效。
  if (remindOffset.value !== null && !remindDesktop.value && !remindEmail.value) {
    toast('请至少选择一种提醒方式')
    return
  }
```

并把 `input` 对象里的 `remindAt,` 替换为：

```typescript
    remindOffsetMinutes: remindOffset.value,
    remindDesktop: remindDesktop.value,
    remindEmail: remindEmail.value,
```

在 `<template>` 中，把提醒日期与提醒时间那个 `todo-editor-grid` 整块：

```html
    <div class="todo-editor-grid">
      <label class="te-field te-field-date">
        提醒日期（选填）
        <input v-model="remindDate" type="date" :disabled="fieldsDisabled" />
      </label>
      <label class="te-field te-field-time">
        提醒时间（选填）
        <input ref="remindTimeInput" v-model="remindTime" type="time" :disabled="fieldsDisabled" />
      </label>
    </div>
```

替换为：

```html
    <div class="todo-editor-grid">
      <label class="te-field te-field-date">
        提醒时间
        <select ref="remindSelect" v-model="remindOffset" :disabled="fieldsDisabled">
          <option v-for="option in REMIND_OPTIONS" :key="String(option.value)" :value="option.value">
            {{ option.label }}
          </option>
        </select>
      </label>
      <div class="te-field te-remind-channels">
        提醒方式
        <div class="te-channel-row">
          <label class="te-channel">
            <input v-model="remindDesktop" type="checkbox" :disabled="channelsDisabled" />
            桌面弹窗
          </label>
          <label class="te-channel">
            <input v-model="remindEmail" type="checkbox" :disabled="channelsDisabled" />
            邮箱
          </label>
        </div>
      </div>
    </div>
```

把「完成日期 / 完成时间 / 优先级」那一行里的优先级 `<label>` 保持不变。

把 `hint` 计算属性的默认分支文案替换。即把：

```typescript
  return '完成时间为必填；提醒时间留空时使用默认提醒计划'
```

替换为：

```typescript
  return '完成时间为必填；提醒会在所选时间点与完成时间到点各触发一次'
```

- [ ] **Step 4: 改提醒徽章**

把 `src/components/todo/RemindBadge.vue` 整体替换为：

```vue
<script setup lang="ts">
import { computed } from 'vue'
import { remindOffsetLabel } from '@/constants/reminder'

/**
 * 提醒徽章。
 *
 * 展示用户选择的提醒偏移与渠道，点击进入提醒编辑。
 * 偏移为 null（不提醒）时显示淡色占位。
 */
const props = withDefaults(
  defineProps<{
    offsetMinutes: number | null
    desktop: boolean
    email: boolean
    readonly?: boolean
  }>(),
  { readonly: false },
)

const emit = defineEmits<{ (e: 'edit'): void }>()

const label = computed(() => (props.offsetMinutes === null ? '' : remindOffsetLabel(props.offsetMinutes)))

/** 渠道图标：弹窗与邮件各占一个，都没勾时不显示。 */
const channels = computed(() => {
  const marks: string[] = []
  if (props.desktop) marks.push('🔔')
  if (props.email) marks.push('✉️')
  return marks.join('')
})

const title = computed(() =>
  props.offsetMinutes === null
    ? '未设置提醒；点击可设置'
    : `将在完成时间${remindOffsetLabel(props.offsetMinutes)}与到点各提醒一次；点击修改`,
)
</script>

<template>
  <span
    class="remind-badge"
    :class="{ 'no-remind': props.offsetMinutes === null }"
    :title="title"
    @click.stop="!props.readonly && emit('edit')"
  >
    ⏰<template v-if="label"> {{ label }}{{ channels ? ` ${channels}` : '' }}</template>
  </span>
</template>
```

- [ ] **Step 5: 加样式**

在 `src/styles/components.css` 的 `.te-field-priority { ... }` 规则之后插入：

```css
/* 编辑弹窗·提醒方式勾选：与左侧下拉同高，勾选项横向排列。 */
.te-remind-channels {
  flex: 1.2;
}
.te-channel-row {
  display: flex;
  align-items: center;
  gap: 12px;
  height: 30px;
}
.te-channel {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12.5px;
  color: var(--text);
  cursor: pointer;
}
.te-channel input[type='checkbox'] {
  width: auto;
  margin: 0;
  accent-color: var(--accent);
  cursor: pointer;
}
.te-channel input[type='checkbox']:disabled {
  cursor: not-allowed;
}
.te-channel:has(input:disabled) {
  opacity: 0.45;
  cursor: not-allowed;
}
```

- [ ] **Step 6: 修复所有调用点并通过类型检查**

Run: `npx vue-tsc --noEmit 2>&1 | head -30`

按报错逐个修复 `RemindBadge` 的使用处（`src/components/card/TodoTree.vue` 与 `src/windows/Main/TodosView.vue`）：把传 `:remind-at="todo.remind_at"` 与 `:due-at="todo.due_at"` 改为：

```html
        :offset-minutes="todo.remind_offset_minutes"
        :desktop="todo.remind_desktop"
        :email="todo.remind_email"
```

以及 `api.todos.reminder(...)` 的调用处改用新签名。

重复运行直到无输出。

- [ ] **Step 7: 提交**

```bash
npx prettier --write "src/**/*.{ts,vue,css}" && npx vue-tsc --noEmit && npx vite build 2>&1 | tail -1
git add src/typings/domain.ts src/constants/reminder.ts src/service/tauri.ts src/components/todo/ src/components/card/TodoTree.vue src/windows/Main/TodosView.vue src/styles/components.css
git commit -m "feat(todo): 编辑弹窗提醒改为偏移下拉 + 渠道勾选"
```

---

### Task 9: 前端 —— 设置页邮件分区

**Files:**
- Modify: `src/windows/Main/SettingsView.vue`
- Modify: `src/styles/components.css`（设置页邮件分区样式）

**Interfaces:**
- Consumes: Task 8 的 `Settings` SMTP 字段与 `api.mail.test`

- [ ] **Step 1: 加界面与逻辑**

`SettingsView.vue` 已经导入了 `ref`、`api`、`logger` 与 `const { toast } = useToast()`，无需再加 import。
在其 `<script setup>` 末尾追加：

```typescript
/** 测试邮件发送中的状态，避免重复点击。 */
const mailTesting = ref(false)

async function sendTestMail(): Promise<void> {
  if (mailTesting.value) return
  mailTesting.value = true
  logger.info('settings', '发送测试邮件')
  try {
    await api.mail.test()
    toast('测试邮件已发送，请查收')
  } catch (error) {
    logger.error('settings', '发送测试邮件失败', error)
    toast(String(error))
  } finally {
    mailTesting.value = false
  }
}
```

若文件顶部尚未引入 `ref`、`api`、`toast`，一并补上对应 import（`import { computed, ref } from 'vue'`、`import { api } from '@/service/tauri'`、`import { useToast } from '@/composables/useToast'` 与 `const { toast } = useToast()`）。

设置页是平铺的 `.setting-row` 列表（注意是单数），没有分组容器。
在 `<template>` 的 `<div class="settings-body">` 内、最后一个 `.setting-row` 之后插入：

```html
      <div class="setting-section-title">邮件提醒</div>
      <p class="setting-hint">
        请填写邮箱的<strong>应用专用密码</strong>而非主账号密码。配置保存在本地数据库中。
      </p>
      <label class="setting-row">
        <span>SMTP 服务器</span>
        <input
          :value="settings.smtp_host"
          placeholder="smtp.qq.com"
          @change="patch({ smtp_host: ($event.target as HTMLInputElement).value.trim() })"
        />
      </label>
      <label class="setting-row">
        <span>端口</span>
        <input
          :value="settings.smtp_port"
          type="number"
          min="1"
          max="65535"
          @change="patch({ smtp_port: Number(($event.target as HTMLInputElement).value) })"
        />
      </label>
      <label class="setting-row">
        <span>启用 TLS</span>
        <input
          :checked="settings.smtp_tls"
          type="checkbox"
          @change="patch({ smtp_tls: ($event.target as HTMLInputElement).checked })"
        />
      </label>
      <label class="setting-row">
        <span>账号</span>
        <input
          :value="settings.smtp_username"
          @change="patch({ smtp_username: ($event.target as HTMLInputElement).value.trim() })"
        />
      </label>
      <label class="setting-row">
        <span>密码</span>
        <input
          :value="settings.smtp_password"
          type="password"
          placeholder="应用专用密码"
          @change="patch({ smtp_password: ($event.target as HTMLInputElement).value })"
        />
      </label>
      <label class="setting-row">
        <span>发件人</span>
        <input
          :value="settings.smtp_from"
          placeholder="you@example.com"
          @change="patch({ smtp_from: ($event.target as HTMLInputElement).value.trim() })"
        />
      </label>
      <label class="setting-row">
        <span>收件人</span>
        <input
          :value="settings.smtp_to"
          placeholder="you@example.com"
          @change="patch({ smtp_to: ($event.target as HTMLInputElement).value.trim() })"
        />
      </label>
      <div class="setting-row">
        <span>连通性</span>
        <button type="button" class="btn" :disabled="mailTesting" @click="sendTestMail">
          {{ mailTesting ? '发送中…' : '发送测试邮件' }}
        </button>
      </div>
```

注意 `.settings-body select, .settings-body input[type='number']` 已有样式，
但纯文本 `input` 没有，需要在下一步补上，否则 SMTP 各输入框会是浏览器默认外观。

- [ ] **Step 2: 加样式**

在 `src/styles/components.css` 的 `.settings-body select,
.settings-body input[type='number'] { ... }` 规则之后追加：

```css
/* 设置页·文本类输入框：既有规则只覆盖了 select 与 number，
   SMTP 配置用到 text / password，不补样式会是浏览器默认外观。 */
.settings-body input[type='text'],
.settings-body input[type='password'],
.settings-body input:not([type]) {
  flex: 1;
  min-width: 0;
  background: rgba(var(--wsa), 0.1);
  border: 1px solid rgba(var(--wsa), 0.15);
  color: var(--text);
  border-radius: 6px;
  padding: 4px 8px;
  font-size: 12.5px;
  outline: none;
}
.settings-body input[type='text']:focus,
.settings-body input[type='password']:focus,
.settings-body input:not([type]):focus {
  border-color: var(--accent);
}
/* 设置页·分区标题与说明文字 */
.setting-section-title {
  margin: 18px 0 2px;
  padding: 0 4px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text);
  letter-spacing: 0.3px;
}
.setting-hint {
  margin: 0 0 4px;
  padding: 0 4px;
  font-size: 11.5px;
  line-height: 1.6;
  color: var(--text-dim);
}
.setting-hint strong {
  color: var(--text);
  font-weight: 600;
}
```

- [ ] **Step 3: 类型检查与构建**

Run: `npx vue-tsc --noEmit && npx vite build 2>&1 | tail -1`
Expected: 无类型错误，`✓ built in ...`

- [ ] **Step 4: 提交**

```bash
npx prettier --write "src/**/*.{ts,vue,css}"
git add src/windows/Main/SettingsView.vue src/styles/components.css
git commit -m "feat(settings): 设置页新增邮件提醒配置与测试发送"
```

---

### Task 10: 实机验证

**Files:** 无代码改动，仅验证与修复

**Interfaces:** 无

- [ ] **Step 1: 全量构建**

```bash
cd src-tauri && cargo fmt && cargo clippy 2>&1 | grep -E "^error" -A5 | head && cargo test 2>&1 | tail -5 && cd ..
npx vue-tsc --noEmit && npx prettier --check "src/**/*.{ts,vue,css}" && npx vite build 2>&1 | tail -1
```

Expected: clippy 无 error、`test result: ok.`、类型检查与格式检查通过、构建成功

- [ ] **Step 2: 向用户索取 SMTP 凭据**

停下来向用户索取：SMTP 主机、端口、是否 TLS、账号、应用专用密码、发件人、收件人。说明这些只写入本地 SQLite、不进仓库、不在输出中回显。

- [ ] **Step 3: 启动并验证测试邮件**

```bash
taskkill //F //IM inkling.exe 2>&1 | tail -1
(pnpm tauri:dev > /tmp/dev-mail.log 2>&1 &) ; sleep 70
```

在设置页填入凭据后点「发送测试邮件」，确认用户收到信。检查日志：

```bash
grep "\[mailer\]" /tmp/dev-mail.log
```

Expected: `[mailer] 邮件已发送：[Inkling] 测试邮件`

- [ ] **Step 4: 验证提醒链路**

新建一条完成时间在 16 分钟后、偏移选「前 15 分钟」、同时勾选弹窗与邮箱的待办。等待约 1 分钟后检查：

```bash
grep -E "\[reminder\]|\[mailer\]" /tmp/dev-mail.log
```

Expected: 依次出现 `[reminder] 弹窗提醒 todo=... slot=offset`、`[reminder] 邮件提醒入队 todo=... slot=offset`、`[mailer] 邮件已发送`；桌面右上角出现提醒卡片，邮箱收到提醒邮件。

- [ ] **Step 5: 验证迁移**

确认既有待办在升级后的提醒设置为「前 15 分钟 + 只弹窗」：打开归档页任一旧待办的编辑弹窗，检查下拉框显示「前 15 分钟」、桌面弹窗已勾选、邮箱未勾选。

- [ ] **Step 6: 提交并推送**

```bash
taskkill //F //IM inkling.exe 2>&1 | tail -1
git status --short
git push origin feature/tauri-vue
```
