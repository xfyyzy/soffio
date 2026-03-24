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
