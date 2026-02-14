mod auth;
mod executor;
mod handlers;
mod session;
mod task;

use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

pub struct AppState {
    pub session_manager: session::SessionManager,
    pub auth_token: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,term_executor=debug".into()),
        )
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let session_ttl_secs: u64 = std::env::var("SESSION_TTL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800); // 30 min default

    let auth_token = std::env::var("AUTH_TOKEN").ok();

    let state = Arc::new(AppState {
        session_manager: session::SessionManager::new(session_ttl_secs),
        auth_token,
    });

    // Spawn session reaper
    let reaper_state = state.clone();
    tokio::spawn(async move {
        reaper_state.session_manager.reaper_loop().await;
    });

    let protected = Router::new()
        .route("/evaluate", post(handlers::start_evaluation))
        .route("/evaluate/{id}", get(handlers::poll_evaluation))
        .route("/evaluate/{id}", delete(handlers::cancel_evaluation))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));

    let app = Router::new()
        .route("/health", get(handlers::health))
        .merge(protected)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    info!("term-executor listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
