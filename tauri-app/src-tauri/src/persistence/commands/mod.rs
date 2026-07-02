mod document_ops;
mod export;
mod load;
mod save;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use crate::state::appstate::AppState;

use super::FILE_EXTENSION;


pub use document_ops::*;
pub use export::*;
pub use load::*;
pub use save::*;

fn set_current_file(state: &AppState, name: Option<String>, path: Option<PathBuf>) {
    *state.current_document_name.lock().unwrap() = name;
    *state.current_document_path.lock().unwrap() = path;
}


fn set_export_path(state: &AppState, path: Option<PathBuf>) {
    *state.current_export_path.lock().unwrap() = path;
}

fn name_from_path(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".to_string())
}

fn ensure_pcdoc_extension(path: PathBuf) -> PathBuf {
    if path.extension().and_then(|e| e.to_str()) == Some(FILE_EXTENSION) {
        path
    } else {
        let mut s = path.into_os_string();
        s.push(".");
        s.push(FILE_EXTENSION);
        PathBuf::from(s)
    }
}
