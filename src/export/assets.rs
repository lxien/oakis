use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::infra::upload::resolve_upload_file;

pub type AssetMap = HashMap<String, PathBuf>;

pub fn rewrite(md: &str, upload_dir: &Path, assets_prefix: &str, assets: &mut AssetMap) -> String {
    let mut out = String::with_capacity(md.len());
    let mut rest = md;

    while let Some(idx) = rest.find("](") {
        out.push_str(&rest[..idx + 2]);
        rest = &rest[idx + 2..];

        let end = rest.find(|c| c == ')' || c == '\n').unwrap_or(rest.len());
        let url = rest[..end].trim();

        if let Some(name) = take_upload(url, upload_dir, assets) {
            out.push_str(assets_prefix);
            out.push_str(&name);
        } else {
            out.push_str(&rest[..end]);
        }
        rest = &rest[end..];
    }

    out.push_str(rest);
    out
}

pub fn collect(md: &str, upload_dir: &Path, assets: &mut AssetMap) {
    let _ = rewrite(md, upload_dir, "assets/", assets);
}

fn take_upload(url: &str, upload_dir: &Path, assets: &mut AssetMap) -> Option<String> {
    let path = resolve_upload_file(url, upload_dir)?;

    if let Some((existing, _)) = assets.iter().find(|(_, p)| *p == &path) {
        return Some(existing.clone());
    }

    let base = path.file_name()?.to_str()?.to_string();
    let name = unique_name(&base, assets);
    assets.insert(name.clone(), path);
    Some(name)
}

fn unique_name(base: &str, assets: &AssetMap) -> String {
    if !assets.contains_key(base) {
        return base.to_string();
    }
    let path = Path::new(base);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|e| format!(".{e}"))
        .unwrap_or_default();
    for n in 2..10_000 {
        let candidate = format!("{stem}-{n}{ext}");
        if !assets.contains_key(&candidate) {
            return candidate;
        }
    }
    format!("{stem}-dup{ext}")
}
