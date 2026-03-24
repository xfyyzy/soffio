use super::*;

// ============ Jobs ============

#[sqlx::test(migrations = "./migrations")]
async fn api_can_list_jobs(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state.api_keys.authenticate(&token).await.unwrap();

    let _list = handlers::list_jobs(
        State(state.clone()),
        Extension(principal.clone()),
        Query(handlers::JobsListQuery {
            state: None,
            job_type: None,
            search: None,
            cursor: None,
            limit: Some(10),
        }),
    )
    .await
    .expect("list jobs via handler");
}
