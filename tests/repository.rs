use gitview::repository::parse_root;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_REPO_ID: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let id = NEXT_REPO_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("gitview-{name}-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

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

#[test]
fn initializes_a_repository_at_a_path_with_spaces() {
    let temp = TempDir::new("init");
    let destination = temp.0.join("new repository");

    gitview::repository::initialize(&destination).unwrap();

    assert!(destination.join(".git").exists());
    let root = git(&destination, &["rev-parse", "--show-toplevel"]);
    assert_eq!(parse_root(&root), destination.canonicalize().unwrap());
}

#[test]
fn refuses_to_initialize_inside_an_existing_repository() {
    let temp = TempDir::new("nested-init");
    git(&temp.0, &["init", "-q"]);
    let nested = temp.0.join("nested");
    fs::create_dir(&nested).unwrap();

    let error = gitview::repository::initialize(&nested).unwrap_err();

    assert!(error.contains("already inside a Git repository"));
    assert!(!nested.join(".git").exists());
}

#[test]
fn clones_into_a_destination_path_with_spaces() {
    let temp = TempDir::new("clone");
    let source = temp.0.join("source repository");
    fs::create_dir(&source).unwrap();
    git(&source, &["init", "-q"]);
    git(&source, &["config", "user.name", "Gitview Test"]);
    git(&source, &["config", "user.email", "gitview@example.com"]);
    fs::write(source.join("README.md"), "cloned content\n").unwrap();
    git(&source, &["add", "README.md"]);
    git(&source, &["commit", "-qm", "initial"]);
    let destination = temp.0.join("cloned repository");

    gitview::repository::clone_repository(&source, Some(&destination)).unwrap();

    assert_eq!(
        fs::read(destination.join("README.md")).unwrap(),
        b"cloned content\n"
    );
    assert!(destination.join(".git").exists());
}

#[test]
fn refuses_to_clone_into_a_non_empty_destination() {
    let temp = TempDir::new("clone-existing");
    let source = temp.0.join("source");
    fs::create_dir(&source).unwrap();
    git(&source, &["init", "-q"]);
    let destination = temp.0.join("existing");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("keep.txt"), "keep\n").unwrap();

    let error = gitview::repository::clone_repository(&source, Some(&destination)).unwrap_err();

    assert!(error.contains("not empty"));
    assert_eq!(fs::read(destination.join("keep.txt")).unwrap(), b"keep\n");
}
