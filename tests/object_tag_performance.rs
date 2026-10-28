use crossterm::event::KeyCode;
use gitview::{
    object_page::ObjectPage,
    tag_page::TagPage,
    ui::{ASCII_BORDER, Screen, render},
};
use ratatui::{Terminal, backend::TestBackend};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn git(root: &Path, args: &[&str], input: &[u8]) {
    let mut child = Command::new("git")
        .current_dir(root)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    assert!(child.wait().unwrap().success());
}

fn repo() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.name", "Test"],
        &["config", "user.email", "test@example.com"],
        &["config", "commit.gpgsign", "false"],
    ] {
        git(root.path(), args, b"");
    }
    root
}

fn p95(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    samples[(samples.len() * 95).div_ceil(100) - 1]
}

fn draw(terminal: &mut Terminal<TestBackend>, screen: Screen<'_>) -> f64 {
    let start = Instant::now();
    terminal
        .draw(|frame| render(frame, screen, ASCII_BORDER))
        .unwrap();
    start.elapsed().as_secs_f64() * 1000.0
}

#[test]
#[ignore = "isolated release-build timing diagnostic; run with --ignored --nocapture"]
fn measures_object_preview_frames_at_the_output_limit() {
    let root = repo();
    // About 940 KB and 4,900 lines, under the 1 MB / 5,000 line preview limit.
    let line = "x".repeat(190);
    std::fs::write(
        root.path().join("big.txt"),
        format!("{line}\n").repeat(4900),
    )
    .unwrap();
    git(root.path(), &["add", "--all"], b"");
    git(root.path(), &["commit", "-qm", "base"], b"");
    let mut page = ObjectPage::open("HEAD:big.txt");
    let load = Instant::now();
    page.key(root.path(), KeyCode::Enter);
    while !page.tick() {
        assert!(load.elapsed() < Duration::from_secs(20));
        std::thread::sleep(Duration::from_millis(2));
    }
    println!(
        "object load_ms={:.1}",
        load.elapsed().as_secs_f64() * 1000.0
    );
    let mut worst = 0.0f64;
    for (width, height) in [(140, 35), (80, 24)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let first = draw(&mut terminal, Screen::Object(&page));
        let mut samples = Vec::new();
        for _ in 0..100 {
            page.key(root.path(), KeyCode::Char('j'));
            samples.push(draw(&mut terminal, Screen::Object(&page)));
        }
        for _ in 0..100 {
            page.key(root.path(), KeyCode::Char('k'));
        }
        let value = p95(samples);
        println!("object {width}x{height} first_ms={first:.3} scroll_p95_ms={value:.3}");
        worst = worst.max(value);
    }
    assert!(worst <= 50.0, "object frame p95 exceeded 50 ms: {worst}");
}

#[test]
#[ignore = "isolated release-build timing diagnostic; run with --ignored --nocapture"]
fn measures_tag_page_with_many_tags() {
    let root = repo();
    git(
        root.path(),
        &["commit", "--allow-empty", "-qm", "base"],
        b"",
    );
    let refs: String = (0..10_000)
        .map(|index| format!("create refs/tags/perf-{index:05} HEAD\n"))
        .collect();
    git(root.path(), &["update-ref", "--stdin"], refs.as_bytes());
    let mut page = TagPage::open(root.path());
    let load = Instant::now();
    while !page.tick() {
        assert!(load.elapsed() < Duration::from_secs(20));
        std::thread::sleep(Duration::from_millis(2));
    }
    println!("tags load_ms={:.1}", load.elapsed().as_secs_f64() * 1000.0);
    let mut worst = 0.0f64;
    for (width, height) in [(140, 35), (80, 24)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let first = draw(&mut terminal, Screen::Tags(&page));
        let mut samples = Vec::new();
        for _ in 0..100 {
            for _ in 0..97 {
                page.key(root.path(), KeyCode::Char('j'));
            }
            samples.push(draw(&mut terminal, Screen::Tags(&page)));
        }
        let value = p95(samples);
        println!("tags {width}x{height} first_ms={first:.3} select_p95_ms={value:.3}");
        worst = worst.max(value);
    }
    assert!(worst <= 50.0, "tag frame p95 exceeded 50 ms: {worst}");
}
