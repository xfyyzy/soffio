use super::*;

// ============ API Key Scope Granularity ============

#[sqlx::test(migrations = "./migrations")]
async fn api_scope_granularity_post_vs_page(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key with only PostRead scope
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "post-only".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    let principal = state.api_keys.authenticate(&issued.token).await.unwrap();

    // Should be able to list posts
    let _posts = handlers::list_posts(
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
    .expect("should be able to list posts with PostRead scope");

    // Should NOT be able to list pages (requires PageRead)
    assert!(
        principal.requires(ApiScope::PageRead).is_err(),
        "PostRead scope should not grant PageRead access"
    );
}
