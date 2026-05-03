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
use log::{debug, info};
use rand::random;
use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_config = AppConfig::load();
    let logging = app_config.logging.clone();
    let tauri_level = logging.level_filter();
    info!("Application configuration loaded");
    #[cfg(debug_assertions)]
    let mut log_targets: Vec<Target> = vec![
        Target::new(TargetKind::Webview),
        Target::new(TargetKind::Stdout),
    ];
    #[cfg(not(debug_assertions))]
    let mut log_targets: Vec<Target> = vec![Target::new(TargetKind::Webview)];
    if logging.tauri_file_logs {
        log_targets.push(Target::new(TargetKind::LogDir {
            file_name: Some("peercode".into()),
        }));
    }
    info!(
        "initializing tauri logging: level={:?}, file_logs_enabled={}, target_count={}",
        tauri_level,
        logging.tauri_file_logs,
        log_targets.len()
    );

    tauri::Builder::default()
        .setup(move |app| {
            let client_id = ClientId::new(random::<u64>());
            info!("Setting up tauri app state");
            app.manage(AppState::new(client_id));
            app.manage(WsState::new(app_config.websocket.connect_timeout()));
            app.manage(app_config.clone());
            debug!(
                "state initialized: websocket_connect_timeout_ms={}",
                app_config.websocket.connect_timeout_ms
            );

            #[cfg(debug_assertions)]
            {
                spawn_linked_list_logger(app.handle().clone());
                debug!("debug linked-list logger spawned");
            }

            info!("tauri setup completed");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                info!("window destroyed; tearing down host resources");
                window.state::<AppState>().teardown_host();
            }
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_level)
                .targets(log_targets)
                .build(),
        )
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
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
            persistence::commands::get_document_text,
            persistence::commands::get_current_document_name,
            persistence::commands::save_text_file,
            #[cfg(debug_assertions)]
            crdt_handler::toggle_crdt_logging
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
