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
                        history_entries: &[],
                        status: Some(&status),
                        feedback: "",
                        search_prompt: None,
                        focused,
                        history_selection,
                        working_tree_selection,
                        branch_selection: &mut ListState::default(),
                        sessions: None,
                        page_open: false,
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

#[test]
#[ignore = "isolated release-build timing diagnostic; run with --ignored --nocapture"]
fn measures_history_search_on_the_ui_thread() {
    let mut worst = 0.0f64;
    // History loads at most 200 rows; larger counts bound the cost if that grows.
    for rows in [200, 10_000, 100_000] {
        let history: Vec<String> = (0..rows)
            .map(|index| {
                format!(
                    "* {index:07x} Fixture commit number {index} - Performance Fixture\u{1f}{index:040x}"
                )
            })
            .collect();
        let mut samples = Vec::new();
        for query in ["a", "fixture commit", "number 99", "zzz-missing"]
            .iter()
            .cycle()
            .take(100)
        {
            let start = Instant::now();
            let matches = gitview::history::search_matches(&history, query);
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
            std::hint::black_box(matches);
        }
        let value = p95(samples);
        println!("history search rows={rows} p95_ms={value:.3}");
        if rows <= 10_000 {
            worst = worst.max(value);
        }
    }
    assert!(worst <= 50.0, "history search p95 exceeded 50 ms: {worst}");
}

/// `count` parsed commits, newest first: a main line where every 100th
/// commit merges a three-commit side branch, with refs on a few commits.
fn structured_fixture(count: usize) -> Vec<gitview::commit_log::Commit> {
    let hash = |index: usize| format!("{index:040x}");
    let mut commits = Vec::with_capacity(count);
    let mut index = count;
    while commits.len() < count {
        index -= 1;
        let main_parent = hash(index - 1);
        let mut commit = gitview::commit_log::Commit {
            hash: hash(index),
            parents: vec![main_parent],
            refs: if index.is_multiple_of(1_000) {
                vec![format!("tag: v{index}")]
            } else {
                Vec::new()
            },
            author: "Performance Fixture".to_owned(),
            time: 1_600_000_000 + index as i64 * 60,
            subject: format!("Fixture commit number {index}"),
        };
        if index.is_multiple_of(100) && index >= 4 {
            // A merge of a side branch whose commits follow it.
            let side = |n: usize| format!("{:040x}", (1usize << 40) + index * 4 + n);
            commit.parents.push(side(0));
            commits.push(commit);
            for n in 0..3 {
                commits.push(gitview::commit_log::Commit {
                    hash: side(n),
                    parents: vec![if n == 2 { hash(index - 1) } else { side(n + 1) }],
                    refs: Vec::new(),
                    author: "Side Author".to_owned(),
                    time: 1_600_000_000 + index as i64 * 60 - n as i64,
                    subject: format!("Side work {n} for {index}"),
                });
            }
        } else {
            if index == 1 {
                commit.parents.clear();
            }
            commits.push(commit);
        }
        if index == 1 {
            break;
        }
    }
    commits.truncate(count);
    commits
}

#[test]
#[ignore = "isolated release-build timing diagnostic; run with --ignored --nocapture"]
fn measures_structured_history_on_100k_commits() {
    let start = Instant::now();
    let structured = gitview::history::structured_from(structured_fixture(100_000));
    let layout_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!("structured layout rows=100000 total_ms={layout_ms:.1}");
    let history = Ok(structured.rows);
    let branches = Ok(Branches {
        head: HeadState::Attached(OsString::from("main")),
        branches: vec![OsString::from("main")],
        head_commit: Some("abcdef0123456789".to_owned()),
    });
    let status = Ok(Vec::new());
    let mut worst = 0.0f64;
    for (width, height) in [(140, 35), (80, 24), (220, 60)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut history_selection = ListState::default();
        let mut working_tree_selection = ListState::default();
        let mut samples = Vec::new();
        // Scroll one row at a time through a window, then jump far.
        let targets = (0..200).chain((0..100).map(|step| step * 999));
        for target in targets {
            history_selection.select(Some(target));
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
                            history_entries: &structured.entries,
                            status: Some(&status),
                            feedback: "",
                            search_prompt: None,
                            focused: View::History,
                            history_selection: &mut history_selection,
                            working_tree_selection: &mut working_tree_selection,
                            branch_selection: &mut ListState::default(),
                            sessions: None,
                            page_open: false,
                        }),
                        ratatui::symbols::border::PLAIN,
                    );
                })
                .unwrap();
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        let value = p95(samples);
        println!("structured history rows=100000 {width}x{height} p95_ms={value:.3}");
        worst = worst.max(value);
    }
    assert!(
        worst <= 16.0,
        "structured frame p95 exceeded 16 ms: {worst}"
    );
}
