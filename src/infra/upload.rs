use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::codecs::png::{CompressionType, FilterType as PngFilterType, PngEncoder};
use image::imageops::FilterType;
use image::{DynamicImage, ExtendedColorType, ImageEncoder};

use crate::infra::error::{AppError, AppResult};

const MAX_POST_IMAGE: usize = 2 * 1024 * 1024;
const MAX_MEDIA: usize = 5 * 1024 * 1024;

const MAIN_MAX_EDGE: u32 = 1920;
const THUMB_MAX_EDGE: u32 = 400;
const JPEG_QUALITY: u8 = 80;

const SKIP_REENCODE_JPEG_BYTES: usize = 450 * 1024;

pub const MEDIA_URL_PREFIX: &str = "/uploads/media/";
pub const POSTS_URL_PREFIX: &str = "/uploads/posts/";
pub const SITE_URL_PREFIX: &str = "/uploads/site/";

#[derive(Debug, Clone)]
pub struct SavedMedia {
    pub name: String,
    pub url: String,
    pub mime: String,
    pub size: i64,
}

fn detect_image_ext(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("png");
    }
    if bytes.len() >= 3 && bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF {
        return Some("jpg");
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Some("webp");
    }
    if bytes.len() >= 4 && bytes[0] == 0 && bytes[1] == 0 && bytes[2] == 1 && bytes[3] == 0 {
        return Some("ico");
    }
    if bytes.len() >= 6 && (&bytes[0..6] == b"GIF87a" || &bytes[0..6] == b"GIF89a") {
        return Some("gif");
    }
    None
}

fn ext_mime(ext: &str) -> Option<(&'static str, &'static str)> {
    match ext {
        "png" => Some(("png", "image/png")),
        "jpg" | "jpeg" => Some(("jpg", "image/jpeg")),
        "webp" => Some(("webp", "image/webp")),
        "gif" => Some(("gif", "image/gif")),
        "ico" => Some(("ico", "image/x-icon")),
        "pdf" => Some(("pdf", "application/pdf")),
        "zip" => Some(("zip", "application/zip")),
        "txt" => Some(("txt", "text/plain")),
        "md" => Some(("md", "text/markdown")),
        "csv" => Some(("csv", "text/csv")),
        "json" => Some(("json", "application/json")),
        _ => None,
    }
}

fn sanitize_original_name(raw: &str) -> String {
    let name = Path::new(raw)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .chars()
        .take(120)
        .collect::<String>();
    if name.trim().is_empty() {
        "file".into()
    } else {
        name
    }
}

fn is_safe_filename(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains("..")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
}

pub fn resolve_upload_file(url: &str, upload_dir: &Path) -> Option<PathBuf> {
    let rest = url.trim().strip_prefix("/uploads/")?;
    if rest.is_empty() || rest.contains("..") || rest.starts_with('/') || rest.contains('\\') {
        return None;
    }
    let mut parts = rest.split('/');
    let bucket = parts.next()?;
    if !matches!(bucket, "media" | "posts" | "site") {
        return None;
    }
    let name = parts.next()?;
    if parts.next().is_some() || !is_safe_filename(name) {
        return None;
    }
    let path = upload_dir.join(bucket).join(name);
    path.is_file().then_some(path)
}

pub fn sanitize_asset_url(url: &str, prefix: &str) -> String {
    let url = url.trim();
    let Some(name) = url.strip_prefix(prefix) else {
        return String::new();
    };
    if !is_safe_filename(name) {
        return String::new();
    }
    format!("{prefix}{name}")
}

pub fn sanitize_cover_url(url: &str) -> String {
    let posts = sanitize_asset_url(url, POSTS_URL_PREFIX);
    if !posts.is_empty() {
        return posts;
    }
    sanitize_asset_url(url, MEDIA_URL_PREFIX)
}

pub fn sanitize_brand_url(url: &str) -> String {
    let site = sanitize_asset_url(url, SITE_URL_PREFIX);
    if !site.is_empty() {
        return site;
    }
    sanitize_asset_url(url, MEDIA_URL_PREFIX)
}

pub fn media_thumb_url(url: &str) -> String {
    let safe = sanitize_asset_url(url, MEDIA_URL_PREFIX);
    let Some(name) = safe.strip_prefix(MEDIA_URL_PREFIX) else {
        return String::new();
    };
    let path = Path::new(name);
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return String::new();
    };
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("jpg");
    let thumb = format!("{stem}_thumb.{ext}");
    if !is_safe_filename(&thumb) {
        return String::new();
    }
    format!("{MEDIA_URL_PREFIX}{thumb}")
}

fn resize_to_max_edge(img: DynamicImage, max_edge: u32) -> DynamicImage {
    let (w, h) = (img.width(), img.height());
    if w <= max_edge && h <= max_edge {
        return img;
    }
    if w >= h {
        img.resize(max_edge, u32::MAX, FilterType::Triangle)
    } else {
        img.resize(u32::MAX, max_edge, FilterType::Triangle)
    }
}

