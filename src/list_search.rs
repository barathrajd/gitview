//! Inline `/` search over a list in display order: the positions that match,
//! where typing jumps, and where `n`/`N` step. Highlighting uses
//! [`crate::history::MatchFinder`], which ranks the same way.

/// Positions of the items matching `query`, in display order. Each word must
/// match somewhere in the item, its letters in order; case is ignored. Empty
/// without a query.
pub fn matches_in_order<S: AsRef<str>>(items: &[S], query: &str) -> Vec<usize> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let config = frizbee::Config::default().casing(frizbee::CaseMatching::Ignore);
    let mut matcher = frizbee::Matcher::from_query(query, &config);
    let mut matches: Vec<usize> = matcher
        .match_list(items)
        .into_iter()
        .map(|found| found.index as usize)
        .collect();
    matches.sort_unstable();
    matches
}

/// The first match at or after `start`, wrapping around to the first match.
pub fn first_from(matches: &[usize], start: usize) -> Option<usize> {
    matches
        .iter()
        .find(|&&position| position >= start)
        .or_else(|| matches.first())
        .copied()
}

/// An inline `/` search on a page that owns its list, where item indices
/// are display positions. The page routes keys here while it is open.
pub struct InlineSearch {
    pub query: String,
    origin: usize,
    matches: Vec<usize>,
}

impl InlineSearch {
    /// Start searching from the selected item.
    pub fn start(origin: usize) -> Self {
        Self {
            query: String::new(),
            origin,
            matches: Vec::new(),
        }
    }

    /// Apply a key, moving `selected`. Returns false once the search closes:
    /// Esc restores the starting item, and Enter keeps the match and saves
    /// the query in `last_search` for `n`/`N`.
    pub fn key<S: AsRef<str>>(
        &mut self,
        key: crossterm::event::KeyCode,
        items: &[S],
        selected: &mut usize,
        last_search: &mut String,
    ) -> bool {
        use crossterm::event::KeyCode;
        match key {
            KeyCode::Esc => {
                *selected = self.origin;
                return false;
            }
            KeyCode::Enter => {
                last_search.clone_from(&self.query);
                return false;
            }
            KeyCode::Backspace => {
                self.query.pop();
            }
            KeyCode::Char(c) if !c.is_control() => self.query.push(c),
            _ => return true,
        }
        self.matches = matches_in_order(items, &self.query);
        *selected = first_from(&self.matches, self.origin).unwrap_or(self.origin);
        true
    }

    /// The status line: the query and which match is selected.
    pub fn line(&self, selected: usize) -> String {
        let status = if self.query.trim().is_empty() {
            String::new()
        } else if let Some(position) = self.matches.iter().position(|&m| m == selected) {
            format!("match {} of {}", position + 1, self.matches.len())
        } else {
            "no match".to_owned()
        };
        format!("/{}_   {status}", self.query)
    }
}

/// `n`/`N` on a page's list: the match of `query` after (or before)
/// `selected`, wrapping around. None without a query or a match.
pub fn step<S: AsRef<str>>(
    items: &[S],
    query: &str,
    selected: usize,
    forward: bool,
) -> Option<usize> {
    crate::history::step_match(&matches_in_order(items, query), Some(selected), forward)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_keep_display_order_and_need_every_word() {
        let items = ["src/beta.rs", "docs/alpha.md", "src/alpha.rs"];
        assert_eq!(matches_in_order(&items, "alpha"), [1, 2]);
        assert_eq!(matches_in_order(&items, "SRC alpha"), [2]);
        assert!(matches_in_order(&items, "  ").is_empty());
        assert!(matches_in_order(&items, "zzz").is_empty());
    }

    #[test]
    fn inline_search_restores_on_esc_and_saves_on_enter() {
        use crossterm::event::KeyCode::{Char, Enter, Esc};
        let items = ["v1-alpha", "v2-beta", "v3-alpha"];
        let mut last = String::new();
        let mut selected = 1;
        let mut search = InlineSearch::start(selected);
        for c in "alpha".chars() {
            assert!(search.key(Char(c), &items, &mut selected, &mut last));
        }
        assert_eq!(selected, 2);
        assert_eq!(search.line(selected), "/alpha_   match 2 of 2");
        assert!(!search.key(Esc, &items, &mut selected, &mut last));
        assert_eq!((selected, last.as_str()), (1, ""));

        let mut search = InlineSearch::start(selected);
        search.key(Char('a'), &items, &mut selected, &mut last);
        assert!(!search.key(Enter, &items, &mut selected, &mut last));
        assert_eq!(last, "a");
        assert_eq!(step(&items, &last, selected, true), Some(2));
        assert_eq!(step(&items, "", selected, true), None);
    }

    #[test]
    fn first_from_wraps_to_the_first_match() {
        assert_eq!(first_from(&[1, 4], 2), Some(4));
        assert_eq!(first_from(&[1, 4], 4), Some(4));
        assert_eq!(first_from(&[1, 4], 5), Some(1));
        assert_eq!(first_from(&[], 0), None);
    }
}
