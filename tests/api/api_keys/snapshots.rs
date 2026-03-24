use super::*;

// ============ API Key Snapshots ============

#[sqlx::test(migrations = "./migrations")]
async fn api_snapshots_cover_create_get_list_and_rollback(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create post
    let (status, post_json) = response_json(
        handlers::create_post(
            State(state.clone()),
            Extension(principal.clone()),
            Json(PostCreateRequest {
                title: "snap-post".into(),
                excerpt: "excerpt".into(),
                body_markdown: "# body".into(),
                summary_markdown: None,
                status: soffio::domain::types::PostStatus::Draft,
                pinned: false,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            }),
        )
        .await
        .expect("create post"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let post_id = uuid_field(&post_json, "id");
    let post_slug = string_field(&post_json, "slug").to_string();

    // Tag to ensure tag list restored
    let (_, tag_json) = response_json(
        handlers::create_tag(
            State(state.clone()),
            Extension(principal.clone()),
            Json(TagCreateRequest {
                name: "snap-tag".into(),
                description: None,
                pinned: false,
            }),
        )
        .await
        .expect("create tag"),
    )
    .await;
    let tag_id = uuid_field(&tag_json, "id");
    let status = handlers::replace_post_tags(
        State(state.clone()),
        Extension(principal.clone()),
        Path(post_id),
        Json(PostTagsRequest {
            tag_ids: vec![tag_id],
        }),
    )
    .await
    .expect("attach tag")
    .into_response()
    .status();
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Snapshot (version 1)
    let (status, snap_json) = response_json(
        handlers::create_snapshot(
            State(state.clone()),
            Extension(principal.clone()),
            Json(SnapshotCreateRequest {
                entity_type: soffio::domain::types::SnapshotEntityType::Post,
                entity_id: post_id,
                description: Some("v1".into()),
            }),
        )
        .await
        .expect("create snapshot"),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let snapshot_id = uuid_field(&snap_json, "id");

    // Mutate post & tags
    let (status, _) = response_json(
        handlers::update_post(
            State(state.clone()),
            Extension(principal.clone()),
            Path(post_id),
            Json(PostUpdateRequest {
                slug: post_slug.clone(),
                title: "changed".into(),
                excerpt: "changed excerpt".into(),
                body_markdown: "changed body".into(),
                summary_markdown: None,
                pinned: false,
            }),
        )
        .await
        .expect("update post"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let status = handlers::replace_post_tags(
        State(state.clone()),
        Extension(principal.clone()),
        Path(post_id),
        Json(PostTagsRequest { tag_ids: vec![] }),
    )
    .await
    .expect("clear tags")
    .into_response()
    .status();
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Rollback
    let (status, _) = response_json(
        handlers::rollback_snapshot(
            State(state.clone()),
            Extension(principal.clone()),
            Path(snapshot_id),
        )
        .await
        .expect("rollback snapshot"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Verify post restored
    let restored = state
        .posts
        .load_post(post_id)
        .await
        .expect("find post")
        .expect("post exists");
    assert_eq!(restored.title, "snap-post");
    assert_eq!(restored.excerpt, "excerpt");

    let restored_tags: Vec<Uuid> = sqlx::query_scalar::<_, Uuid>(
        "SELECT tag_id FROM post_tags WHERE post_id = $1 ORDER BY tag_id",
    )
    .bind(post_id)
    .fetch_all(state.db.pool())
    .await
    .expect("post tags");
    assert_eq!(restored_tags, vec![tag_id]);

    // List & get
    let (status, list_json) = response_json(
        handlers::list_snapshots(
            State(state.clone()),
            Query(SnapshotListQuery {
                entity_type: Some(soffio::domain::types::SnapshotEntityType::Post),
                entity_id: Some(post_id),
                search: None,
                cursor: None,
                limit: Some(10),
            }),
            Extension(principal.clone()),
        )
        .await
        .expect("list snapshots"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        list_json
            .get("items")
            .and_then(|v| v.as_array())
            .map(Vec::len)
            .unwrap_or(0),
        1
    );

    let (status, get_json) = response_json(
        handlers::get_snapshot(
            State(state.clone()),
            Path(snapshot_id),
            Extension(principal),
        )
        .await
        .expect("get snapshot"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(uuid_field(&get_json, "id"), snapshot_id);
}
