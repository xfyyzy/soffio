use super::*;

// ============ API Key Rotation ============

#[sqlx::test(migrations = "./migrations")]
async fn api_rotate_reactivates_revoked_key(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "rotate-revoke-test".to_string(),
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

    // Rotation should succeed and reactivate the key
    let rotated = state
        .api_keys
        .rotate(issued.record.id)
        .await
        .expect("rotation should succeed for revoked key");

    // The key should now be active
    assert_eq!(
        rotated.record.status,
        soffio::domain::api_keys::ApiKeyStatus::Active,
        "key should be reactivated after rotation"
    );

    // The new token should work for authentication
    let auth_result = state.api_keys.authenticate(&rotated.token).await;
    assert!(
        auth_result.is_ok(),
        "authentication should succeed with rotated token"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn api_rotate_recalculates_expiration_preserves_created_at(pool: PgPool) {
    let (state, _token) = build_state(pool).await;

    // Issue a key with 30-day expiration duration
    let issued = state
        .api_keys
        .issue(IssueApiKeyCommand {
            name: "rotate-preserve-test".to_string(),
            description: None,
            scopes: vec![ApiScope::PostRead],
            expires_in: Some(time::Duration::days(30)),
            created_by: "tests".to_string(),
        })
        .await
        .unwrap();

    let original_created_at = issued.record.created_at;
    let original_expires_in = issued.record.expires_in;
    let original_expires_at = issued.record.expires_at;

    // Small delay to ensure recalculated expires_at is different
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    // Rotate the key
    let rotated = state
        .api_keys
        .rotate(issued.record.id)
        .await
        .expect("rotation should succeed");

    // created_at and expires_in should be preserved
    assert_eq!(
        rotated.record.created_at, original_created_at,
        "created_at should be preserved after rotation"
    );
    assert_eq!(
        rotated.record.expires_in, original_expires_in,
        "expires_in duration should be preserved after rotation"
    );

    // expires_at should be recalculated (should be later than original)
    assert!(
        rotated.record.expires_at > original_expires_at,
        "expires_at should be recalculated to a later time after rotation"
    );

    // The token should be different
    assert_ne!(
        issued.token, rotated.token,
        "token should change after rotation"
    );

    // Old token should no longer work
    let old_auth = state.api_keys.authenticate(&issued.token).await;
    assert!(old_auth.is_err(), "old token should not authenticate");

    // New token should work
    let new_auth = state.api_keys.authenticate(&rotated.token).await;
    assert!(new_auth.is_ok(), "new token should authenticate");
}
