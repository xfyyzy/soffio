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

#[sqlx::test(migrations = "./migrations")]
async fn api_can_update_navigation(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create navigation item via service to get the ID
    let nav = state
        .navigation
        .create_item(
            "test",
            soffio::application::admin::navigation::CreateNavigationItemCommand {
                label: "Original".into(),
                destination_type: soffio::domain::types::NavigationDestinationType::External,
                destination_page_id: None,
                destination_url: Some("https://original.com".into()),
                sort_order: 1,
                visible: true,
                open_in_new_tab: false,
            },
        )
        .await
        .expect("create navigation via service");

    // Update navigation via handler
    let update_payload = NavigationUpdateRequest {
        label: "Updated".into(),
        destination_type: soffio::domain::types::NavigationDestinationType::External,
        destination_page_id: None,
        destination_url: Some("https://updated.com".into()),
        sort_order: 2,
        visible: false,
        open_in_new_tab: true,
    };

    let _updated = handlers::update_navigation(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(nav.id),
        Json(update_payload),
    )
    .await
    .expect("update navigation via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_delete_navigation(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    // Create navigation item via service to get the ID
    let nav = state
        .navigation
        .create_item(
            "test",
            soffio::application::admin::navigation::CreateNavigationItemCommand {
                label: "Deletable".into(),
                destination_type: soffio::domain::types::NavigationDestinationType::External,
                destination_page_id: None,
                destination_url: Some("https://delete.me".into()),
                sort_order: 99,
                visible: true,
                open_in_new_tab: false,
            },
        )
        .await
        .expect("create navigation via service");

    // Delete navigation via handler
    let _deleted = handlers::delete_navigation(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(nav.id),
    )
    .await
    .expect("delete navigation via handler");
}

#[sqlx::test(migrations = "./migrations")]
async fn api_can_partial_update_navigation(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let nav = state
        .navigation
        .create_item(
            "test",
            soffio::application::admin::navigation::CreateNavigationItemCommand {
                label: "Nav".into(),
                destination_type: soffio::domain::types::NavigationDestinationType::External,
                destination_page_id: None,
                destination_url: Some("https://example.com".into()),
                sort_order: 1,
                visible: true,
                open_in_new_tab: false,
            },
        )
        .await
        .expect("create navigation");

    handlers::update_navigation_label(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(nav.id),
        Json(NavigationLabelRequest {
            label: "Nav Updated".into(),
        }),
    )
    .await
    .expect("update label");

    handlers::update_navigation_destination(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(nav.id),
        Json(NavigationDestinationRequest {
            destination_type: soffio::domain::types::NavigationDestinationType::External,
            destination_page_id: None,
            destination_url: Some("https://example.org".into()),
        }),
    )
    .await
    .expect("update destination");

    handlers::update_navigation_sort_order(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(nav.id),
        Json(NavigationSortOrderRequest { sort_order: 5 }),
    )
    .await
    .expect("update sort order");

    handlers::update_navigation_visibility(
        State(state.clone()),
        Extension(principal.clone()),
        axum::extract::Path(nav.id),
        Json(NavigationVisibilityRequest { visible: false }),
    )
    .await
    .expect("update visibility");

    handlers::update_navigation_open_in_new_tab(
        State(state.clone()),
        Extension(principal),
        axum::extract::Path(nav.id),
        Json(NavigationOpenInNewTabRequest {
            open_in_new_tab: true,
        }),
    )
    .await
    .expect("update open in new tab");

    let latest = state.navigation.find_by_id(nav.id).await.unwrap().unwrap();
    assert_eq!(latest.label, "Nav Updated");
    assert_eq!(
        latest.destination_url.as_deref(),
        Some("https://example.org")
    );
    assert_eq!(latest.sort_order, 5);
    assert!(!latest.visible);
    assert!(latest.open_in_new_tab);
}
