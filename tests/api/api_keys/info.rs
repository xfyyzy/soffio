use super::*;

// ============ API Keys ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_get_api_key_info(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let Json(info) = handlers::get_api_key_info(State(state.clone()), Extension(principal))
        .await
        .expect("get api key info");

    assert_eq!(info.prefix.len(), 12);
    assert!(info.scopes.contains(&ApiScope::PostRead));
    assert_eq!(info.status, soffio::domain::api_keys::ApiKeyStatus::Active);
}
