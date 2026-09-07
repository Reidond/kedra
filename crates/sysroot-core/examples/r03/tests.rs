use super::*;

fn prepared() -> Result<Fixture> {
    let mut f = Fixture::new(BASE, SOURCE_PATH)?;
    f.app_write(LIVE)?;
    f.capture()?;
    f.ignore(Change::new(BASE, "border=2\n", "border=4\n"))?;
    f.stage(&[Change::new(BASE, "font=12\n", "font=14\n")])?;
    Ok(f)
}

fn upstream(f: &Fixture, text: &str) -> Result<()> {
    write(&f.source.root, &f.source_path, text)?;
    f.source.stage(&f.source_path, text)?;
    f.source.run(
        &["commit", "--quiet", "-m", "upstream fixture edit"],
        None,
        None,
    )?;
    Ok(())
}

#[test]
fn partial_line_staging_survives_later_application_writes() -> Result<()> {
    let f = prepared()?;
    let whole = f
        .review
        .run(&["diff", "HEAD", "--", SOURCE_PATH], None, None)?;
    assert_eq!(
        whole.lines().filter(|line| line.starts_with("@@")).count(),
        1
    );
    assert!(whole.contains("+font=14\n+border=4\n+animation=150"));
    let patch = f.patch()?;
    assert!(patch.contains("+font=14"));
    assert!(!patch.contains("+border=4"));
    assert!(!patch.contains("+animation=150"));
    let tree = f.review.oid(&["write-tree"])?;
    f.app_write("font=16\nborder=4\nanimation=150\n")?;
    f.capture()?;
    assert_eq!(f.review.oid(&["write-tree"])?, tree);
    assert_eq!(f.patch()?, patch);
    let unstaged = f.review.run(&["diff", "--", SOURCE_PATH], None, None)?;
    assert!(unstaged.contains("-font=14\n"));
    assert!(unstaged.contains("+font=16\n"));
    assert!(unstaged.contains("+animation=150"));
    Ok(())
}

#[test]
fn exact_export_no_private_objects_or_ancestry_and_pending_deployment() -> Result<()> {
    let mut f = prepared()?;
    let before = f.live_text()?;
    // An unknown synthetic credential is excluded BEFORE private Git capture.
    write(&f.live, "auth.json", "SYNTHETIC_EXCLUDED_CREDENTIAL")?;
    f.capture()?;
    // Retain a real private snapshot commit containing BOTH local dispositions.
    // Its temporary index leaves S unchanged; export must not import its objects.
    let private_index = f.root.join("private-capture.index");
    f.review.run(
        &["read-tree", &f.baseline_commit],
        None,
        Some(&private_index),
    )?;
    let live_blob = f.review.blob(&before)?;
    f.review.run(
        &[
            "update-index",
            "--cacheinfo",
            "100644",
            &live_blob,
            SOURCE_PATH,
        ],
        None,
        Some(&private_index),
    )?;
    let private_tree = f.review.run(&["write-tree"], None, Some(&private_index))?;
    let private_commit = f.review.run(
        &["commit-tree", private_tree.trim(), "-p", &f.baseline_commit],
        Some(b"private synthetic snapshot\n"),
        None,
    )?;
    f.review.run(
        &[
            "update-ref",
            "refs/heads/private-capture",
            private_commit.trim(),
        ],
        None,
        None,
    )?;
    let review_objects =
        f.review
            .output(&["cat-file", "--batch-all-objects", "--batch"], None, None)?;
    assert!(review_objects.status.success());
    let review_objects = String::from_utf8_lossy(&review_objects.stdout);
    assert!(!review_objects.contains("SYNTHETIC_EXCLUDED_CREDENTIAL"));
    assert!(review_objects.contains("border=4"));
    assert!(review_objects.contains("animation=150"));
    let export = f.export()?;
    f.publish(export)?;
    assert_eq!(
        fs::read_to_string(f.source.root.join(SOURCE_PATH))?,
        "font=14\nborder=2\nanimation=200\n"
    );
    let objects = f
        .source
        .output(&["cat-file", "--batch-all-objects", "--batch"], None, None)?;
    assert!(objects.status.success());
    let objects = String::from_utf8_lossy(&objects.stdout);
    assert!(!objects.contains("border=4"));
    assert!(!objects.contains("animation=150"));
    assert!(!objects.contains("SYNTHETIC_EXCLUDED_CREDENTIAL"));
    assert!(
        !f.source
            .output(&["cat-file", "-e", &f.baseline_commit], None, None)?
            .status
            .success()
    );
    assert!(
        !f.source
            .output(&["cat-file", "-e", private_commit.trim()], None, None)?
            .status
            .success()
    );
    assert_eq!(f.source.oid(&["rev-parse", "HEAD^"])?, f.source_base);
    assert!(!f.source.root.join("home/.config/demo.conf").exists());
    assert_eq!(f.live_text()?, before);
    assert_eq!(f.baseline, BASE);
    let published = f.published.as_ref().ok_or(Error::MissingSelection)?;
    assert_eq!(
        published.selection,
        f.selected.as_ref().ok_or(Error::MissingSelection)?.tree
    );
    assert_eq!(published.commit, f.source.oid(&["rev-parse", "HEAD"])?);
    Ok(())
}

