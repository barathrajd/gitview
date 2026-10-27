use gitview::{
    app::View,
    branches::{Branches, HeadState},
    details::CommitDetails,
    ui::{Cockpit, Screen, render},
    worktree::{Entry, Section},
};
use ratatui::{Terminal, backend::TestBackend, widgets::ListState};
use std::{ffi::OsString, path::Path};

fn render_cockpit(width: u16) -> String {
    render_cockpit_focused(width, View::History)
}

fn render_cockpit_focused(width: u16, focused: View) -> String {
    let backend = TestBackend::new(width, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let branches = Branches {
        head: HeadState::Attached(OsString::from("main")),
        branches: vec![OsString::from("main")],
        head_commit: Some("abcdef0123456789".to_owned()),
    };
    let history = Ok(vec!["* abcdef0 Ship the cockpit".to_owned()]);
    let status = Ok(vec![Entry {
        section: Section::Unstaged,
        path: OsString::from("src/app.rs"),
        original: None,
    }]);
    let mut history_selection = ListState::default();
    history_selection.select(Some(0));
    let mut working_tree_selection = ListState::default();
    working_tree_selection.select(Some(0));
    terminal
        .draw(|frame| {
            render(
                frame,
                Screen::Cockpit(Cockpit {
                    repository: Path::new("/work/gitview"),
                    branches: Some(&Ok(branches)),
                    history: Some(&history),
                    status: Some(&status),
                    focused,
                    history_selection: &mut history_selection,
                    working_tree_selection: &mut working_tree_selection,
                }),
                ratatui::symbols::border::PLAIN,
            );
        })
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
fn renders_responsive_repository_cockpit() {
    let narrow = render_cockpit(80);
    assert!(narrow.contains("GITVIEW"));
    assert!(narrow.contains("main"));
    assert!(narrow.contains("HISTORY"));
    assert!(narrow.contains("Ship the cockpit"));
    assert!(narrow.contains("Enter/l"));
    assert!(narrow.contains("help"));
    assert!(narrow.contains("quit"));
    assert!(!narrow.contains("DETAILS"));
    assert!(!narrow.contains("WORKTREE"));

    let compact = render_cockpit(100);
    assert!(compact.contains("WORKTREE"));
    assert!(compact.contains("HISTORY"));
    assert!(!compact.contains("INSPECTOR"));

    let wide = render_cockpit(140);
    assert!(wide.contains("WORKTREE"));
    assert!(wide.contains("HISTORY"));
    assert!(wide.contains("INSPECTOR"));
    assert!(wide.contains("quit"));
    let worktree_focused = render_cockpit_focused(140, View::WorkingTree);
    assert!(worktree_focused.contains("diff"));
    assert!(!worktree_focused.contains("details"));
}

#[test]
fn replacing_the_cockpit_with_commit_details_clears_old_panes() {
    let backend = TestBackend::new(140, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let branches = Branches {
        head: HeadState::Attached(OsString::from("main")),
        branches: vec![OsString::from("main")],
        head_commit: Some("abcdef0123456789".to_owned()),
    };
    let history = Ok(vec!["* abcdef0 cockpit-only-marker".to_owned()]);
    let status = Ok(Vec::<Entry>::new());
    let mut history_selection = ListState::default();
    history_selection.select(Some(0));
    let mut working_tree_selection = ListState::default();
    terminal
        .draw(|frame| {
            render(
                frame,
                Screen::Cockpit(Cockpit {
                    repository: Path::new("/work/gitview"),
                    branches: Some(&Ok(branches)),
                    history: Some(&history),
                    status: Some(&status),
                    focused: View::History,
                    history_selection: &mut history_selection,
                    working_tree_selection: &mut working_tree_selection,
                }),
                ratatui::symbols::border::PLAIN,
            );
        })
        .unwrap();

    let details = (
        1,
        Some(Ok(CommitDetails {
            hash: "1234567890abcdef".to_owned(),
            author: "Test Author".to_owned(),
            date: "2026-10-27".to_owned(),
            subject: "Details screen".to_owned(),
            files: vec![OsString::from("src/main.rs")],
        })),
    );
    let mut file_selection = ListState::default();
    terminal
        .draw(|frame| {
            render(
                frame,
                Screen::CommitDetails {
                    details: &details,
                    selection: &mut file_selection,
                },
                ratatui::symbols::border::PLAIN,
            );
        })
        .unwrap();
    let visible: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(visible.contains("Details screen"));
    assert!(!visible.contains("cockpit-only-marker"));
    assert!(!visible.contains("WORKTREE SUMMARY"));
}
