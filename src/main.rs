mod auth;
mod cleanup;
mod config;
mod consensus;
mod executor;
mod handlers;
mod metrics;
mod session;
mod task;
mod validator_whitelist;
mod ws;

use std::sync::Arc;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    let default_directive = "term_executor=info"
        .parse()
        .unwrap_or_else(|_| tracing_subscriber::filter::Directive::from(tracing::Level::INFO));

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(default_directive),
        )
        .init();

    let config = match config::Config::from_env() {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!("Invalid configuration: {}", e);
            std::process::exit(1);
        }
    };
    config.print_banner();

    if let Err(e) = tokio::fs::create_dir_all(&config.workspace_base).await {
        error!("Failed to create workspace directory: {}", e);
        std::process::exit(1);
    }

    let sessions = Arc::new(session::SessionManager::new(config.session_ttl_secs));
    let metrics_store = metrics::Metrics::new();
    let nonce_store = Arc::new(auth::NonceStore::new());
    let executor = Arc::new(executor::Executor::new(
        config.clone(),
        sessions.clone(),
        metrics_store.clone(),
    ));

    let validator_whitelist = validator_whitelist::ValidatorWhitelist::new();
    let consensus_manager = consensus::ConsensusManager::new(config.max_pending_consensus);

    let state = Arc::new(handlers::AppState {
        config: config.clone(),
        sessions: sessions.clone(),
        metrics: metrics_store,
        executor,
        nonce_store: nonce_store.clone(),
        started_at: chrono::Utc::now(),
        validator_whitelist: validator_whitelist.clone(),
        consensus_manager: consensus_manager.clone(),
    });

    let app = handlers::router(state);
    let addr = format!("0.0.0.0:{}", config.port);

    let sessions_reaper = sessions.clone();
    tokio::spawn(async move {
        sessions_reaper.reaper_loop().await;
    });

    let nonce_reaper = nonce_store.clone();
    tokio::spawn(async move {
        nonce_reaper.reaper_loop().await;
    });

    let workspace = config.workspace_base.clone();
    let ttl = config.session_ttl_secs;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            cleanup::reap_stale_sessions(&workspace, ttl).await;
        }
    });

    let wl = validator_whitelist.clone();
    let netuid = config.bittensor_netuid;
    let min_stake = config.min_validator_stake_tao;
    let refresh_secs = config.validator_refresh_secs;
    tokio::spawn(async move {
        wl.refresh_loop(netuid, min_stake, refresh_secs).await;
    });

    let cm = consensus_manager.clone();
    let consensus_ttl = config.consensus_ttl_secs;
    tokio::spawn(async move {
        cm.reaper_loop(consensus_ttl).await;
    });

    info!("Listening on {}", addr);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    let shutdown = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            error!("Failed to install CTRL+C handler: {}", e);
            return;
        }
        info!("Shutdown signal received, draining...");
    };

    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
    {
        error!("Server error: {}", e);
        std::process::exit(1);
    }

    info!("Shutdown complete");
}
