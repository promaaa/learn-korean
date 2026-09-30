//! Overlay window behaviour: summon with Super+Z (or `learn-korean --toggle`), dismiss with Esc.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

use crate::sync;

const MAIN: &str = "main";

/// What a launch (first start or forwarded second instance) asks the window to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launch {
    /// Bring the window up (default).
    Show,
    /// Show if hidden or unfocused, hide otherwise. Bound to Super+Z.
    Toggle,
    /// Start in the background, e.g. from autostart, so the first Super+Z is instant.
    Hidden,
}

impl Launch {
    pub fn from_args<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut launch = Launch::Show;
        for arg in args {
            match arg.as_ref() {
                "--toggle" => launch = Launch::Toggle,
                "--hidden" => launch = Launch::Hidden,
                "--show" => launch = Launch::Show,
                other => log::warn!("ignoring unknown argument {other:?}"),
            }
        }
        launch
    }
}

/// Payload of `shell://shown`.
#[derive(Clone, Serialize)]
struct Shown {
    /// Progress from another device was merged: the running session is stale.
    synced: bool,
}

pub fn apply(app: &AppHandle, launch: Launch) {
    let Some(window) = app.get_webview_window(MAIN) else {
        log::error!("main window missing");
        return;
    };
    let result = match launch {
        Launch::Show => {
            summon(app, window);
            Ok(())
        }
        Launch::Hidden => Ok(()),
        Launch::Toggle => toggle(app, window),
    };
    if let Err(err) = result {
        log::error!("window {launch:?} failed: {err}");
    }
}

fn toggle(app: &AppHandle, window: WebviewWindow) -> tauri::Result<()> {
    if window.is_visible()? && window.is_focused()? {
        hide(app, &window)?;
    } else {
        summon(app, window);
    }
    Ok(())
}

/// Hides the window. On macOS the app is hidden too, as Cmd+H does, so the keyboard goes back
/// to the previous app instead of staying with an app that has no visible window.
fn hide(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
    window.hide()?;
    #[cfg(target_os = "macos")]
    app.hide()?;
    sync::push_later(app);
    Ok(())
}

/// Merges the other devices' progress, then shows the window.
fn summon(app: &AppHandle, window: WebviewWindow) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let synced = sync::pull_now(&app).await;
        if let Err(err) = show(&window, synced) {
            log::error!("showing the window failed: {err}");
        }
    });
}

fn show(window: &WebviewWindow, synced: bool) -> tauri::Result<()> {
    // Unhides and activates the app hidden by `hide`; showing the window alone would leave it
    // behind the active app, without the keyboard.
    #[cfg(target_os = "macos")]
    window.app_handle().show()?;
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    window.emit("shell://shown", Shown { synced })
}

#[tauri::command]
pub fn hide_window(window: WebviewWindow) -> Result<(), String> {
    hide(window.app_handle(), &window).map_err(|e| e.to_string())
}

/// Physical key (`Code`, a QWERTY position) that carries the letter Z in a macOS keyboard
/// layout, e.g. `com.apple.keylayout.French-PC`: global shortcuts are registered by position,
/// but the learner presses the key labelled Z.
fn z_key_for_layout(layout: &str) -> tauri_plugin_global_shortcut::Code {
    use tauri_plugin_global_shortcut::Code;
    let name = layout.rsplit('.').next().unwrap_or(layout);
    const AZERTY: [&str; 2] = ["French", "Belgian"];
    const QWERTZ: [&str; 12] = [
        "German",
        "Swiss",
        "Austrian",
        "Czech",
        "Slovak",
        "Hungarian",
        "Croatian",
        "Slovenian",
        "Serbian-Latin",
        "Polish",
        "Bosnian",
        "Albanian",
    ];
    if name.contains("Dvorak") {
        Code::Slash
    } else if name.contains("QWERTY") || name == "PolishPro" {
        Code::KeyZ
    } else if AZERTY.iter().any(|l| name.starts_with(l)) {
        Code::KeyW
    } else if QWERTZ.iter().any(|l| name.starts_with(l)) {
        Code::KeyY
    } else {
        Code::KeyZ
    }
}

