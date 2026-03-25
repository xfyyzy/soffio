use super::*;

/// End-to-end coverage of `soffio-cli snapshots` commands against a running server.
#[tokio::test]
#[ignore]
async fn live_cli_snapshots_cover_all_flows() -> TestResult<()> {
    let cfg = load_config()?;
    let base = cfg.base_url.trim_end_matches('/').to_string();
    let all_key = &cfg.keys.all;
    let read_key = &cfg.keys.read;
    let suf = current_suffix();

    // Create a published post with summary.
    let post_title_v1 = format!("CLI Snap Post {suf}");
    let post_title_v2 = format!("CLI Snap Post Updated {suf}");
    let post_body_v1 = format!("# Post V1 {suf}\n\nBody marker V1-{suf}");
    let post_body_v2 = format!("# Post V2 {suf}\n\nBody marker V2-{suf}");
    let post_summary_v1 = format!("Summary V1 {suf}");
    let post_summary_v2 = format!("Summary V2 {suf}");

    let post_json = cli_json(
        &[
            "posts",
            "create",
            "--title",
            &post_title_v1,
            "--excerpt",
            "cli snap test",
            "--body",
            &post_body_v1,
            "--summary",
            &post_summary_v1,
            "--status",
            "published",
        ],
        &base,
        all_key,
    )
    .await?;
    let post_id = post_json
        .get("id")
        .and_then(Value::as_str)
        .ok_or("missing post id")?
        .to_string();
    let post_slug = post_json
        .get("slug")
        .and_then(Value::as_str)
        .ok_or("missing post slug")?
        .to_string();

    // Create a published page.
    let page_title_v1 = format!("CLI Snap Page {suf}");
    let page_title_v2 = format!("CLI Snap Page Updated {suf}");
    let page_body_v1 = format!("# Page V1 {suf}\n\nPage body V1");
    let page_body_v2 = format!("# Page V2 {suf}\n\nPage body V2");

    let page_json = cli_json(
        &[
            "pages",
            "create",
            "--title",
            &page_title_v1,
            "--body",
            &page_body_v1,
            "--status",
            "published",
        ],
        &base,
        all_key,
    )
    .await?;
    let page_id = page_json
        .get("id")
        .and_then(Value::as_str)
        .ok_or("missing page id")?
        .to_string();
    let page_slug = page_json
        .get("slug")
        .and_then(Value::as_str)
        .ok_or("missing page slug")?
        .to_string();

    // Create snapshots for post and page.
    let post_snap = cli_json(
        &[
            "snapshots",
            "create",
            "--entity-type",
            "post",
            "--entity-id",
            &post_id,
            "--description",
            "cli snap post v1",
        ],
        &base,
        all_key,
    )
    .await?;
    let post_snap_id = post_snap
        .get("id")
        .and_then(Value::as_str)
        .ok_or("missing post snapshot id")?
        .to_string();

    let page_snap = cli_json(
        &[
            "snapshots",
            "create",
            "--entity-type",
            "page",
            "--entity-id",
            &page_id,
            "--description",
            "cli snap page v1",
        ],
        &base,
        all_key,
    )
    .await?;
    let page_snap_id = page_snap
        .get("id")
        .and_then(Value::as_str)
        .ok_or("missing page snapshot id")?
        .to_string();

    // List snapshots filtered by entity_id.
    let list_post = cli_json(
        &[
            "snapshots",
            "list",
            "--entity-type",
            "post",
            "--entity-id",
            &post_id,
        ],
        &base,
        all_key,
    )
    .await?;
    let items = list_post
        .get("items")
        .and_then(Value::as_array)
        .ok_or("missing items in list")?;
    assert!(
        items
            .iter()
            .any(|v| v.get("id").and_then(Value::as_str) == Some(post_snap_id.as_str())),
        "post snapshot not present in list"
    );

    // Get snapshot detail.
    let snap_detail = cli_json(&["snapshots", "get", &post_snap_id], &base, all_key).await?;
    assert_eq!(
        snap_detail.get("entity_id").and_then(Value::as_str),
        Some(post_id.as_str())
    );
    assert_eq!(
        snap_detail.get("entity_type").and_then(Value::as_str),
        Some("post")
    );

    // Mutate post & page.
    cli_json(
        &[
            "posts",
            "update",
            "--id",
            &post_id,
            "--slug",
            &post_slug,
            "--title",
            &post_title_v2,
            "--excerpt",
            "cli snap test updated",
            "--body",
            &post_body_v2,
            "--summary",
            &post_summary_v2,
        ],
        &base,
        all_key,
    )
    .await?;

    cli_json(
        &[
            "pages",
            "update",
            "--id",
            &page_id,
            "--slug",
            &page_slug,
            "--title",
            &page_title_v2,
            "--body",
            &page_body_v2,
        ],
        &base,
        all_key,
    )
    .await?;

    // Rollback post snapshot and verify content restored.
    let rollback_msg = cli_plain(&["snapshots", "rollback", &post_snap_id], &base, all_key).await?;
    assert!(
        rollback_msg.contains("Rolled back snapshot"),
        "unexpected rollback message: {rollback_msg}"
    );

    let post_after = cli_json(&["posts", "get", "--id", &post_id], &base, all_key).await?;
    let body_after = post_after
        .get("body_markdown")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let summary_after = post_after
        .get("summary_markdown")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        body_after.contains(&post_body_v1),
        "post body not rolled back"
    );
    assert_eq!(summary_after, post_summary_v1);

    // Rollback page snapshot and verify.
    cli_plain(&["snapshots", "rollback", &page_snap_id], &base, all_key).await?;
    let page_after = cli_json(&["pages", "get", "--id", &page_id], &base, all_key).await?;
    let page_body_after = page_after
        .get("body_markdown")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        page_body_after.contains(&page_body_v1),
        "page body not rolled back"
    );

    // Permission check: create snapshot with read-only key should fail.
    cli_expect_fail(
        &[
            "snapshots",
            "create",
            "--entity-type",
            "post",
            "--entity-id",
            &post_id,
        ],
        &base,
        read_key,
    )
    .await?;

    // Cleanup test content.
    let _ = cli_plain(&["posts", "delete", &post_id], &base, all_key).await?;
    let _ = cli_plain(&["pages", "delete", &page_id], &base, all_key).await?;

    Ok(())
}
