use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::infra::auth_limit::LoginLimiter;
use crate::infra::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
    pub installed: Arc<RwLock<bool>>,
    pub login_limiter: Arc<LoginLimiter>,
}
