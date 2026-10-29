//! The external editors: the commit composer's `Ctrl+E` and the profile page's
//! `e`. Git prefers the `GIT_EDITOR` environment variable over `core.editor`,
//! so the editors are chosen through the environment. That is set once, before
//! any other thread exists, to a stub whose behavior is read from files beside
//! it. Everything runs in one test so the environment is never changed while
//! worker threads may be reading it.

#[path = "support/replay.rs"]
mod replay;

use std::{fs, os::unix::fs::PermissionsExt, path::Path};

use gitview::config::GitConfigContext;
use replay::{Repo, Step::*, replay};

const STUB: &str = r#"#!/bin/sh
dir=$(dirname "$0")
case "$1" in
  *profiles.json) cp "$dir/registry.json" "$1" ;;
  *)
    [ "$(cat "$dir/mode")" = fail ] && exit 1
    cp "$dir/message" "$1"
    ;;
esac
"#;

/// A profile registry naming one profile whose folder and config file exist.
fn registry(base: &Path, name: &str) -> String {
    let folder = base.join(name);
    fs::create_dir_all(&folder).unwrap();
    let config = base.join(format!("{name}.gitconfig"));
    fs::write(&config, "").unwrap();
    format!(
        r#"{{"profiles":[{{"name":"{name}","worktree_root":"{}","config_path":"{}"}}]}}"#,
        folder.display(),
        config.display()
    )
}

fn commit_subject(repository: &Repo) -> String {
    repository
        .output(&["log", "-1", "--format=%s"])
        .trim()
        .to_owned()
}

fn set_stub(dir: &Path, mode: &str, message: &str) {
    fs::write(dir.join("mode"), mode).unwrap();
    fs::write(dir.join("message"), message).unwrap();
}

#[test]
fn external_editors_feed_the_composer_and_the_profile_registry() {
    let stub_dir = tempfile::tempdir().unwrap();
    let stub = stub_dir.path().join("editor.sh");
    fs::write(&stub, STUB).unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    set_stub(stub_dir.path(), "write", "Edited in editor\n");
    let profile_dir = tempfile::tempdir().unwrap();
    fs::write(
        stub_dir.path().join("registry.json"),
        registry(profile_dir.path(), "Work"),
    )
    .unwrap();
    // SAFETY: nothing else is running yet; this is the first statement that
    // touches the process environment and no threads have been started.
    unsafe {
        std::env::set_var("GIT_EDITOR", &stub);
        std::env::set_var("VISUAL", &stub);
    }
    // The profile page reloads the Git config context, which must exist.
    gitview::process::install_git_config_context(GitConfigContext::new(None)).unwrap();

    // A saved editor message is committed without a second Enter.
    let repository = Repo::with_changes();
    replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('c'),
            WaitFor("COMMIT MESSAGE · ACTIVE"),
            Text("draft"),
            Ctrl('e'),
            WaitFor("Edited in editor - Test"),
        ],
    );
    assert_eq!(commit_subject(&repository), "Edited in editor");

    // A failing editor keeps the typed message and reports the failure.
    set_stub(stub_dir.path(), "fail", "");
    let repository = Repo::with_changes();
    let screen = replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('c'),
            WaitFor("COMMIT MESSAGE · ACTIVE"),
            Text("draft"),
            Ctrl('e'),
            WaitFor("kept the original message"),
        ],
    );
    assert!(screen.contains("draft"), "{screen}");
    assert_eq!(commit_subject(&repository), "Change library");

    // An editor that saves an unchanged message does not commit by itself.
    set_stub(stub_dir.path(), "write", "draft");
    let repository = Repo::with_changes();
    let screen = replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('c'),
            WaitFor("COMMIT MESSAGE · ACTIVE"),
            Text("draft"),
            Ctrl('e'),
            WaitFor("Returned from editor; review the message before committing."),
        ],
    );
    assert!(screen.contains("draft"), "{screen}");
    assert_eq!(commit_subject(&repository), "Change library");

    // The profile registry edited externally is reloaded and listed.
    let repository = Repo::with_changes();
    let screen = replay(
        &repository.path(),
        [
            WaitFor("Change library"),
            Key(crossterm::event::KeyCode::Tab),
            Char('p'),
            WaitFor("No folder profiles configured."),
            Char('e'),
            WaitFor("Git config profiles reloaded."),
        ],
    );
    assert!(screen.contains("Work"), "{screen}");

    // `r` reloads a registry changed outside Gitview.
    fs::write(
        repository.path().join("profiles.json"),
        registry(profile_dir.path(), "Personal"),
    )
    .unwrap();
    let screen = replay(
        &repository.path(),
        [
            WaitFor("Change library"),
            Key(crossterm::event::KeyCode::Tab),
            Char('p'),
            WaitFor("No folder profiles configured."),
            Char('r'),
            WaitFor("Git config profiles reloaded."),
        ],
    );
    assert!(screen.contains("Personal"), "{screen}");
}
