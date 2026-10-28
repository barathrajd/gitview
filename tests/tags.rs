use gitview::tags;
use std::process::Command;
fn repo() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    for args in [
        vec!["init", "-q"],
        vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-qm",
            "base",
        ],
    ] {
        assert!(
            Command::new("git")
                .current_dir(root.path())
                .args(args)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    root
}
#[test]
fn creates_lists_and_deletes_a_lightweight_tag_without_overwriting() {
    let root = repo();
    let preview = tags::preview_create(root.path(), "v-test").unwrap();
    tags::create(root.path(), &preview).unwrap();
    assert!(tags::create(root.path(), &preview).is_err());
    let list = tags::list(root.path()).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "v-test");
    assert_eq!(list[0].kind, "commit");
    let result = tags::verify(root.path(), &list[0]).unwrap();
    assert!(!result.verified);
    tags::delete(root.path(), &list[0]).unwrap();
    assert!(tags::list(root.path()).unwrap().is_empty());
}
#[test]
fn rejects_invalid_names_and_stale_tag_deletion() {
    let root = repo();
    assert!(tags::preview_create(root.path(), "bad name").is_err());
    let preview = tags::preview_create(root.path(), "v-test").unwrap();
    tags::create(root.path(), &preview).unwrap();
    let old = tags::list(root.path()).unwrap().remove(0);
    let future = tags::preview_create(root.path(), "v-future").unwrap();
    assert!(
        Command::new("git")
            .current_dir(root.path())
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--allow-empty",
                "-qm",
                "next"
            ])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(tags::create(root.path(), &preview).is_err());
    assert!(
        tags::create(root.path(), &future)
            .unwrap_err()
            .contains("HEAD changed")
    );
    assert!(
        Command::new("git")
            .current_dir(root.path())
            .args(["tag", "-f", "v-test"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(tags::delete(root.path(), &old).is_err());
    assert!(tags::verify(root.path(), &old).is_err());
    assert_eq!(tags::list(root.path()).unwrap().len(), 1);
}

#[test]
fn lists_existing_annotated_tags_and_reports_unsigned_verification() {
    let root = repo();
    assert!(
        Command::new("git")
            .current_dir(root.path())
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "-c",
                "tag.gpgSign=false",
                "tag",
                "--no-sign",
                "-a",
                "annotated",
                "-m",
                "Existing annotation"
            ])
            .output()
            .unwrap()
            .status
            .success()
    );
    let tag = tags::list(root.path()).unwrap().remove(0);
    assert_eq!(tag.kind, "tag");
    let result = tags::verify(root.path(), &tag).unwrap();
    assert!(!result.verified);
    assert!(!result.output.is_empty());
    tags::delete(root.path(), &tag).unwrap();
    assert!(tags::list(root.path()).unwrap().is_empty());
}