fn encode_jpeg(img: &DynamicImage, quality: u8) -> AppResult<Vec<u8>> {
    let rgb = img.to_rgb8();
    let mut buf = Cursor::new(Vec::new());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    encoder
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(|e| AppError::bad_request(format!("图片编码失败: {e}")))?;
    Ok(buf.into_inner())
}

fn encode_png_fast(img: &DynamicImage) -> AppResult<Vec<u8>> {
    let rgba = img.to_rgba8();
    let mut buf = Vec::new();
    let encoder =
        PngEncoder::new_with_quality(&mut buf, CompressionType::Fast, PngFilterType::Adaptive);
    encoder
        .write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(|e| AppError::bad_request(format!("图片编码失败: {e}")))?;
    Ok(buf)
}

fn encode_main(img: &DynamicImage) -> AppResult<(Vec<u8>, &'static str, &'static str)> {
    if img.color().has_alpha() {
        let out = encode_png_fast(img)?;
        Ok((out, "png", "image/png"))
    } else {
        let out = encode_jpeg(img, JPEG_QUALITY)?;
        Ok((out, "jpg", "image/jpeg"))
    }
}

struct ProcessedImage {
    main: Vec<u8>,
    ext: &'static str,
    mime: &'static str,
    thumb: Vec<u8>,
    thumb_ext: &'static str,
}

fn process_image_pair(bytes: &[u8]) -> AppResult<ProcessedImage> {
    let src_ext = detect_image_ext(bytes).ok_or_else(|| AppError::bad_request("无法识别图片"))?;
    if src_ext == "gif" || src_ext == "ico" {
        return Err(AppError::bad_request("请使用位图处理路径"));
    }

    let img = image::load_from_memory(bytes).map_err(|_| AppError::bad_request("图片内容无效"))?;

    let skip_main = src_ext == "jpg"
        && bytes.len() <= SKIP_REENCODE_JPEG_BYTES
        && img.width() <= MAIN_MAX_EDGE
        && img.height() <= MAIN_MAX_EDGE;

    let (main, ext, mime, thumb) = if skip_main {
        let thumb_img = resize_to_max_edge(img, THUMB_MAX_EDGE);
        let thumb = encode_jpeg(&thumb_img, JPEG_QUALITY)?;
        (bytes.to_vec(), "jpg", "image/jpeg", thumb)
    } else {
        let main_img = resize_to_max_edge(img, MAIN_MAX_EDGE);
        let (main, ext, mime) = encode_main(&main_img)?;
        let thumb_img = resize_to_max_edge(main_img, THUMB_MAX_EDGE);
        let thumb = encode_jpeg(&thumb_img, JPEG_QUALITY)?;
        (main, ext, mime, thumb)
    };

    Ok(ProcessedImage {
        main,
        ext,
        mime,
        thumb,
        thumb_ext: "jpg",
    })
}

fn process_bitmap(
    bytes: &[u8],
    max_edge: u32,
    _make_jpeg_if_opaque: bool,
) -> AppResult<(Vec<u8>, &'static str, &'static str)> {
    let ext = detect_image_ext(bytes).ok_or_else(|| AppError::bad_request("无法识别图片"))?;
    if ext == "gif" || ext == "ico" {
        let mime = ext_mime(ext)
            .map(|(_, m)| m)
            .unwrap_or("application/octet-stream");
        return Ok((bytes.to_vec(), ext, mime));
    }

    let img = image::load_from_memory(bytes).map_err(|_| AppError::bad_request("图片内容无效"))?;
    let img = resize_to_max_edge(img, max_edge);
    encode_main(&img)
}

async fn write_file(dir: &Path, filename: &str, bytes: &[u8]) -> AppResult<PathBuf> {
    if !is_safe_filename(filename) {
        return Err(AppError::bad_request("非法文件名"));
    }
    let path = dir.join(filename);

    if path.file_name().and_then(|s| s.to_str()) != Some(filename) {
        return Err(AppError::bad_request("非法上传路径"));
    }
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|e| AppError::bad_request(format!("保存失败: {e}")))?;
    Ok(path)
}

