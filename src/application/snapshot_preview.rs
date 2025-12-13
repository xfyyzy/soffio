use std::sync::Arc;

use axum::http::StatusCode;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::admin::snapshot_types::{
    PageSnapshotPayload, PageSnapshotSource, PostSectionSnapshot, PostSnapshotPayload,
    PostSnapshotSource,
};
use crate::application::error::HttpError;
use crate::application::repos::{RepoError, SettingsRepo, SnapshotRecord, SnapshotsRepo, TagsRepo};
use crate::domain::entities::PostSectionRecord;
use crate::domain::posts;
use crate::domain::sections::{PostSectionNode, SectionTreeError, build_section_tree};
use crate::domain::snapshots::Snapshotable;
use crate::domain::types::SnapshotEntityType;
use crate::presentation::views::{
    PageView, PostDetailContext, PostSectionEvent, PostTocEvent, TagBadge, build_tag_badges,
};
use crate::util::timezone;

const SOURCE: &str = "application::snapshot_preview";

#[derive(Clone)]
pub struct SnapshotPreviewService {
    snapshots: Arc<dyn SnapshotsRepo>,
    tags: Arc<dyn TagsRepo>,
    settings: Arc<dyn SettingsRepo>,
}

impl SnapshotPreviewService {
    pub fn new(
        snapshots: Arc<dyn SnapshotsRepo>,
        tags: Arc<dyn TagsRepo>,
        settings: Arc<dyn SettingsRepo>,
    ) -> Self {
        Self {
            snapshots,
            tags,
            settings,
        }
    }

    pub async fn post_snapshot_view(
        &self,
        id: Uuid,
    ) -> Result<Option<PostDetailContext>, HttpError> {
        let snapshot = match self.snapshots.find_snapshot(id).await {
            Ok(Some(record)) => record,
            Ok(None) => return Ok(None),
            Err(err) => return Err(repo_error("find_snapshot", err)),
        };

        if snapshot.entity_type != SnapshotEntityType::Post {
            return Err(HttpError::new(
                SOURCE,
                StatusCode::BAD_REQUEST,
                "Snapshot type mismatch",
                "snapshot is not a post snapshot",
            ));
        }

        let payload: PostSnapshotPayload = deserialize_payload(&snapshot)?;
        PostSnapshotSource::validate_snapshot(&payload).map_err(|err| {
            HttpError::new(
                SOURCE,
                StatusCode::BAD_REQUEST,
                "Invalid snapshot",
                err.to_string(),
            )
        })?;

        let settings = self
            .settings
            .load_site_settings()
            .await
            .map_err(|err| repo_error("load_site_settings", err))?;

        let section_records = payload
            .sections
            .iter()
            .map(|section| {
                snapshot_section_to_record(section, snapshot.entity_id, snapshot.created_at)
            })
            .collect::<Vec<_>>();

        let section_nodes = build_section_tree(section_records).map_err(section_tree_error)?;

        let tag_badges = self.load_tag_badges(&payload.tags).await?;

        let has_code_blocks = PostSectionNode::any_contains_code(&section_nodes);
        let has_math_blocks = PostSectionNode::any_contains_math(&section_nodes);
        let has_mermaid_diagrams = PostSectionNode::any_contains_mermaid(&section_nodes);
        let sections = build_post_section_events(&section_nodes);
        let toc = if settings.global_toc_enabled {
            build_post_toc_view(&section_nodes)
        } else {
            None
        };

        let published_at = payload.published_at.unwrap_or(snapshot.created_at);
        let localized = timezone::localized_datetime(published_at, settings.timezone);
        let date = timezone::localized_date(published_at, settings.timezone);

        let detail = PostDetailContext {
            slug: payload.slug,
            title: payload.title,
            published: posts::format_human_date(date),
            iso_date: localized.to_rfc3339(),
            tags: tag_badges,
            excerpt: payload.excerpt,
            summary_html: payload.summary_html,
            sections,
            has_code_blocks,
            has_math_blocks,
            has_mermaid_diagrams,
            toc,
            is_pinned: payload.pinned,
        };

        Ok(Some(detail))
    }

    pub async fn page_snapshot_view(&self, id: Uuid) -> Result<Option<PageView>, HttpError> {
        let snapshot = match self.snapshots.find_snapshot(id).await {
            Ok(Some(record)) => record,
            Ok(None) => return Ok(None),
            Err(err) => return Err(repo_error("find_snapshot", err)),
        };

        if snapshot.entity_type != SnapshotEntityType::Page {
            return Err(HttpError::new(
                SOURCE,
                StatusCode::BAD_REQUEST,
                "Snapshot type mismatch",
                "snapshot is not a page snapshot",
            ));
        }

        let payload: PageSnapshotPayload = deserialize_payload(&snapshot)?;
        PageSnapshotSource::validate_snapshot(&payload).map_err(|err| {
            HttpError::new(
                SOURCE,
                StatusCode::BAD_REQUEST,
                "Invalid snapshot",
                err.to_string(),
            )
        })?;

        let (contains_code, contains_math, contains_mermaid) =
            render_feature_flags(&payload.rendered_html);

        Ok(Some(PageView {
            title: payload.title,
            content_html: payload.rendered_html,
            contains_code,
            contains_math,
            contains_mermaid,
        }))
    }

