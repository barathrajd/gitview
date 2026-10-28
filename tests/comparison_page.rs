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
fn draw(page: &ComparisonPage) -> String {
    let mut terminal = Terminal::new(TestBackend::new(140, 45)).unwrap();
    terminal
        .draw(|frame| render(frame, Screen::Comparison(page), gitview::ui::ASCII_BORDER))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}
#[test]
fn reviews_and_folds_files_in_one_comparison_screen() {
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
    for (file, text) in [
        ("first.txt", "first before unique\n"),
        ("second.txt", "second before unique\n"),
    ] {
        std::fs::write(root.path().join(file), text).unwrap();
    }
    git(&["add", "--all"]);
    git(&["commit", "-qm", "base"]);
    for (file, text) in [
        ("first.txt", "first after unique\n"),
        ("second.txt", "second after unique\n"),
    ] {
        std::fs::write(root.path().join(file), text).unwrap();
    }
    git(&["add", "--all"]);
    git(&["commit", "-qm", "target"]);
    let mut page = ComparisonPage::open("HEAD", None);
    page.key(root.path(), KeyCode::Enter);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        page.tick();
        if !draw(&page).contains("Loading comparison...") {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = draw(&page);
    assert!(output.contains("COMPARISON REVIEW"));
    assert!(output.contains("first before unique"));
    assert!(output.contains("second after unique"));
    assert!(output.contains("+1"));
    assert!(output.contains("-1"));
    for (width, height) in [(1, 8), (20, 12), (60, 20)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render(frame, Screen::Comparison(&page), gitview::ui::ASCII_BORDER))
            .unwrap();
    }
    page.key(root.path(), KeyCode::Enter);
    let output = draw(&page);
    assert!(output.contains("first.txt"));
    assert!(!output.contains("first before unique"));
    assert!(output.contains("second after unique"));
    page.key(root.path(), KeyCode::Char(']'));
    page.key(root.path(), KeyCode::Char(' '));
    assert!(!draw(&page).contains("second after unique"));
    page.key(root.path(), KeyCode::Char('E'));
    assert!(draw(&page).contains("first before unique"));
    page.key(root.path(), KeyCode::Tab);
    page.key(root.path(), KeyCode::Char('v'));
    assert!(draw(&page).contains("first after unique"));
    page.key(root.path(), KeyCode::Char('C'));
    let output = draw(&page);
    assert!(output.contains("second.txt"));
    assert!(!output.contains("first after unique"));
}
