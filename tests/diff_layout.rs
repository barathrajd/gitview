use gitview::diff::side_by_side;

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