    async fn load_tag_badges(&self, tag_ids: &[Uuid]) -> Result<Vec<TagBadge>, HttpError> {
        let mut tags = Vec::with_capacity(tag_ids.len());
        for id in tag_ids {
            let Some(tag) = self
                .tags
                .find_by_id(*id)
                .await
                .map_err(|err| repo_error("find_tag", err))?
            else {
                return Err(HttpError::new(
                    SOURCE,
                    StatusCode::BAD_REQUEST,
                    "Invalid snapshot",
                    format!("tag {id} not found"),
                ));
            };
            tags.push((tag.slug, tag.name));
        }

        Ok(build_tag_badges(
            tags.iter()
                .map(|(slug, name)| (slug.as_str(), name.as_str())),
        ))
    }
}

fn deserialize_payload<T: serde::de::DeserializeOwned>(
    snapshot: &SnapshotRecord,
) -> Result<T, HttpError> {
    serde_json::from_value::<T>(snapshot.content.clone()).map_err(|err| {
        HttpError::new(
            SOURCE,
            StatusCode::BAD_REQUEST,
            "Invalid snapshot",
            err.to_string(),
        )
    })
}

fn snapshot_section_to_record(
    section: &PostSectionSnapshot,
    post_id: Uuid,
    created_at: OffsetDateTime,
) -> PostSectionRecord {
    PostSectionRecord {
        id: section.id,
        post_id,
        position: section.position,
        level: section.level,
        parent_id: section.parent_id,
        heading_html: section.heading_html.clone(),
        heading_text: section.heading_text.clone(),
        body_html: section.body_html.clone(),
        contains_code: section.contains_code,
        contains_math: section.contains_math,
        contains_mermaid: section.contains_mermaid,
        anchor_slug: section.anchor_slug.clone(),
        created_at,
    }
}

fn repo_error(operation: &'static str, err: RepoError) -> HttpError {
    HttpError::new(
        SOURCE,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Repository error",
        format!("{operation} failed: {err}"),
    )
}

fn section_tree_error(err: SectionTreeError) -> HttpError {
    HttpError::new(
        SOURCE,
        StatusCode::BAD_REQUEST,
        "Invalid snapshot sections",
        err.to_string(),
    )
}

fn build_post_section_events(nodes: &[PostSectionNode]) -> Vec<PostSectionEvent> {
    let mut events = Vec::new();
    for node in nodes {
        append_section_events(node, &mut events);
    }
    events
}

fn append_section_events(node: &PostSectionNode, events: &mut Vec<PostSectionEvent>) {
    events.push(PostSectionEvent::StartSection {
        anchor: node.anchor_slug.clone(),
        level: node.level,
        heading_html: node.heading_html.clone(),
        body_html: node.body_html.clone(),
    });

    if !node.children.is_empty() {
        events.push(PostSectionEvent::StartChildren);
        for child in &node.children {
            append_section_events(child, events);
        }
        events.push(PostSectionEvent::EndChildren);
    }

    events.push(PostSectionEvent::EndSection);
}

fn build_post_toc_view(
    nodes: &[PostSectionNode],
) -> Option<crate::presentation::views::PostTocView> {
    if nodes.is_empty() {
        return None;
    }

    let mut events = Vec::new();
    append_toc_events(nodes, &mut events);
    Some(crate::presentation::views::PostTocView { events })
}

fn append_toc_events(nodes: &[PostSectionNode], events: &mut Vec<PostTocEvent>) {
    events.push(PostTocEvent::StartList);

    for node in nodes {
        let title = node.heading_text.trim().to_string();
        events.push(PostTocEvent::StartItem {
            anchor: node.anchor_slug.clone(),
            title,
            level: node.level,
        });

        if !node.children.is_empty() {
            append_toc_events(&node.children, events);
        }

        events.push(PostTocEvent::EndItem);
    }

    events.push(PostTocEvent::EndList);
}

fn render_feature_flags(html: &str) -> (bool, bool, bool) {
    let contains_code = html.contains("syntax-") || html.contains("<pre") || html.contains("<code");
    let contains_math = html.contains("data-math-style");
    let contains_mermaid = html.contains("data-role=\"diagram-mermaid\"");
    (contains_code, contains_math, contains_mermaid)
}
