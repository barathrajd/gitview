//! Characterization tests for the mutating flows: discard, ignore rules,
//! untrack, remove, move, amend, tags, and branch rename. Each is checked
//! against Git itself. These operations finish on background threads, so the
//! tests poll Git for the result.

#[path = "support/replay.rs"]
mod replay;

use std::time::{Duration, Instant};

use crossterm::event::KeyCode::{Enter, Esc, Tab};
use replay::{Repo, Step, Step::*, replay};

/// `with_changes` plus a `feature` branch and an annotated `v2.0` tag.
fn repository() -> Repo {
    let repository = Repo::with_changes();
    repository.git(&["branch", "feature"]);
    repository.git(&["tag", "-a", "v2.0", "-m", "Release two"]);
    repository
}

/// Waits until `check` accepts the output of `git args`, or fails after 10 s.
fn wait_for_git(repository: &Repo, args: &[&str], check: impl Fn(&str) -> bool) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let output = repository.output(args);
        if check(&output) {
            return output;
        }
        assert!(
            Instant::now() < deadline,
            "git {args:?} never matched; last output:\n{output}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn branch_steps() -> Vec<Step> {
    vec![WaitFor("Change library"), Char('b'), WaitFor("feature")]
}

fn tag_steps() -> Vec<Step> {
    let mut steps = branch_steps();
    steps.extend([Char('t'), WaitFor("[annotated]")]);
    steps
}

#[test]
fn discard_restores_a_changed_file_after_confirmation() {
    let repository = repository();

    replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('j'),
            Char('d'),
            WaitFor("Restore this file to its staged version?"),
            Char('y'),
            WaitFor("Selected file discarded."),
        ],
    );

    assert_eq!(
        std::fs::read_to_string(repository.path().join("README.md")).unwrap(),
        "# Demo\n"
    );
}

#[test]
fn declining_a_discard_keeps_the_changes() {
    let repository = repository();

    replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('j'),
            Char('d'),
            WaitFor("CONFIRM DISCARD"),
            Char('n'),
            WaitForGone("CONFIRM DISCARD"),
        ],
    );

    assert_eq!(
        std::fs::read_to_string(repository.path().join("README.md")).unwrap(),
        "# Demo\nmore\n"
    );
}

#[test]
fn discarding_an_untracked_file_deletes_it_after_confirmation() {
    let repository = repository();

    replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('j'),
            Char('j'),
            Char('d'),
            WaitFor("Permanently delete this untracked file?"),
            Char('y'),
            WaitFor("Selected file discarded."),
        ],
    );

    assert!(!repository.path().join("notes.txt").exists());
}

#[test]
fn ignore_rules_are_inspected_for_the_selected_path_and_for_a_typed_path() {
    let repository = repository();
    repository.write(".gitignore", "*.log\n");
    repository.write("debug.log", "noise\n");

    let screen = replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('j'),
            Char('i'),
            WaitFor("No ignore rule matches the selected path."),
            Char('I'),
            WaitFor("INSPECT IGNORE RULE"),
            Text("debug.log"),
            Key(Enter),
            WaitFor("Ignored by .gitignore:1"),
        ],
    );

    assert!(screen.contains("\"*.log\""), "{screen}");
}

#[test]
fn stop_tracking_keeps_the_file_and_stages_the_deletion() {
    let repository = repository();

    replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('j'),
            Char('x'),
            WaitFor("Stop tracking this path and keep the working copy?"),
            Char('y'),
            WaitFor("Stopped tracking the selected file"),
        ],
    );

    assert!(repository.path().join("README.md").exists());
    let tracked = repository.output(&["ls-files"]);
    assert!(
        !tracked.lines().any(|line| line == "README.md"),
        "{tracked}"
    );
}

#[test]
fn remove_deletes_a_tracked_path_and_stages_the_deletion() {
    let repository = repository();

    replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('R'),
            WaitFor("REMOVE TRACKED PATH"),
            Text("src/lib.rs"),
            Key(Enter),
            WaitFor("Remove this tracked path and stage its deletion?"),
            Char('y'),
            WaitForGone("CONFIRM TRACKED PATH REMOVAL"),
        ],
    );

    wait_for_git(&repository, &["status", "--porcelain"], |status| {
        status.contains("D  src/lib.rs")
    });
    assert!(!repository.path().join("src/lib.rs").exists());
}

#[test]
fn move_renames_a_clean_tracked_path_and_stages_the_rename() {
    let repository = repository();

    replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('M'),
            WaitFor("MOVE TRACKED PATH"),
            Text("src/lib.rs"),
            Key(Tab),
            Text("src/core.rs"),
            Key(Enter),
            WaitFor("CONFIRM"),
            Char('y'),
            WaitForGone("CONFIRM"),
        ],
    );

    wait_for_git(&repository, &["status", "--porcelain"], |status| {
        status.contains("R  src/lib.rs -> src/core.rs")
    });
}

