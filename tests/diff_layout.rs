use gitview::diff::{numbered_lines, numbered_side_by_side, side_by_side};

#[test]
fn aligns_removed_and_added_lines_and_preserves_context() {
    let rows = side_by_side("@@ -1,2 +1,2 @@\n same\n-old\n+new\n");
    assert!(
        rows.iter()
            .any(|(left, right)| left == "-old" && right == "+new")
    );
    assert!(
        rows.iter()
            .any(|(left, right)| left == " same" && right == " same")
    );
}

#[test]
fn tracks_old_and_new_line_numbers_across_hunks_and_replacements() {
    let diff = "diff --git a/file b/file\n--- a/file\n+++ b/file\n@@ -10,2 +20,3 @@\n context\n-old\n+new one\n+new two\n@@ -30 +41 @@\n-last\n+replacement\n";
    let lines = numbered_lines(diff);
    assert!(lines.iter().any(|line| {
        line.old_number == Some(10) && line.new_number == Some(20) && line.text == " context"
    }));
    assert!(lines.iter().any(|line| {
        line.old_number == Some(11) && line.new_number.is_none() && line.text == "-old"
    }));
    assert!(lines.iter().any(|line| {
        line.old_number.is_none() && line.new_number == Some(22) && line.text == "+new two"
    }));

    let rows = numbered_side_by_side(diff);
    assert!(rows.iter().any(|row| {
        row.old_number == Some(11)
            && row.old_text == "-old"
            && row.new_number == Some(21)
            && row.new_text == "+new one"
    }));
    assert!(rows.iter().any(|row| {
        row.old_number.is_none() && row.old_text.is_empty() && row.new_number == Some(22)
    }));
    assert!(rows.iter().any(|row| {
        row.old_number == Some(30)
            && row.old_text == "-last"
            && row.new_number == Some(41)
            && row.new_text == "+replacement"
    }));
}
