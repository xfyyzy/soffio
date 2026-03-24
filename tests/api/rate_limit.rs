use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn api_rate_limit_uses_route_template(pool: PgPool) {
    let (state, token) = build_state(pool).await;
    let principal = state
        .api_keys
        .authenticate(&token)
        .await
        .expect("authenticate token");

    let state = ApiState {
        rate_limiter: Arc::new(soffio::infra::http::api::rate_limit::ApiRateLimiter::new(
            std::time::Duration::from_secs(60),
            1,
        )),
        ..state
    };

    let app = Router::new()
        .route("/api/v1/posts/{id}", get(|| async { StatusCode::OK }))
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(
            state,
            soffio::infra::http::api::middleware::api_rate_limit,
        ));

    let first_path = format!("/api/v1/posts/{}", Uuid::new_v4());
    let mut first_request = Request::builder()
        .method("GET")
        .uri(first_path)
        .body(Body::empty())
        .expect("build first request");
    first_request.extensions_mut().insert(principal.clone());

    let first_response = app
        .clone()
        .oneshot(first_request)
        .await
        .expect("send first request");
    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(
        first_response
            .headers()
            .get("x-ratelimit-remaining")
            .and_then(|value| value.to_str().ok()),
        Some("0")
    );

    let second_path = format!("/api/v1/posts/{}", Uuid::new_v4());
    let mut second_request = Request::builder()
        .method("GET")
        .uri(second_path)
        .body(Body::empty())
        .expect("build second request");
    second_request.extensions_mut().insert(principal);

    let second_response = app
        .oneshot(second_request)
        .await
        .expect("send second request");
    assert_eq!(second_response.status(), StatusCode::TOO_MANY_REQUESTS);
}
