mod export;
mod handlers;
mod infra;
mod models;
mod store;
mod views;
mod web;

use std::net::SocketAddr;
use std::sync::Arc;

use tower_sessions::cookie::SameSite;
use tower_sessions::{MemoryStore, SessionManagerLayer};
use tracing_subscriber::EnvFilter;

use crate::infra::auth_limit::LoginLimiter;
use crate::infra::config::Config;
use crate::infra::db;
use crate::infra::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("oakis=info,tower_http=info")),
        )
        .init();

    let config = Config::from_env()?;
    config.ensure_dirs()?;

    let pool = db::connect(&config.database_url).await?;
    db::migrate(&pool).await?;
    crate::store::ensure_nav_items(&pool).await?;

    let installed = db::is_installed(&pool).await?;
    let login_path = config.login_path.clone();
    let upload_dir = config.upload_dir.clone();
    let state = AppState {
        pool,
        config: Arc::new(config.clone()),
        installed: Arc::new(tokio::sync::RwLock::new(installed)),
        login_limiter: Arc::new(LoginLimiter::new()),
    };

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(config.session_secure)
        .with_http_only(true)
        .with_same_site(SameSite::Strict);

    let app = web::routes::app(state, &login_path, upload_dir, session_layer);

    let addr = SocketAddr::from((config.host, config.port));
    tracing::info!("oakis listening on http://{addr}");
    tracing::debug!(%login_path, "admin login path configured");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
