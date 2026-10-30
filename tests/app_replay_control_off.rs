//! When the control socket can't start, the agent still runs and the pane
//! says Gitview's tools are off. Its own file: it changes the environment.

#[path = "support/replay.rs"]
mod replay;

use std::os::unix::fs::PermissionsExt;

use replay::{Repo, Step::*, replay_with_agent};

#[test]
fn the_pane_says_when_gitviews_tools_are_off() {
    let repository = Repo::with_history();
    let shared =
        std::env::temp_dir().join(format!("gitview-shared-runtime-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&shared);
    std::fs::create_dir(&shared).unwrap();
    std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o777)).unwrap();
    // A runtime folder others can write to is refused, so no socket starts.
    // SAFETY: this test file has one test, so no other thread reads the
    // environment while it changes.
    unsafe { std::env::set_var("XDG_RUNTIME_DIR", shared.join("gitview").parent().unwrap()) };
    std::fs::create_dir(shared.join("gitview")).unwrap();
    std::fs::set_permissions(
        shared.join("gitview"),
        std::fs::Permissions::from_mode(0o777),
    )
    .unwrap();

    let screen = replay_with_agent(
        &repository.path(),
        Some(&["sh", "-c", "echo socket=${GITVIEW_SOCKET:-none}; sleep 30"]),
        160,
        30,
        [WaitFor("socket=none"), WaitFor("tools off")],
    );

    assert!(
        screen.contains("AGENT · sh · running · tools off"),
        "{screen}"
    );
}
