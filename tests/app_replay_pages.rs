//! Characterization tests for the branch, tag, object, comparison, and
//! profile pages. Mutating flows are checked against Git itself.

#[path = "support/replay.rs"]
mod replay;

use crossterm::event::KeyCode::{Enter, Esc, Tab};
use replay::{Repo, Step, Step::*, replay, replay_sized};

/// `with_changes` plus a `feature` branch and an annotated `v2.0` tag.
fn repository() -> Repo {
    let repository = Repo::with_changes();
    repository.git(&["branch", "feature"]);
    repository.git(&["tag", "-a", "v2.0", "-m", "Release two"]);
    repository
}

/// Opens the branch list. `feature` sorts above the current `main`, and the
/// first row starts selected.
fn branch_steps() -> Vec<Step> {
    vec![WaitFor("Change library"), Char('b'), WaitFor("feature")]
}

/// Opens the History pane, where the page-launching keys live.
fn history_steps() -> Vec<Step> {
    vec![WaitFor("Change library"), Key(Tab)]
}

fn current_branch(repository: &Repo) -> String {
    repository
        .output(&["branch", "--show-current"])
        .trim()
        .to_owned()
}

#[test]
fn branch_list_marks_the_current_branch() {
    let repository = repository();

    let screen = replay(&repository.path(), branch_steps());

    assert!(
        screen.contains("Local branches - current: main"),
        "{screen}"
    );
    assert!(screen.contains("* main"), "{screen}");
    let selected = screen.lines().find(|line| line.contains('›')).unwrap();
    assert!(selected.contains("feature"), "{screen}");
}

#[test]
fn entering_the_current_branch_reports_it_without_switching() {
    let repository = repository();
    let mut steps = branch_steps();
    steps.extend([Char('j'), Key(Enter), WaitFor("Already on this branch.")]);

    replay(&repository.path(), steps);

    assert_eq!(current_branch(&repository), "main");
}

#[test]
fn switching_a_branch_needs_confirmation() {
    let repository = repository();
    let mut steps = branch_steps();
    steps.extend([
        Key(Enter),
        WaitFor("Switch to local branch \"feature\"?"),
        Char('y'),
        WaitFor("current: feature"),
    ]);
    replay(&repository.path(), steps);
    assert_eq!(current_branch(&repository), "feature");

    let repository = self::repository();
    let mut steps = branch_steps();
    steps.extend([
        Key(Enter),
        WaitFor("CONFIRM BRANCH OPERATION"),
        Char('n'),
        WaitForGone("CONFIRM BRANCH OPERATION"),
    ]);
    replay(&repository.path(), steps);
    assert_eq!(current_branch(&repository), "main");
}

#[test]
fn creating_a_branch_asks_for_a_name_then_confirmation() {
    let repository = repository();
    let mut steps = branch_steps();
    steps.extend([
        Char('c'),
        WaitFor("New local branch name:"),
        Text("topic"),
        Key(Enter),
        WaitFor("Create and switch to local branch \"topic\""),
        Char('y'),
        WaitFor("current: topic"),
    ]);

    replay(&repository.path(), steps);

    assert_eq!(current_branch(&repository), "topic");
}

#[test]
fn deleting_a_branch_needs_confirmation_and_never_forces() {
    let repository = repository();
    let mut steps = branch_steps();
    steps.extend([
        Char('d'),
        WaitFor("Delete local branch \"feature\"?"),
        WaitFor("No force deletion is used."),
        Char('y'),
        WaitFor("Local branches - current: main"),
        WaitForGone("feature"),
    ]);

    replay(&repository.path(), steps);

    assert!(
        repository
            .output(&["branch", "--list", "feature"])
            .is_empty()
    );
}

#[test]
fn tag_page_lists_tags_by_kind_and_escape_returns_to_branches() {
    let repository = repository();
    let mut steps = branch_steps();
    steps.extend([
        Char('t'),
        WaitFor("LOCAL TAGS"),
        WaitFor("[annotated]"),
        Key(Esc),
        WaitFor("Local branches - current: main"),
    ]);

    replay(&repository.path(), steps);
}

