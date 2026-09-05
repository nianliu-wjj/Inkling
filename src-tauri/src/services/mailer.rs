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
        host: settings.smtp_host().clone(),
        port: *settings.smtp_port(),
        tls: *settings.smtp_tls(),
        username: settings.smtp_username().clone(),
        password,
        from: settings.smtp_from().clone(),
        to: settings.smtp_to().clone(),
    })
}

/// 邮件提醒是否已具备发送条件，供保存待办时拦截使用。
pub fn is_configured(app: &AppHandle) -> bool {
    load_config(app)
        .map(|config| config.validate().is_ok())
        .unwrap_or(false)
}

/// 同步发送一封邮件。供「发送测试邮件」按钮直接调用，错误原样回传给界面。
pub fn send_now(app: &AppHandle, request: &MailRequest) -> Result<(), String> {
    let config = load_config(app)?;
    config.validate()?;
    deliver(&config, request)
}

fn deliver(config: &MailConfig, request: &MailRequest) -> Result<(), String> {
    let email = Message::builder()
        .from(
            config
                .from
                .parse()
                .map_err(|e| format!("发件人地址无效: {e}"))?,
        )
        .to(config
            .to
            .parse()
            .map_err(|e| format!("收件人地址无效: {e}"))?)
        .subject(request.subject.clone())
        .body(request.body.clone())
        .map_err(|e| format!("构造邮件失败: {e}"))?;

    // TLS 端口用隐式 TLS（relay），非 TLS 走明文连接（仅适用于内网自建服务）。
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
