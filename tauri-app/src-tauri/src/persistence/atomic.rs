use std::fs;
use std::path::{Path, PathBuf};

use super::PersistError;

/// Writes via a sibling temp file + rename so a crash never leaves a
/// half-written target.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), PersistError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp = tmp_path(path);
    fs::write(&tmp, data)?;

    if let Err(e) = fs::rename(&tmp, path) {
        if cfg!(windows) && path.exists() {
            if let Err(remove_err) = fs::remove_file(path) {
                let _ = fs::remove_file(&tmp);
                return Err(PersistError::Io(remove_err));
            }
            if let Err(rename_err) = fs::rename(&tmp, path) {
                let _ = fs::remove_file(&tmp);
                return Err(PersistError::Io(rename_err));
            }
        } else {
            let _ = fs::remove_file(&tmp);
            return Err(PersistError::Io(e));
        }
    }

    Ok(())
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_else(|| "file".into());
    name.push(".tmp");
    path.with_file_name(name)
}