#[test]
fn tag_page_shows_lightweight_and_annotated_tags() {
    let repository = repository();
    let mut steps = branch_steps();
    steps.extend([Char('t'), WaitFor("[annotated]")]);

    let screen = replay(&repository.path(), steps);

    assert!(screen.contains("v1.0  [lightweight]"), "{screen}");
    assert!(screen.contains("v2.0  [annotated]"), "{screen}");
}

#[test]
fn object_page_describes_the_selected_commit_and_escape_edits_the_revision() {
    let repository = repository();
    let mut steps = history_steps();
    steps.extend([
        Char('o'),
        WaitFor("INSPECT GIT OBJECT"),
        Key(Enter),
        WaitFor("OBJECT DESCRIPTION"),
    ]);
    let screen = replay(&repository.path(), steps);
    assert!(screen.contains("Type: commit"), "{screen}");
    assert!(screen.contains("Describe: v2.0"), "{screen}");

    let mut steps = history_steps();
    steps.extend([
        Char('o'),
        Key(Enter),
        WaitFor("OBJECT DESCRIPTION"),
        Key(Esc),
        WaitFor("INSPECT GIT OBJECT"),
    ]);
    replay(&repository.path(), steps);
}

#[test]
fn comparison_page_reviews_the_selected_commit_and_escape_edits_the_form() {
    let repository = repository();
    let mut steps = history_steps();
    steps.extend([
        Char('D'),
        WaitFor("COMPARE COMMIT SNAPSHOTS"),
        Key(Enter),
        WaitFor("COMPARISON REVIEW"),
        WaitFor("▾ src/lib.rs"),
    ]);
    let screen = replay(&repository.path(), steps);
    assert!(screen.contains("+2 -2"), "{screen}");
    assert!(screen.contains("1 first changed"), "{screen}");

    let mut steps = history_steps();
    steps.extend([
        Char('D'),
        Key(Enter),
        WaitFor("COMPARISON REVIEW"),
        Key(Esc),
        WaitFor("COMPARE COMMIT SNAPSHOTS"),
    ]);
    replay(&repository.path(), steps);
}

#[test]
fn profile_page_shows_the_account_and_escape_returns_to_history() {
    let repository = repository();
    let mut steps = history_steps();
    steps.extend([
        Char('p'),
        WaitFor("GIT CONFIG PROFILES"),
        WaitFor("Active account: Primary"),
        WaitFor("No folder profiles configured."),
        Key(Esc),
        WaitForGone("GIT CONFIG PROFILES"),
    ]);

    let screen = replay(&repository.path(), steps);

    assert!(screen.contains("Change library"), "{screen}");
}

/// `main` plus `topic-01` through `topic-30`, listed after `main`.
fn many_branches() -> Repo {
    let repository = Repo::with_history();
    for number in 1..=30 {
        repository.git(&["branch", &format!("topic-{number:02}")]);
    }
    repository
}

/// The branch the list marks as selected.
fn selected_branch(screen: &str) -> Option<&str> {
    let line = screen.lines().find(|line| line.contains('›'))?;
    ["main"]
        .into_iter()
        .chain((1..=30).map(|_| "topic-"))
        .find_map(|name| line.find(name).map(|start| &line[start..]))
        .map(|rest| rest.split_whitespace().next().unwrap_or(rest))
}

#[test]
fn branch_list_accepts_motions() {
    let repository = many_branches();
    let cases = [
        (vec![Char('G')], "topic-30"),
        (vec![Char('G'), Char('g'), Char('g')], "main"),
        (vec![Char('5'), Char('j')], "topic-05"),
        (vec![Char('G'), Char('2'), Char('k')], "topic-28"),
        // 24 rows leave 20 for branches, so half a page is 10.
        (vec![Ctrl('d')], "topic-10"),
    ];
    for (keys, branch) in cases {
        let mut steps = vec![WaitFor("Add library"), Char('b'), WaitFor("topic-01")];
        steps.extend(keys);
        let screen = replay_sized(&repository.path(), 140, 24, steps);
        assert_eq!(selected_branch(&screen), Some(branch), "{screen}");
    }
}

