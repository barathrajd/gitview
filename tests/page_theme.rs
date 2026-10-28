use gitview::{
    comparison_page::ComparisonPage,
    tag_page::TagPage,
    ui::{Screen, render},
};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
fn draw(screen: Screen<'_>, width: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, 20)).unwrap();
    terminal
        .draw(|frame| render(frame, screen, gitview::ui::ASCII_BORDER))
        .unwrap();
    terminal.backend().buffer().clone()
}
#[test]
fn new_pages_match_existing_title_border_and_keycap_colors() {
    let baseline = draw(Screen::BranchInput("example"), 80);
    let comparison = ComparisonPage::open("HEAD", None);
    let root = tempfile::tempdir().unwrap();
    let tags = TagPage::open(root.path());
    for width in [60, 80, 140] {
        for buffer in [
            draw(Screen::Comparison(&comparison), width),
            draw(Screen::Tags(&tags), width),
        ] {
            assert_eq!(buffer[(0, 0)].symbol(), "+");
            assert_eq!(buffer[(0, 0)].fg, baseline[(0, 0)].fg, "border color");
            assert_eq!(buffer[(1, 0)].fg, baseline[(1, 0)].fg, "title color");
            let keycap = &baseline[(0, 19)];
            assert!(
                buffer
                    .content()
                    .iter()
                    .any(|cell| cell.bg == keycap.bg && cell.fg == keycap.fg),
                "styled keyboard hints"
            );
        }
    }
}
