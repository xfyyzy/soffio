use super::*;

// ============ Posts ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_create_and_list_posts(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();
    let post_payload = PostCreateRequest {
        title: "handler-post".into(),
        excerpt: "excerpt".into(),
        body_markdown: "# body".into(),
        summary_markdown: None,
        status: soffio::domain::types::PostStatus::Draft,
        pinned: false,
        scheduled_at: None,
        published_at: None,
        archived_at: None,
    };

    let (status, created_post) = response_json(
        handlers::create_post(
            State(state.clone()),
            Extension(principal.clone()),
            Json(post_payload),
        )
        .await
        .expect("create post via handler"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let created_post_id = string_field(&created_post, "id").to_string();
    let created_post_slug = string_field(&created_post, "slug").to_string();

    let (status, found_by_id) = response_json(
        handlers::get_post_by_id(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_post_id.parse().unwrap()),
        )
        .await
        .expect("get post by id"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(string_field(&found_by_id, "id"), created_post_id.as_str());

    let (status, found_by_slug) = response_json(
        handlers::get_post(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_post_slug.clone()),
        )
        .await
        .expect("get post by slug"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        string_field(&found_by_slug, "slug"),
        created_post_slug.as_str()
    );

    let _list = handlers::list_posts(
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
    .expect("list posts via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_post_content(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a post via service to get the ID
    let post = state
        .posts
        .create_post(
            "test",
            soffio::application::admin::posts::CreatePostCommand {
                title: "original-title".into(),
                excerpt: "original".into(),
                body_markdown: "# original".into(),
                summary_markdown: None,
                status: soffio::domain::types::PostStatus::Draft,
                pinned: false,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create post via service");

    // Update the post via handler
    let update_payload = PostUpdateRequest {
        slug: post.slug.clone(),
        title: "updated-title".into(),
        excerpt: "updated".into(),
        body_markdown: "# updated".into(),
        summary_markdown: None,
        pinned: true,
    };

    let _updated = handlers::update_post(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(update_payload),
    )
    .await
    .expect("update post via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_post_status(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create a post via service to get the ID
    let post = state
        .posts
        .create_post(
            "test",
            soffio::application::admin::posts::CreatePostCommand {
                title: "status-test".into(),
                excerpt: "excerpt".into(),
                body_markdown: "# body".into(),
                summary_markdown: None,
                status: soffio::domain::types::PostStatus::Draft,
                pinned: false,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create post via service");

    // Update status to published via handler
    let status_payload = PostStatusRequest {
        status: soffio::domain::types::PostStatus::Published,
        scheduled_at: None,
        published_at: Some(OffsetDateTime::now_utc()),
        archived_at: None,
    };

    let _updated = handlers::update_post_status(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(status_payload),
    )
    .await
    .expect("update post status via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_partial_update_post(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let post = state
        .posts
        .create_post(
            "test",
            soffio::application::admin::posts::CreatePostCommand {
                title: "partial".into(),
                excerpt: "orig".into(),
                body_markdown: "# body".into(),
                summary_markdown: Some("sum".into()),
                status: soffio::domain::types::PostStatus::Draft,
                pinned: false,
                scheduled_at: None,
                published_at: None,
                archived_at: None,
            },
        )
        .await
        .expect("create post via service");

    handlers::update_post_pin(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(PostPinRequest { pinned: true }),
    )
    .await
    .expect("pin post");
    let mut latest = state.posts.load_post(post.id).await.unwrap().unwrap();
    assert!(latest.pinned);

    handlers::update_post_title(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(PostTitleRequest {
            title: "new title".into(),
        }),
    )
    .await
    .expect("update title");
    latest = state.posts.load_post(post.id).await.unwrap().unwrap();
    assert_eq!(latest.title, "new title");

    handlers::update_post_excerpt(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(PostExcerptRequest {
            excerpt: "new excerpt".into(),
        }),
    )
    .await
    .expect("update excerpt");
    latest = state.posts.load_post(post.id).await.unwrap().unwrap();
    assert_eq!(latest.excerpt, "new excerpt");

    handlers::update_post_body(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(post.id),
        Json(PostBodyRequest {
            body_markdown: "## changed".into(),
        }),
    )
    .await
    .expect("update body");
    latest = state.posts.load_post(post.id).await.unwrap().unwrap();
    assert_eq!(latest.body_markdown, "## changed");

    handlers::update_post_summary(
        State(state.clone()),
        Extension(principal),
        axum::extract::Path(post.id),
        Json(PostSummaryRequest {
            summary_markdown: Some("updated summary".into()),
        }),
    )
    .await
    .expect("update summary");
    latest = state.posts.load_post(post.id).await.unwrap().unwrap();
    assert_eq!(latest.summary_markdown.as_deref(), Some("updated summary"));
}