#[test]
fn control_letters_do_not_run_branch_actions() {
    let repository = repository();
    let mut steps = branch_steps();
    steps.extend([Ctrl('d'), Ctrl('c'), Ctrl('r'), Char('?'), WaitFor("HELP")]);

    let screen = replay(&repository.path(), steps);

    assert!(!screen.contains("Delete local branch"), "{screen}");
    assert!(!screen.contains("New local branch name:"), "{screen}");
}

#[test]
fn control_y_does_not_confirm_a_branch_switch() {
    let repository = repository();
    let mut steps = branch_steps();
    steps.extend([
        Key(Enter),
        WaitFor("Switch to local branch \"feature\"?"),
        Ctrl('y'),
        Char('?'),
        WaitFor("HELP"),
    ]);
    replay(&repository.path(), steps);
    std::thread::sleep(std::time::Duration::from_millis(500));

    assert_eq!(current_branch(&repository), "main");
}

#[test]
fn comparison_review_motions_scroll_to_the_end_and_back() {
    let repository = Repo::with_history();
    let body: String = (1..=120)
        .map(|line| format!("long line {line:03}\n"))
        .collect();
    repository.write("long.txt", &body);
    repository.commit_all("Add a long file");

    replay(
        &repository.path(),
        [
            WaitFor("Add a long file"),
            Key(Tab),
            Char('D'),
            WaitFor("COMPARE COMMIT SNAPSHOTS"),
            Key(Enter),
            WaitFor("▾ long.txt"),
            Char('G'),
            WaitFor("long line 120"),
            WaitForGone("▾ long.txt"),
            Text("gg"),
            WaitFor("▾ long.txt"),
            Ctrl('d'),
            WaitForGone("▾ long.txt"),
            Ctrl('u'),
            WaitFor("▾ long.txt"),
            Text("50j"),
            WaitFor("long line 051"),
            WaitForGone("long line 049"),
        ],
    );
}

#[test]
fn object_preview_motions_scroll_to_the_end_and_back() {
    let repository = Repo::with_history();
    let body: String = (1..=120)
        .map(|line| format!("long line {line:03}\n"))
        .collect();
    repository.write("long.txt", &body);
    repository.commit_all("Add a long file");

    replay(
        &repository.path(),
        [
            WaitFor("Add a long file"),
            Key(Tab),
            Char('o'),
            WaitFor("INSPECT GIT OBJECT"),
            // The revision field keeps g and digits as text.
            Text(":long.txt"),
            WaitFor(":long.txt_"),
            Key(Enter),
            WaitFor("Type: blob"),
            Char('G'),
            WaitFor("long line 120"),
            WaitForGone("Type: blob"),
            Text("gg"),
            WaitFor("Type: blob"),
            Ctrl('d'),
            WaitForGone("Type: blob"),
            Ctrl('u'),
            WaitFor("Type: blob"),
        ],
    );
}

#[test]
fn tag_list_motions_jump_to_the_last_and_first_tag() {
    let repository = Repo::with_history();
    for number in 1..=40 {
        repository.git(&["tag", &format!("t-{number:02}")]);
    }

    let screen = replay(
        &repository.path(),
        [
            WaitFor("Change library"),
            Char('b'),
            WaitFor("Local branches"),
            Char('t'),
            WaitFor("› t-01"),
            Char('G'),
            WaitFor("› t-40"),
            Text("gg"),
            WaitFor("› t-01"),
            Text("5j"),
            WaitFor("› t-06"),
            Ctrl('d'),
            WaitForGone("› t-06"),
            Char('c'),
            Text("gG5"),
            WaitFor("gG5"),
        ],
    );

    assert!(!screen.contains("Delete local tag"), "{screen}");
}
