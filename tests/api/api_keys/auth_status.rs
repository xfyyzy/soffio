use super::*;

// ============ API Key Authentication Status ============

#[sqlx::test(migrations = "./migrations")]
async fn api_auth_rejects_revoked_key(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "revoke-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: None,
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    // Revoke the key
    state
        .api_keys
        .revoke(issued.record.id)
        .await
        .expect("revoke should succeed");

    // Authentication should fail with Revoked error
    let result = state.api_keys.authenticate(&issued.token).await;
    assert!(
        result.is_err(),
        "authentication should fail for revoked key"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, soffio::application::api_keys::ApiAuthError::Revoked),
        "should get Revoked error, got: {:?}",
        err
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn api_auth_rejects_expired_key(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key that expires immediately (expires_in = 0 means expires_at = now)
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "expired-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: Some(time::Duration::ZERO),
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    // Small delay to ensure we're past the expires_at time
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Authentication should fail with Expired error
    let result = state.api_keys.authenticate(&issued.token).await;
    assert!(
        result.is_err(),
        "authentication should fail for expired key"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, soffio::application::api_keys::ApiAuthError::Expired),
        "should get Expired error, got: {:?}",
        err
    );
}
