use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

use lol_html::{RewriteStrSettings, element, rewrite_str, text};
use url::Url;
use uuid::Uuid;

use crate::application::render::types::{
    ContentMetrics, RenderError, RenderTarget, RenderedSection, ResourceHints,
};

use super::rewrite::HeadingInfo;

pub(crate) struct ProcessedHtml {
    pub(crate) html: String,
    pub(crate) sections: Option<Vec<RenderedSection>>,
    pub(crate) contains_code: bool,
    pub(crate) contains_math: bool,
    pub(crate) contains_mermaid: bool,
    pub(crate) resource_hints: ResourceHints,
    pub(crate) content_metrics: ContentMetrics,
}

#[derive(Clone)]
struct HeadingSlice {
    start: usize,
    end: usize,
    heading_html: String,
    heading_text: String,
    slug: String,
    level: u8,
    contains_code: bool,
    contains_math: bool,
    contains_mermaid: bool,
}

pub(crate) fn post_process(
    sanitized_html: &str,
    target: &RenderTarget,
    headings: &[HeadingInfo],
) -> Result<ProcessedHtml, RenderError> {
    match target {
        RenderTarget::PostBody { .. } => process_post_html(sanitized_html, headings),
        _ => Ok(ProcessedHtml {
            html: sanitized_html.to_string(),
            sections: None,
            contains_code: sanitized_html.contains("syntax-")
                || sanitized_html.contains("<pre")
                || sanitized_html.contains("<code"),
            contains_math: sanitized_html.contains("data-math-style"),
            contains_mermaid: sanitized_html.contains("data-role=\"diagram-mermaid\""),
            resource_hints: ResourceHints::default(),
            content_metrics: ContentMetrics::default(),
        }),
    }
}

fn process_post_html(
    sanitized_html: &str,
    headings: &[HeadingInfo],
) -> Result<ProcessedHtml, RenderError> {
    if headings.is_empty() {
        let augmentation = augment_semantics(sanitized_html)?;
        let metrics = build_content_metrics(&augmentation);
        let resource_hints = build_resource_hints(&augmentation);
        let contains_code = metrics.code_blocks_count > 0;
        let contains_math = metrics.math_blocks_count > 0;
        let contains_mermaid = metrics.mermaid_diagram_count > 0;
        let AugmentOutcome {
            html: augmented_html,
            ..
        } = augmentation;

        return Ok(ProcessedHtml {
            html: augmented_html,
            sections: Some(Vec::new()),
            contains_code,
            contains_math,
            contains_mermaid,
            resource_hints,
            content_metrics: metrics,
        });
    }

    let html_with_ids = apply_heading_ids(sanitized_html, headings)?;
    let augmentation = augment_semantics(&html_with_ids)?;
    let sections = build_sections(&augmentation.html, headings)?;
    let metrics = build_content_metrics(&augmentation);
    let resource_hints = build_resource_hints(&augmentation);
    let contains_code =
        sections.iter().any(|section| section.contains_code) || metrics.code_blocks_count > 0;
    let contains_math =
        sections.iter().any(|section| section.contains_math) || metrics.math_blocks_count > 0;
    let contains_mermaid = sections.iter().any(|section| section.contains_mermaid)
        || metrics.mermaid_diagram_count > 0;
    let AugmentOutcome {
        html: augmented_html,
        ..
    } = augmentation;

    Ok(ProcessedHtml {
        html: augmented_html,
        sections: Some(sections),
        contains_code,
        contains_math,
        contains_mermaid,
        resource_hints,
        content_metrics: metrics,
    })
}

#[derive(Default, Clone)]
struct AugmentState {
    image_domains: BTreeSet<String>,
    link_domains: BTreeSet<String>,
    internal_links: u32,
    external_links: u32,
    images: u32,
    images_missing_alt: u32,
    images_missing_dimensions: u32,
    code_blocks: u32,
    word_count: u32,
    math_blocks: u32,
    mermaid_diagrams: u32,
}

struct AugmentOutcome {
    html: String,
    state: AugmentState,
}

