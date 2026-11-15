use crate::branches::{Branches, HeadState};
use crate::ui::Viewport;
use ratatui::{
    layout::{Constraint, Layout},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph, Wrap},
};

use super::hints::{hint_row_count, render_key_hints};
use super::theme::{page_block, selection_style};

pub(super) fn render_branches(
    frame: &mut Viewport<'_, '_>,
    branches: Option<&Result<Branches, String>>,
    feedback: &str,
    search_prompt: Option<super::screen::SearchPrompt<'_>>,
    selection: &mut ListState,
    border: ratatui::symbols::border::Set<'static>,
) {
    let mut finder =
        crate::history::MatchFinder::new(search_prompt.as_ref().map_or("", |prompt| prompt.query));
    let (heading, lines): (String, Vec<Line<'static>>) = match branches {
        None => (
            "Local branches - loading".into(),
            vec![Line::raw("Loading branches...")],
        ),
        Some(Err(error)) => (
            "Local branches - error".into(),
            vec![Line::raw(error.clone())],
        ),
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
                vec![Line::raw("No local branches")]
            } else {
                data.branches
                    .iter()
                    .map(|name| {
                        let current = matches!(
                            &data.head,
                            HeadState::Attached(head) if head == name
                        );
                        let name = name.to_string_lossy();
                        let mut spans = vec![Span::raw(if current { "* " } else { "  " })];
                        spans.extend(super::highlighted_spans(&name, &finder.positions(&name)));
                        Line::from(spans)
                    })
                    .collect()
            };
            (heading, lines)
        }
    };
    let hints: &[(&str, &str)] = if search_prompt.is_some() {
        &[("Enter", "stay"), ("Esc", "back")]
    } else {
        &[
            ("j/k", "move"),
            ("Enter/l", "switch"),
            ("/", "search"),
            ("c", "create"),
            ("t", "tags"),
            ("r", "rename"),
            ("-", "previous checkout"),
            ("d", "delete"),
            ("Esc/h", "back"),
        ]
    };
    // While searching, the prompt takes the feedback row.
    let prompt = search_prompt
        .as_ref()
        .map(|prompt| format!(" {}", prompt.line));
    let feedback = prompt.as_deref().unwrap_or(feedback);
    let hint_height = hint_row_count(hints, frame.area().width);
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
    let feedback_style = if prompt.is_some() {
        ratatui::style::Style::default().fg(super::ACCENT)
    } else {
        ratatui::style::Style::default()
    };
    frame.render_widget(
        Paragraph::new(feedback)
            .style(feedback_style)
            .wrap(Wrap { trim: false }),
        areas[1],
    );
    render_key_hints(frame, areas[2], hints);
}
