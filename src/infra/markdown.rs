use std::borrow::Cow;
use std::collections::HashSet;

use ammonia::{Builder, UrlRelative};
use pulldown_cmark::{CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};

fn allow_content_relative(url: &str) -> Option<Cow<'_, str>> {
    let url = url.trim();
    if url.is_empty() || url.contains('\0') || url.contains('\\') {
        return None;
    }
    if url.starts_with('#') {
        return Some(Cow::Borrowed(url));
    }
    if url.starts_with("//") {
        return None;
    }
    if url.starts_with('/') {
        for prefix in [
            crate::infra::upload::MEDIA_URL_PREFIX,
            crate::infra::upload::POSTS_URL_PREFIX,
            crate::infra::upload::SITE_URL_PREFIX,
        ] {
            if url.starts_with(prefix) {
                let safe = crate::infra::upload::sanitize_asset_url(url, prefix);
                return if safe.is_empty() {
                    None
                } else if safe == url {
                    Some(Cow::Borrowed(url))
                } else {
                    Some(Cow::Owned(safe))
                };
            }
        }
        return Some(Cow::Borrowed(url));
    }
    None
}

fn wrap_tables_in_events(events: &mut Vec<Event<'static>>) {
    let mut i = 0;
    while i < events.len() {
        if matches!(&events[i], Event::Start(Tag::Table(_))) {
            events.insert(i, Event::Html(CowStr::from(r#"<div class="md-table">"#)));
            i += 1;
            let mut depth = 1usize;
            let mut j = i + 1;
            while j < events.len() {
                match &events[j] {
                    Event::Start(Tag::Table(_)) => depth += 1,
                    Event::End(TagEnd::Table) => {
                        depth -= 1;
                        if depth == 0 {
                            events.insert(j + 1, Event::Html(CowStr::from("</div>")));
                            i = j + 1;
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
        }
        i += 1;
    }
}

fn clean_html(html_output: &str) -> String {
    Builder::default()
        .link_rel(Some("noopener noreferrer"))
        .add_tags(&["input"])
        .add_tag_attributes("code", &["class"])
        .add_tag_attributes("pre", &["class"])
        .add_tag_attributes("h2", &["id"])
        .add_tag_attributes("h3", &["id"])
        .add_tag_attributes("div", &["id"])
        .add_tag_attributes("td", &["style"])
        .add_tag_attributes("th", &["style"])
        .add_tag_attributes("input", &["checked"])
        .add_tag_attribute_values("input", "type", &["checkbox"])
        .set_tag_attribute_value("input", "disabled", "")
        .add_allowed_classes("div", &["md-table", "footnote-definition"])
        .add_allowed_classes("sup", &["footnote-reference", "footnote-definition-label"])
        .filter_style_properties(HashSet::from(["text-align"]))
        .url_relative(UrlRelative::Custom(Box::new(allow_content_relative)))
        .clean(html_output)
        .to_string()
}

fn md_options() -> Options {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options
}

#[derive(Debug, Clone)]
pub struct TocItem {
    pub level: u8,
    pub id: String,
    pub text: String,
}

fn unique_id(base: &str, used: &mut HashSet<String>) -> String {
    let id = if base.is_empty() {
        "section".into()
    } else {
        base.to_string()
    };
    if used.insert(id.clone()) {
        return id;
    }
    let mut n = 2u32;
    loop {
        let candidate = format!("{id}-{n}");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        n += 1;
    }
}

fn plain_text_from_events(events: &[Event<'_>]) -> String {
    let mut text = String::new();
    for ev in events {
        match ev {
            Event::Text(t) | Event::Code(t) => text.push_str(t),
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            _ => {}
        }
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn render_markdown_with_toc(md: &str) -> (String, Vec<TocItem>) {
    let parser = Parser::new_ext(md, md_options());
    let mut events: Vec<Event<'_>> = parser.map(Event::into_static).collect();
    let mut toc = Vec::new();
    let mut used_ids = HashSet::new();

    let mut i = 0;
    while i < events.len() {
        let level = match &events[i] {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                ..
            }) => Some(2u8),
            Event::Start(Tag::Heading {
                level: HeadingLevel::H3,
                ..
            }) => Some(3u8),
            _ => None,
        };

        if let Some(level) = level {
            let mut j = i + 1;
            while j < events.len() {
                if matches!(
                    &events[j],
                    Event::End(TagEnd::Heading(HeadingLevel::H2 | HeadingLevel::H3))
                ) {
                    break;
                }
                j += 1;
            }
            let text = plain_text_from_events(&events[i + 1..j]);
            let id = unique_id(&slugify(&text), &mut used_ids);
            if let Event::Start(Tag::Heading { id: heading_id, .. }) = &mut events[i] {
                *heading_id = Some(CowStr::from(id.clone()));
            }
            if !text.is_empty() {
                toc.push(TocItem { level, id, text });
            }
            i = j;
        }
        i += 1;
    }

    wrap_tables_in_events(&mut events);

    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());
    (clean_html(&html_output), toc)
}

pub fn render_markdown(md: &str) -> String {
    render_markdown_with_toc(md).0
}

pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if ch.is_alphanumeric() {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        format!("post-{}", &uuid::Uuid::new_v4().to_string()[..8])
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_gfm_tables_with_scroll_wrapper_and_alignment() {
        let md = "\
| A | B | C |
|:-|:-:|--:|
| 1 | 2 | 3 |
";
        let html = render_markdown(md);
        assert!(html.contains(r#"class="md-table""#));
        assert!(html.contains("<table>"));
        assert!(html.contains("</table></div>") || html.contains("</table>\n</div>"));
        assert!(html.contains("<thead>"));
        assert!(html.contains("<tbody>"));
        assert!(
            html.contains(r#"style="text-align:center""#)
                || html.contains(r#"style="text-align: center""#)
        );
        assert!(
            html.contains(r#"style="text-align:right""#)
                || html.contains(r#"style="text-align: right""#)
        );
    }

    #[test]
    fn preserves_task_lists() {
        let html = render_markdown("- [ ] todo\n- [x] done\n");
        assert!(html.contains(r#"type="checkbox""#));
        assert!(html.contains("disabled"));
        assert!(html.contains("checked"));
        assert!(!html.contains(r#"type="text""#));
    }

    #[test]
    fn preserves_footnotes_and_fragment_links() {
        let md = "See note[^1].\n\n[^1]: Foot body\n";
        let html = render_markdown(md);
        assert!(html.contains(r#"class="footnote-reference""#));
        assert!(html.contains(r#"class="footnote-definition""#));
        assert!(html.contains(r##"href="#1""##) || html.contains(r##"href="#fn1""##));
        assert!(html.contains(r##"id="1""##) || html.contains(r##"id="fn1""##));
    }

    #[test]
    fn preserves_site_and_fragment_links_strips_protocol_relative() {
        let html = render_markdown(
            "[post](/posts/hello) [sec](#section) [bad](//evil.example/x) [ok](https://example.com)\n",
        );
        assert!(html.contains(r#"href="/posts/hello""#));
        assert!(html.contains(r##"href="#section""##));
        assert!(html.contains(r#"href="https://example.com""#));
        assert!(!html.contains("//evil.example"));
    }

    #[test]
    fn preserves_strikethrough() {
        let html = render_markdown("~~gone~~\n");
        assert!(html.contains("<del>") || html.contains("<s>"));
    }

    #[test]
    fn forces_task_checkbox_disabled_even_from_raw_html() {
        let html = render_markdown(r#"<input type="checkbox">"#);
        if html.contains("checkbox") {
            assert!(html.contains("disabled"));
        }
        let html2 = render_markdown(r#"<input type="text" name="x">"#);
        assert!(!html2.contains(r#"type="text""#));
        assert!(!html2.contains("name="));
    }
}
