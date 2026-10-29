//! Spots the questions a program asks its terminal.
//!
//! Terminals answer queries such as "where is the cursor?" by writing a reply
//! to the program's input. The emulator behind the pane doesn't, so a program
//! that waits for an answer may stall or fall back to a plainer mode. Counting
//! the queries shows which answers would matter.

use std::collections::BTreeMap;

const ESCAPE: u8 = 0x1b;
/// Enough to hold the longest query, so one split across reads is still found.
const CARRY: usize = 16;

const MODE_STATUS: &str = "mode status (CSI ? Pm $ p)";

/// Query sequences with a fixed spelling, and what they ask.
const FIXED: &[(&[u8], &str)] = &[
    (b"\x1b[6n", "cursor position (CSI 6 n)"),
    (b"\x1b[5n", "device status (CSI 5 n)"),
    (b"\x1b[c", "device attributes (CSI c)"),
    (b"\x1b[0c", "device attributes (CSI c)"),
    (b"\x1b[>c", "secondary device attributes (CSI > c)"),
    (b"\x1b[>0c", "secondary device attributes (CSI > c)"),
    (b"\x1b[?u", "kitty keyboard flags (CSI ? u)"),
    (b"\x1b[>q", "terminal version (CSI > q)"),
    (b"\x1b[>0q", "terminal version (CSI > q)"),
    (b"\x1b]10;?", "foreground color (OSC 10)"),
    (b"\x1b]11;?", "background color (OSC 11)"),
    (b"\x1b[14t", "text area size in pixels (CSI 14 t)"),
    (b"\x1b[16t", "cell size in pixels (CSI 16 t)"),
    (b"\x1b[18t", "text area size (CSI 18 t)"),
];

#[derive(Default)]
pub(super) struct QueryScanner {
    carry: Vec<u8>,
    counts: BTreeMap<&'static str, u32>,
}

impl QueryScanner {
    pub(super) fn scan(&mut self, chunk: &[u8]) {
        let already_seen = self.carry.len();
        let mut buffer = std::mem::take(&mut self.carry);
        buffer.extend_from_slice(chunk);

        for start in 0..buffer.len() {
            if buffer[start] != ESCAPE {
                continue;
            }
            let rest = &buffer[start..];
            let found = FIXED
                .iter()
                .find(|(pattern, _)| rest.starts_with(pattern))
                .map(|(pattern, name)| (pattern.len(), *name))
                .or_else(|| mode_status_length(rest).map(|length| (length, MODE_STATUS)));
            // A query that ended before this chunk was counted when it arrived.
            if let Some((length, name)) = found
                && start + length > already_seen
            {
                *self.counts.entry(name).or_default() += 1;
            }
        }

        let keep = buffer.len().min(CARRY);
        self.carry = buffer[buffer.len() - keep..].to_vec();
    }

    pub(super) fn counts(&self) -> Vec<(String, u32)> {
        self.counts
            .iter()
            .map(|(name, count)| ((*name).to_owned(), *count))
            .collect()
    }
}

/// The length of `ESC [ ? digits $ p` (or without the `?`) at the start of
/// `bytes`, which asks whether a terminal mode is set.
fn mode_status_length(bytes: &[u8]) -> Option<usize> {
    let rest = bytes.strip_prefix(b"\x1b[")?;
    let mut length = 2;
    let rest = match rest.strip_prefix(b"?") {
        Some(after) => {
            length += 1;
            after
        }
        None => rest,
    };
    let digits = rest.iter().take_while(|byte| byte.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    rest[digits..]
        .starts_with(b"$p")
        .then_some(length + digits + 2)
}
