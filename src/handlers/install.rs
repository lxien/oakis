use axum::Form;
use axum::extract::State;
use axum::response::{Html, Redirect};
use serde::Deserialize;

use crate::infra::error::{AppResult, render, see_other};
use crate::infra::password::{hash_password, validate_password_len};
use crate::infra::state::AppState;
use crate::store::{count_users, create_user, upsert_setting};
use crate::views::InstallTemplate;

#[derive(Deserialize)]
pub struct InstallForm {
    pub site_title: String,
    pub site_tagline: String,
    pub author_name: String,
    pub username: String,
    pub password: String,
    pub password_confirm: String,
}

pub async fn install_page(State(state): State<AppState>) -> AppResult<Html<String>> {
    if *state.installed.read().await {
        return Err(crate::infra::error::AppError::bad_request("站点已安装"));
    }

    render(InstallTemplate {
        error: None,
        site_title: "oakis".into(),
        site_tagline: String::new(),
        author_name: String::new(),
        username: "admin".into(),
    })
}

pub async fn install_submit(
    State(state): State<AppState>,
    Form(form): Form<InstallForm>,
) -> AppResult<Result<Redirect, Html<String>>> {
    if *state.installed.read().await {
        return Ok(Ok(see_other("/")));
    }

    let site_title = form.site_title.trim().to_string();
    let site_tagline = form.site_tagline.trim().to_string();
    let author_name = form.author_name.trim().to_string();
    let username = form.username.trim().to_string();
    let password = form.password;
    let password_confirm = form.password_confirm;

    let err = |message: &str| -> AppResult<Result<Redirect, Html<String>>> {
        Ok(Err(render(InstallTemplate {
            error: Some(message.to_string()),
            site_title: site_title.clone(),
            site_tagline: site_tagline.clone(),
            author_name: author_name.clone(),
            username: username.clone(),
        })?))
    };

    if site_title.is_empty() {
        return err("请填写站点名称");
    }
    if author_name.is_empty() {
        return err("请填写作者名称");
    }
    if username.len() < 3 {
        return err("用户名至少 3 个字符");
    }
    if let Err(msg) = validate_password_len(&password) {
        return err(msg);
    }
    if password != password_confirm {
        return err("两次输入的密码不一致");
    }

    let password_hash = hash_password(&password)?;

    let mut tx = state.pool.begin().await?;
    let existing = count_users(&mut *tx).await?;
    if existing > 0 {
        let _ = tx.rollback().await;
        return Ok(Ok(see_other("/")));
    }

    if let Err(e) = create_user(&mut *tx, &username, &password_hash).await {
        let _ = tx.rollback().await;
        tracing::warn!(error = %e, "install insert user failed");
        return err("安装失败，请稍后重试");
    }

    for (key, value) in [
        ("installed", "1"),
        ("site_title", site_title.as_str()),
        ("site_tagline", site_tagline.as_str()),
        ("author_name", author_name.as_str()),
    ] {
        upsert_setting(&mut *tx, key, value).await?;
    }
    tx.commit().await?;

    *state.installed.write().await = true;
    Ok(Ok(see_other(&state.config.login_path)))
}
