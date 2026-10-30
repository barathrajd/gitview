//! Rendering an `AgentPane`'s emulated screen into a ratatui area.

use std::{
    path::Path,
    time::{Duration, Instant},
};

use gitview::{
    pane::{AgentPane, PaneState},
    ui::{ASCII_BORDER, Viewport, render_agent_pane},
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    layout::Rect,
    style::{Color, Modifier},
};

fn argv(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// Runs `script` under `sh` and waits until the pane shows `until`.
fn pane_showing(script: &str, until: &str) -> AgentPane {
    let mut pane = AgentPane::spawn(&argv(&["sh", "-c", script]), Path::new("."), 10, 40).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !pane.screen_text().contains(until) {
        pane.poll();
        assert!(
            Instant::now() < deadline,
            "never showed {until:?}; screen was:\n{}",
            pane.screen_text()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    pane
}

const PLAIN: ratatui::symbols::border::Set<'static> = ratatui::symbols::border::PLAIN;

fn draw(pane: &AgentPane, focused: bool) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(60, 14)).unwrap();
    terminal
        .draw(|frame| {
            let area = Rect::new(2, 1, 42, 12);
            render_agent_pane(&mut Viewport::new(frame, area), area, pane, focused, PLAIN);
        })
        .unwrap();
    terminal
}

/// The pane's content starts one cell inside its border.
const ORIGIN: (u16, u16) = (3, 2);

#[test]
fn draws_the_command_output_inside_a_titled_border() {
    let pane = pane_showing("printf 'hello pane'; sleep 5", "hello pane");

    let terminal = draw(&pane, false);
    let buffer = terminal.backend().buffer();

    let top: String = (2..44)
        .map(|x| buffer[(x, 1)].symbol().to_owned())
        .collect();
    assert!(top.contains("AGENT"), "{top}");
    assert!(top.contains("sh"), "{top}");
    assert!(top.contains("running"), "{top}");
    let first: String = (0..10)
        .map(|i| buffer[(ORIGIN.0 + i, ORIGIN.1)].symbol().to_owned())
        .collect();
    assert_eq!(first, "hello pane");
}

#[test]
fn maps_colors_and_attributes() {
    let pane = pane_showing(
        r"printf '\033[31mred\033[0m \033[1;44mbold\033[0m \033[38;2;1;2;3mtrue\033[0m \033[3;4mfancy\033[0m \033[7minv\033[0m ok'; sleep 5",
        " ok",
    );

    let terminal = draw(&pane, false);
    let buffer = terminal.backend().buffer();
    let at = |column: u16| &buffer[(ORIGIN.0 + column, ORIGIN.1)];

    assert_eq!(at(0).fg, Color::Indexed(1));
    assert_eq!(at(0).bg, Color::Rgb(24, 24, 32));
    assert!(at(4).modifier.contains(Modifier::BOLD));
    assert_eq!(at(4).bg, Color::Indexed(4));
    assert_eq!(at(9).fg, Color::Rgb(1, 2, 3));
    assert!(at(14).modifier.contains(Modifier::ITALIC));
    assert!(at(14).modifier.contains(Modifier::UNDERLINED));
    assert!(at(20).modifier.contains(Modifier::REVERSED));
    // Plain text carries no style.
    assert_eq!(at(24).fg, Color::Reset);
    assert_eq!(at(24).modifier, Modifier::empty());
}

#[test]
fn draws_wide_characters_once() {
    let pane = pane_showing("printf '日本x'; sleep 5", "日本x");

    let terminal = draw(&pane, false);
    let buffer = terminal.backend().buffer();

    assert_eq!(buffer[(ORIGIN.0, ORIGIN.1)].symbol(), "日");
    assert_eq!(buffer[(ORIGIN.0 + 2, ORIGIN.1)].symbol(), "本");
    assert_eq!(buffer[(ORIGIN.0 + 4, ORIGIN.1)].symbol(), "x");
}

#[test]
fn shows_the_alternate_screen_while_a_program_uses_it() {
    let pane = pane_showing(
        r"printf 'main text'; printf '\033[?1049h\033[2J\033[HALT SCREEN'; sleep 5",
        "ALT SCREEN",
    );

    let terminal = draw(&pane, false);
    let buffer = terminal.backend().buffer();
    let first: String = (0..10)
        .map(|i| buffer[(ORIGIN.0 + i, ORIGIN.1)].symbol().to_owned())
        .collect();

    assert_eq!(first, "ALT SCREEN");
    assert!(!pane.screen_text().contains("main text"));
}

#[test]
fn places_the_cursor_after_the_text_only_while_focused() {
    let pane = pane_showing("printf 'abc'; sleep 5", "abc");

    let mut focused = draw(&pane, true);
    assert_eq!(
        focused.get_cursor_position().unwrap(),
        (ORIGIN.0 + 3, ORIGIN.1).into()
    );

    let mut unfocused = draw(&pane, false);
    assert_eq!(unfocused.get_cursor_position().unwrap(), (0, 0).into());
}

#[test]
fn a_hidden_cursor_is_not_placed() {
    let pane = pane_showing(r"printf '\033[?25labc'; sleep 5", "abc");

    let mut terminal = draw(&pane, true);

    assert_eq!(terminal.get_cursor_position().unwrap(), (0, 0).into());
}

#[test]
fn the_title_reports_how_the_command_ended() {
    let mut pane = pane_showing("echo done; exit 7", "done");
    let deadline = Instant::now() + Duration::from_secs(5);
    while !matches!(pane.poll(), PaneState::Exited(_)) {
        assert!(Instant::now() < deadline, "never exited");
        std::thread::sleep(Duration::from_millis(10));
    }

    let terminal = draw(&pane, false);
    let buffer = terminal.backend().buffer();
    let top: String = (2..44)
        .map(|x| buffer[(x, 1)].symbol().to_owned())
        .collect();

    assert!(top.contains("exited 7"), "{top}");
}

#[test]
fn nothing_is_drawn_outside_the_given_area() {
    let pane = pane_showing("printf 'hello'; sleep 5", "hello");

    let terminal = draw(&pane, true);
    let buffer = terminal.backend().buffer();

    for y in 0..14 {
        for x in 0..60 {
            let inside = (2..44).contains(&x) && (1..13).contains(&y);
            if !inside {
                assert_eq!(buffer[(x, y)].symbol(), " ", "cell ({x}, {y})");
            }
        }
    }
}

fn top_row(pane: &AgentPane, width: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, 6)).unwrap();
    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, width, 6);
            render_agent_pane(&mut Viewport::new(frame, area), area, pane, false, PLAIN);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..width)
        .map(|x| buffer[(x, 0)].symbol().to_owned())
        .collect()
}

