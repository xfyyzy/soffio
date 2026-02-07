use std::fs;

use tempfile::tempdir;

#[path = "../build_support/incremental.rs"]
mod incremental;

#[test]
fn fingerprint_changes_only_when_relevant_inputs_change() {
    let temp = tempdir().expect("temp dir should be created");
    let tracked_dir = temp.path().join("tracked");
    let nested_dir = tracked_dir.join("nested");
    let ignored_file = temp.path().join("ignored.txt");

    fs::create_dir_all(&nested_dir).expect("tracked directories should be created");
    fs::write(tracked_dir.join("a.txt"), "alpha").expect("tracked file should be written");
    fs::write(nested_dir.join("b.txt"), "beta").expect("nested tracked file should be written");
    fs::write(&ignored_file, "ignored-v1").expect("ignored file should be written");

    let before = incremental::fingerprint_inputs(&[tracked_dir.as_path()], "salt")
        .expect("fingerprint should be computed");

    fs::write(&ignored_file, "ignored-v2").expect("ignored file should be updated");
    let after_ignored = incremental::fingerprint_inputs(&[tracked_dir.as_path()], "salt")
        .expect("fingerprint should still be computed");
    assert_eq!(before, after_ignored);

    fs::write(tracked_dir.join("a.txt"), "alpha-updated").expect("tracked file should be updated");
    let after_tracked = incremental::fingerprint_inputs(&[tracked_dir.as_path()], "salt")
        .expect("fingerprint should still be computed");
    assert_ne!(before, after_tracked);
}

#[test]
fn stamp_roundtrip_requires_existing_output_dir() {
    let temp = tempdir().expect("temp dir should be created");
    let output_dir = temp.path().join("output");
    let stamp_path = temp.path().join("stamp.txt");

    fs::create_dir_all(&output_dir).expect("output directory should be created");

    let fingerprint = 0xA11CE_u64;
    assert!(
        !incremental::stamp_matches(&stamp_path, &output_dir, fingerprint)
            .expect("stamp check should succeed")
    );

    incremental::write_stamp(&stamp_path, fingerprint).expect("stamp should be written");
    assert!(
        incremental::stamp_matches(&stamp_path, &output_dir, fingerprint)
            .expect("stamp check should succeed")
    );

    fs::remove_dir_all(&output_dir).expect("output directory should be removed");
    assert!(
        !incremental::stamp_matches(&stamp_path, &output_dir, fingerprint)
            .expect("stamp check should succeed")
    );
}