async fn save_bytes(
    bytes: &[u8],
    dir: &Path,
    url_prefix: &str,
    max: usize,
    allow_ico: bool,
    compress: bool,
) -> AppResult<String> {
    if bytes.is_empty() {
        return Err(AppError::bad_request("上传文件为空"));
    }
    if bytes.len() > max {
        return Err(AppError::bad_request("文件过大"));
    }
    let ext = detect_image_ext(bytes).ok_or_else(|| {
        AppError::bad_request("仅支持 PNG / JPEG / WebP / GIF（Favicon 另支持 ICO）")
    })?;
    if ext == "ico" && !allow_ico {
        return Err(AppError::bad_request(
            "该位置不支持 ICO，请使用 PNG / JPEG / WebP",
        ));
    }

    tokio::fs::create_dir_all(dir)
        .await
        .map_err(|e| AppError::bad_request(format!("无法创建上传目录: {e}")))?;

    let (data, out_ext) = if compress && ext != "gif" && ext != "ico" {
        let bytes = bytes.to_vec();
        let (out, e, _) =
            tokio::task::spawn_blocking(move || process_bitmap(&bytes, MAIN_MAX_EDGE, true))
                .await
                .map_err(|e| AppError::bad_request(format!("图片处理失败: {e}")))??;
        (out, e)
    } else {
        (bytes.to_vec(), ext)
    };

    let filename = format!("{}.{}", uuid::Uuid::new_v4(), out_ext);
    write_file(dir, &filename, &data).await?;
    Ok(format!("{url_prefix}{filename}"))
}

pub async fn save_post_image(bytes: &[u8], posts_dir: &Path) -> AppResult<String> {
    save_bytes(
        bytes,
        posts_dir,
        POSTS_URL_PREFIX,
        MAX_POST_IMAGE,
        false,
        true,
    )
    .await
}

pub async fn save_media(
    bytes: &[u8],
    original_name: &str,
    media_dir: &Path,
) -> AppResult<SavedMedia> {
    if bytes.is_empty() {
        return Err(AppError::bad_request("上传文件为空"));
    }
    if bytes.len() > MAX_MEDIA {
        return Err(AppError::bad_request("文件过大（最大 5MB）"));
    }

    let display_name = sanitize_original_name(original_name);
    let name_ext = Path::new(&display_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let detected = detect_image_ext(bytes);
    let is_bitmap = matches!(detected, Some("png" | "jpg" | "webp"));
    let is_gif = detected == Some("gif");

    tokio::fs::create_dir_all(media_dir)
        .await
        .map_err(|e| AppError::bad_request(format!("无法创建上传目录: {e}")))?;

    let id = uuid::Uuid::new_v4();

    if is_bitmap {
        let bytes = bytes.to_vec();
        let started = std::time::Instant::now();
        let processed = tokio::task::spawn_blocking(move || process_image_pair(&bytes))
            .await
            .map_err(|e| AppError::bad_request(format!("图片处理失败: {e}")))??;
        tracing::debug!(
            elapsed_ms = started.elapsed().as_millis() as u64,
            main_kb = processed.main.len() / 1024,
            "media image processed"
        );

        let stored = format!("{id}.{}", processed.ext);
        write_file(media_dir, &stored, &processed.main).await?;
        let thumb_name = format!("{id}_thumb.{}", processed.thumb_ext);
        let _ = write_file(media_dir, &thumb_name, &processed.thumb).await;

        return Ok(SavedMedia {
            name: display_name,
            url: format!("{MEDIA_URL_PREFIX}{stored}"),
            mime: processed.mime.into(),
            size: processed.main.len() as i64,
        });
    }

    let (data, ext, mime) = if is_gif {
        (bytes.to_vec(), "gif", "image/gif")
    } else if detected.is_some() {
        return Err(AppError::bad_request("不支持的图片类型"));
    } else {
        let (e, m) = ext_mime(&name_ext).ok_or_else(|| {
            AppError::bad_request(
                "仅支持 PNG / JPEG / WebP / GIF / PDF / ZIP / TXT / MD / CSV / JSON",
            )
        })?;
        (bytes.to_vec(), e, m)
    };

    let stored = format!("{id}.{ext}");
    write_file(media_dir, &stored, &data).await?;

    Ok(SavedMedia {
        name: display_name,
        url: format!("{MEDIA_URL_PREFIX}{stored}"),
        mime: mime.into(),
        size: data.len() as i64,
    })
}

pub async fn delete_upload(url: &str, prefix: &str, dir: &Path) {
    let safe = sanitize_asset_url(url, prefix);
    let Some(name) = safe.strip_prefix(prefix) else {
        return;
    };
    let path = dir.join(name);
    let _ = tokio::fs::remove_file(path).await;
}

pub async fn delete_site_image(url: &str, site_dir: &Path) {
    delete_upload(url, SITE_URL_PREFIX, site_dir).await;
}

pub async fn delete_post_image(url: &str, posts_dir: &Path) {
    delete_upload(url, POSTS_URL_PREFIX, posts_dir).await;
}

pub async fn delete_media_file(url: &str, media_dir: &Path) {
    let thumb = media_thumb_url(url);
    delete_upload(url, MEDIA_URL_PREFIX, media_dir).await;
    if !thumb.is_empty() {
        delete_upload(&thumb, MEDIA_URL_PREFIX, media_dir).await;
    }
}
