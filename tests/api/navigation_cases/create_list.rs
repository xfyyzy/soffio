use super::*;

// ============ Navigation ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_create_and_list_navigation(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let nav_payload = NavigationCreateRequest {
        label: "Home".into(),
        destination_type: soffio::domain::types::NavigationDestinationType::External,
        destination_page_id: None,
        destination_url: Some("https://example.com".into()),
        sort_order: 1,
        visible: true,
        open_in_new_tab: false,
    };

    let (status, created_nav) = response_json(
        handlers::create_navigation(
            State(state.clone()),
            Extension(principal.clone()),
            Json(nav_payload),
        )
        .await
        .expect("create navigation via handler"),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED);
    let created_id = string_field(&created_nav, "id").to_string();

    let (status, fetched) = response_json(
        handlers::get_navigation_item(
            State(state.clone()),
            Extension(principal.clone()),
            Path(created_id.parse().unwrap()),
        )
        .await
        .expect("get navigation by id"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(string_field(&fetched, "id"), created_id.as_str());

    let _list = handlers::list_navigation(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::NavigationListQuery {
            search: None,
            cursor: None,
            limit: Some(10),
            visible: None,
        }),
    )
    .await
    .expect("list navigation via handler");
}
