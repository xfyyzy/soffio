use std::sync::Arc;

use askama::Template;
use axum::response::Response;
use datastar::prelude::ElementPatchMode;
use serde_json::json;
use uuid::Uuid;

use crate::application::error::HttpError;
use crate::application::pagination::{PageRequest, PostCursor};
use crate::application::repos::{
    PostListScope, PostQueryFilter, PostsRepo, SectionsRepo, SettingsRepo, TagWithCount, TagsRepo,
};
use crate::application::stream::StreamBuilder;
use crate::cache::{L0Store, hash_cursor_str, hash_post_list_key};
use crate::domain::entities::{PostRecord, SiteSettingsRecord, TagRecord};
use crate::domain::posts;
use crate::domain::sections::PostSectionNode;
use crate::domain::sections::build_section_tree;
use crate::domain::types::PostStatus;
use crate::presentation::views::{
    self, FeedLoaderContext, FeedLoaderTemplate, PageContext, PostCard, PostCardsAppendTemplate,
    PostDetailContext, PostSectionEvent, PostTocEvent, PostTocView, TemplateRenderError,
    build_tag_badges,
};
use crate::util::timezone;

mod presentation;
mod sections;
mod service;
mod summaries;
mod types;

pub use types::{AppendPayload, FeedError, FeedFilter, FeedService};

const DEFAULT_PAGE_SIZE: usize = 6;

pub fn build_datastar_append_response(
    payload: AppendPayload,
    load_more_query: String,
) -> Result<Response, HttpError> {
    presentation::build_datastar_append_response(payload, load_more_query)
}

pub(crate) fn order_tags_with_pins(counts: &[TagWithCount]) -> Vec<&TagWithCount> {
    summaries::order_tags_with_pins(counts)
}
