use gitview::{
    ui::{self, Screen},
    worktree::{Entry, Section},
};
use ratatui::{Terminal, backend::TestBackend};
use std::time::Instant;
fn patch() -> String {
    let mut text = String::from(
        "diff --git a/sample.rs b/sample.rs\n--- a/sample.rs\n+++ b/sample.rs\n@@ -1,400 +1,400 @@\n",
    );
    for (marker, number) in [('-', 1), ('+', 2)] {
        for index in 0..400 {
            text.push_str(&format!("{marker}pub fn value_{index}() -> usize {{ let number = {number}; number + {index} }}\n"));
        }
    }
    text
}
#[test]
fn measures_ordinary_diff_first_render_and_warm_actions() {
    let diff = (987654, Some(Ok(patch())));
    let entry = Entry {
        section: Section::Unstaged,
        path: "sample.rs".into(),
        original: None,
    };
    let mut terminal = Terminal::new(TestBackend::new(140, 35)).unwrap();
    let mut times = Vec::new();
    for (raw, split, scroll) in [
        (false, true, 0),
        (true, true, 0),
        (false, true, 0),
        (false, false, 0),
        (false, false, 1),
        (false, true, 2),
    ] {
        let start = Instant::now();
        terminal
            .draw(|frame| {
                ui::render(
                    frame,
                    Screen::Diff {
                        diff: &diff,
                        scroll,
                        selected_hunk: 0,
                        raw,
                        side_by_side: split,
                        worktree_entry: Some(&entry),
                    },
                    ui::ASCII_BORDER,
                )
            })
            .unwrap();
        times.push(start.elapsed());
    }
    println!("Ordinary diff first-render / raw / IDE / unified / scroll / split: {times:?}");
}

#[test]
fn prepares_off_thread_and_renders_all_variants_without_stale_rows() {
    let text = patch();
    let preparation_start = Instant::now();
    let prepared = std::thread::spawn(move || {
        ui::PreparedDiff::new(text, Some(std::path::Path::new("sample.rs")))
    })
    .join()
    .unwrap();
    println!(
        "Background preparation time: {:?}",
        preparation_start.elapsed()
    );
    let diff = (888, Some(Ok(prepared.text().to_owned())));
    let entry = Entry {
        section: Section::Unstaged,
        path: "sample.rs".into(),
        original: None,
    };
    let mut terminal = Terminal::new(TestBackend::new(140, 35)).unwrap();
    let mut times = Vec::new();
    for width in [140, 80, 140] {
        terminal.backend_mut().resize(width, 35);
        terminal
            .resize(ratatui::layout::Rect::new(0, 0, width, 35))
            .unwrap();
        for (raw, split, scroll) in [
            (false, true, 0),
            (true, true, 0),
            (false, true, 0),
            (false, false, 0),
            (false, false, 1),
            (false, true, 2),
        ] {
            let start = Instant::now();
            terminal
                .draw(|frame| {
                    ui::render(
                        frame,
                        Screen::PreparedDiff {
                            diff: &diff,
                            prepared: &prepared,
                            scroll,
                            selected_hunk: 0,
                            raw,
                            side_by_side: split,
                            worktree_entry: Some(&entry),
                        },
                        ui::ASCII_BORDER,
                    )
                })
                .unwrap();
            times.push(start.elapsed());
            assert_eq!(
                prepared.hunk_positions(width, raw, split),
                ui::diff_hunk_line_positions(prepared.text(), width, raw, split)
            );
        }
    }
    println!("Prepared ordinary diff view-switch / scroll timings: {times:?}");
    for (width, height) in [(140, 35), (80, 24)] {
        terminal.backend_mut().resize(width, height);
        terminal
            .resize(ratatui::layout::Rect::new(0, 0, width, height))
            .unwrap();
        let draw = |terminal: &mut Terminal<TestBackend>, raw, split, scroll| {
            terminal
                .draw(|frame| {
                    ui::render(
                        frame,
                        Screen::PreparedDiff {
                            diff: &diff,
                            prepared: &prepared,
                            scroll,
                            selected_hunk: 0,
                            raw,
                            side_by_side: split,
                            worktree_entry: Some(&entry),
                        },
                        ui::ASCII_BORDER,
                    )
                })
                .unwrap();
        };
        for (raw, split) in [(false, true), (true, true), (false, false), (true, false)] {
            draw(&mut terminal, raw, split, 0);
        }
        let mut samples = Vec::new();
        for index in 0..100 {
            let start = Instant::now();
            draw(
                &mut terminal,
                index % 2 == 1,
                index % 4 < 2,
                (index % 8) as u16,
            );
            samples.push(start.elapsed());
        }
        samples.sort();
        println!(
            "Warm ordinary diff {width}x{height}: samples=100 p50={:?} p95={:?}",
            samples[49], samples[94]
        );
    }
    let changed=ui::PreparedDiff::new("diff --git a/sample.rs b/sample.rs\n--- a/sample.rs\n+++ b/sample.rs\n@@ -1 +1 @@\n-old_unique\n+new_unique\n".into(),Some(std::path::Path::new("sample.rs")));
    let diff = (889, Some(Ok(changed.text().into())));
    terminal
        .draw(|frame| {
            ui::render(
                frame,
                Screen::PreparedDiff {
                    diff: &diff,
                    prepared: &changed,
                    scroll: 0,
                    selected_hunk: 0,
                    raw: false,
                    side_by_side: true,
                    worktree_entry: Some(&entry),
                },
                ui::ASCII_BORDER,
            )
        })
        .unwrap();
    let output = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(output.contains("new_unique"));
    assert!(!output.contains("value_0"));
    terminal
        .draw(|frame| {
            ui::render(
                frame,
                Screen::PreparedDiff {
                    diff: &diff,
                    prepared: &changed,
                    scroll: 0,
                    selected_hunk: 0,
                    raw: false,
                    side_by_side: false,
                    worktree_entry: Some(&entry),
                },
                ui::ASCII_BORDER,
            )
        })
        .unwrap();
    let unified = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(unified.contains("old_unique"));
    assert!(unified.contains("new_unique"));
}
