use gitview::{
    app::View,
    branches::{Branches, HeadState},
    ui::{Cockpit, Screen, render},
    worktree::{Entry, Section},
};
use ratatui::{Terminal, backend::TestBackend, widgets::ListState};
use std::{ffi::OsString, path::Path, time::Instant};

fn p95(mut samples: Vec<f64>) -> f64 {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    samples[(samples.len() * 95).div_ceil(100) - 1]
}

/// Draws the cockpit while moving the selection through large lists and returns
/// the p95 frame time in milliseconds over 100 actions.
fn measure(size: usize, refs: usize, width: u16, height: u16, focused: View) -> f64 {
    let branches = Ok(Branches {
        head: HeadState::Attached(OsString::from("main")),
        branches: (0..refs)
            .map(|index| OsString::from(format!("perf-{index:04}")))
            .collect(),
        head_commit: Some("abcdef0123456789".to_owned()),
    });
    let history = Ok((0..size)
        .map(|index| format!("* {index:07x} fixture commit number {index}"))
        .collect::<Vec<_>>());
    let status = Ok((0..size)
        .map(|index| Entry {
            section: if index % 2 == 0 {
                Section::Staged
            } else {
                Section::Unstaged
            },
            path: OsString::from(format!("dir-{}/file-{index:05}.txt", index % 100)),
            original: None,
        })
        .collect::<Vec<_>>());
    let mut history_selection = ListState::default();
    let mut working_tree_selection = ListState::default();
    history_selection.select(Some(0));
    working_tree_selection.select(Some(0));
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let mut draw = |history_selection: &mut ListState, working_tree_selection: &mut ListState| {
        let start = Instant::now();
        terminal
            .draw(|frame| {
                render(
                    frame,
                    Screen::Cockpit(Cockpit {
                        repository: Path::new("/work/gitview"),
                        profile: "Primary",
                        branches: Some(&branches),
                        history: Some(&history),
                        status: Some(&status),
                        feedback: "",
                        focused,
                        history_selection,
                        working_tree_selection,
                    }),
                    ratatui::symbols::border::PLAIN,
                );
            })
            .unwrap();
        start.elapsed().as_secs_f64() * 1000.0
    };
    draw(&mut history_selection, &mut working_tree_selection);
    let mut samples = Vec::new();
    for step in 0..100 {
        // Jump through the list so the viewport offset moves far from the start.
        let target = (step * size / 100).min(size.saturating_sub(1));
        history_selection.select(Some(target));
        working_tree_selection.select(Some(target));
        samples.push(draw(&mut history_selection, &mut working_tree_selection));
    }
    p95(samples)
}

#[test]
#[ignore = "isolated release-build timing diagnostic; run with --ignored --nocapture"]
fn measures_cockpit_frames_with_large_lists() {
    let mut worst = 0.0f64;
    for (size, refs) in [(100, 10), (1_000, 100), (10_000, 1_000)] {
        for (width, height) in [(140, 35), (80, 24)] {
            for focused in [View::History, View::WorkingTree] {
                let value = measure(size, refs, width, height, focused);
                println!(
                    "cockpit list_len={size} refs={refs} {width}x{height} focus={focused:?} p95_ms={value:.3}"
                );
                if size == 10_000 {
                    worst = worst.max(value);
                }
            }
        }
    }
    assert!(
        worst <= 50.0,
        "large-list frame p95 exceeded 50 ms: {worst}"
    );
}
