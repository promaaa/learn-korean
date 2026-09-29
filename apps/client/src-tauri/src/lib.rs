mod images;
mod memory;
mod progress;
mod session;
mod shell;
mod speech;
mod state;
mod sync;
mod typing;

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
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(false) = event {
                sync::push_later(window.app_handle());
            }
        })
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .manage(session::ActiveSession::default())
        .manage(typing::TypingGym::default())
        .register_asynchronous_uri_scheme_protocol(images::SCHEME, |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                responder.respond(images::serve(&app, request.uri().path()).await);
            });
        })
        .invoke_handler(tauri::generate_handler![
            shell::hide_window,
            state::app_status,
            session::session_start,
            session::session_current,
            session::session_answer,
            session::set_focus,
            speech::speak,
            images::item_image,
            progress::profile,
            typing::typing_layout,
            typing::typing_start,
            typing::typing_press,
            typing::typing_next,
        ])
        .setup(|app| {
            // Created here, not on the builder: a second instance exits before setup runs, so it
            // never opens the database or the audio device.
            let state = state::init(app.handle());
            let progress_sync = sync::ProgressSync::load(app.handle());
            // Before the UI starts a session, so it plans from the merged progress.
            if let Ok(db) = &state.db {
                tauri::async_runtime::block_on(async {
                    progress_sync.pull(db).await;
                    memory::replay_logged(db.pool()).await;
                });
            }
            app.manage(state);
            app.manage(progress_sync);
            app.manage(speech::Speech::new());
            app.manage(images::Images::new());
            memory::optimize_later(app.handle());
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
