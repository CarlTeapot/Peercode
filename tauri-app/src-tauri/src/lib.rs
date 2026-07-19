mod app_config;
mod debug;
mod garbage_collection;
mod gateway;
mod persistence;
mod processes;
mod session;
mod state;
mod ws_management;

use crate::app_config::config::AppConfig;
#[cfg(debug_assertions)]
use crate::debug::document_logger::spawn_linked_list_logger;
use crate::gateway::gateway_api::destroy_room;
use crate::state::appstate::{AppRole, AppState};
use crate::state::document::{commands as doc_commands, spawn as spawn_doc_actor};
use crate::state::ws_state::WsState;
use crdt_core::types::ClientId;
use log::{debug, info, warn};
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

            let doc_tx = spawn_doc_actor(client_id, app.handle().clone());
            app.manage(AppState::new(doc_tx));
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
                let state = window.state::<AppState>();
                let local_room_url = match state.current_role() {
                    AppRole::Host { local_room_url, .. } => Some(local_room_url),
                    _ => None,
                };
                if let Some(url) = local_room_url {
                    match state.gateway_auth_token() {
                        Some(ref t) => {
                            if let Err(e) = tauri::async_runtime::block_on(destroy_room(url, t)) {
                                warn!("destroy_room on window close: {e}");
                            }
                        }
                        None => warn!("destroy_room on window close: gateway token missing"),
                    }
                }
                state.leave_session(&window.state::<WsState>());
                state.kill_host_processes();
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
            doc_commands::insert,
            doc_commands::delete,
            doc_commands::replace,
            session::host_commands::host_session,
            session::host_commands::end_session,
            session::host_commands::kill_host_processes,
            session::guest_commands::join_session,
            session::guest_commands::parse_join_url,
            session::joint_commands::get_session_info,
            session::joint_commands::leave_session,
            session::permission_commands::set_peer_permission,
            processes::commands::get_process_status,
            app_config::commands::get_identity,
            app_config::commands::set_username,
            persistence::commands::open_file,
            persistence::commands::save_file,
            persistence::commands::save_file_as,
            persistence::commands::list_recent_files,
            persistence::commands::remove_recent_file,
            persistence::commands::get_documents_dir,
            persistence::commands::get_current_file,
            persistence::commands::fork_document,
            persistence::commands::reset_document,
            #[cfg(debug_assertions)]
            doc_commands::toggle_crdt_logging
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
