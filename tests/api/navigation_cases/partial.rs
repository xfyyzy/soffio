use super::*;

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
