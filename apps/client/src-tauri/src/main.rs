// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    disable_webkit_dmabuf_on_nvidia();
    learn_korean_lib::run();
}

/// WebKitGTK's DMA-BUF renderer crashes on the proprietary NVIDIA driver under Wayland
/// ("Error 71 (Protocol error) dispatching to Wayland display").
#[cfg(target_os = "linux")]
fn disable_webkit_dmabuf_on_nvidia() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(VAR).is_none() && std::path::Path::new("/proc/driver/nvidia").exists() {
        // SAFETY: called first thing in `main`, before any other thread exists.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var(VAR, "1");
        }
    }
}
