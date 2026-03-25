use super::*;

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