#[test]
fn amend_rewrites_head_with_the_new_message_after_confirmation() {
    let repository = repository();
    let before = repository.output(&["rev-parse", "HEAD"]);

    replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('a'),
            WaitFor("AMEND MESSAGE · ACTIVE"),
            Text(" again"),
            Key(Enter),
            WaitFor("This rewrites HEAD and changes its commit ID."),
            Char('y'),
            WaitFor("Change library again - Test"),
        ],
    );

    assert_eq!(
        repository.output(&["log", "-1", "--format=%s"]).trim(),
        "Change library again"
    );
    assert_ne!(repository.output(&["rev-parse", "HEAD"]), before);
    // The staged file went into the amended commit.
    assert!(
        repository
            .output(&["show", "--stat", "--format=", "HEAD"])
            .contains("staged.txt")
    );
}

#[test]
fn declining_an_amend_leaves_head_alone() {
    let repository = repository();
    let before = repository.output(&["rev-parse", "HEAD"]);

    replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('a'),
            WaitFor("AMEND MESSAGE · ACTIVE"),
            Text(" again"),
            Key(Enter),
            WaitFor("CONFIRM AMEND"),
            Char('n'),
            WaitForGone("CONFIRM AMEND"),
        ],
    );

    assert_eq!(repository.output(&["rev-parse", "HEAD"]), before);
}

#[test]
fn creating_a_tag_asks_for_confirmation_and_never_overwrites() {
    let repository = repository();
    let mut steps = tag_steps();
    steps.extend([
        Char('c'),
        WaitFor("CREATE LIGHTWEIGHT TAG"),
        Text("v3.0"),
        Key(Enter),
        WaitFor("Create lightweight tag v3.0 at commit"),
        WaitFor("Existing tags are never overwritten."),
        Char('y'),
        WaitFor("Lightweight tag created."),
    ]);

    replay(&repository.path(), steps);

    assert!(
        repository
            .output(&["tag", "--list", "v3.0"])
            .contains("v3.0")
    );
}

#[test]
fn verifying_a_tag_without_a_signature_reports_that_it_cannot_be_verified() {
    let repository = repository();
    let mut steps = tag_steps();
    steps.extend([
        Char('v'),
        WaitFor("TAG SIGNATURE VERIFICATION"),
        WaitFor("no verifiable signature"),
    ]);

    replay(&repository.path(), steps);
}

#[test]
fn deleting_a_tag_asks_for_confirmation_and_removes_only_that_tag() {
    let repository = repository();
    let mut steps = tag_steps();
    steps.extend([
        Char('j'),
        Char('d'),
        WaitFor("Delete local tag v2.0?"),
        WaitFor("Remote tags are unaffected."),
        Char('y'),
        WaitForGone("Delete local tag"),
    ]);

    replay(&repository.path(), steps);

    wait_for_git(&repository, &["tag", "--list"], |tags| {
        !tags.contains("v2.0")
    });
    assert!(repository.output(&["tag", "--list"]).contains("v1.0"));
}

#[test]
fn renaming_a_branch_asks_for_a_name_then_confirmation() {
    let repository = repository();
    let mut steps = branch_steps();
    steps.extend([
        Char('r'),
        WaitFor("RENAME LOCAL BRANCH"),
        Text("renamed"),
        Key(Enter),
        WaitFor("Rename local branch \"feature\" to \"renamed\"?"),
        Char('y'),
        WaitForGone("CONFIRM BRANCH OPERATION"),
    ]);

    replay(&repository.path(), steps);

    wait_for_git(&repository, &["branch", "--list"], |branches| {
        branches.contains("renamed") && !branches.contains("feature")
    });
}

#[test]
fn dash_returns_to_the_previous_checkout_after_confirmation() {
    let repository = repository();
    let mut steps = branch_steps();
    steps.extend([
        Key(Enter),
        WaitFor("Switch to local branch \"feature\"?"),
        Char('y'),
        WaitFor("current: feature"),
        Char('-'),
        WaitFor("Return to local branch main"),
        Char('y'),
        WaitFor("Returned to previous checkout."),
    ]);

    replay(&repository.path(), steps);

    assert_eq!(
        repository.output(&["branch", "--show-current"]).trim(),
        "main"
    );
}

#[test]
fn escape_closes_a_confirmation_without_changing_anything() {
    let repository = repository();

    replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('R'),
            WaitFor("REMOVE TRACKED PATH"),
            Text("src/lib.rs"),
            Key(Enter),
            WaitFor("Remove this tracked path"),
            Key(Esc),
            WaitForGone("CONFIRM TRACKED PATH REMOVAL"),
        ],
    );

    assert!(repository.path().join("src/lib.rs").exists());
}

#[test]
fn control_y_does_not_confirm_a_discard() {
    let repository = repository();

    replay(
        &repository.path(),
        [
            WaitFor("STAGED 1"),
            Char('j'),
            Char('d'),
            WaitFor("CONFIRM DISCARD"),
            Ctrl('y'),
            Ctrl('n'),
            // Still asking: neither modified letter answered the prompt.
            Char('?'),
            WaitFor("HELP"),
        ],
    );
    // A misread Ctrl-y discards on a worker thread; give it time to land.
    std::thread::sleep(std::time::Duration::from_millis(500));

    assert_eq!(
        std::fs::read_to_string(repository.path().join("README.md")).unwrap(),
        "# Demo\nmore\n"
    );
}
