use std::path::{Path, PathBuf};

use super::{ensure_pcdoc_extension, name_from_path};

#[test]
fn appends_extension_when_missing() {
    assert_eq!(
        ensure_pcdoc_extension(PathBuf::from("/tmp/notes")),
        PathBuf::from("/tmp/notes.pcdoc")
    );
}

#[test]
fn keeps_existing_pcdoc_extension() {
    assert_eq!(
        ensure_pcdoc_extension(PathBuf::from("/tmp/notes.pcdoc")),
        PathBuf::from("/tmp/notes.pcdoc")
    );
}

#[test]
fn appends_instead_of_replacing_dotted_names() {
    assert_eq!(
        ensure_pcdoc_extension(PathBuf::from("/tmp/notes.v2")),
        PathBuf::from("/tmp/notes.v2.pcdoc")
    );
}

#[test]
fn name_from_path_uses_file_stem() {
    assert_eq!(name_from_path(Path::new("/home/user/foo.py")), "foo");
    assert_eq!(name_from_path(Path::new("bar.pcdoc")), "bar");
}

#[test]
fn name_from_path_falls_back_when_no_stem() {
    assert_eq!(name_from_path(Path::new("/")), "document");
}
