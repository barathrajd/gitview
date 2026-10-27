use gitview::repository::parse_root;
use std::path::Path;

#[test]
fn preserves_whitespace_in_repository_paths() {
    assert_eq!(
        parse_root(b"/tmp/repo with space \n"),
        Path::new("/tmp/repo with space ")
    );
}

#[cfg(unix)]
#[test]
fn preserves_non_utf8_repository_paths() {
    use std::os::unix::ffi::OsStrExt;
    assert_eq!(
        parse_root(b"/tmp/repo-\xff\n").as_os_str().as_bytes(),
        b"/tmp/repo-\xff"
    );
}
