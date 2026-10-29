use std::path::Path;
use std::time::{Duration, Instant};

use gitview::pane::{AgentPane, PaneState, SCROLLBACK_LIMIT};

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

fn wait_until(pane: &mut AgentPane, mut done: impl FnMut(&AgentPane) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        pane.poll();
        if done(pane) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out; screen was:\n{}",
            pane.screen_text()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn shows_command_output_and_reports_exit_code() {
    let mut pane = AgentPane::spawn(
        &argv(&["sh", "-c", "echo hello; exit 3"]),
        Path::new("."),
        24,
        80,
    )
    .unwrap();

    wait_until(&mut pane, |pane| {
        matches!(pane.state(), PaneState::Exited(_))
    });

    assert!(pane.screen_text().contains("hello"));
    assert_eq!(pane.state(), PaneState::Exited(Some(3)));
}

#[test]
fn forwards_input_bytes_to_the_command() {
    let mut pane = AgentPane::spawn(&argv(&["cat"]), Path::new("."), 24, 80).unwrap();

    pane.send(b"typed text\n").unwrap();

    wait_until(&mut pane, |pane| pane.screen_text().contains("typed text"));
    assert_eq!(pane.state(), PaneState::Running);
}

#[test]
fn resize_reaches_the_command() {
    let mut pane = AgentPane::spawn(
        &argv(&[
            "sh",
            "-c",
            "trap 'stty size' WINCH; stty size; while :; do sleep 0.05; done",
        ]),
        Path::new("."),
        24,
        80,
    )
    .unwrap();
    wait_until(&mut pane, |pane| pane.screen_text().contains("24 80"));

    pane.resize(30, 100);

    wait_until(&mut pane, |pane| pane.screen_text().contains("30 100"));
}

#[test]
fn scrollback_is_capped() {
    let mut pane = AgentPane::spawn(
        &argv(&[
            "sh",
            "-c",
            "i=0; while [ $i -lt 5000 ]; do echo line$i; i=$((i+1)); done",
        ]),
        Path::new("."),
        24,
        80,
    )
    .unwrap();

    wait_until(&mut pane, |pane| {
        matches!(pane.state(), PaneState::Exited(_))
    });

    assert_eq!(pane.scrollback_len(), SCROLLBACK_LIMIT);
}

#[test]
fn missing_executable_is_an_actionable_error() {
    let error = AgentPane::spawn(&argv(&["gitview-no-such-agent"]), Path::new("."), 24, 80)
        .err()
        .expect("spawn must fail");

    assert!(
        error.to_string().contains("gitview-no-such-agent"),
        "{error}"
    );
}

#[test]
fn starts_the_command_in_the_given_directory() {
    let dir = tempfile::tempdir().unwrap();
    let mut pane = AgentPane::spawn(&argv(&["pwd"]), dir.path(), 24, 200).unwrap();

    wait_until(&mut pane, |pane| {
        matches!(pane.state(), PaneState::Exited(_))
    });

    let name = dir.path().file_name().unwrap().to_str().unwrap();
    assert!(pane.screen_text().contains(name), "{}", pane.screen_text());
}

#[test]
fn records_sequences_the_emulator_does_not_understand() {
    let mut pane = AgentPane::spawn(
        &argv(&[
            "sh",
            "-c",
            r"printf '\033[99z\033[99z\033]7777;payload\007\033(Zdone'",
        ]),
        Path::new("."),
        24,
        80,
    )
    .unwrap();

    wait_until(&mut pane, |pane| {
        matches!(pane.state(), PaneState::Exited(_))
    });

    let unhandled = pane.unhandled_sequences();
    assert!(
        unhandled.contains(&("CSI 99z".to_owned(), 2)),
        "{unhandled:?}"
    );
    assert!(
        unhandled.contains(&("OSC 7777".to_owned(), 1)),
        "{unhandled:?}"
    );
    assert!(pane.screen_text().contains("done"));
}

#[test]
fn remembers_the_window_title_the_program_sets() {
    let mut pane = AgentPane::spawn(
        &argv(&["sh", "-c", r"printf '\033]2;Fix the parser\007ready'"]),
        Path::new("."),
        24,
        80,
    )
    .unwrap();

    wait_until(&mut pane, |pane| pane.screen_text().contains("ready"));

    assert_eq!(pane.title(), Some("Fix the parser"));
}

#[test]
fn generation_changes_only_when_output_arrives() {
    let mut pane = AgentPane::spawn(&argv(&["cat"]), Path::new("."), 24, 80).unwrap();
    pane.poll();
    let idle = pane.generation();
    std::thread::sleep(Duration::from_millis(50));
    pane.poll();
    assert_eq!(pane.generation(), idle, "no output yet");

    pane.send(b"hi\n").unwrap();
    wait_until(&mut pane, |pane| pane.screen_text().contains("hi"));

    assert!(pane.generation() > idle);
}
