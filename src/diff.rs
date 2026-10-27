#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumberedDiffLine<'a> {
    pub old_number: Option<u32>,
    pub new_number: Option<u32>,
    pub text: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumberedDiffRow<'a> {
    pub old_number: Option<u32>,
    pub old_text: &'a str,
    pub new_number: Option<u32>,
    pub new_text: &'a str,
}

/// Attach old/new file line numbers to unified diff lines.
pub fn numbered_lines(diff: &str) -> Vec<NumberedDiffLine<'_>> {
    let mut old_number = None;
    let mut new_number = None;
    let mut result = Vec::new();
    for text in diff.lines() {
        if let Some((old_start, new_start)) = parse_hunk_header(text) {
            old_number = Some(old_start);
            new_number = Some(new_start);
            result.push(NumberedDiffLine {
                old_number: None,
                new_number: None,
                text,
            });
            continue;
        }
        if text.starts_with("diff --git ") || text.starts_with("diff --cc ") {
            old_number = None;
            new_number = None;
        }
        let (old, new) = match (old_number, new_number) {
            (Some(old), Some(new)) if text.starts_with(' ') => {
                old_number = Some(old.saturating_add(1));
                new_number = Some(new.saturating_add(1));
                (Some(old), Some(new))
            }
            (Some(old), Some(_)) if text.starts_with('-') && !text.starts_with("---") => {
                old_number = Some(old.saturating_add(1));
                (Some(old), None)
            }
            (Some(_), Some(new)) if text.starts_with('+') && !text.starts_with("+++") => {
                new_number = Some(new.saturating_add(1));
                (None, Some(new))
            }
            _ => (None, None),
        };
        result.push(NumberedDiffLine {
            old_number: old,
            new_number: new,
            text,
        });
    }
    result
}

/// Align changed lines and retain their source and destination line numbers.
pub fn numbered_side_by_side(diff: &str) -> Vec<NumberedDiffRow<'_>> {
    let lines = numbered_lines(diff);
    let mut rows = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if line.old_number.is_some() && line.new_number.is_some() {
            rows.push(NumberedDiffRow {
                old_number: line.old_number,
                old_text: line.text,
                new_number: line.new_number,
                new_text: line.text,
            });
            index += 1;
        } else if line.old_number.is_some() || line.new_number.is_some() {
            let mut removed = Vec::new();
            let mut added = Vec::new();
            while index < lines.len()
                && (lines[index].old_number.is_some() || lines[index].new_number.is_some())
                && !(lines[index].old_number.is_some() && lines[index].new_number.is_some())
            {
                let changed = lines[index];
                if changed.old_number.is_some() {
                    removed.push(changed);
                } else if changed.new_number.is_some() {
                    added.push(changed);
                }
                index += 1;
            }
            for offset in 0..removed.len().max(added.len()) {
                let old = removed.get(offset);
                let new = added.get(offset);
                rows.push(NumberedDiffRow {
                    old_number: old.and_then(|line| line.old_number),
                    old_text: old.map_or("", |line| line.text),
                    new_number: new.and_then(|line| line.new_number),
                    new_text: new.map_or("", |line| line.text),
                });
            }
        } else {
            rows.push(NumberedDiffRow {
                old_number: None,
                old_text: line.text,
                new_number: None,
                new_text: line.text,
            });
            index += 1;
        }
    }
    rows
}

fn parse_hunk_header(line: &str) -> Option<(u32, u32)> {
    let body = line.strip_prefix("@@ ")?;
    let (old, rest) = body.split_once(" +")?;
    let (new, _) = rest.split_once(" @@")?;
    let parse_start = |range: &str| {
        let range = range
            .strip_prefix('-')
            .or_else(|| range.strip_prefix('+'))
            .unwrap_or(range);
        range.split(',').next()?.parse::<u32>().ok()
    };
    Some((parse_start(old)?, parse_start(new)?))
}

/// Align adjacent removed and added lines while retaining diff headers and context.
pub fn side_by_side(diff: &str) -> Vec<(String, String)> {
    let lines: Vec<_> = diff.lines().collect();
    let mut rows = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if line.starts_with('-') && !line.starts_with("---")
            || line.starts_with('+') && !line.starts_with("+++")
        {
            let mut removed = Vec::new();
            let mut added = Vec::new();
            while index < lines.len() {
                let line = lines[index];
                if line.starts_with('-') && !line.starts_with("---") {
                    removed.push(line);
                } else if line.starts_with('+') && !line.starts_with("+++") {
                    added.push(line);
                } else {
                    break;
                }
                index += 1;
            }
            for offset in 0..removed.len().max(added.len()) {
                rows.push((
                    removed.get(offset).copied().unwrap_or("").to_owned(),
                    added.get(offset).copied().unwrap_or("").to_owned(),
                ));
            }
        } else {
            rows.push((line.to_owned(), line.to_owned()));
            index += 1;
        }
    }
    rows
}
