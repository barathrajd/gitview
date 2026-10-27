use gitview::{
    app::View,
    branches::{Branches, HeadState},
    ui::{Cockpit, Screen, render},
    worktree::{Entry, Section},
};
use ratatui::{Terminal, backend::TestBackend, widgets::ListState};
use std::{ffi::OsString, path::Path};

fn render_cockpit(width: u16) -> String {
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
                    focused: View::History,
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
    assert!(!narrow.contains("WORKTREE"));

    let compact = render_cockpit(100);
    assert!(compact.contains("WORKTREE"));
    assert!(compact.contains("HISTORY"));
    assert!(!compact.contains("INSPECTOR"));

    let wide = render_cockpit(140);
    assert!(wide.contains("WORKTREE"));
    assert!(wide.contains("HISTORY"));
    assert!(wide.contains("INSPECTOR"));
}
