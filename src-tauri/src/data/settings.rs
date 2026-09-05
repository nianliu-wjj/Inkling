//! 偏好设置数据访问。

use super::{db_err, Store};
use crate::domain::models::Settings;

/// 取字符串设置项，缺失时回落到默认值。
///
/// 17 个字段里大半都是这个形状，抽出来避免 builder 链里堆满
/// `.get(k).cloned().unwrap_or(defaults.x().clone())`。
fn pick(values: &std::collections::HashMap<String, String>, key: &str, fallback: &str) -> String {
    values
        .get(key)
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

/// 取布尔设置项：存储层统一用 "true" / "false" 字符串。
fn flag(values: &std::collections::HashMap<String, String>, key: &str, fallback: bool) -> bool {
    values.get(key).map(|x| x == "true").unwrap_or(fallback)
}

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
        Settings::builder()
            .collapse_policy(pick(&values, "collapse_policy", defaults.collapse_policy()))
            .clipboard_retention_days(
                values
                    .get("clipboard_retention_days")
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(*defaults.clipboard_retention_days()),
            )
            .start_on_boot(values.get("start_on_boot").is_some_and(|x| x == "true"))
            .shortcut(pick(&values, "shortcut", defaults.shortcut()))
            .remark_style(pick(&values, "remark_style", defaults.remark_style()))
            .theme(pick(&values, "theme", defaults.theme()))
            // 缺省视为开启，与 Settings::default 一致。
            .main_acrylic(flag(&values, "main_acrylic", *defaults.main_acrylic()))
            .panel_position(pick(&values, "panel_position", defaults.panel_position()))
            .panel_plugins(values.get("panel_plugins").cloned().unwrap_or_default())
            .glass_level(pick(&values, "glass_level", defaults.glass_level()))
            .smtp_host(values.get("smtp_host").cloned().unwrap_or_default())
            .smtp_port(
                values
                    .get("smtp_port")
                    .and_then(|x| x.parse().ok())
                    .unwrap_or(*defaults.smtp_port()),
            )
            .smtp_tls(flag(&values, "smtp_tls", *defaults.smtp_tls()))
            .smtp_username(values.get("smtp_username").cloned().unwrap_or_default())
            // 空密码不加掩码，前端据此判断「尚未配置」。
            .smtp_password(if stored_password.is_empty() {
                String::new()
            } else {
                SMTP_PASSWORD_MASK.into()
            })
            .smtp_from(values.get("smtp_from").cloned().unwrap_or_default())
            .smtp_to(values.get("smtp_to").cloned().unwrap_or_default())
            .build()
    }

    /// 读取真实 SMTP 密码，仅供发信服务使用。
    pub fn smtp_password_raw(&self) -> Result<String, String> {
        Ok(self.setting_value("smtp_password")?.unwrap_or_default())
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        // 掩码代表「不改密码」：保留库中原值，否则前端一保存就会把真实密码冲成掩码串。
        let password = if settings.smtp_password() == SMTP_PASSWORD_MASK {
            self.smtp_password_raw()?
        } else {
            settings.smtp_password().clone()
        };
        for (key, value) in [
            ("collapse_policy", settings.collapse_policy().clone()),
            (
                "clipboard_retention_days",
                settings.clipboard_retention_days().to_string(),
            ),
            ("start_on_boot", settings.start_on_boot().to_string()),
            ("shortcut", settings.shortcut().clone()),
            ("remark_style", settings.remark_style().clone()),
            ("theme", settings.theme().clone()),
            ("main_acrylic", settings.main_acrylic().to_string()),
            ("panel_position", settings.panel_position().clone()),
            ("panel_plugins", settings.panel_plugins().clone()),
            ("glass_level", settings.glass_level().clone()),
            ("smtp_host", settings.smtp_host().clone()),
            ("smtp_port", settings.smtp_port().to_string()),
            ("smtp_tls", settings.smtp_tls().to_string()),
            ("smtp_username", settings.smtp_username().clone()),
            ("smtp_password", password),
            ("smtp_from", settings.smtp_from().clone()),
            ("smtp_to", settings.smtp_to().clone()),
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
        settings.set_smtp_password("secret-token".into());
        settings.set_smtp_host("smtp.example.com".into());
        store.save_settings(&settings).unwrap();

        // 读出来是掩码，真实密码不外泄给前端。
        let read = store.get_settings().unwrap();
        assert_eq!(read.smtp_password(), SMTP_PASSWORD_MASK);
        assert_eq!(read.smtp_host(), "smtp.example.com");

        // 前端拿着掩码回存其他项，密码保持不变。
        let mut again = read.clone();
        again.set_smtp_host("smtp.other.com".into());
        store.save_settings(&again).unwrap();
        assert_eq!(store.smtp_password_raw().unwrap(), "secret-token");
        assert_eq!(store.get_settings().unwrap().smtp_host(), "smtp.other.com");
    }

    #[test]
    fn empty_password_is_not_masked() {
        let store = store();
        assert_eq!(store.get_settings().unwrap().smtp_password(), "");
    }
}
