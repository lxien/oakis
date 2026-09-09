use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::infra::error::AppResult;

const FLASH_KEY: &str = "flash";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flash {
    pub level: String,
    pub message: String,
}

impl Flash {
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            level: "ok".into(),
            message: message.into(),
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            level: "err".into(),
            message: message.into(),
        }
    }
}

pub async fn set_flash(session: &Session, flash: Flash) -> AppResult<()> {
    session.insert(FLASH_KEY, flash).await?;
    Ok(())
}

pub async fn take_flash(session: &Session) -> AppResult<Option<Flash>> {
    Ok(session.remove::<Flash>(FLASH_KEY).await?)
}

pub async fn flash_ok(session: &Session, message: impl Into<String>) -> AppResult<()> {
    set_flash(session, Flash::ok(message)).await
}

pub async fn flash_err(session: &Session, message: impl Into<String>) -> AppResult<()> {
    set_flash(session, Flash::err(message)).await
}
