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

/// 当前前台应用的名字（可执行文件名去掉 `.exe`），用于标注粘贴板条目来源（spec D20）。
///
/// 只在 Windows 上实现：`GetForegroundWindow` → `GetWindowThreadProcessId` →
/// `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` → `QueryFullProcessImageNameW`。
/// 前台是本应用自身（面板里手动捕获）时返回 None——回声抑制之外再防一手；
/// 提权进程 `OpenProcess` 会失败，同样返回 None，前端不渲染来源。
pub fn foreground_app_name() -> Option<String> {
    foreground_app_path().and_then(|path| app_name_from_path(&path))
}

#[cfg(target_os = "windows")]
fn foreground_app_path() -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    // SAFETY: 全部是只读查询类 Win32 调用；进程句柄在本函数内打开并在返回前关闭，
    // 缓冲区由我们分配并把长度传给系统，系统只在长度范围内写入。
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 || pid == GetCurrentProcessId() {
            return None;
        }
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            eprintln!("[platform] 打开前台进程 {pid} 失败（可能是提权进程），来源留空");
            return None;
        }
        let mut buffer = [0u16; 1024];
        let mut length = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            buffer.as_mut_ptr(),
            &mut length,
        );
        CloseHandle(handle);
        if ok == 0 || length == 0 {
            return None;
        }
        Some(String::from_utf16_lossy(&buffer[..length as usize]))
    }
}

#[cfg(not(target_os = "windows"))]
fn foreground_app_path() -> Option<String> {
    None
}

/// 从可执行文件完整路径取应用名：只留文件名，去掉 `.exe` 后缀（不区分大小写）。
///
/// 抽成纯函数以便单测；空路径或以分隔符结尾（无文件名）返回 None。
pub fn app_name_from_path(path: &str) -> Option<String> {
    let file = path.rsplit(['\\', '/']).next()?.trim();
    if file.is_empty() {
        return None;
    }
    let name = match file.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && ext.eq_ignore_ascii_case("exe") => stem,
        _ => file,
    };
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::app_name_from_path;

    /// 只取文件名并去掉 .exe（不区分大小写）；空路径与目录结尾返回 None。
    #[test]
    fn app_name_from_path_strips_directory_and_exe() {
        assert_eq!(
            app_name_from_path(r"C:\Windows\System32\notepad.exe").as_deref(),
            Some("notepad")
        );
        assert_eq!(
            app_name_from_path(r"C:\Program Files\App\Code.EXE").as_deref(),
            Some("Code")
        );
        assert_eq!(app_name_from_path("/usr/bin/vim").as_deref(), Some("vim"));
        assert!(app_name_from_path("").is_none());
        assert!(app_name_from_path(r"C:\dir\").is_none());
    }
}
