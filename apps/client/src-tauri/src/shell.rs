//! Overlay window behaviour: summon with Super+Z (or `learn-korean --toggle`), dismiss with Esc.

use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

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

pub fn apply(app: &AppHandle, launch: Launch) {
    let Some(window) = app.get_webview_window(MAIN) else {
        log::error!("main window missing");
        return;
    };
    let result = match launch {
        Launch::Show => show(&window),
        Launch::Hidden => Ok(()),
        Launch::Toggle => toggle(&window),
    };
    if let Err(err) = result {
        log::error!("window {launch:?} failed: {err}");
    }
}

fn toggle(window: &WebviewWindow) -> tauri::Result<()> {
    if window.is_visible()? && window.is_focused()? {
        window.hide()
    } else {
        show(window)
    }
}

fn show(window: &WebviewWindow) -> tauri::Result<()> {
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    window.emit("shell://shown", ())
}

#[tauri::command]
pub fn hide_window(window: WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

/// Registers Super+Z where the platform allows applications to grab global keys (macOS, X11,
/// Windows). Wayland compositors do not; there the compositor binds the key to
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
    let shortcut = Shortcut::new(Some(Modifiers::SUPER), Code::KeyZ);
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
        log::warn!("could not register Super+Z: {err}");
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
