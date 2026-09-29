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
        window.hide()?;
        sync::push_later(app);
    } else {
        summon(app, window);
    }
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
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    window.emit("shell://shown", Shown { synced })
}

#[tauri::command]
pub fn hide_window(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())?;
    sync::push_later(window.app_handle());
    Ok(())
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
    let (modifiers, label) = if cfg!(target_os = "macos") {
        (Modifiers::CONTROL | Modifiers::ALT, "Ctrl+Option+Z")
    } else {
        (Modifiers::SUPER, "Super+Z")
    };
    let shortcut = Shortcut::new(Some(modifiers), Code::KeyZ);
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
    use super::Launch;

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
}
