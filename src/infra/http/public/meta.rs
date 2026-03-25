use crate::presentation::views::{LayoutChrome, PageMetaView, PageView, PostDetailContext};

pub(super) fn post_meta(
    chrome: &LayoutChrome,
    content: &PostDetailContext,
    canonical: String,
) -> PageMetaView {
    let description = fallback_description(&content.excerpt, &chrome.meta.description);

    chrome
        .meta
        .clone()
        .with_canonical(canonical)
        .with_content(content.title.clone(), description)
}

pub(super) fn page_meta(chrome: &LayoutChrome, page: &PageView, canonical: String) -> PageMetaView {
    let derived = summarize_html(&page.content_html, 180);
    let description = fallback_description(&derived, &chrome.meta.description);

    chrome
        .meta
        .clone()
        .with_canonical(canonical)
        .with_content(page.title.clone(), description)
}

pub(super) fn canonical_url(base: &str, path: &str) -> String {
    let root = normalize_public_site_url(base);
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        root.clone()
    } else {
        format!("{root}{trimmed}")
    }
}

fn fallback_description(candidate: &str, fallback: &str) -> String {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn summarize_html(html: &str, max_len: usize) -> String {
    let mut text = String::with_capacity(max_len);
    let mut in_tag = false;
    let mut last_was_space = false;

    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
                continue;
            }
            '>' => {
                in_tag = false;
                last_was_space = false;
                continue;
            }
            _ if in_tag => continue,
            c if c.is_whitespace() => {
                if !last_was_space && !text.is_empty() {
                    text.push(' ');
                }
                last_was_space = true;
            }
            c => {
                text.push(c);
                last_was_space = false;
            }
        }

        if text.len() >= max_len {
            break;
        }
    }

    text.trim().to_string()
}

fn normalize_public_site_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    format!("{trimmed}/")
}
