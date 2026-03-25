use super::*;

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