#[test]
fn selection_and_policy_overlap_fail_in_both_orders() -> Result<()> {
    let mut f = prepared()?;
    let before = f.review.oid(&["write-tree"])?;
    assert!(matches!(
        f.stage(&[Change::new(BASE, "border=2\n", "border=4\n")]),
        Err(Error::Classification(_))
    ));
    assert!(matches!(
        f.ignore(Change::new(BASE, "font=12\n", "font=14\n")),
        Err(Error::Classification(_))
    ));
    assert_eq!(f.review.oid(&["write-tree"])?, before);
    // Export also checks the policy independently of the stage UI.
    f.ignored.push(Change::new(BASE, "font=12\n", "font=14\n"));
    assert!(matches!(f.export(), Err(Error::Classification(_))));
    Ok(())
}

#[test]
fn ignored_value_changes_again_becomes_explicit_review() -> Result<()> {
    let mut f = prepared()?;
    f.app_write("font=14\nborder=5\nanimation=150\n")?;
    f.capture()?;
    assert!(matches!(f.export(), Err(Error::Classification(_))));
    assert!(
        f.review
            .run(&["diff", "--", SOURCE_PATH], None, None)?
            .contains("+border=5")
    );
    // Explicitly clearing the stale decision permits only the original S export.
    f.ignored.clear();
    let export = f.export()?;
    assert_eq!(
        f.source.run(
            &["show", &format!("{}:{SOURCE_PATH}", export.tree)],
            None,
            None
        )?,
        "font=14\nborder=2\nanimation=200\n"
    );
    Ok(())
}

#[test]
fn duplicate_reordered_inserted_deleted_and_changed_baseline_anchors_refuse() {
    let c = Change::new(BASE, "border=2\n", "border=4\n");
    for live in [
        "border=4\nfont=14\nanimation=150\n",
        "font=14\nborder=4\nborder=4\n",
        "font=14\nanimation=150\n",
        "extra=1\nfont=14\nborder=4\nanimation=150\n",
    ] {
        assert!(matches!(
            c.validate(BASE, live),
            Err(Error::Classification(_))
        ));
    }
    assert!(matches!(
        c.validate("font=12\nborder=3\nanimation=200\n", LIVE),
        Err(Error::Classification(_))
    ));
    let duplicate = "border=2\nborder=2\n";
    assert!(
        Change::new(duplicate, "border=2\n", "border=4\n")
            .validate(duplicate, "border=4\nborder=2\n")
            .is_err()
    );
}

#[test]
fn source_same_line_conflict_preserves_head_index_worktree_live_and_staging() -> Result<()> {
    let mut f = prepared()?;
    upstream(&f, "font=18\nborder=2\nanimation=200\n")?;
    let head = f.source.oid(&["rev-parse", "HEAD"])?;
    let index = fs::read(f.source.root.join(".git/index"))?;
    let live = f.live_text()?;
    let selected = f.review.oid(&["write-tree"])?;
    assert!(matches!(f.export(), Err(Error::ContentConflict)));
    assert_eq!(f.source.oid(&["rev-parse", "HEAD"])?, head);
    assert_eq!(fs::read(f.source.root.join(".git/index"))?, index);
    assert_eq!(
        fs::read_to_string(f.source.root.join(SOURCE_PATH))?,
        "font=18\nborder=2\nanimation=200\n"
    );
    assert_eq!(f.live_text()?, live);
    assert_eq!(f.review.oid(&["write-tree"])?, selected);
    assert!(f.published.is_none());
    // User resolves by explicitly reviewing the upstream value for publication.
    // This prototype does not silently pick ours/theirs or apply to live.
    Ok(())
}