fn augment_semantics(html: &str) -> Result<AugmentOutcome, RenderError> {
    let state = Rc::new(RefCell::new(AugmentState::default()));

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
                    move |el| {
                        if let Some(href) = el.get_attribute("href") {
                            let classification = classify_link(&href);
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

fn build_content_metrics(outcome: &AugmentOutcome) -> ContentMetrics {
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

fn build_resource_hints(outcome: &AugmentOutcome) -> ResourceHints {
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

fn classify_link(href: &str) -> LinkKind {
    if href.starts_with('#') || href.is_empty() {
        return LinkKind::Anchor;
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

fn apply_heading_ids(html: &str, headings: &[HeadingInfo]) -> Result<String, RenderError> {
    let headings_shared = Rc::new(headings.to_vec());
    let index = Rc::new(RefCell::new(0usize));
    let error_slot = Rc::new(RefCell::new(None));

    let rewritten = rewrite_str(
        html,
        RewriteStrSettings {
            element_content_handlers: vec![element!("h1, h2, h3, h4, h5, h6", {
                let headings_shared = Rc::clone(&headings_shared);
                let index = Rc::clone(&index);
                let error_slot = Rc::clone(&error_slot);
                move |el| {
                    let mut idx = index.borrow_mut();
                    if *idx >= headings_shared.len() {
                        *error_slot.borrow_mut() = Some(RenderError::Document {
                            message: "unexpected extra heading".to_string(),
                        });
                        return Ok(());
                    }
                    let info = &headings_shared[*idx];
                    *idx += 1;

                    let tag_name = el.tag_name();
                    let level = tag_name
                        .strip_prefix('h')
                        .and_then(|value| value.parse::<u8>().ok())
                        .unwrap_or(0);
                    if level != info.level {
                        *error_slot.borrow_mut() = Some(RenderError::Document {
                            message: format!(
                                "heading level mismatch: expected h{}, found {}",
                                info.level, tag_name
                            ),
                        });
                        return Ok(());
                    }

                    el.set_attribute("id", &info.slug)?;
                    Ok(())
                }
            })],
            ..RewriteStrSettings::default()
        },
    )
    .map_err(|err| RenderError::Document {
        message: err.to_string(),
    })?;

    if let Some(err) = error_slot.borrow_mut().take() {
        return Err(err);
    }

    Ok(rewritten)
}

fn build_sections(
    html: &str,
    headings: &[HeadingInfo],
) -> Result<Vec<RenderedSection>, RenderError> {
    let slices = collect_heading_slices(html, headings)?;
    let parent_indices = compute_parent_indices(&slices);
    let ids = allocate_section_ids(slices.len());
    assemble_sections(html, &slices, &parent_indices, &ids)
}

fn collect_heading_slices(
    html: &str,
    headings: &[HeadingInfo],
) -> Result<Vec<HeadingSlice>, RenderError> {
    let mut slices = Vec::with_capacity(headings.len());
    let mut cursor = 0;

    for info in headings {
        let tag_prefix = format!("<h{} ", info.level);
        let tag_prefix_alt = format!("<h{}>", info.level);

        let heading_start = html[cursor..]
            .find(&tag_prefix)
            .map(|idx| idx + cursor)
            .or_else(|| html[cursor..].find(&tag_prefix_alt).map(|idx| idx + cursor))
            .ok_or_else(|| RenderError::Document {
                message: format!("unable to locate heading `{}`", info.slug),
            })?;

        let id_attr = format!("id=\"{}\"", info.slug);
        let id_index = html[heading_start..]
            .find(&id_attr)
            .map(|idx| idx + heading_start)
            .ok_or_else(|| RenderError::Document {
                message: format!("missing id attribute for heading `{}`", info.slug),
            })?;

        let heading_start = html[..id_index]
            .rfind(&tag_prefix)
            .or_else(|| html[..id_index].rfind(&tag_prefix_alt))
            .ok_or_else(|| RenderError::Document {
                message: format!("unable to locate start for heading `{}`", info.slug),
            })?;

        let closing_tag = format!("</h{}>", info.level);
        let closing_index = html[id_index..]
            .find(&closing_tag)
            .map(|idx| idx + id_index)
            .ok_or_else(|| RenderError::Document {
                message: format!("unable to locate closing tag for heading `{}`", info.slug),
            })?;
        let heading_end = closing_index + closing_tag.len();

        slices.push(HeadingSlice {
            start: heading_start,
            end: heading_end,
            heading_html: html[heading_start..heading_end].to_string(),
            heading_text: info.text.clone(),
            slug: info.slug.clone(),
            level: info.level,
            contains_code: info.has_block_code,
            contains_math: info.has_math_block || info.has_inline_math,
            contains_mermaid: info.has_mermaid_block,
        });

        cursor = heading_end;
    }

    Ok(slices)
}

fn compute_parent_indices(slices: &[HeadingSlice]) -> Vec<Option<usize>> {
    let mut parent_indices = vec![None; slices.len()];
    let mut stack: Vec<(u8, usize)> = Vec::new();

    for (idx, slice) in slices.iter().enumerate() {
        while let Some(&(level, _)) = stack.last() {
            if level < slice.level {
                break;
            }
            stack.pop();
        }
        parent_indices[idx] = stack.last().map(|&(_, parent_idx)| parent_idx);
        stack.push((slice.level, idx));
    }

    parent_indices
}

fn allocate_section_ids(len: usize) -> Vec<Uuid> {
    (0..len).map(|_| Uuid::new_v4()).collect()
}

fn assemble_sections(
    html: &str,
    slices: &[HeadingSlice],
    parent_indices: &[Option<usize>],
    ids: &[Uuid],
) -> Result<Vec<RenderedSection>, RenderError> {
    let mut child_counts = vec![0u32; slices.len()];
    let mut root_position: u32 = 0;
    let mut sections = Vec::with_capacity(slices.len());

    for (idx, slice) in slices.iter().enumerate() {
        let body_start = slice.end;
        let body_end = slices
            .get(idx + 1)
            .map(|next| next.start)
            .unwrap_or_else(|| html.len());
        let body_html = html[body_start..body_end].to_string();

        let parent_idx = parent_indices[idx];
        let position = if let Some(parent_idx) = parent_idx {
            let next_position =
                child_counts[parent_idx]
                    .checked_add(1)
                    .ok_or_else(|| RenderError::Document {
                        message: format!(
                            "section position overflow for parent `{}`",
                            slices[parent_idx].slug
                        ),
                    })?;
            child_counts[parent_idx] = next_position;
            next_position
        } else {
            root_position = root_position
                .checked_add(1)
                .ok_or_else(|| RenderError::Document {
                    message: "section position overflow for root sections".to_string(),
                })?;
            root_position
        };

        sections.push(RenderedSection {
            id: ids[idx],
            parent_id: parent_idx.map(|p| ids[p]),
            anchor_slug: slice.slug.clone(),
            heading_html: slice.heading_html.clone(),
            heading_text: slice.heading_text.clone(),
            body_html,
            level: slice.level,
            contains_code: slice.contains_code,
            contains_math: slice.contains_math,
            contains_mermaid: slice.contains_mermaid,
            position,
        });
    }

    Ok(sections)
}
