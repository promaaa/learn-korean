mod session;
mod shell;
mod speech;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // Must be registered first: a second launch forwards its CLI to this instance and exits.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
        shell::apply(app, shell::Launch::from_args(argv.iter().skip(1)));
    }));

    builder
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .manage(session::ActiveSession::default())
        .invoke_handler(tauri::generate_handler![
            shell::hide_window,
            state::app_status,
            session::session_start,
            session::session_current,
            session::session_answer,
            speech::speak,
        ])
        .setup(|app| {
            // Created here, not on the builder: a second instance exits before setup runs, so it
            // never opens the database or the audio device.
            app.manage(state::init(app.handle()));
            app.manage(speech::Speech::new());
            #[cfg(desktop)]
            shell::register_global_shortcut(app.handle());
            shell::apply(
                app.handle(),
                shell::Launch::from_args(std::env::args().skip(1)),
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running learn-korean");
}
