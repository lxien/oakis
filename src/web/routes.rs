use std::path::Path;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tower_sessions::SessionManagerLayer;

use crate::handlers::export as export_api;
use crate::handlers::flash_api;
use crate::handlers::{
    admin, auth, docs, install, media, nav, pages, posts, public, settings, sparks, taxonomy,
};
use crate::infra::state::AppState;
use crate::web::csrf::{csrf_protect, security_headers};
use crate::web::middleware::{require_installed, site_access_gate};

pub fn app(
    state: AppState,
    login_path: &str,
    upload_dir: impl AsRef<Path>,
    session_layer: SessionManagerLayer<tower_sessions::MemoryStore>,
) -> Router {
    Router::new()
        .route("/", get(public::home))
        .route("/posts", get(public::posts_index))
        .route("/feed.xml", get(public::feed_rss))
        .route("/posts/{slug}", get(public::post_detail))
        .route("/p/{slug}", get(public::page_detail))
        .route("/search", get(public::search))
        .route("/categories/{slug}", get(public::category_page))
        .route("/tags", get(public::tags_index))
        .route("/tags/{slug}", get(public::tag_page))
        .route("/sparks", get(sparks::sparks_list))
        .route(
            "/spark",
            get(sparks::spark_capture_page).post(sparks::spark_create),
        )
        .route("/docs", get(docs::docs_home))
        .route("/docs/{book_slug}", get(docs::docs_book))
        .route("/docs/{book_slug}/{page_slug}", get(docs::docs_page))
        .route(
            "/install",
            get(install::install_page).post(install::install_submit),
        )
        .route(login_path, get(auth::login_page).post(auth::login_submit))
        .route("/logout", post(auth::logout))
        .route("/admin", get(admin::dashboard))
        .route("/admin/api/flash", get(flash_api::admin_flash))
        .route("/admin/api/media", get(media::media_list_api))
        .route("/admin/api/posts", get(posts::posts_search_api))
        .route("/admin/posts", get(posts::posts_list))
        .route(
            "/admin/posts/new",
            get(posts::post_new_page).post(posts::post_create),
        )
        .route(
            "/admin/posts/{id}/edit",
            get(posts::post_edit_page).post(posts::post_update),
        )
        .route("/admin/posts/{id}/delete", post(posts::post_delete))
        .route("/admin/posts/batch-delete", post(posts::posts_batch_delete))
        .route("/admin/sparks", get(sparks::admin_sparks_list))
        .route(
            "/admin/sparks/{id}/edit",
            get(sparks::admin_spark_edit_page).post(sparks::admin_spark_update),
        )
        .route(
            "/admin/sparks/{id}/delete",
            post(sparks::admin_spark_delete),
        )
        .route(
            "/admin/sparks/batch-delete",
            post(sparks::admin_sparks_batch_delete),
        )
        .route(
            "/admin/sparks/{id}/to-post",
            post(sparks::admin_spark_to_post),
        )
        .route("/admin/pages", get(pages::pages_list))
        .route(
            "/admin/pages/new",
            get(pages::page_new_page).post(pages::page_create),
        )
        .route(
            "/admin/pages/{id}/edit",
            get(pages::page_edit_page).post(pages::page_update),
        )
        .route("/admin/pages/{id}/delete", post(pages::page_delete))
        .route("/admin/pages/batch-delete", post(pages::pages_batch_delete))
        .route("/admin/upload", post(posts::upload_image))
        .route(
            "/admin/media",
            get(media::media_page).post(media::media_upload),
        )
        .route("/admin/media/{id}/delete", post(media::media_delete))
        .route("/admin/media/batch-delete", post(media::media_batch_delete))
        .route(
            "/admin/taxonomies",
            get(taxonomy::taxonomies_page).post(taxonomy::taxonomy_create),
        )
        .route(
            "/admin/taxonomies/{id}/edit",
            get(taxonomy::taxonomy_edit_page).post(taxonomy::taxonomy_update),
        )
        .route(
            "/admin/taxonomies/{id}/delete",
            post(taxonomy::taxonomy_delete),
        )
        .route(
            "/admin/taxonomies/batch-delete",
            post(taxonomy::taxonomies_batch_delete),
        )
        .route(
            "/admin/settings",
            get(settings::settings_page).post(settings::settings_save),
        )
        .route(
            "/admin/settings/password",
            post(settings::settings_password),
        )
        .route("/admin/nav", get(nav::nav_page).post(nav::nav_save))
        .route("/admin/docs", get(docs::admin_docs_list))
        .route("/admin/docs/books", post(docs::admin_book_create))
        .route(
            "/admin/docs/books/{id}/settings",
            get(docs::admin_book_settings_page),
        )
        .route("/admin/docs/books/{id}/save", post(docs::admin_book_save))
        .route(
            "/admin/docs/books/{id}/delete",
            post(docs::admin_book_delete),
        )
        .route("/admin/docs/nodes/{id}/save", post(docs::admin_node_save))
        .route(
            "/admin/docs/nodes/{id}/rename",
            post(docs::admin_node_rename),
        )
        .route(
            "/admin/docs/nodes/{id}/delete",
            post(docs::admin_node_delete),
        )
        .route("/admin/docs/{book_id}", get(docs::admin_book_page))
        .route(
            "/admin/docs/{book_id}/create",
            post(docs::admin_node_create),
        )
        .route(
            "/admin/export/{source}/{id}",
            get(export_api::export_download),
        )
        .route("/static/{*path}", get(crate::web::assets::static_file))
        .nest_service("/uploads", ServeDir::new(upload_dir))
        .fallback(|| async { crate::infra::error::AppError::not_found("页面不存在") })
        .layer(DefaultBodyLimit::max(12 * 1024 * 1024))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            site_access_gate,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_installed,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            csrf_protect,
        ))
        .layer(session_layer)
        .layer(axum::middleware::from_fn(security_headers))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
