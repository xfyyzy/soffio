use super::*;

// ============ Audit ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_list_audit_logs(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Add AuditRead scope for this test
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "audit-test".to_string(),
            description: None,
            scopes: vec![ApiScope::AuditRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    let principal = state.api_keys.authenticate(&issued.token).await.unwrap();

    let _list = handlers::list_audit_logs(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::AuditListQuery {
            actor: None,
            action: None,
            entity_type: None,
            search: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("list audit logs via handler");
}
