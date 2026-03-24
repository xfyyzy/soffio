use super::*;

// ============ API Keys ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_get_api_key_info(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let Json(info) = handlers::get_api_key_info(State(state.clone()), Extension(principal))
        .await
        .expect("get api key info");

    assert_eq!(info.prefix.len(), 12);
    assert!(info.scopes.contains(&ApiScope::PostRead));
    assert_eq!(info.status, soffio::domain::api_keys::ApiKeyStatus::Active);
}

// ============ API Key Scope Granularity ============

#[sqlx::test(migrations = "./migrations")]
async fn api_scope_granularity_post_vs_page(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key with only PostRead scope
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "post-only".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    let principal = state.api_keys.authenticate(&issued.token).await.unwrap();

    // Should be able to list posts
    let _posts = handlers::list_posts(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::PostListQuery {
            status: None,
            search: None,
            tag: None,
            month: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("should be able to list posts with PostRead scope");

    // Should NOT be able to list pages (requires PageRead)
    assert!(
        principal.requires(ApiScope::PageRead).is_err(),
        "PostRead scope should not grant PageRead access"
    );
}

// ============ API Key Authentication Status ============

#[sqlx::test(migrations = "./migrations")]
async fn api_auth_rejects_revoked_key(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "revoke-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    // Revoke the key
    state
        .api_keys
        .revoke(issued.record.id)
        .await
        .expect("revoke should succeed");

    // Authentication should fail with Revoked error
    let result = state.api_keys.authenticate(&issued.token).await;
    assert!(
        result.is_err(),
        "authentication should fail for revoked key"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, soffio::application::api_keys::ApiAuthError::Revoked),
        "should get Revoked error, got: {:?}",
        err
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn api_auth_rejects_expired_key(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key that expires immediately (expires_in = 0 means expires_at = now)
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "expired-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: Some(time::Duration::ZERO),
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    // Small delay to ensure we're past the expires_at time
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Authentication should fail with Expired error
    let result = state.api_keys.authenticate(&issued.token).await;
    assert!(
        result.is_err(),
        "authentication should fail for expired key"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, soffio::application::api_keys::ApiAuthError::Expired),
        "should get Expired error, got: {:?}",
        err
    );
}

// ============ API Key Rotation ============

#[sqlx::test(migrations = "./migrations")]
async fn api_rotate_reactivates_revoked_key(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "rotate-revoke-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    // Revoke the key
    state
        .api_keys
        .revoke(issued.record.id)
        .await
        .expect("revoke should succeed");

    // Rotation should succeed and reactivate the key
    let rotated = state
        .api_keys
        .rotate(issued.record.id)
        .await
        .expect("rotation should succeed for revoked key");

    // The key should now be active
    assert_eq!(
        rotated.record.status,
        soffio::domain::api_keys::ApiKeyStatus::Active,
        "key should be reactivated after rotation"
    );

    // The new token should work for authentication
    let auth_result = state.api_keys.authenticate(&rotated.token).await;
    assert!(
        auth_result.is_ok(),
        "authentication should succeed with rotated token"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn api_rotate_recalculates_expiration_preserves_created_at(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key with 30-day expiration duration
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "rotate-preserve-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: Some(time::Duration::days(30)),
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    let original_created_at = issued.record.created_at;
    let original_expires_in = issued.record.expires_in;
    let original_expires_at = issued.record.expires_at;

    // Small delay to ensure recalculated expires_at is different
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Rotate the key
    let rotated = state
        .api_keys
        .rotate(issued.record.id)
        .await
        .expect("rotation should succeed");

    // created_at and expires_in should be preserved
    assert_eq!(
        rotated.record.created_at, original_created_at,
        "created_at should be preserved after rotation"
    );
    assert_eq!(
        rotated.record.expires_in, original_expires_in,
        "expires_in duration should be preserved after rotation"
    );

    // expires_at should be recalculated (should be later than original)
    assert!(
        rotated.record.expires_at > original_expires_at,
        "expires_at should be recalculated to a later time after rotation"
    );

    // The token should be different
    assert_ne!(
        issued.token, rotated.token,
        "token should change after rotation"
    );

    // Old token should no longer work
    let old_auth = state.api_keys.authenticate(&issued.token).await;
    assert!(old_auth.is_err(), "old token should not authenticate");

    // New token should work
    let new_auth = state.api_keys.authenticate(&rotated.token).await;
    assert!(new_auth.is_ok(), "new token should authenticate");
}

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
