use crossterm::event::KeyCode;
use gitview::{
    comparison_page::ComparisonPage,
    ui::{Screen, render},
};
use ratatui::{Terminal, backend::TestBackend};
use std::{
    process::Command,
    time::{Duration, Instant},
};
#[test]
fn switches_comparison_styles_and_reuses_the_returned_view() {
    let root = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(root.path())
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Test"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "commit.gpgsign", "false"]);
    for (number, message) in [(1, "base"), (2, "target")] {
        let source=(0..400).map(|index|format!("pub fn value_{index}() -> usize {{ let number = {number}; number + {index} }}\n")).collect::<String>();
        std::fs::write(root.path().join("sample.rs"), source).unwrap();
        git(&["add", "--all"]);
        git(&["commit", "-qm", message]);
    }
    let mut page = ComparisonPage::open("HEAD", None);
    let mut terminal = Terminal::new(TestBackend::new(140, 35)).unwrap();
    page.key(root.path(), KeyCode::Enter);
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        page.tick();
        terminal
            .draw(|frame| render(frame, Screen::Comparison(&page), gitview::ui::ASCII_BORDER))
            .unwrap();
        let text = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        if text.contains("COMPARISON REVIEW") {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut elapsed = Vec::new();
    for key in [
        KeyCode::Char('v'),
        KeyCode::Char('v'),
        KeyCode::Tab,
        KeyCode::Tab,
        KeyCode::Char('v'),
        KeyCode::Char('v'),
    ] {
        page.key(root.path(), key);
        let start = Instant::now();
        terminal
            .draw(|frame| render(frame, Screen::Comparison(&page), gitview::ui::ASCII_BORDER))
            .unwrap();
        elapsed.push(start.elapsed());
    }
    println!("Style/layout switch render times: {elapsed:?}");
    let output = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("sample.rs"));
    assert!(output.contains("IDE"));
}
