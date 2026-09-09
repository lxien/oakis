use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    NotFound(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error(transparent)]
    Internal(#[from] anyhow::Error),

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),

    #[error(transparent)]
    Template(#[from] askama::Error),

    #[error(transparent)]
    Session(#[from] tower_sessions::session::Error),
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn user_message(&self) -> String {
        match self {
            Self::BadRequest(m) | Self::NotFound(m) => m.clone(),
            Self::Unauthorized => "请先登录".into(),
            Self::Forbidden => "请求被拒绝".into(),
            Self::Internal(_) | Self::Sqlx(_) | Self::Template(_) | Self::Session(_) => {
                "服务器繁忙，请稍后重试".into()
            }
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[derive(Template)]
#[template(path = "themes/default/error.html")]
struct ErrorPageTemplate {
    status: u16,
    title: String,
    message: String,
    home_href: String,
}

fn error_page(status: StatusCode, title: &str, message: &str) -> Response {
    let body = ErrorPageTemplate {
        status: status.as_u16(),
        title: title.into(),
        message: message.into(),
        home_href: "/".into(),
    }
    .render()
    .unwrap_or_else(|_| format!("{title}: {message}"));
    (status, Html(body)).into_response()
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Unauthorized => {
                tracing::debug!("unauthorized cloaked as 404");
                error_page(StatusCode::NOT_FOUND, "未找到", "页面不存在")
            }
            AppError::Forbidden => {
                tracing::debug!("forbidden (csrf or access)");
                error_page(
                    StatusCode::FORBIDDEN,
                    "请求被拒绝",
                    "请求无效或已过期，请刷新页面后重试",
                )
            }
            AppError::NotFound(msg) => {
                tracing::debug!("not found: {msg}");
                error_page(StatusCode::NOT_FOUND, "未找到", &msg)
            }
            AppError::BadRequest(msg) => {
                tracing::debug!("bad request: {msg}");
                error_page(StatusCode::BAD_REQUEST, "请求无效", &msg)
            }
            other => {
                tracing::error!("internal error: {other}");
                error_page(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "服务器错误",
                    "服务器繁忙，请稍后重试",
                )
            }
        }
    }
}

pub fn render<T: Template>(template: T) -> AppResult<Html<String>> {
    Ok(Html(template.render()?))
}

pub fn see_other(path: &str) -> axum::response::Redirect {
    axum::response::Redirect::to(path)
}
