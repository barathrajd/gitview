use crate::branches::{Branches, HeadState};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    widgets::{List, ListItem, ListState, Paragraph, Wrap},
};

use super::hints::{hint_row_count, render_key_hints};
use super::theme::{page_block, selection_style};

pub(super) fn render_branches(
    frame: &mut Frame<'_>,
    branches: Option<&Result<Branches, String>>,
    feedback: &str,
    selection: &mut ListState,
    border: ratatui::symbols::border::Set<'static>,
) {
    let (heading, lines): (String, Vec<String>) = match branches {
        None => (
            "Local branches - loading".into(),
            vec!["Loading branches...".into()],
        ),
        Some(Err(error)) => ("Local branches - error".into(), vec![error.clone()]),
        Some(Ok(data)) => {
            let heading = match &data.head {
                HeadState::Attached(name) => {
                    format!("Local branches - current: {}", name.to_string_lossy())
                }
                HeadState::Detached => "Local branches - detached HEAD".into(),
                HeadState::Unborn(name) => {
                    format!("Local branches - unborn: {}", name.to_string_lossy())
                }
            };
            let lines = if data.branches.is_empty() {
                vec!["No local branches".into()]
            } else {
                data.branches
                    .iter()
                    .map(|name| {
                        let current = matches!(
                            &data.head,
                            HeadState::Attached(head) if head == name
                        );
                        format!(
                            "{} {}",
                            if current { "*" } else { " " },
                            name.to_string_lossy()
                        )
                    })
                    .collect()
            };
            (heading, lines)
        }
    };
    let hints = [
        ("j/k", "move"),
        ("Enter/l", "switch"),
        ("c", "create"),
        ("t", "tags"),
        ("r", "rename"),
        ("-", "previous checkout"),
        ("d", "delete"),
        ("Esc/h", "back"),
    ];
    let hint_height = hint_row_count(&hints, frame.area().width);
    let available = frame.area().height.saturating_sub(3 + hint_height + 1);
    let width = usize::from(frame.area().width.max(1));
    let feedback_lines = feedback
        .lines()
        .map(|line| line.chars().count().max(1).div_ceil(width))
        .sum::<usize>()
        .max(1);
    let feedback_height = u16::try_from(feedback_lines)
        .unwrap_or(u16::MAX)
        .min(available)
        .min(8);
    let areas = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(feedback_height),
        Constraint::Length(hint_height),
    ])
    .split(frame.area());
    let list = List::new(lines.into_iter().map(ListItem::new).collect::<Vec<_>>())
        .block(page_block(heading, border))
        .highlight_symbol("› ")
        .highlight_style(selection_style());
    frame.render_stateful_widget(list, areas[0], selection);
    frame.render_widget(
        Paragraph::new(feedback).wrap(Wrap { trim: false }),
        areas[1],
    );
    render_key_hints(frame, areas[2], &hints);
}
