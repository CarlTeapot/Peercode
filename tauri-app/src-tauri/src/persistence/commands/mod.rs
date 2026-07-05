mod document_ops;
mod open;
mod recent;
mod save;
#[cfg(test)]
mod tests;

use crate::state::appstate::{AppState, CurrentFile};

pub use document_ops::*;
pub use open::*;
pub use recent::*;
pub use save::*;

fn set_current_file(state: &AppState, file: Option<CurrentFile>) {
    *state.current_file.lock().unwrap() = file;
}

fn name_from_path(path: &std::path::Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "untitled".to_string())
}
