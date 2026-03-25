use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

use lol_html::{RewriteStrSettings, element, rewrite_str, text};
use url::Url;

use crate::application::render::types::{ContentMetrics, RenderError, ResourceHints};

const CODE_COPY_BUTTON_HTML: &str = "<button type=\"button\" data-role=\"code-copy-button\" data-copy-label-default=\"Copy\" data-copy-label-success=\"Copied\" data-copy-label-error=\"Copy failed\" data-copy-reset-ms=\"2000\" data-copy-state=\"idle\" aria-label=\"Copy code block\" data-on-click__prevent=\"(@copyCodeBlockText())\">Copy</button>";

#[derive(Default, Clone)]
pub(super) struct AugmentState {
    pub(super) image_domains: BTreeSet<String>,
    pub(super) link_domains: BTreeSet<String>,
    pub(super) internal_links: u32,
    pub(super) external_links: u32,
    pub(super) images: u32,
    pub(super) images_missing_alt: u32,
    pub(super) images_missing_dimensions: u32,
    pub(super) code_blocks: u32,
    pub(super) word_count: u32,
    pub(super) math_blocks: u32,
    pub(super) mermaid_diagrams: u32,
}

pub(super) struct AugmentOutcome {
    pub(super) html: String,
    pub(super) state: AugmentState,
}

fn is_highlighted_code_block(el: &lol_html::html_content::Element<'_, '_>) -> bool {
    let is_syntax_highlight = el
        .get_attribute("class")
        .map(|class| {
            class
                .split_whitespace()
                .any(|token| token == "syntax-highlight")
        })
        .unwrap_or(false);
    is_syntax_highlight || el.get_attribute("data-language").is_some()
}

fn apply_code_block_accessibility(
    el: &mut lol_html::html_content::Element<'_, '_>,
) -> Result<(), lol_html::errors::AttributeNameError> {
    if let Some(lang) = el.get_attribute("data-language") {
        if el.get_attribute("role").is_none() {
            el.set_attribute("role", "region")?;
        }

        if el.get_attribute("aria-label").is_none() {
            let trimmed = lang.trim();
            if !trimmed.is_empty() {
                let label = format!("Code block in {}", trimmed);
                el.set_attribute("aria-label", &label)?;
            }
        }
    }

    Ok(())
}

fn attach_code_copy_button(
    el: &mut lol_html::html_content::Element<'_, '_>,
) -> Result<(), lol_html::errors::AttributeNameError> {
    if !is_highlighted_code_block(el) {
        return Ok(());
    }

    el.set_attribute("data-role", "code-block")?;
    if el.get_attribute("data-copy-enabled").is_none() {
        el.prepend(
            CODE_COPY_BUTTON_HTML,
            lol_html::html_content::ContentType::Html,
        );
        el.set_attribute("data-copy-enabled", "true")?;
    }

    Ok(())
}

pub(super) fn augment_code_blocks_only(html: &str) -> Result<String, RenderError> {
    rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![element!("pre", |el| {
                apply_code_block_accessibility(el)?;
                attach_code_copy_button(el)?;
                Ok(())
            })],
            ..RewriteStrSettings::default()
        },
    )
    .map_err(|err| RenderError::Document {
        message: err.to_string(),
    })
}

