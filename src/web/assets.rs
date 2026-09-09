use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "static/"]
struct StaticAssets;

pub async fn static_file(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    let path = path.trim_start_matches('/');
    if path.is_empty() || path.split('/').any(|s| s == ".." || s.is_empty()) {
        return StatusCode::NOT_FOUND.into_response();
    }

    let Some(file) = StaticAssets::get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mime = mime_guess::from_path(path).first_or_octet_stream();
    (
        [
            (header::CONTENT_TYPE, mime.as_ref()),
            (
                header::CACHE_CONTROL,
                "public, max-age=86400, must-revalidate",
            ),
        ],
        file.data,
    )
        .into_response()
}
