use super::*;

// ============ Pages ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_create_and_list_pages(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let page_payload = PageCreateRequest {
        slug: None,
        title: "test-page".into(),
        body_markdown: "# Page content".into(),
        status: soffio::domain::types::PageStatus::Draft,
        scheduled_at: None,
        published_at: None,
        archived_at: None,
    };

    let (status, created_page) = response_json(
        handlers::create_page(
            State(state.clone()),
            Extension(principal.clone()),
            Json(page_payload),
        )
        .await
        .expect("create page via handler"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let created_page_id = string_field(&created_page, "id").to_string();
    let created_page_slug = string_field(&created_page, "slug").to_string();

    let (status, found_by_id) = response_json(
        handlers::get_page_by_id(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_page_id.parse().unwrap()),
        )
        .await
        .expect("get page by id"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(string_field(&found_by_id, "id"), created_page_id.as_str());

    let (status, found_by_slug) = response_json(
        handlers::get_page(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_page_slug.clone()),
        )
        .await
        .expect("get page by slug"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        string_field(&found_by_slug, "slug"),
        created_page_slug.as_str()
    );

    let _list = handlers::list_pages(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::PageListQuery {
            status: None,
            search: None,
            month: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("list pages via handler");
}
