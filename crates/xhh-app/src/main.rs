// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK 的 DMABUF 渲染路径在 NVIDIA 驱动上滚动掉帧，禁用后回退 GL 渲染
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    xhh_app_lib::run()
}
