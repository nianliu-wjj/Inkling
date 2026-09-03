//! 平台适配层：毛玻璃效果与静默启动参数收敛于条件编译，业务层无感知。

use tauri::WebviewWindow;

/// 为窗口应用（或撤销）背景效果。
///
/// Windows：优先 Mica（Win11），失败则回退 Acrylic（Win10+），再失败则保持透明。
/// 这里**不做系统版本探测**——`apply_mica` 在不支持的系统上会返回 Err，
/// 直接以返回值为准比版本号判断更可靠（此前的版本探测读取一个从未设置的
/// 环境变量并硬编码回退为 Win10，导致 Win11 上毛玻璃始终不生效）。
///
/// macOS：使用深色 HudWindow 材质。
/// 其他平台：无对应实现，保持透明，由 CSS 的 `[data-acrylic="off"]` 降级配色兜底。
pub fn apply_backdrop(window: &WebviewWindow, enabled: bool) {
    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::{apply_acrylic, apply_mica, clear_acrylic, clear_mica};

        if !enabled {
            // 两种效果都尝试清除：此前可能应用的是其中任意一种。
            let _ = clear_mica(window);
            let _ = clear_acrylic(window);
            return;
        }

        if apply_mica(window, None).is_ok() {
            return;
        }
        // Mica 不可用（Win10 或系统策略禁用）时回退 Acrylic。
        if let Err(error) = apply_acrylic(window, Some((18, 18, 28, 200))) {
            eprintln!("[platform] 毛玻璃不可用，窗口将保持透明：{error}");
        }
    }

    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};
        if enabled {
            let _ = apply_vibrancy(
                window,
                NSVisualEffectMaterial::HudWindow,
                Some(NSVisualEffectState::Active),
                None,
            );
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (window, enabled);
    }
}

/// 把窗口四角改成圆角。
///
/// 无边框窗口（decorations(false)）在 Windows 上默认是直角，而 Mica / Acrylic 是由
/// **DWM 在窗口矩形上**绘制的——CSS 的 border-radius / clip-path 只作用于 WebView 内容，
/// 裁不掉底下那层系统背景，面板看起来就是个方块。改用 DWM 的圆角偏好，
/// 让系统连同毛玻璃一起按圆角裁剪。
///
/// 仅 Windows 11 (build 22000+) 支持，旧系统上该属性被忽略，返回值一并忽略即可。
pub fn apply_rounded_corners(window: &WebviewWindow) {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::HWND;
        use windows_sys::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        };

        let Ok(handle) = window.hwnd() else {
            eprintln!("[platform] 取窗口句柄失败，跳过圆角设置");
            return;
        };
        let preference = DWMWCP_ROUND;
        // SAFETY: handle 来自 Tauri 的有效窗口；传入的指针与长度描述的是同一个 i32 值。
        unsafe {
            DwmSetWindowAttribute(
                handle.0 as HWND,
                DWMWA_WINDOW_CORNER_PREFERENCE as u32,
                &preference as *const _ as *const core::ffi::c_void,
                core::mem::size_of_val(&preference) as u32,
            );
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = window;
    }
}

/// 为呼出面板应用毛玻璃与圆角。面板始终启用效果，不受主窗口开关影响。
pub fn apply_panel_effects(window: &WebviewWindow) {
    apply_backdrop(window, true);
    apply_rounded_corners(window);
}

/// 主窗口背景。是否启用由偏好设置 `main_acrylic` 决定。
pub fn apply_main_backdrop(window: &WebviewWindow, enabled: bool) {
    apply_backdrop(window, enabled);
}

/// 当前是否为静默启动（开机自启）。
pub fn is_silent_start() -> bool {
    std::env::args().any(|arg| arg == "--silent" || arg == "/silent")
}
