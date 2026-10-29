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

fn process_is_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn dropping_the_pane_kills_the_program() {
    let pane = AgentPane::spawn(&argv(&["sleep", "30"]), Path::new("."), 24, 80).unwrap();
    let pid = pane.pid().expect("a running process has an id");
    assert!(process_is_alive(pid));

    drop(pane);

    let deadline = Instant::now() + Duration::from_secs(5);
    while process_is_alive(pid) {
        assert!(Instant::now() < deadline, "process {pid} outlived the pane");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn reports_its_size() {
    let mut pane = AgentPane::spawn(&argv(&["cat"]), Path::new("."), 24, 80).unwrap();
    assert_eq!(pane.size(), (24, 80));

    pane.resize(30, 100);

    assert_eq!(pane.size(), (30, 100));
}

#[test]
fn a_program_killed_by_a_signal_reports_no_exit_code() {
    let mut pane = AgentPane::spawn(
        &argv(&["sh", "-c", "echo before; kill -9 $$"]),
        Path::new("."),
        24,
        80,
    )
    .unwrap();

    wait_until(&mut pane, |pane| {
        matches!(pane.state(), PaneState::Exited(_))
    });

    assert_eq!(pane.state(), PaneState::Exited(None));
    assert!(pane.screen_text().contains("before"));
}

#[test]
fn many_panes_can_be_opened_at_once() {
    let handles: Vec<_> = (0..16)
        .map(|_| {
            std::thread::spawn(|| {
                for _ in 0..5 {
                    let pane = AgentPane::spawn(&argv(&["true"]), Path::new("."), 24, 80)
                        .expect("opening a pane must not fail under concurrency");
                    drop(pane);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }
}

#[test]
fn counts_the_terminal_queries_a_program_sends() {
    // Gitview does not answer these yet; a program waiting on a reply may stall,
    // so they are recorded.
    let script = r"printf '\033[6n\033[6n\033[c\033[>0c\033[?u\033[?2026$p\033]11;?\007\033]10;?\033\\\033[>0q\033[18t'";
    let mut pane = AgentPane::spawn(&argv(&["sh", "-c", script]), Path::new("."), 24, 80).unwrap();

    wait_until(&mut pane, |pane| {
        matches!(pane.state(), PaneState::Exited(_))
    });

    let queries = pane.unanswered_queries();
    let count = |name: &str| {
        queries
            .iter()
            .find(|(query, _)| query == name)
            .map_or(0, |(_, count)| *count)
    };
    assert_eq!(count("cursor position (CSI 6 n)"), 2, "{queries:?}");
    assert_eq!(count("device attributes (CSI c)"), 1, "{queries:?}");
    assert_eq!(
        count("secondary device attributes (CSI > c)"),
        1,
        "{queries:?}"
    );
    assert_eq!(count("kitty keyboard flags (CSI ? u)"), 1, "{queries:?}");
    assert_eq!(count("mode status (CSI ? Pm $ p)"), 1, "{queries:?}");
    assert_eq!(count("background color (OSC 11)"), 1, "{queries:?}");
    assert_eq!(count("foreground color (OSC 10)"), 1, "{queries:?}");
    assert_eq!(count("terminal version (CSI > q)"), 1, "{queries:?}");
    assert_eq!(count("text area size (CSI 18 t)"), 1, "{queries:?}");
}

#[test]
fn finds_a_query_split_across_reads() {
    // The shell writes the sequence in two parts with a pause between them.
    let script = r"printf '\033[6'; sleep 0.2; printf 'n'; sleep 0.2";
    let mut pane = AgentPane::spawn(&argv(&["sh", "-c", script]), Path::new("."), 24, 80).unwrap();

    wait_until(&mut pane, |pane| {
        matches!(pane.state(), PaneState::Exited(_))
    });

    assert_eq!(
        pane.unanswered_queries(),
        vec![("cursor position (CSI 6 n)".to_owned(), 1)]
    );
}

#[test]
fn ordinary_output_is_not_mistaken_for_a_query() {
    let mut pane = AgentPane::spawn(
        &argv(&["sh", "-c", r"printf 'plain \033[31mred\033[0m 6n c'"]),
        Path::new("."),
        24,
        80,
    )
    .unwrap();

    wait_until(&mut pane, |pane| {
        matches!(pane.state(), PaneState::Exited(_))
    });

    assert!(pane.unanswered_queries().is_empty());
}

#[test]
fn a_query_is_counted_once_when_more_output_follows() {
    let script =
        r"printf '\033[6n'; sleep 0.2; printf 'more output'; sleep 0.2; printf ' and more'";
    let mut pane = AgentPane::spawn(&argv(&["sh", "-c", script]), Path::new("."), 24, 80).unwrap();

    wait_until(&mut pane, |pane| {
        matches!(pane.state(), PaneState::Exited(_))
    });

    assert_eq!(
        pane.unanswered_queries(),
        vec![("cursor position (CSI 6 n)".to_owned(), 1)]
    );
}
