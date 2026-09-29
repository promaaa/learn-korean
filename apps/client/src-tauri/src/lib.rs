mod shell;
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
        .invoke_handler(tauri::generate_handler![
            shell::hide_window,
            state::app_status
        ])
        .setup(|app| {
            let state = state::init(app.handle());
            app.manage(state);
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
