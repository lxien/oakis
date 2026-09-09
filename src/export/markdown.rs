use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::path::Path;

use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use super::assets::{self, AssetMap};
use super::{ExportArtifact, MarkdownBook, MarkdownDoc, safe_filename};
use crate::infra::error::{AppError, AppResult};
use crate::models::KbNode;

pub struct MarkdownExporter;

impl MarkdownExporter {
    pub fn export_doc(doc: MarkdownDoc<'_>, upload_dir: &Path) -> AppResult<ExportArtifact> {
        let mut assets = AssetMap::new();
        let body = assets::rewrite(doc.body_md, upload_dir, "assets/", &mut assets);
        let md = with_title(doc.title, &body);
        let base = safe_filename(doc.title);

        if assets.is_empty() {
            return Ok(ExportArtifact {
                filename: format!("{base}.md"),
                content_type: "text/markdown; charset=utf-8".into(),
                body: md.into_bytes(),
            });
        }

        let mut zip_buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut zip_buf);
            let opts = file_opts();
            write_str(&mut zip, &format!("{base}.md"), &md, opts)?;
            write_assets(&mut zip, &assets, "assets/", opts)?;
            zip.finish()
                .map_err(|_| AppError::bad_request("导出失败"))?;
        }

        Ok(ExportArtifact {
            filename: format!("{base}.zip"),
            content_type: "application/zip".into(),
            body: zip_buf.into_inner(),
        })
    }

    pub fn export_book(book: MarkdownBook<'_>, upload_dir: &Path) -> AppResult<ExportArtifact> {
        let root = safe_filename(&book.book.title);
        let paths = build_node_paths(book.nodes);

        let mut assets = AssetMap::new();
        for node in book.nodes.iter().filter(|n| n.is_doc()) {
            assets::collect(&node.content_md, upload_dir, &mut assets);
        }

        let mut zip_buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut zip_buf);
            let opts = file_opts();

            for node in book.nodes.iter().filter(|n| n.is_folder()) {
                if let Some(rel) = paths.get(&node.id) {
                    let dir = format!("{root}/{rel}/");
                    zip.add_directory(&dir, opts)
                        .map_err(|_| AppError::bad_request("导出失败"))?;
                }
            }

            for node in book.nodes.iter().filter(|n| n.is_doc()) {
                let Some(rel) = paths.get(&node.id) else {
                    continue;
                };
                let depth = rel.matches('/').count();
                let prefix = assets_prefix(depth);
                let body = assets::rewrite(&node.content_md, upload_dir, &prefix, &mut assets);
                let md = with_title(&node.title, &body);
                let entry = format!("{root}/{rel}.md");
                write_str(&mut zip, &entry, &md, opts)?;
            }

            if !assets.is_empty() {
                write_assets(&mut zip, &assets, &format!("{root}/assets/"), opts)?;
            }

            zip.finish()
                .map_err(|_| AppError::bad_request("导出失败"))?;
        }

        Ok(ExportArtifact {
            filename: format!("{root}.zip"),
            content_type: "application/zip".into(),
            body: zip_buf.into_inner(),
        })
    }
}

fn with_title(title: &str, body: &str) -> String {
    let body = body.trim();
    if body.is_empty() {
        format!("# {title}\n")
    } else {
        format!("# {title}\n\n{body}\n")
    }
}

fn assets_prefix(depth: usize) -> String {
    if depth == 0 {
        "assets/".into()
    } else {
        format!("{}assets/", "../".repeat(depth))
    }
}

fn file_opts() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

fn write_str<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    name: &str,
    content: &str,
    opts: SimpleFileOptions,
) -> AppResult<()> {
    zip.start_file(name, opts)
        .map_err(|_| AppError::bad_request("导出失败"))?;
    zip.write_all(content.as_bytes())
        .map_err(|_| AppError::bad_request("导出失败"))?;
    Ok(())
}

fn write_assets<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    assets: &AssetMap,
    dir_prefix: &str,
    opts: SimpleFileOptions,
) -> AppResult<()> {
    let mut names: Vec<_> = assets.keys().cloned().collect();
    names.sort();
    for name in names {
        let path = &assets[&name];
        let bytes = std::fs::read(path).map_err(|_| AppError::bad_request("读取资源失败"))?;
        let entry = format!("{dir_prefix}{name}");
        zip.start_file(&entry, opts)
            .map_err(|_| AppError::bad_request("导出失败"))?;
        zip.write_all(&bytes)
            .map_err(|_| AppError::bad_request("导出失败"))?;
    }
    Ok(())
}

fn build_node_paths(nodes: &[KbNode]) -> HashMap<i64, String> {
    let by_id: HashMap<i64, &KbNode> = nodes.iter().map(|n| (n.id, n)).collect();
    let mut used: HashMap<(Option<i64>, String), usize> = HashMap::new();
    let mut out = HashMap::new();

    for node in nodes {
        let mut chain = Vec::new();
        let mut cur = Some(node.id);
        let mut guard = 0;
        while let Some(id) = cur {
            guard += 1;
            if guard > 64 {
                break;
            }
            let Some(n) = by_id.get(&id) else { break };
            chain.push(*n);
            cur = n.parent_id;
        }
        chain.reverse();

        let mut parts = Vec::new();
        for (i, n) in chain.iter().enumerate() {
            let parent = if i == 0 { None } else { Some(chain[i - 1].id) };
            let base = safe_filename(&n.title);
            let key = (parent, base.clone());
            let count = used.entry(key).or_insert(0);
            *count += 1;
            let name = if *count == 1 {
                base
            } else {
                format!("{base}-{}", n.id)
            };
            parts.push(name);
        }
        out.insert(node.id, parts.join("/"));
    }
    out
}