pub(super) fn augment_semantics(
    html: &str,
    public_site_url: Option<&Url>,
) -> Result<AugmentOutcome, RenderError> {
    let state = Rc::new(RefCell::new(AugmentState::default()));
    let site_url = Rc::new(public_site_url.cloned());

    let rewritten = rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![
                element!("img", {
                    let state = Rc::clone(&state);
                    move |el| {
                        {
                            let mut state = state.borrow_mut();
                            state.images = state.images.saturating_add(1);
                            let has_width = el.get_attribute("width").is_some();
                            let has_height = el.get_attribute("height").is_some();
                            if !has_width || !has_height {
                                state.images_missing_dimensions =
                                    state.images_missing_dimensions.saturating_add(1);
                            }

                            if el.get_attribute("alt").is_none() {
                                state.images_missing_alt =
                                    state.images_missing_alt.saturating_add(1);
                                if let Some(title) = el.get_attribute("title") {
                                    let trimmed = title.trim();
                                    if trimmed.is_empty() {
                                        el.set_attribute("alt", "")?;
                                    } else {
                                        el.set_attribute("alt", trimmed)?;
                                    }
                                } else {
                                    el.set_attribute("alt", "")?;
                                }
                            }

                            if let Some(src) = el.get_attribute("src")
                                && is_external_http_url(&src)
                                && let Some(domain) = extract_domain(&src)
                            {
                                state.image_domains.insert(domain);
                            }
                        }

                        if el.get_attribute("loading").is_none() {
                            el.set_attribute("loading", "lazy")?;
                        }
                        if el.get_attribute("decoding").is_none() {
                            el.set_attribute("decoding", "async")?;
                        }
                        if el.get_attribute("width").is_some()
                            && el.get_attribute("height").is_some()
                        {
                            el.remove_attribute("data-default-aspect");
                        } else {
                            el.set_attribute("data-default-aspect", "16:9")?;
                        }
                        Ok(())
                    }
                }),
                element!("a", {
                    let state = Rc::clone(&state);
                    let site_url = Rc::clone(&site_url);
                    move |el| {
                        if let Some(href) = el.get_attribute("href") {
                            let classification = classify_link(&href, site_url.as_ref().as_ref());
                            match classification {
                                LinkKind::External { domain } => {
                                    {
                                        let mut state = state.borrow_mut();
                                        state.external_links =
                                            state.external_links.saturating_add(1);
                                        if let Some(domain) = domain.clone() {
                                            state.link_domains.insert(domain);
                                        }
                                    }

                                    let rel_value = merge_rel(
                                        el.get_attribute("rel"),
                                        &["noopener", "noreferrer"],
                                    );
                                    el.set_attribute("rel", &rel_value)?;
                                    el.set_attribute("target", "_blank")?;
                                    el.set_attribute("data-link-kind", "external")?;
                                }
                                LinkKind::Internal => {
                                    {
                                        let mut state = state.borrow_mut();
                                        state.internal_links =
                                            state.internal_links.saturating_add(1);
                                    }
                                    el.set_attribute("data-link-kind", "internal")?;
                                }
                                LinkKind::Anchor => {
                                    el.set_attribute("data-link-kind", "anchor")?;
                                }
                                LinkKind::Other => {
                                    el.set_attribute("data-link-kind", "other")?;
                                }
                            }
                        }
                        Ok(())
                    }
                }),
                element!("pre", {
                    let state = Rc::clone(&state);
                    move |el| {
                        {
                            let mut state = state.borrow_mut();
                            state.code_blocks = state.code_blocks.saturating_add(1);
                        }

                        apply_code_block_accessibility(el)?;
                        attach_code_copy_button(el)?;
                        Ok(())
                    }
                }),
                element!("code", |el| {
                    if let Some(lang) = el.get_attribute("data-language") {
                        let trimmed = lang.trim();
                        if !trimmed.is_empty() {
                            el.set_attribute("data-lang", trimmed)?;
                        }
                    }
                    Ok(())
                }),
                element!("div", {
                    let state = Rc::clone(&state);
                    move |el| {
                        if el.get_attribute("data-math-style").is_some() {
                            let mut state = state.borrow_mut();
                            state.math_blocks = state.math_blocks.saturating_add(1);
                        }
                        Ok(())
                    }
                }),
                element!("span", {
                    let state = Rc::clone(&state);
                    move |el| {
                        if el.get_attribute("data-math-style").is_some() {
                            let mut state = state.borrow_mut();
                            state.math_blocks = state.math_blocks.saturating_add(1);
                        }
                        Ok(())
                    }
                }),
                element!("figure", {
                    let state = Rc::clone(&state);
                    move |el| {
                        if let Some(role) = el.get_attribute("data-role")
                            && role == "diagram-mermaid"
                        {
                            let mut state = state.borrow_mut();
                            state.mermaid_diagrams = state.mermaid_diagrams.saturating_add(1);
                        }
                        Ok(())
                    }
                }),
                element!("table", |el| {
                    el.set_attribute("data-role", "post-table")?;
                    if el.get_attribute("role").is_none() {
                        el.set_attribute("role", "table")?;
                    }
                    Ok(())
                }),
                element!("thead", |el| {
                    el.set_attribute("data-role", "table-head")?;
                    if el.get_attribute("role").is_none() {
                        el.set_attribute("role", "rowgroup")?;
                    }
                    Ok(())
                }),
                element!("tbody", |el| {
                    el.set_attribute("data-role", "table-body")?;
                    if el.get_attribute("role").is_none() {
                        el.set_attribute("role", "rowgroup")?;
                    }
                    Ok(())
                }),
                element!("tr", |el| {
                    el.set_attribute("data-role", "table-row")?;
                    if el.get_attribute("role").is_none() {
                        el.set_attribute("role", "row")?;
                    }
                    Ok(())
                }),
                element!("th", |el| {
                    el.set_attribute("data-role", "table-header-cell")?;
                    if el.get_attribute("scope").is_none() {
                        el.set_attribute("scope", "col")?;
                    }
                    Ok(())
                }),
                element!("td", |el| {
                    el.set_attribute("data-role", "table-cell")?;
                    Ok(())
                }),
                element!("blockquote", |el| {
                    el.set_attribute("data-role", "post-quote")?;
                    if el.get_attribute("role").is_none() {
                        el.set_attribute("role", "note")?;
                    }
                    Ok(())
                }),
                text!("*", {
                    let state = Rc::clone(&state);
                    move |t| {
                        let words = t
                            .as_str()
                            .split_whitespace()
                            .filter(|segment| !segment.is_empty())
                            .count() as u32;
                        if words > 0 {
                            let mut state = state.borrow_mut();
                            state.word_count = state.word_count.saturating_add(words);
                        }
                        Ok(())
                    }
                }),
            ],
            ..RewriteStrSettings::default()
        },
    )
    .map_err(|err| RenderError::Document {
        message: err.to_string(),
    })?;

    let state = Rc::try_unwrap(state)
        .map(|cell| cell.into_inner())
        .unwrap_or_else(|rc| rc.borrow().clone());

    Ok(AugmentOutcome {
        html: rewritten,
        state,
    })
}

