//! 启动动作：打开 / 管理员运行 / 打开所在文件夹。
//!
//! 一律经 explorer / PowerShell 代理并以分离进程方式启动：被启动的程序不成为 Inkling 的子进程，
//! 关掉 Inkling 不影响它们；内置命令（`cmd:*`）不在此处理，由 IPC 层直接执行。

use super::model::{Candidate, Kind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchMode {
    Open,
    Admin,
    Reveal,
}

impl LaunchMode {
    pub fn parse(mode: &str) -> Option<Self> {
        match mode {
            "open" => Some(Self::Open),
            "admin" => Some(Self::Admin),
            "reveal" => Some(Self::Reveal),
            _ => None,
        }
    }
}

/// 执行动作；成功返回 Ok。
pub fn launch(candidate: &Candidate, mode: LaunchMode) -> Result<(), String> {
    if candidate.kind == Kind::Command {
        return Err("内置命令不能在此启动".into());
    }
    eprintln!(
        "[launcher] 启动 mode={mode:?} kind={:?} path={}",
        candidate.kind, candidate.path
    );
    match mode {
        LaunchMode::Open => spawn_detached("explorer.exe", &[candidate.path.as_str()]),
        LaunchMode::Admin => {
            if candidate.kind == Kind::Uwp {
                return Err("商店应用不支持以管理员身份运行".into());
            }
            // 单引号内的单引号翻倍转义
            let escaped = candidate.path.replace('\'', "''");
            spawn_detached(
                "powershell",
                &[
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!("Start-Process -FilePath '{escaped}' -Verb RunAs"),
                ],
            )
        }
        LaunchMode::Reveal => {
            if candidate.kind == Kind::Uwp {
                return Err("商店应用没有所在文件夹".into());
            }
            spawn_detached("explorer.exe", &[&format!("/select,{}", candidate.path)])
        }
    }
}

/// 分离启动：不等待、不接管子进程；Windows 上不弹控制台窗口。
fn spawn_detached(program: &str, args: &[&str]) -> Result<(), String> {
    let mut command = std::process::Command::new(program);
    command.args(args);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        command.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("启动失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_parse_and_command_rejection() {
        assert_eq!(LaunchMode::parse("open"), Some(LaunchMode::Open));
        assert_eq!(LaunchMode::parse("admin"), Some(LaunchMode::Admin));
        assert_eq!(LaunchMode::parse("reveal"), Some(LaunchMode::Reveal));
        assert_eq!(LaunchMode::parse("x"), None);
        let cmd = Candidate {
            id: 0,
            kind: Kind::Command,
            name: "退出".into(),
            path: "cmd:quit".into(),
            keywords: vec!["退出".into()],
            bias: 0.0,
        };
        assert!(launch(&cmd, LaunchMode::Open).is_err());
    }
}
