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
