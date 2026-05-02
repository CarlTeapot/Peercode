mod app_config;
mod crdt;
mod debug;
mod persistence;
mod processes;
mod session;
mod state;
mod ws_management;

use crate::app_config::config::AppConfig;
use crate::app_config::identity;
use crate::crdt::crdt_handler;
use crate::debug::document_logger::spawn_linked_list_logger;
use crate::state::appstate::AppState;
use crate::state::ws_state::WsState;
use crdt_core::types::ClientId;
use rand::random;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_config = AppConfig::load();

            app.manage(AppState::new(ClientId::new(random::<u64>())));
            app.manage(WsState::new(app_config.websocket.connect_timeout()));
            app.manage(app_config);

            #[cfg(debug_assertions)]
            spawn_linked_list_logger(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                window.state::<AppState>().teardown_host();
            }
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            crdt_handler::insert,
            crdt_handler::delete,
            session::command::start_host_session,
            session::command::stop_host_session,
            session::command::join_session,
            session::command::disconnect_websocket,
            session::command::parse_join_url,
            session::command::get_session_info,
            identity::get_identity,
            identity::set_username,
            persistence::commands::save_document,
            persistence::commands::load_document,
            persistence::commands::list_saved_documents,
            persistence::commands::fork_document,
            persistence::commands::delete_document,
            #[cfg(debug_assertions)]
            crdt_handler::toggle_crdt_logging
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
