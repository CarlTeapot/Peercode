mod app_config;
mod crdt;
mod debug;
mod gateway;
mod persistence;
mod processes;
mod session;
mod state;
mod ws_management;

use crate::app_config::config::AppConfig;
use crate::app_config::identity;
use crate::crdt::local_op_handler;
#[cfg(debug_assertions)]
use crate::debug::document_logger::spawn_linked_list_logger;
use crate::gateway::gateway_api::destroy_room;
use crate::state::appstate::{AppRole, AppState};
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
                let state = window.state::<AppState>();
                let local_room_url = {
                    let role = state.role.lock().unwrap();
                    match &*role {
                        AppRole::Host { local_room_url, .. } => Some(local_room_url.clone()),
                        _ => None,
                    }
                };
                if let Some(url) = local_room_url {
                    if let Err(e) = tauri::async_runtime::block_on(destroy_room(url)) {
                        warn!("destroy_room on window close: {e}");
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
            local_op_handler::insert,
            local_op_handler::delete,
            session::host_commands::host_session,
            session::host_commands::end_session,
            session::host_commands::kill_host_processes,
            session::guest_commands::join_session,
            session::guest_commands::parse_join_url,
            session::joint_commands::get_session_info,
            session::joint_commands::leave_session,
            processes::commands::get_process_status,
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
            persistence::commands::reset_document,
            #[cfg(debug_assertions)]
            local_op_handler::toggle_crdt_logging
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
