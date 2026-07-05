use std::path::Path;

use super::name_from_path;

#[test]
fn name_is_the_full_file_name() {
    assert_eq!(name_from_path(Path::new("/tmp/main.py")), "main.py");
    assert_eq!(name_from_path(Path::new("/tmp/notes.pcdoc")), "notes.pcdoc");
}

#[test]
fn name_falls_back_to_untitled() {
    assert_eq!(name_from_path(Path::new("/")), "untitled");
}
