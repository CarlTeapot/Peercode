use log::{debug, error, info};
use tauri_plugin_shell::process::CommandEvent;
use tokio::sync::mpsc::Receiver;

pub async fn pipe_process_logs(name: &str, mut rx: Receiver<CommandEvent>) {
    debug!("process log pipe started: name={name}");
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(b) => {
                info!("[{name}] {}", String::from_utf8_lossy(&b).trim_end());
            }
            CommandEvent::Stderr(b) => {
                error!("[{name}] {}", String::from_utf8_lossy(&b).trim_end());
            }
            CommandEvent::Terminated(status) => {
                info!("[{name}] terminated: {status:?}");
                break;
            }
            _ => {
                debug!("process log pipe ignored non-output event: name={name}");
            }
        }
    }
    debug!("process log pipe stopped: name={name}");
}
