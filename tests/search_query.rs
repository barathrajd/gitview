use gitview::search_query::{SearchQuery, parse_query};

fn args(query: &SearchQuery) -> Vec<String> {
    query
        .git_log_args(1000)
        .into_iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn plain_text_has_no_filters() {
    let query = parse_query("fix login");
    assert!(!query.is_filtered());
    assert_eq!(query.text, "fix login");
}

#[test]
fn filters_are_separated_from_free_text() {
    let query = parse_query("author:alice fix msg:login");
    assert!(query.is_filtered());
    assert_eq!(query.authors, ["alice"]);
    assert_eq!(query.messages, ["login"]);
    assert_eq!(query.text, "fix");
}

#[test]
fn quoted_values_keep_spaces() {
    let query = parse_query("author:\"Alice Smith\" path:\"docs/my notes.md\"");
    assert_eq!(query.authors, ["Alice Smith"]);
    assert_eq!(query.paths, ["docs/my notes.md"]);
    assert_eq!(query.text, "");
}

#[test]
fn unterminated_quote_takes_the_rest() {
    assert_eq!(parse_query("author:\"Alice Sm").authors, ["Alice Sm"]);
}

#[test]
fn repeated_filters_collect_every_value() {
    let query = parse_query("author:alice author:bob ref:main ref:dev");
    assert_eq!(query.authors, ["alice", "bob"]);
    assert_eq!(query.refs, ["main", "dev"]);
}

#[test]
fn unknown_keys_stay_free_text() {
    let query = parse_query("http://example.com fix");
    assert!(!query.is_filtered());
    assert!(!query.names_a_filter);
    assert_eq!(query.text, "http://example.com fix");
}

#[test]
fn a_filter_key_without_a_value_is_left_out_until_typed() {
    let query = parse_query("path: fix");
    assert!(!query.is_filtered());
    assert!(query.names_a_filter);
    assert_eq!(query.text, "fix");
}

#[test]
fn filter_keys_ignore_case() {
    assert_eq!(parse_query("Author:alice").authors, ["alice"]);
}

#[test]
fn git_args_search_all_refs_by_default() {
    let query = parse_query(
        "author:alice author:bob msg:fix since:\"2 weeks ago\" until:2026-10-01 path:src",
    );
    assert_eq!(
        args(&query),
        [
            "--literal-pathspecs",
            "log",
            "--format=* %h %d %s - %an%x1f%H",
            "--decorate=short",
            "--color=never",
            "--regexp-ignore-case",
            "--fixed-strings",
            "-n",
            "1000",
            "--author=alice",
            "--author=bob",
            "--grep=fix",
            "--since=2 weeks ago",
            "--until=2026-10-01",
            "--all",
            "--",
            "src",
        ]
    );
}

#[test]
fn git_args_use_refs_instead_of_all() {
    let query = parse_query("ref:main ref:dev");
    let args = args(&query);
    assert!(!args.contains(&"--all".to_owned()));
    assert_eq!(&args[args.len() - 3..], ["main", "dev", "--"]);
}

#[test]
fn a_ref_that_looks_like_an_option_is_not_passed_as_one() {
    let args = args(&parse_query("ref:--output=x"));
    assert!(args.contains(&"--end-of-options".to_owned()));
}
