use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::http::{HeaderValue, Method, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures_util::stream;
use tower_sessions::Session;
use uuid::Uuid;

use crate::infra::error::AppError;
use crate::infra::state::AppState;

pub const SESSION_KEY: &str = "csrf_token";
pub const COOKIE_NAME: &str = "csrf_token";
pub const FORM_FIELD: &str = "csrf";
pub const HEADER_NAME: &str = "x-csrf-token";

const MAX_BODY_PEEK: usize = 16 * 1024 * 1024;

pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub async fn ensure_token(session: &Session) -> Result<String, AppError> {
    if let Some(existing) = session.get::<String>(SESSION_KEY).await? {
        if !existing.is_empty() {
            return Ok(existing);
        }
    }
    let token = Uuid::new_v4().simple().to_string();
    session.insert(SESSION_KEY, token.clone()).await?;
    Ok(token)
}

fn is_safe_method(method: &Method) -> bool {
    matches!(
        *method,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    )
}

fn is_exempt_path(path: &str) -> bool {
    path == "/static"
        || path.starts_with("/static/")
        || path == "/uploads"
        || path.starts_with("/uploads/")
}

fn header_token(request: &Request) -> Option<String> {
    request
        .headers()
        .get(HEADER_NAME)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn form_token_from_urlencoded(bytes: &[u8]) -> Option<String> {
    for (k, v) in form_urlencoded::parse(bytes) {
        if k == FORM_FIELD {
            let t = v.trim();
            if !t.is_empty() {
                return Some(t.to_owned());
            }
        }
    }
    None
}

fn content_type_raw(request: &Request) -> String {
    request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned()
}

fn is_urlencoded(ct: &str) -> bool {
    ct.to_ascii_lowercase()
        .starts_with("application/x-www-form-urlencoded")
}

fn is_multipart(ct: &str) -> bool {
    ct.to_ascii_lowercase().starts_with("multipart/form-data")
}

async fn form_token_from_multipart(bytes: Bytes, boundary: &str) -> Option<String> {
    let stream = stream::once(async move { Result::<Bytes, std::io::Error>::Ok(bytes) });
    let mut multipart = multer::Multipart::new(stream, boundary.to_string());
    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let is_csrf = field.name() == Some(FORM_FIELD);
                if is_csrf {
                    let text = field.text().await.ok()?;
                    let t = text.trim();
                    return if t.is_empty() {
                        None
                    } else {
                        Some(t.to_owned())
                    };
                }

                let _ = field.bytes().await;
            }
            Ok(None) => return None,
            Err(_) => return None,
        }
    }
}

fn append_csrf_cookie(response: &mut Response, token: &str, secure: bool) {
    let mut value = format!("{COOKIE_NAME}={token}; Path=/; SameSite=Strict; Max-Age=1209600");
    if secure {
        value.push_str("; Secure");
    }
    if let Ok(hv) = HeaderValue::from_str(&value) {
        response.headers_mut().append(header::SET_COOKIE, hv);
    }
}

fn csrf_rejected(wants_json: bool) -> Response {
    if wants_json {
        return (
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "error": "请求无效或已过期，请刷新页面后重试"
            })),
        )
            .into_response();
    }
    AppError::Forbidden.into_response()
}

fn accepts_json(request: &Request) -> bool {
    request
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_ascii_lowercase().contains("application/json"))
        .unwrap_or(false)
}

pub async fn csrf_protect(
    State(state): State<AppState>,
    session: Session,
    request: Request,
    next: Next,
) -> Response {
    let token = match ensure_token(&session).await {
        Ok(t) => t,
        Err(e) => return e.into_response(),
    };

    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let wants_json = accepts_json(&request);

    let request = if !is_safe_method(&method) && !is_exempt_path(&path) {
        match verify_and_restore(request, &token).await {
            Ok(req) => req,
            Err(()) => {
                tracing::debug!(%path, %method, "csrf rejected");
                return csrf_rejected(wants_json);
            }
        }
    } else {
        request
    };

    let mut response = next.run(request).await;
    append_csrf_cookie(&mut response, &token, state.config.session_secure);
    response
}

async fn verify_and_restore(request: Request, expected: &str) -> Result<Request, ()> {
    if let Some(got) = header_token(&request) {
        return if constant_time_eq(&got, expected) {
            Ok(request)
        } else {
            Err(())
        };
    }

    let ct = content_type_raw(&request);
    if is_urlencoded(&ct) {
        let (parts, body) = request.into_parts();
        let bytes = match to_bytes(body, MAX_BODY_PEEK).await {
            Ok(b) => b,
            Err(_) => return Err(()),
        };
        let ok = form_token_from_urlencoded(&bytes)
            .map(|got| constant_time_eq(&got, expected))
            .unwrap_or(false);
        if !ok {
            return Err(());
        }
        return Ok(Request::from_parts(parts, Body::from(bytes)));
    }

    if is_multipart(&ct) {
        let boundary = multer::parse_boundary(&ct).map_err(|_| ())?;
        let (parts, body) = request.into_parts();
        let bytes = match to_bytes(body, MAX_BODY_PEEK).await {
            Ok(b) => b,
            Err(_) => return Err(()),
        };
        let got = form_token_from_multipart(bytes.clone(), &boundary).await;
        let ok = got
            .as_deref()
            .map(|g| constant_time_eq(g, expected))
            .unwrap_or(false);
        if !ok {
            return Err(());
        }
        return Ok(Request::from_parts(parts, Body::from(bytes)));
    }

    Err(())
}

pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    response
}
