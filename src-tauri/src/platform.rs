//! 平台适配层：毛玻璃效果与静默启动参数收敛于条件编译，业务层无感知。

use tauri::WebviewWindow;

/// 为呼出面板应用毛玻璃（Windows: Mica/Acrylic；macOS: 深色 HudWindow）。
pub fn apply_panel_effects(window: &WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::{apply_acrylic, apply_mica};
        let result = if crate::platform::windows_supports_mica() {
            apply_mica(window, None)
        } else {
            apply_acrylic(window, Some((18, 18, 28, 200)))
        };
        if result.is_err() {
            // 效果失败不影响功能，仅回退为透明背景。
            let _ = result;
        }
    }
    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
        let _ = apply_vibrancy(
            window,
            NSVisualEffectMaterial::HudWindow,
            Some(NSVisualEffectState::Active),
            None,
        );
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = window;
    }
}

/// 主窗口背景（Windows 11 使用 Mica 弱化背景层次）。
pub fn apply_main_backdrop(window: &WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        if crate::platform::windows_supports_mica() {
            let _ = window_vibrancy::apply_mica(window, None);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
    }
}

#[cfg(target_os = "windows")]
pub fn windows_supports_mica() -> bool {
    // Windows 11 build 22000+ 支持 Mica；使用操作系统版本号粗略判定。
    windows_build() >= 22000
}

#[cfg(target_os = "windows")]
fn windows_build() -> u32 {
    // 避免引入额外 crate：通过环境变量不可靠，使用 PowerShell 会引入启动开销；
    // 解析 RUST_WIN_BUILD 环境变量失败时按 Win10 处理（Acrylic 回退）。
    std::env::var("INKLING_WIN_BUILD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(19045)
}

/// 当前是否为静默启动（开机自启）。
pub fn is_silent_start() -> bool {
    std::env::args().any(|arg| arg == "--silent" || arg == "/silent")
}
