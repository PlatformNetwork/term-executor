mod auth;
mod basilica;
mod cleanup;
mod config;
mod executor;
mod handlers;
mod metrics;
mod sandbox;
mod session;
mod task;

use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("term_executor=info".parse().unwrap()),
        )
        .init();

    let config = Arc::new(config::Config::from_env());
    config.print_banner();

    // Create workspace base directory
    tokio::fs::create_dir_all(&config.workspace_base)
        .await
        .expect("Failed to create workspace directory");

    let sessions = Arc::new(session::SessionManager::new(config.session_ttl_secs));
    let metrics_store = metrics::Metrics::new();
    let semaphore = Arc::new(Semaphore::new(config.max_concurrent_evals));
    let executor = Arc::new(executor::Executor::new(
        config.clone(),
        sessions.clone(),
        metrics_store.clone(),
    ));

    let state = Arc::new(handlers::AppState {
        config: config.clone(),
        sessions: sessions.clone(),
        metrics: metrics_store,
        executor,
        semaphore,
        started_at: chrono::Utc::now(),
    });

    let app = handlers::router(state);
    let addr = format!("0.0.0.0:{}", config.port);

    // Basilica enrollment (fire and forget)
    if let (Some(ref token), Some(ref name)) =
        (&config.basilica_api_token, &config.basilica_instance_name)
    {
        let token = token.clone();
        let name = name.clone();
        tokio::spawn(async move {
            basilica::try_enroll_metadata(&token, &name).await;
        });
    }

    // Session reaper
    let sessions_reaper = sessions.clone();
    tokio::spawn(async move {
        sessions_reaper.reaper_loop().await;
    });

    // Stale dir reaper
    let workspace = config.workspace_base.clone();
    let ttl = config.session_ttl_secs;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            cleanup::reap_stale_sessions(&workspace, ttl).await;
        }
    });

    info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    // Graceful shutdown on SIGTERM
    let shutdown = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C handler");
        info!("Shutdown signal received, draining...");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .unwrap();

    info!("Shutdown complete");
}
