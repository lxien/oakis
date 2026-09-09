use axum::Form;
use axum::Json;
use axum::extract::{Multipart, Path, Query, State};
use axum::response::{Html, Redirect};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::infra::error::{AppError, AppResult, render, see_other};
use crate::infra::flash::flash_ok;
use crate::infra::pagination::{ADMIN_PER_PAGE, PageQuery, Pagination};
use crate::infra::state::AppState;
use crate::infra::upload::{delete_media_file, save_media};
use crate::models::{Media, SiteSettings};
use crate::store::{
    count_media, count_media_picker, delete_media_row, find_media_by_id, insert_media, list_media,
    list_media_picker,
};
use crate::views::AdminMediaTemplate;
use crate::web::authz::{cloak_auth, require_user};
use crate::web::forms::{BatchIdsForm, parse_ids};

async fn page(
    settings: SiteSettings,
    username: String,
    items: Vec<Media>,
    pagination: Pagination,
    error: Option<String>,
) -> AppResult<Html<String>> {
    render(AdminMediaTemplate {
        settings,
        username,
        items,
        pagination,
        error,
    })
}

async fn load_media_page(
    state: &AppState,
    settings: SiteSettings,
    username: String,
    page_num: Option<u32>,
    error: Option<String>,
) -> AppResult<Html<String>> {
    let total = count_media(&state.pool).await?;
    let pagination = Pagination::new(page_num, ADMIN_PER_PAGE, total, "/admin/media");
    let items = list_media(&state.pool, pagination.limit(), pagination.offset()).await?;
    page(settings, username, items, pagination, error).await
}

pub async fn media_page(
    State(state): State<AppState>,
    session: Session,
    Query(query): Query<PageQuery>,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };
    Ok(Err(load_media_page(
        &state,
        settings,
        user.username,
        query.page,
        None,
    )
    .await?))
}

pub async fn media_upload(
    State(state): State<AppState>,
    session: Session,
    mut multipart: Multipart,
) -> AppResult<Result<Redirect, Html<String>>> {
    let (user, settings) = match require_user(&state, &session).await {
        Ok(v) => v,
        Err(e) => return Err(cloak_auth(e)),
    };

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut original_name = String::from("file");

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::bad_request("表单数据无效或过大，请减少附件后重试"))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            if let Some(fname) = field.file_name().map(|s| s.to_string()) {
                if !fname.trim().is_empty() {
                    original_name = fname;
                }
            }
            let bytes = field
                .bytes()
                .await
                .map_err(|_| AppError::bad_request("读取文件失败，请重试"))?;
            if !bytes.is_empty() {
                file_bytes = Some(bytes.to_vec());
            }
        } else {
            let _ = field.bytes().await;
        }
    }

    let Some(bytes) = file_bytes else {
        return Ok(Err(load_media_page(
            &state,
            settings,
            user.username,
            Some(1),
            Some("请选择要上传的文件".into()),
        )
        .await?));
    };

    let saved = match save_media(&bytes, &original_name, &state.config.media_dir()).await {
        Ok(s) => s,
        Err(_) => {
            return Ok(Err(load_media_page(
                &state,
                settings,
                user.username,
                Some(1),
                Some("上传失败，请检查文件类型或大小".into()),
            )
            .await?));
        }
    };

    if let Err(_) = insert_media(
        &state.pool,
        &saved.name,
        &saved.url,
        &saved.mime,
        saved.size,
    )
    .await
    {
        delete_media_file(&saved.url, &state.config.media_dir()).await;
        return Ok(Err(load_media_page(
            &state,
            settings,
            user.username,
            Some(1),
            Some("保存失败，请稍后重试".into()),
        )
        .await?));
    }

    flash_ok(&session, "上传成功").await?;
    Ok(Ok(see_other("/admin/media")))
}

pub async fn media_delete(
    State(state): State<AppState>,
    session: Session,
    Path(id): Path<i64>,
) -> AppResult<Redirect> {
    if let Err(e) = require_user(&state, &session).await {
        return Err(cloak_auth(e));
    }
    if let Some(item) = find_media_by_id(&state.pool, id).await? {
        delete_media_file(&item.url, &state.config.media_dir()).await;
        delete_media_row(&state.pool, id).await?;
    }
    flash_ok(&session, "已删除").await?;
    Ok(see_other("/admin/media"))
}

pub async fn media_batch_delete(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<BatchIdsForm>,
) -> AppResult<Redirect> {
    if let Err(e) = require_user(&state, &session).await {
        return Err(cloak_auth(e));
    }
    for id in parse_ids(&form.ids) {
        if let Some(item) = find_media_by_id(&state.pool, id).await? {
            delete_media_file(&item.url, &state.config.media_dir()).await;
            delete_media_row(&state.pool, id).await?;
        }
    }
    flash_ok(&session, "批量删除完成").await?;
    Ok(see_other("/admin/media"))
}

const PICKER_LIMIT: i64 = 24;

#[derive(Deserialize)]
pub struct MediaApiQuery {
    pub page: Option<u32>,

    pub kind: Option<String>,
}

#[derive(Serialize)]
pub struct MediaApiItem {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub mime: String,
    pub thumb: String,
}

#[derive(Serialize)]
pub struct MediaApiResponse {
    pub page: u32,
    pub limit: u32,
    pub total: i64,
    pub items: Vec<MediaApiItem>,
}

fn picker_images_only(kind: Option<&str>) -> bool {
    !matches!(kind.map(str::trim), Some("all"))
}

pub async fn media_list_api(
    State(state): State<AppState>,
    session: Session,
    Query(q): Query<MediaApiQuery>,
) -> AppResult<Json<MediaApiResponse>> {
    if require_user(&state, &session).await.is_err() {
        return Err(AppError::not_found("页面不存在"));
    }

    let images_only = picker_images_only(q.kind.as_deref());
    let page = q.page.unwrap_or(1).max(1);
    let total = count_media_picker(&state.pool, images_only).await?;
    let offset = ((page as i64) - 1) * PICKER_LIMIT;
    let rows = list_media_picker(&state.pool, images_only, PICKER_LIMIT, offset).await?;
    let items = rows
        .into_iter()
        .map(|m| {
            let is_image = m.is_image();
            let thumb = if is_image {
                m.thumb_url()
            } else {
                String::new()
            };
            MediaApiItem {
                id: m.id,
                name: m.name,
                url: m.url,
                mime: m.mime,
                thumb,
            }
        })
        .collect();

    Ok(Json(MediaApiResponse {
        page,
        limit: PICKER_LIMIT as u32,
        total,
        items,
    }))
}