#[test]
fn the_focus_hint_gives_way_to_the_status_as_the_pane_narrows() {
    let pane = pane_showing("printf 'hello'; sleep 5", "hello");

    let wide = top_row(&pane, 70);
    assert!(wide.contains("running"), "{wide}");
    assert!(wide.contains("Ctrl+\\ focuses the agent"), "{wide}");

    let medium = top_row(&pane, 36);
    assert!(medium.contains("running"), "{medium}");
    assert!(medium.contains("Ctrl+\\"), "{medium}");
    assert!(!medium.contains("focuses"), "{medium}");

    let narrow = top_row(&pane, 31);
    assert!(narrow.contains("running"), "{narrow}");
    assert!(!narrow.contains("Ctrl"), "{narrow}");
}

#[test]
fn ascii_mode_draws_the_border_with_plain_characters() {
    let pane = pane_showing("printf 'ascii'; sleep 5", "ascii");
    let mut terminal = Terminal::new(TestBackend::new(40, 6)).unwrap();
    terminal
        .draw(|frame| {
            let area = Rect::new(0, 0, 40, 6);
            render_agent_pane(
                &mut Viewport::new(frame, area),
                area,
                &pane,
                false,
                ASCII_BORDER,
            );
        })
        .unwrap();
    let buffer = terminal.backend().buffer();

    for (x, y) in [(0, 0), (39, 0), (0, 5), (39, 5), (10, 5), (0, 3)] {
        let symbol = buffer[(x, y)].symbol();
        assert!(symbol.is_ascii(), "({x}, {y}) is {symbol:?}");
    }
    assert_eq!(buffer[(0, 0)].symbol(), "+");
}

#[test]
fn agent_cells_replace_existing_symbols_and_modifiers() {
    use ratatui::{style::Style, widgets::Paragraph};
    let pane = pane_showing("printf '├─ tests/'; sleep 5", "tests/");
    let mut terminal = Terminal::new(TestBackend::new(60, 14)).unwrap();
    terminal
        .draw(|frame| {
            let area = Rect::new(2, 1, 42, 12);
            frame.render_widget(
                Paragraph::new("stale text").style(
                    Style::default()
                        .bg(Color::White)
                        .add_modifier(Modifier::REVERSED),
                ),
                area,
            );
            render_agent_pane(&mut Viewport::new(frame, area), area, &pane, false, PLAIN);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    for x in ORIGIN.0..ORIGIN.0 + 40 {
        let cell = &buffer[(x, ORIGIN.1)];
        assert_eq!(cell.modifier, Modifier::empty());
        assert_eq!(cell.bg, Color::Rgb(24, 24, 32));
    }
    assert_eq!(buffer[(ORIGIN.0 + 15, ORIGIN.1)].symbol(), " ");
}
