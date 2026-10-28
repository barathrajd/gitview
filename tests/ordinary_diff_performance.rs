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

fn stress_patch(lines: usize, length: usize) -> String {
    let mut text = format!(
        "diff --git a/sample.txt b/sample.txt\n--- a/sample.txt\n+++ b/sample.txt\n@@ -0,0 +1,{lines} @@\n"
    );
    for _ in 0..lines {
        text.push('+');
        text.push_str(&"x".repeat(length));
        text.push('\n');
    }
    text
}

fn draw_prepared(
    terminal: &mut Terminal<TestBackend>,
    prepared: &ui::PreparedDiff,
    diff: &(u64, Option<Result<String, String>>),
    raw: bool,
    split: bool,
    scroll: u16,
) {
    terminal
        .draw(|frame| {
            ui::render(
                frame,
                Screen::PreparedDiff {
                    diff,
                    prepared,
                    scroll,
                    selected_hunk: 0,
                    raw,
                    side_by_side: split,
                    worktree_entry: None,
                },
                ui::ASCII_BORDER,
            )
        })
        .unwrap();
}

#[test]
#[ignore = "release-build stress diagnostic; run explicitly with --ignored --nocapture"]
fn measures_supported_limit_layouts() {
    for (name, lines, length) in [
        ("medium", 1000, 120),
        ("stress", 4990, 190),
        ("long-line", 2, 450_000),
    ] {
        let text = stress_patch(lines, length);
        let start = Instant::now();
        let prepared = ui::PreparedDiff::new(text, Some(std::path::Path::new("sample.txt")));
        println!(
            "{name}: bytes={} prepare={:?}",
            prepared.text().len(),
            start.elapsed()
        );
        let diff = (990, Some(Ok(prepared.text().to_owned())));
        for (width, height) in [(140, 35), (80, 24)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            for (raw, split) in [(false, true), (true, true), (false, false), (true, false)] {
                let start = Instant::now();
                draw_prepared(&mut terminal, &prepared, &diff, raw, split, 0);
                let elapsed = start.elapsed();
                println!("{name} {width}x{height} raw={raw} split={split} first={elapsed:?}");
                assert!(
                    elapsed <= std::time::Duration::from_millis(100),
                    "UI-thread layout exceeded the plan budget"
                );
            }
            let mut samples = Vec::new();
            for index in 0..100 {
                let start = Instant::now();
                draw_prepared(
                    &mut terminal,
                    &prepared,
                    &diff,
                    index % 2 == 1,
                    index % 4 < 2,
                    (index * 10) as u16,
                );
                samples.push(start.elapsed());
            }
            samples.sort();
            println!(
                "{name} {width}x{height} warm samples=100 p50={:?} p95={:?}",
                samples[49], samples[94]
            );
            assert!(
                samples[94] <= std::time::Duration::from_millis(50),
                "warm actions exceeded the plan budget"
            );
        }
    }
}

fn rss_kib() -> u64 {
    let result = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    assert!(result.status.success());
    String::from_utf8(result.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

#[test]
#[ignore = "isolated release-build RSS diagnostic; run explicitly with --ignored --nocapture"]
fn measures_diff_cache_lifecycle() {
    let patch = stress_patch(4990, 190);
    let cycle = || {
        let prepared =
            ui::PreparedDiff::new(patch.clone(), Some(std::path::Path::new("sample.txt")));
        let diff = (991, Some(Ok(prepared.text().to_owned())));
        for (width, height) in [(140, 35), (80, 24), (140, 35)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            for (raw, split) in [(false, true), (true, true), (false, false), (true, false)] {
                draw_prepared(&mut terminal, &prepared, &diff, raw, split, 10);
            }
        }
        // Dropping the prepared view models closing or replacing an ordinary diff.
    };
    for _ in 0..10 {
        cycle();
    }
    let warm = rss_kib();
    let mut peak = warm;
    for index in 0..100 {
        cycle();
        if index % 10 == 9 {
            let rss = rss_kib();
            peak = peak.max(rss);
            println!("cache lifecycle cycle={} rss_kib={rss}", index + 1);
        }
    }
    let final_rss = rss_kib();
    let growth = final_rss.saturating_sub(warm);
    let budget = 10240.max(warm / 10);
    println!(
        "Cache lifecycle: cycles=100 warm_rss_kib={warm} final_rss_kib={final_rss} peak_settled_rss_kib={peak} growth_kib={growth} budget_kib={budget}"
    );
    assert!(
        growth <= budget,
        "settled cache memory exceeded the performance plan budget"
    );
}

#[cfg(feature = "test-support")]
#[test]
fn bounds_retained_diff_layout_bytes_without_losing_rows() {
    let prepared = ui::PreparedDiff::new(
        stress_patch(1000, 190),
        Some(std::path::Path::new("sample.txt")),
    );
    for width in [80, 140, 80] {
        for (raw, split) in [(false, true), (true, true), (false, false), (true, false)] {
            assert_eq!(
                prepared.hunk_positions(width, raw, split),
                ui::diff_hunk_line_positions(prepared.text(), width, raw, split)
            );
            let (entries, bytes, active_bytes) = prepared.layout_cache_usage();
            assert!(entries <= 4);
            assert!(bytes <= 8 * 1024 * 1024 || (entries == 1 && bytes == active_bytes));
        }
    }
}
