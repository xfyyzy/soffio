use super::*;

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
