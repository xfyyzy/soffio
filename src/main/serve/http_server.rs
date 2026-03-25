use soffio::{
    application::error::AppError,
    config,
    infra::{
        error::InfraError,
        http::{self, AdminState, ApiState, HttpState, RouterState},
    },
};
use tokio::try_join;

pub(super) async fn serve_http(
    settings: &config::Settings,
    http_state: HttpState,
    admin_state: AdminState,
    api_state: ApiState,
) -> Result<(), AppError> {
    let router_state = RouterState {
        http: http_state,
        api: api_state,
    };
    let public_router = http::build_router(router_state.clone());
    let upload_body_limit = settings.uploads.max_request_bytes.get() as usize;
    let admin_router = http::build_admin_router(admin_state, upload_body_limit);
    let api_router = http::build_api_v1_router(router_state.clone());

    let public_router = public_router
        .merge(api_router)
        .with_state(router_state.clone());

    let public_listener = tokio::net::TcpListener::bind(settings.server.public_addr)
        .await
        .map_err(|err| AppError::from(InfraError::from(err)))?;
    let admin_listener = tokio::net::TcpListener::bind(settings.server.admin_addr)
        .await
        .map_err(|err| AppError::from(InfraError::from(err)))?;

    let public_server = axum::serve(public_listener, public_router.into_make_service());
    let admin_server = axum::serve(admin_listener, admin_router.into_make_service());

    try_join!(public_server, admin_server)
        .map_err(|err| AppError::unexpected(format!("server error: {err}")))?;

    Ok(())
}