#[test]
fn source_advance_nonoverlap_and_adopted_value_and_repeat_export() -> Result<()> {
    let baseline = "font=12\nkeep1=1\nkeep2=2\nkeep3=3\nkeep4=4\nborder=2\n";
    let mut f = Fixture::new(baseline, SOURCE_PATH)?;
    f.app_write(&baseline.replace("font=12", "font=14"))?;
    f.stage(&[Change::new(baseline, "font=12\n", "font=14\n")])?;
    upstream(&f, &baseline.replace("border=2", "border=3"))?;
    let export = f.export()?;
    f.publish(export)?;
    assert_eq!(
        fs::read_to_string(f.source.root.join(SOURCE_PATH))?,
        baseline
            .replace("font=12", "font=14")
            .replace("border=2", "border=3")
    );
    let first = f.source.oid(&["rev-parse", "HEAD"])?;
    let export = f.export()?;
    f.publish(export)?;
    assert_eq!(f.source.oid(&["rev-parse", "HEAD"])?, first);
    // Independent upstream adoption also produces no extra commit.
    let mut f = prepared()?;
    upstream(&f, "font=14\nborder=2\nanimation=200\n")?;
    let first = f.source.oid(&["rev-parse", "HEAD"])?;
    let export = f.export()?;
    f.publish(export)?;
    assert_eq!(f.source.oid(&["rev-parse", "HEAD"])?, first);
    Ok(())
}

#[test]
fn dirty_source_and_post_prepare_advance_refuse() -> Result<()> {
    let mut f = prepared()?;
    write(&f.source.root, "untracked.txt", "user work")?;
    assert!(matches!(f.export(), Err(Error::DirtySource)));
    assert_eq!(
        fs::read_to_string(f.source.root.join("untracked.txt"))?,
        "user work"
    );
    let mut f = prepared()?;
    let export = f.export()?;
    upstream(&f, "font=12\nborder=3\nanimation=200\n")?;
    assert!(matches!(f.publish(export), Err(Error::Classification(_))));
    assert_eq!(
        fs::read_to_string(f.source.root.join(SOURCE_PATH))?,
        "font=12\nborder=3\nanimation=200\n"
    );
    Ok(())
}

#[test]
fn baseline_conflict_never_writes_live_markers_or_advances_baseline() -> Result<()> {
    let f = prepared()?;
    let before = f.live_text()?;
    assert!(matches!(
        f.merge_candidate("font=12\nborder=3\nanimation=250\n"),
        Err(Error::ContentConflict)
    ));
    assert_eq!(f.live_text()?, before);
    assert_eq!(f.baseline, BASE);
    assert!(fs::read_to_string(f.root.join("merge-candidate.txt"))?.contains("<<<<<<< live"));
    assert_eq!(f.merge_candidate(BASE)?, before);
    Ok(())
}

#[test]
fn non_ascii_path_and_no_trailing_newline_round_trip() -> Result<()> {
    let baseline = "font=12\nborder=2";
    let path = "hosts/desktop/home/.config/тема.conf";
    let mut f = Fixture::new(baseline, path)?;
    f.app_write("font=12\nborder=4")?;
    f.stage(&[Change::new(baseline, "border=2", "border=4")])?;
    let export = f.export()?;
    f.publish(export)?;
    assert_eq!(fs::read(f.source.root.join(path))?, b"font=12\nborder=4");
    Ok(())
}

#[test]
fn unsupported_encoding_path_and_rename_refuse() -> Result<()> {
    for text in ["font=12\r\n", "font=12\0", "\u{feff}font=12"] {
        assert!(validate_text(text).is_err());
    }
    for path in [
        "../home",
        "/home",
        "C:/home",
        "a\\b",
        "a/.git/config",
        "a/./b",
        "a//b",
        "",
    ] {
        assert!(validate_path(path).is_err());
    }
    let f = prepared()?;
    fs::write(f.live.join(LIVE_PATH), [0xff, 0xfe])?;
    assert!(matches!(f.capture(), Err(Error::Io(_))));
    let f = prepared()?;
    fs::rename(f.live.join(LIVE_PATH), f.live.join(".config/renamed.conf"))?;
    assert!(matches!(f.capture(), Err(Error::Io(_))));
    Ok(())
}

#[test]
fn selected_snapshot_no_loss_no_leak_across_generated_values() -> Result<()> {
    for value in 14..18 {
        let mut f = Fixture::new(BASE, SOURCE_PATH)?;
        let selected_line = format!("font={value}\n");
        f.app_write(&format!("{selected_line}border=4\nanimation=150\n"))?;
        f.ignore(Change::new(BASE, "border=2\n", "border=4\n"))?;
        f.stage(&[Change::new(BASE, "font=12\n", &selected_line)])?;
        let later = format!("font={}\nborder=4\nanimation=151\n", value + 10);
        f.app_write(&later)?;
        f.capture()?;
        let export = f.export()?;
        f.publish(export)?;
        assert_eq!(
            fs::read_to_string(f.source.root.join(SOURCE_PATH))?,
            format!("{selected_line}border=2\nanimation=200\n")
        );
        assert_eq!(f.live_text()?, later);
    }
    Ok(())
}
