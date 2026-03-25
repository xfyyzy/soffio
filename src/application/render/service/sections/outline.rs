use std::{cell::RefCell, rc::Rc};

use lol_html::{RewriteStrSettings, element, rewrite_str};
use uuid::Uuid;

use crate::application::render::{
    service::rewrite::HeadingInfo,
    types::{RenderError, RenderedSection},
};

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

pub(super) fn apply_heading_ids(
    html: &str,
    headings: &[HeadingInfo],
) -> Result<String, RenderError> {
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

pub(super) fn build_sections(
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
