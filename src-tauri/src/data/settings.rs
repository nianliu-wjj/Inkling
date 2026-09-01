//! 偏好设置数据访问。

use super::{db_err, Store};
use crate::domain::models::Settings;

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
        })
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
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