pub(super) fn build_content_metrics(outcome: &AugmentOutcome) -> ContentMetrics {
    let mut metrics = ContentMetrics {
        internal_links_count: outcome.state.internal_links,
        external_links_count: outcome.state.external_links,
        images_count: outcome.state.images,
        images_missing_alt: outcome.state.images_missing_alt,
        images_missing_dimensions: outcome.state.images_missing_dimensions,
        code_blocks_count: outcome.state.code_blocks,
        math_blocks_count: outcome.state.math_blocks,
        mermaid_diagram_count: outcome.state.mermaid_diagrams,
        ..ContentMetrics::default()
    };

    if outcome.state.word_count == 0 {
        metrics.reading_time_minutes = 0;
    } else {
        let minutes = (outcome.state.word_count as f32 / 225.0).ceil() as u32;
        metrics.reading_time_minutes = minutes.max(1);
    }

    metrics
}

pub(super) fn build_resource_hints(outcome: &AugmentOutcome) -> ResourceHints {
    ResourceHints {
        preconnect_domains: outcome.state.image_domains.iter().cloned().collect(),
        dns_prefetch_domains: outcome.state.link_domains.iter().cloned().collect(),
    }
}

#[derive(Debug, Clone)]
enum LinkKind {
    Internal,
    External { domain: Option<String> },
    Anchor,
    Other,
}

fn classify_link(href: &str, site_url: Option<&Url>) -> LinkKind {
    if href.starts_with('#') || href.is_empty() {
        return LinkKind::Anchor;
    }

    if let (Some(base), Ok(url)) = (site_url, Url::parse(href)) {
        if same_origin(&url, base) {
            return LinkKind::Internal;
        }

        return LinkKind::External {
            domain: extract_domain_from_url(&url),
        };
    }

    if is_external_http_url(href) {
        return LinkKind::External {
            domain: extract_domain(href),
        };
    }

    if is_internal_path(href) {
        return LinkKind::Internal;
    }

    LinkKind::Other
}

fn is_internal_path(href: &str) -> bool {
    href.starts_with('/')
        || href.starts_with("./")
        || href.starts_with("../")
        || (!href.contains(':') && !href.starts_with("//"))
}

fn is_external_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn extract_domain(url: &str) -> Option<String> {
    Url::parse(url).ok().and_then(|parsed| {
        parsed.host_str().map(|host| {
            let mut domain = format!("{}://{}", parsed.scheme(), host);
            if let Some(port) = parsed.port() {
                domain.push(':');
                domain.push_str(&port.to_string());
            }
            domain
        })
    })
}

fn extract_domain_from_url(url: &Url) -> Option<String> {
    url.host_str().map(|host| {
        let mut domain = format!("{}://{}", url.scheme(), host);
        if let Some(port) = url.port() {
            domain.push(':');
            domain.push_str(&port.to_string());
        }
        domain
    })
}

fn same_origin(url: &Url, base: &Url) -> bool {
    url.scheme() == base.scheme()
        && url.host_str() == base.host_str()
        && url.port_or_known_default() == base.port_or_known_default()
}

fn merge_rel(existing: Option<String>, required: &[&str]) -> String {
    let mut tokens: BTreeSet<String> = existing
        .unwrap_or_default()
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect();
    for &token in required {
        tokens.insert(token.to_string());
    }
    tokens.into_iter().collect::<Vec<_>>().join(" ")
}
