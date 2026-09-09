use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use tower_sessions::Session;

use crate::export::{
    ExportArtifact, ExportFormat, ExportSource, build_book, build_doc, build_page, build_post,
};
use crate::infra::error::{AppError, AppResult};
use crate::infra::state::AppState;
use crate::store::{
    find_kb_book_by_id, find_kb_node_by_id, find_page_by_id, find_post_by_id, list_kb_nodes_by_book,
};
use crate::web::authz::{cloak_auth, require_user};

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "markdown".into()
}

pub async fn export_download(
    State(state): State<AppState>,
    session: Session,
    Path((source, id)): Path<(String, i64)>,
    Query(query): Query<ExportQuery>,
) -> AppResult<Response> {
    if let Err(e) = require_user(&state, &session).await {
        return Err(cloak_auth(e));
    }

    let source =
        ExportSource::parse(&source).ok_or_else(|| AppError::bad_request("不支持的导出对象"))?;
    let format = ExportFormat::parse(&query.format)
        .ok_or_else(|| AppError::bad_request("不支持的导出格式"))?;

    let upload_dir = &state.config.upload_dir;
    let artifact = match source {
        ExportSource::Post => {
            let post = find_post_by_id(&state.pool, id)
                .await?
                .ok_or_else(|| AppError::not_found("文章不存在"))?;
            build_post(&post, upload_dir, format)?
        }
        ExportSource::Page => {
            let page = find_page_by_id(&state.pool, id)
                .await?
                .ok_or_else(|| AppError::not_found("页面不存在"))?;
            build_page(&page, upload_dir, format)?
        }
        ExportSource::Doc => {
            let node = find_kb_node_by_id(&state.pool, id)
                .await?
                .ok_or_else(|| AppError::not_found("文档不存在"))?;
            build_doc(&node, upload_dir, format)?
        }
        ExportSource::Book => {
            let book = find_kb_book_by_id(&state.pool, id)
                .await?
                .ok_or_else(|| AppError::not_found("知识库不存在"))?;
            let nodes = list_kb_nodes_by_book(&state.pool, book.id).await?;
            build_book(&book, &nodes, upload_dir, format)?
        }
    };

    Ok(download_response(artifact))
}

fn download_response(artifact: ExportArtifact) -> Response {
    let mut res = artifact.body.into_response();
    let headers = res.headers_mut();
    if let Ok(v) = HeaderValue::from_str(&artifact.content_type) {
        headers.insert(header::CONTENT_TYPE, v);
    }
    let disposition = content_disposition(&artifact.filename);
    if let Ok(v) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, v);
    }
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

fn content_disposition(filename: &str) -> String {
    let ascii: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let encoded = urlencoding_minimal(filename);
    format!("attachment; filename=\"{ascii}\"; filename*=UTF-8''{encoded}")
}

fn urlencoding_minimal(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
