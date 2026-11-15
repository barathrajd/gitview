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
    fn first_from_wraps_to_the_first_match() {
        assert_eq!(first_from(&[1, 4], 2), Some(4));
        assert_eq!(first_from(&[1, 4], 4), Some(4));
        assert_eq!(first_from(&[1, 4], 5), Some(1));
        assert_eq!(first_from(&[], 0), None);
    }
}
