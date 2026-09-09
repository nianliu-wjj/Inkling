//! 固定磁盘枚举：仅 DRIVE_FIXED，跳过可移动盘 / 网络盘 / 光驱。
//!
//! 全盘文件索引的扫描起点——把系统当前挂载的固定磁盘根（如 `C:\`、`D:\`）交给
//! scan 逐盘 WalkDir。位掩码解析抽成纯函数便于单测；平台 API 调用仅在 Windows 编译。

use std::path::PathBuf;

/// 把 `GetLogicalDrives` 的位掩码解析成盘符列表（bit0=A … bit25=Z）。纯函数，便于单测。
pub fn drive_letters_from_bitmask(mask: u32) -> Vec<char> {
    (0..26)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| (b'A' + i as u8) as char)
        .collect()
}

/// 枚举所有固定磁盘的根路径。非 Windows 平台返回空。
#[cfg(target_os = "windows")]
pub fn fixed_drive_roots() -> Vec<PathBuf> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};

    // Win32 DRIVE_FIXED 固定为 3（本地磁盘）；windows-sys 0.59 未在该模块导出常量，直接内联。
    const DRIVE_FIXED: u32 = 3;

    let mask = unsafe { GetLogicalDrives() };
    drive_letters_from_bitmask(mask)
        .into_iter()
        .filter_map(|letter| {
            let root = format!("{letter}:\\");
            // GetDriveTypeW 需要以 NUL 结尾的宽字符串。
            let wide: Vec<u16> = root.encode_utf16().chain(std::iter::once(0)).collect();
            let kind = unsafe { GetDriveTypeW(wide.as_ptr()) };
            (kind == DRIVE_FIXED).then(|| PathBuf::from(root))
        })
        .collect()
}

/// 非 Windows：无固定盘概念，返回空（全盘索引仅 Windows 生效）。
#[cfg(not(target_os = "windows"))]
pub fn fixed_drive_roots() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmask_maps_bits_to_letters() {
        // bit0=A, bit2=C, bit3=D
        assert_eq!(drive_letters_from_bitmask(0b1101), vec!['A', 'C', 'D']);
        assert_eq!(drive_letters_from_bitmask(0), Vec::<char>::new());
    }
}
