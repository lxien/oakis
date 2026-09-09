use argon2::Argon2;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use rand_core::OsRng;

use crate::infra::error::{AppError, AppResult};

pub const MIN_PASSWORD_LEN: usize = 6;

pub fn validate_password_len(password: &str) -> Result<(), &'static str> {
    if password.len() < MIN_PASSWORD_LEN {
        Err("密码至少 6 位")
    } else {
        Ok(())
    }
}

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::bad_request(format!("密码加密失败: {e}")))?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, password_hash: &str) -> AppResult<bool> {
    let parsed = match PasswordHash::new(password_hash) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "invalid stored password hash");
            return Ok(false);
        }
    };
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}
