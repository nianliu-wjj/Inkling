//! 构建脚本：将应用图标嵌入 Windows 可执行文件（任务栏 / 资源管理器图标）。

fn main() {
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        winresource::WindowsResource::new()
            .set_icon("assets/icon.ico")
            .compile()
            .expect("嵌入应用图标失败（assets/icon.ico）");
    }
}
