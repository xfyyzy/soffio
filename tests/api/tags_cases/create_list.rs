use super::*;

// ============ Tags ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_create_and_list_tags(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let tag_payload = TagCreateRequest {
        name: "test-tag".into(),
        description: Some("A test tag".into()),
        pinned: false,
    };

    let (status, created_tag) = response_json(
        handlers::create_tag(
            State(state.clone()),
            Extension(principal.clone()),
            Json(tag_payload),
        )
        .await
        .expect("create tag via handler"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let created_tag_id = string_field(&created_tag, "id").to_string();
    let created_tag_slug = string_field(&created_tag, "slug").to_string();

    let (status, found_by_id) = response_json(
        handlers::get_tag_by_id(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_tag_id.parse().unwrap()),
        )
        .await
        .expect("get tag by id"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(string_field(&found_by_id, "id"), created_tag_id.as_str());

    let (status, found_by_slug) = response_json(
        handlers::get_tag_by_slug(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_tag_slug.clone()),
        )
        .await
        .expect("get tag by slug"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        string_field(&found_by_slug, "slug"),
        created_tag_slug.as_str()
    );

    let _list = handlers::list_tags(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::TagListQuery {
            search: None,
            month: None,
            cursor: None,
            limit: Some(10),
            pinned: None,
        }),
    )
    .await
    .expect("list tags via handler");
}
