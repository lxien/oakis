mod assets;
mod markdown;

use std::path::Path;

use crate::infra::error::{AppError, AppResult};
use crate::models::{KbBook, KbNode, Page, Post};

pub use markdown::MarkdownExporter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportSource {
    Post,
    Page,
    Doc,
    Book,
}

impl ExportSource {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "post" => Some(Self::Post),
            "page" => Some(Self::Page),
            "doc" => Some(Self::Doc),
            "book" => Some(Self::Book),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Markdown,
}

impl ExportFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "markdown" | "md" => Some(Self::Markdown),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ExportArtifact {
    pub filename: String,
    pub content_type: String,
    pub body: Vec<u8>,
}

pub struct MarkdownDoc<'a> {
    pub title: &'a str,
    pub body_md: &'a str,
}

pub struct MarkdownBook<'a> {
    pub book: &'a KbBook,
    pub nodes: &'a [KbNode],
}

pub fn build_post(
    post: &Post,
    upload_dir: &Path,
    format: ExportFormat,
) -> AppResult<ExportArtifact> {
    match format {
        ExportFormat::Markdown => MarkdownExporter::export_doc(
            MarkdownDoc {
                title: &post.title,
                body_md: &post.content_md,
            },
            upload_dir,
        ),
    }
}

pub fn build_page(
    page: &Page,
    upload_dir: &Path,
    format: ExportFormat,
) -> AppResult<ExportArtifact> {
    match format {
        ExportFormat::Markdown => MarkdownExporter::export_doc(
            MarkdownDoc {
                title: &page.title,
                body_md: &page.content_md,
            },
            upload_dir,
        ),
    }
}

pub fn build_doc(
    node: &KbNode,
    upload_dir: &Path,
    format: ExportFormat,
) -> AppResult<ExportArtifact> {
    if !node.is_doc() {
        return Err(AppError::bad_request("仅文档可导出"));
    }
    match format {
        ExportFormat::Markdown => MarkdownExporter::export_doc(
            MarkdownDoc {
                title: &node.title,
                body_md: &node.content_md,
            },
            upload_dir,
        ),
    }
}

pub fn build_book(
    book: &KbBook,
    nodes: &[KbNode],
    upload_dir: &Path,
    format: ExportFormat,
) -> AppResult<ExportArtifact> {
    match format {
        ExportFormat::Markdown => {
            MarkdownExporter::export_book(MarkdownBook { book, nodes }, upload_dir)
        }
    }
}

pub fn safe_filename(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .take(80)
        .collect();
    let s = s.trim().trim_matches('.');
    if s.is_empty() {
        "untitled".into()
    } else {
        s.to_string()
    }
}
