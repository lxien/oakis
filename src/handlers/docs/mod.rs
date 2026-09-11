use serde::Deserialize;

use crate::infra::markdown::slugify;
use crate::models::KbTreeNode;

pub mod admin;
pub mod public;

pub use admin::{
    admin_book_create, admin_book_delete, admin_book_page, admin_book_save,
    admin_book_settings_page, admin_docs_list, admin_node_create, admin_node_delete,
    admin_node_move, admin_node_rename, admin_node_save,
};
pub use public::{docs_book, docs_home, docs_page};

#[derive(Debug, Deserialize)]
pub struct NodeQuery {
    pub id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct BookForm {
    pub title: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub cover_url: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub visibility: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateNodeForm {
    #[serde(default)]
    pub parent_id: String,
    #[serde(default)]
    pub node_type: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveNodeForm {
    pub title: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub content_md: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub visibility: String,
    #[serde(default)]
    pub published_at: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameNodeForm {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct MoveNodeForm {
    #[serde(default)]
    pub parent_id: String,
    #[serde(default)]
    pub before_id: String,
}

fn status_value(status: &str) -> &'static str {
    if status == "published" {
        "published"
    } else {
        "draft"
    }
}

fn visibility_value(visibility: &str) -> &'static str {
    if visibility == "private" {
        "private"
    } else {
        "public"
    }
}

fn normalize_slug(title: &str, slug: &str) -> String {
    let slug = slug.trim();
    if slug.is_empty() {
        slugify(title)
    } else {
        slugify(slug)
    }
}

fn parse_parent_id(raw: &str) -> Option<i64> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    s.parse().ok().filter(|id| *id > 0)
}

fn html_escape(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
}

fn tree_html(
    nodes: &[KbTreeNode],
    active_id: Option<i64>,
    book_id: i64,
    book_slug: &str,
    admin: bool,
) -> String {
    fn walk(
        out: &mut String,
        nodes: &[KbTreeNode],
        active_id: Option<i64>,
        book_id: i64,
        book_slug: &str,
        admin: bool,
        parent_id: Option<i64>,
        depth: usize,
    ) {
        if nodes.is_empty() {
            return;
        }
        out.push_str("<ul class=\"kb-tree-list\">");
        for n in nodes {
            let active = active_id == Some(n.id);
            let as_branch = n.is_folder() || n.has_children();
            out.push_str("<li class=\"kb-tree-item");
            if n.is_folder() {
                out.push_str(" is-folder");
            } else {
                out.push_str(" is-doc");
            }
            if as_branch {
                out.push_str(" has-children");
            }
            if active {
                out.push_str(" is-active");
            }
            out.push_str("\" style=\"--kb-depth:");
            out.push_str(&depth.to_string());
            out.push_str("\" data-id=\"");
            out.push_str(&n.id.to_string());
            out.push_str("\" data-type=\"");
            out.push_str(if n.is_folder() { "folder" } else { "doc" });
            out.push_str("\" data-parent-id=\"");
            if let Some(pid) = parent_id {
                out.push_str(&pid.to_string());
            }
            out.push_str("\"");
            if admin {
                out.push_str(" draggable=\"true\"");
            }
            out.push_str(">");

            let href = if admin {
                format!("/admin/docs/{book_id}?id={}", n.id)
            } else if n.is_doc() {
                format!("/docs/{book_slug}/{}", n.slug)
            } else {
                String::new()
            };

            if as_branch {
                out.push_str("<details class=\"kb-branch\">");
                out.push_str("<summary class=\"kb-row\">");
                out.push_str("<span class=\"kb-chevron\" aria-hidden=\"true\"></span>");
                if href.is_empty() {
                    out.push_str("<span class=\"kb-tree-link kb-tree-folder-label\">");
                    html_escape(out, &n.title);
                    out.push_str("</span>");
                } else {
                    out.push_str("<a class=\"kb-tree-link\" href=\"");
                    out.push_str(&href);
                    out.push_str("\">");
                    html_escape(out, &n.title);
                    out.push_str("</a>");
                }
                if admin {
                    push_admin_actions(out, n, book_id, book_slug);
                }
                out.push_str("</summary>");
                walk(
                    out,
                    &n.children,
                    active_id,
                    book_id,
                    book_slug,
                    admin,
                    Some(n.id),
                    depth + 1,
                );
                out.push_str("</details>");
            } else {
                out.push_str("<div class=\"kb-row\">");
                out.push_str(
                    "<span class=\"kb-chevron kb-chevron-leaf\" aria-hidden=\"true\"></span>",
                );
                out.push_str("<a class=\"kb-tree-link\" href=\"");
                out.push_str(&href);
                out.push_str("\">");
                html_escape(out, &n.title);
                out.push_str("</a>");
                if admin {
                    push_admin_actions(out, n, book_id, book_slug);
                }
                out.push_str("</div>");
            }
            out.push_str("</li>");
        }
        out.push_str("</ul>");
    }

    let mut out = String::new();
    walk(
        &mut out, nodes, active_id, book_id, book_slug, admin, None, 0,
    );
    out
}

fn format_kb_time(iso: &str) -> String {
    use chrono::{Datelike, Local};

    let Some(dt) = crate::infra::timefmt::parse_to_local(iso) else {
        return crate::infra::timefmt::format_local(iso);
    };

    let today = Local::now().date_naive();
    let d = dt.date_naive();
    if d == today {
        format!("今天 {}", dt.format("%H:%M"))
    } else if d == today - chrono::Duration::days(1) {
        format!("昨天 {}", dt.format("%H:%M"))
    } else if d.year() == today.year() {
        dt.format("%m-%d %H:%M").to_string()
    } else {
        dt.format("%Y-%m-%d %H:%M").to_string()
    }
}

fn catalog_html(nodes: &[KbTreeNode], book_slug: &str, admin_book_id: Option<i64>) -> String {
    fn push_body_inner(out: &mut String, n: &KbTreeNode) {
        out.push_str("<span class=\"docs-catalog-title\">");
        html_escape(out, &n.title);
        out.push_str("</span>");
        out.push_str("<span class=\"docs-catalog-leader\" aria-hidden=\"true\"></span>");
        out.push_str("<time class=\"docs-catalog-time\" datetime=\"");
        html_escape(out, &n.updated_at);
        out.push_str("\">");
        html_escape(out, &format_kb_time(&n.updated_at));
        out.push_str("</time>");
    }

    fn push_row(
        out: &mut String,
        n: &KbTreeNode,
        book_slug: &str,
        admin_book_id: Option<i64>,
        branch: bool,
        open: bool,
    ) {
        out.push_str("<div class=\"docs-catalog-row\">");
        if branch {
            out.push_str("<button type=\"button\" class=\"docs-catalog-toggle\" aria-label=\"展开或折叠\" aria-expanded=\"");
            out.push_str(if open { "true" } else { "false" });
            out.push_str(
                "\"><span class=\"docs-catalog-chevron\" aria-hidden=\"true\"></span></button>",
            );
        } else {
            out.push_str("<span class=\"docs-catalog-toggle docs-catalog-toggle-leaf\" aria-hidden=\"true\"></span>");
        }

        let href = if n.is_doc() && !n.slug.is_empty() {
            Some(match admin_book_id {
                Some(bid) => format!("/admin/docs/{bid}?id={}", n.id),
                None => format!("/docs/{book_slug}/{}", n.slug),
            })
        } else {
            None
        };

        if let Some(href) = href {
            out.push_str("<a class=\"docs-catalog-body\" href=\"");
            out.push_str(&href);
            out.push_str("\">");
            push_body_inner(out, n);
            out.push_str("</a>");
        } else {
            out.push_str("<div class=\"docs-catalog-body is-folder\">");
            push_body_inner(out, n);
            out.push_str("</div>");
        }
        out.push_str("</div>");
    }

    fn walk(
        out: &mut String,
        nodes: &[KbTreeNode],
        book_slug: &str,
        admin_book_id: Option<i64>,
        depth: usize,
    ) {
        if nodes.is_empty() {
            return;
        }
        out.push_str("<ul class=\"docs-catalog-list\">");
        for n in nodes {
            let branch = n.has_children();
            let open = false;
            out.push_str("<li class=\"docs-catalog-item");
            if n.is_folder() {
                out.push_str(" is-folder");
            } else {
                out.push_str(" is-doc");
            }
            if branch {
                out.push_str(" has-children");
            }
            if open {
                out.push_str(" is-open");
            }
            out.push_str("\" style=\"--kb-depth:");
            out.push_str(&depth.to_string());
            out.push_str("\">");
            push_row(out, n, book_slug, admin_book_id, branch, open);
            if branch {
                walk(out, &n.children, book_slug, admin_book_id, depth + 1);
            }
            out.push_str("</li>");
        }
        out.push_str("</ul>");
    }

    let mut out = String::new();
    walk(&mut out, nodes, book_slug, admin_book_id, 0);
    out
}

fn push_admin_actions(out: &mut String, n: &KbTreeNode, _book_id: i64, book_slug: &str) {
    out.push_str("<span class=\"kb-row-actions\">");

    out.push_str("<span class=\"kb-action-wrap\">");
    out.push_str(
        "<button type=\"button\" class=\"kb-icon-btn\" data-kb-more aria-label=\"更多\">⋮</button>",
    );
    out.push_str("<div class=\"kb-menu\" hidden>");
    out.push_str("<button type=\"button\" data-kb-rename data-id=\"");
    out.push_str(&n.id.to_string());
    out.push_str("\" data-title=\"");
    html_escape(out, &n.title);
    out.push_str("\">重命名</button>");
    if n.is_doc() {
        out.push_str("<a href=\"/docs/");
        out.push_str(book_slug);
        out.push_str("/");
        out.push_str(&n.slug);
        out.push_str("\" target=\"_blank\" rel=\"noopener\">预览</a>");
        out.push_str("<button type=\"button\" data-kb-copy=\"/docs/");
        out.push_str(book_slug);
        out.push_str("/");
        out.push_str(&n.slug);
        out.push_str("\">复制链接</button>");
        out.push_str("<button type=\"button\" data-export=\"doc\" data-export-id=\"");
        out.push_str(&n.id.to_string());
        out.push_str("\">导出</button>");
    }
    out.push_str("<hr>");
    out.push_str("<button type=\"button\" class=\"is-danger\" data-kb-delete data-id=\"");
    out.push_str(&n.id.to_string());
    out.push_str("\" data-type=\"");
    out.push_str(if n.is_folder() { "folder" } else { "doc" });
    out.push_str("\">删除</button>");
    out.push_str("</div></span>");

    out.push_str("<span class=\"kb-action-wrap\">");
    out.push_str("<button type=\"button\" class=\"kb-icon-btn\" data-kb-add title=\"新增\" aria-label=\"新增\">+</button>");
    out.push_str("<div class=\"kb-menu\" hidden>");
    out.push_str("<button type=\"button\" data-kb-create=\"folder\" data-parent=\"");
    out.push_str(&n.id.to_string());
    out.push_str("\">文件夹</button>");
    out.push_str("<button type=\"button\" data-kb-create=\"doc\" data-parent=\"");
    out.push_str(&n.id.to_string());
    out.push_str("\">文档</button>");
    out.push_str("</div></span>");

    if n.is_doc() && n.is_draft() {
        out.push_str("<span class=\"kb-badge\">草稿</span>");
    }
    if n.is_doc() && n.is_private() {
        out.push_str("<span class=\"kb-badge\">私密</span>");
    }
    out.push_str("</span>");
}
