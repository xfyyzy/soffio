use std::sync::Arc;

use thiserror::Error;

use crate::application::repos::{
    PostQueryFilter, PostsRepo, RepoError, SectionsRepo, SettingsRepo, TagsRepo,
};
use crate::cache::L0Store;
use crate::domain::sections::SectionTreeError;

#[derive(Clone)]
pub enum FeedFilter {
    All,
    Tag(String),
    Month(String),
}

impl FeedFilter {
    pub fn tag(&self) -> Option<&str> {
        match self {
            FeedFilter::Tag(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn month(&self) -> Option<&str> {
        match self {
            FeedFilter::Month(value) => Some(value.as_str()),
            _ => None,
        }
    }

    pub fn load_more_query(&self) -> String {
        match self {
            FeedFilter::All => String::new(),
            FeedFilter::Tag(value) => format!("&tag={value}"),
            FeedFilter::Month(value) => format!("&month={value}"),
        }
    }

    pub fn base_path(&self) -> String {
        match self {
            FeedFilter::All => "/".to_string(),
            FeedFilter::Tag(value) => format!("/tags/{value}"),
            FeedFilter::Month(value) => format!("/months/{value}"),
        }
    }

    pub(super) fn to_query_filter(&self) -> PostQueryFilter {
        let mut filter = PostQueryFilter::default();
        match self {
            FeedFilter::All => {}
            FeedFilter::Tag(tag) => filter.tag = Some(tag.clone()),
            FeedFilter::Month(month) => filter.month = Some(month.clone()),
        }
        filter
    }
}

#[derive(Clone)]
pub struct AppendPayload {
    pub offset: usize,
    pub cards: Vec<crate::presentation::views::PostCard>,
    pub next_cursor: Option<String>,
    pub total_visible: usize,
}

#[derive(Clone)]
pub struct FeedService {
    pub(super) posts: Arc<dyn PostsRepo>,
    pub(super) sections: Arc<dyn SectionsRepo>,
    pub(super) tags: Arc<dyn TagsRepo>,
    pub(super) settings: Arc<dyn SettingsRepo>,
    pub(super) cache: Option<Arc<L0Store>>,
}

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("invalid cursor: {0}")]
    InvalidCursor(String),
    #[error("unknown tag")]
    UnknownTag,
    #[error("unknown month")]
    UnknownMonth,
    #[error("invalid section hierarchy: {0}")]
    SectionTree(#[from] SectionTreeError),
    #[error(transparent)]
    Repo(#[from] RepoError),
}
