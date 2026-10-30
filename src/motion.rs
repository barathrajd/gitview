//! Vim-style list motions shared by every browsing screen: counts, `gg`, `G`,
//! `zz`, and the page keys. Screens with text input must not feed it their
//! typed characters.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// `k` or Up, repeated by a count.
    Up(usize),
    /// `j` or Down, repeated by a count.
    Down(usize),
    /// `gg`.
    Top,
    /// `G`.
    Bottom,
    /// Ctrl-u.
    HalfPageUp,
    /// Ctrl-d.
    HalfPageDown,
    /// Ctrl-b or Page Up.
    PageUp,
    /// Ctrl-f or Page Down.
    PageDown,
    /// `zz`.
    Center,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parsed {
    /// The key was a count digit or the first key of `gg`/`zz`.
    Pending,
    Motion(Motion),
    /// Not a motion: the screen handles the key as usual.
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MotionParser {
    /// Digits typed before a motion, such as the `12` of `12j`.
    count: Option<usize>,
    /// The first `g` of `gg`.
    pending_g: bool,
    /// The first `z` of `zz`.
    pending_z: bool,
}

impl MotionParser {
    /// Drops a pending count or `g`/`z`, as any non-motion key does.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Reads one key. Pending input applies only to the key right after it.
    pub fn key(&mut self, key: KeyEvent) -> Parsed {
        let Self {
            count,
            pending_g,
            pending_z,
        } = std::mem::take(self);
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('u') => Parsed::Motion(Motion::HalfPageUp),
                KeyCode::Char('d') => Parsed::Motion(Motion::HalfPageDown),
                KeyCode::Char('b') => Parsed::Motion(Motion::PageUp),
                KeyCode::Char('f') => Parsed::Motion(Motion::PageDown),
                _ => Parsed::Other,
            };
        }
        if key
            .modifiers
            .intersects(KeyModifiers::ALT | KeyModifiers::SUPER)
        {
            return Parsed::Other;
        }
        let repeat = count.unwrap_or(1);
        Parsed::Motion(match key.code {
            KeyCode::Char('g') if pending_g => Motion::Top,
            KeyCode::Char('g') => {
                self.pending_g = true;
                return Parsed::Pending;
            }
            KeyCode::Char('z') if pending_z => Motion::Center,
            KeyCode::Char('z') => {
                self.pending_z = true;
                return Parsed::Pending;
            }
            KeyCode::Char(digit @ '0'..='9') if digit != '0' || count.is_some() => {
                let digit = digit as usize - '0' as usize;
                self.count = Some(count.unwrap_or(0).saturating_mul(10).saturating_add(digit));
                return Parsed::Pending;
            }
            KeyCode::Char('G') => Motion::Bottom,
            KeyCode::Up | KeyCode::Char('k') => Motion::Up(repeat),
            KeyCode::Down | KeyCode::Char('j') => Motion::Down(repeat),
            KeyCode::PageUp => Motion::PageUp,
            KeyCode::PageDown => Motion::PageDown,
            _ => return Parsed::Other,
        })
    }
}

impl Motion {
    /// The selection after this motion in a list of `len` rows showing `page`
    /// rows at a time. `Center` keeps the selection; it scrolls instead.
    pub fn select(self, selected: Option<usize>, len: usize, page: usize) -> Option<usize> {
        let last = len.checked_sub(1)?;
        let current = selected.unwrap_or(0);
        let half = (page / 2).max(1);
        let page = page.max(1);
        Some(match self {
            Self::Up(count) => current.saturating_sub(count),
            Self::Down(count) => current.saturating_add(count).min(last),
            Self::Top => 0,
            Self::Bottom => last,
            Self::HalfPageUp => current.saturating_sub(half),
            Self::HalfPageDown => current.saturating_add(half).min(last),
            Self::PageUp => current.saturating_sub(page),
            Self::PageDown => current.saturating_add(page).min(last),
            Self::Center => current,
        })
    }

    /// The scroll offset after this motion in a view of `max + 1` possible
    /// offsets showing `page` rows at a time. `Center` keeps the offset.
    pub fn scroll(self, offset: usize, max: usize, page: usize) -> usize {
        self.select(Some(offset), max.saturating_add(1), page)
            .unwrap_or(0)
    }
}
