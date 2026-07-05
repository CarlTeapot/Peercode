use std::path::PathBuf;

use super::read_tests::TempFile;
use super::recents::{meta_for_path, push_front_dedup};

#[test]
fn push_front_moves_existing_entry_to_front() {
    let recents = vec![PathBuf::from("/a"), PathBuf::from("/b")];
    let result = push_front_dedup(recents, PathBuf::from("/b"));
    assert_eq!(result, vec![PathBuf::from("/b"), PathBuf::from("/a")]);
}

#[test]
fn push_front_truncates_to_fifty() {
    let recents: Vec<PathBuf> = (0..50).map(|i| PathBuf::from(format!("/f{i}"))).collect();
    let result = push_front_dedup(recents, PathBuf::from("/new"));
    assert_eq!(result.len(), 50);
    assert_eq!(result[0], PathBuf::from("/new"));
}

#[test]
fn meta_uses_the_full_file_name() {
    let file = TempFile::new(b"hello");
    let meta = meta_for_path(&file.0).unwrap();
    assert_eq!(meta.name, file.0.file_name().unwrap().to_string_lossy());
    assert_eq!(meta.size_bytes, 5);
}

#[test]
fn meta_for_missing_path_is_none() {
    assert!(meta_for_path(std::path::Path::new("/does/not/exist")).is_none());
}
