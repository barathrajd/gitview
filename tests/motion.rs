use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use gitview::motion::{Motion, MotionParser, Parsed};

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(letter: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(letter), KeyModifiers::CONTROL)
}

fn feed(parser: &mut MotionParser, keys: &str) -> Parsed {
    let mut last = Parsed::Other;
    for key in keys.chars() {
        last = parser.key(plain(KeyCode::Char(key)));
    }
    last
}

#[test]
fn counts_g_and_z_build_motions() {
    let mut parser = MotionParser::default();

    assert_eq!(feed(&mut parser, "12j"), Parsed::Motion(Motion::Down(12)));
    assert_eq!(feed(&mut parser, "k"), Parsed::Motion(Motion::Up(1)));
    assert_eq!(feed(&mut parser, "gg"), Parsed::Motion(Motion::Top));
    assert_eq!(feed(&mut parser, "G"), Parsed::Motion(Motion::Bottom));
    assert_eq!(feed(&mut parser, "zz"), Parsed::Motion(Motion::Center));
    assert_eq!(feed(&mut parser, "0"), Parsed::Other);
    assert_eq!(feed(&mut parser, "g"), Parsed::Pending);
}

#[test]
fn other_keys_drop_pending_input() {
    let mut parser = MotionParser::default();

    assert_eq!(feed(&mut parser, "4s"), Parsed::Other);
    assert_eq!(feed(&mut parser, "j"), Parsed::Motion(Motion::Down(1)));
    assert_eq!(feed(&mut parser, "gzg"), Parsed::Pending);
    assert_eq!(feed(&mut parser, "j"), Parsed::Motion(Motion::Down(1)));
    feed(&mut parser, "5");
    parser.reset();
    assert_eq!(feed(&mut parser, "j"), Parsed::Motion(Motion::Down(1)));
}

#[test]
fn page_keys_and_modified_letters() {
    let mut parser = MotionParser::default();

    assert_eq!(parser.key(ctrl('d')), Parsed::Motion(Motion::HalfPageDown));
    assert_eq!(parser.key(ctrl('u')), Parsed::Motion(Motion::HalfPageUp));
    assert_eq!(parser.key(ctrl('f')), Parsed::Motion(Motion::PageDown));
    assert_eq!(parser.key(ctrl('b')), Parsed::Motion(Motion::PageUp));
    assert_eq!(parser.key(ctrl('j')), Parsed::Other);
    assert_eq!(
        parser.key(plain(KeyCode::PageDown)),
        Parsed::Motion(Motion::PageDown)
    );
    assert_eq!(
        parser.key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::ALT)),
        Parsed::Other
    );
}

#[test]
fn motions_select_within_the_list() {
    assert_eq!(Motion::Down(3).select(Some(8), 10, 4), Some(9));
    assert_eq!(Motion::Up(3).select(Some(1), 10, 4), Some(0));
    assert_eq!(Motion::Bottom.select(None, 10, 4), Some(9));
    assert_eq!(Motion::HalfPageDown.select(Some(0), 10, 4), Some(2));
    assert_eq!(Motion::PageDown.select(Some(0), 10, 4), Some(4));
    assert_eq!(Motion::Top.select(Some(5), 0, 4), None);
    assert_eq!(Motion::PageDown.scroll(3, 5, 4), 5);
    assert_eq!(Motion::HalfPageUp.scroll(1, 5, 4), 0);
}