/// The current macOS keyboard layout id, if one is selected.
#[cfg(target_os = "macos")]
fn current_layout() -> Option<String> {
    let out = std::process::Command::new("defaults")
        .args([
            "read",
            "com.apple.HIToolbox",
            "AppleCurrentKeyboardLayoutInputSourceID",
        ])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Registers the global shortcut where the platform allows applications to grab global keys:
/// Ctrl+Option+Z on macOS, where Super is Command and Command+Z is Undo in every app; Super+Z on
/// X11 and Windows. Wayland compositors do not; there the compositor binds the key to
/// `learn-korean --toggle` (see `docs/omarchy.md`).
#[cfg(desktop)]
pub fn register_global_shortcut(app: &AppHandle) {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    if cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some() {
        log::info!("Wayland session: global shortcut is provided by the compositor");
        return;
    }
    let (modifiers, key, label) = if cfg!(target_os = "macos") {
        #[cfg(target_os = "macos")]
        let layout = current_layout().unwrap_or_default();
        #[cfg(not(target_os = "macos"))]
        let layout = String::new();
        let key = z_key_for_layout(&layout);
        log::info!("global shortcut: Ctrl+Option+Z is {key:?} in layout {layout:?}");
        (Modifiers::CONTROL | Modifiers::ALT, key, "Ctrl+Option+Z")
    } else {
        (Modifiers::SUPER, Code::KeyZ, "Super+Z")
    };
    let shortcut = Shortcut::new(Some(modifiers), key);
    let plugin = tauri_plugin_global_shortcut::Builder::new()
        .with_handler(move |app, pressed, event| {
            if pressed == &shortcut && event.state() == ShortcutState::Pressed {
                apply(app, Launch::Toggle);
            }
        })
        .build();
    if let Err(err) = app.plugin(plugin) {
        log::error!("global shortcut plugin: {err}");
        return;
    }
    if let Err(err) = app.global_shortcut().register(shortcut) {
        log::warn!("could not register {label}: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::{Launch, z_key_for_layout};
    use tauri_plugin_global_shortcut::Code;

    #[test]
    fn no_argument_shows_the_window() {
        assert_eq!(Launch::from_args(Vec::<String>::new()), Launch::Show);
    }

    #[test]
    fn toggle_and_hidden_flags() {
        assert_eq!(Launch::from_args(["--toggle"]), Launch::Toggle);
        assert_eq!(Launch::from_args(["--hidden"]), Launch::Hidden);
    }

    #[test]
    fn last_flag_wins_and_unknown_flags_are_ignored() {
        assert_eq!(
            Launch::from_args(["--hidden", "--bogus", "--toggle"]),
            Launch::Toggle
        );
    }

    #[test]
    fn z_is_found_where_each_layout_puts_it() {
        for (layout, code) in [
            ("com.apple.keylayout.US", Code::KeyZ),
            ("com.apple.keylayout.ABC", Code::KeyZ),
            ("com.apple.keylayout.French-PC", Code::KeyW),
            ("com.apple.keylayout.French", Code::KeyW),
            ("com.apple.keylayout.Belgian", Code::KeyW),
            ("com.apple.keylayout.German", Code::KeyY),
            ("com.apple.keylayout.SwissFrench", Code::KeyY),
            ("com.apple.keylayout.Czech-QWERTY", Code::KeyZ),
            ("com.apple.keylayout.PolishPro", Code::KeyZ),
            ("com.apple.keylayout.Dvorak", Code::Slash),
            ("", Code::KeyZ),
        ] {
            assert_eq!(z_key_for_layout(layout), code, "{layout}");
        }
    }
}
