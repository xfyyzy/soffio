mod api_keys_panel;
mod navigation_panel;
mod pages_panel;
mod posts_panel;
mod tags_panel;
mod uploads_panel;

use std::sync::Arc;

use axum::http::StatusCode;

use crate::application::{
    error::HttpError,
    repos::{ApiKeysRepo, NavigationRepo, PagesRepo, PostsRepo, RepoError, TagsRepo, UploadsRepo},
};
use crate::presentation::admin::views::AdminDashboardView;

const SOURCE: &str = "application::admin::dashboard::AdminDashboardService";
const POSTS_FAILURE_MESSAGE: &str = "Failed to compute post dashboard metrics";
const PAGES_FAILURE_MESSAGE: &str = "Failed to compute page dashboard metrics";
const TAGS_FAILURE_MESSAGE: &str = "Failed to compute tag dashboard metrics";
const TAGS_LIST_FAILURE_MESSAGE: &str = "Failed to enumerate tags for dashboard metrics";
const NAVIGATION_FAILURE_MESSAGE: &str = "Failed to compute navigation dashboard metrics";
const NAVIGATION_LIST_FAILURE_MESSAGE: &str = "Failed to enumerate navigation entries";
const UPLOADS_FAILURE_MESSAGE: &str = "Failed to compute upload dashboard metrics";
const API_KEYS_FAILURE_MESSAGE: &str = "Failed to compute API key dashboard metrics";

const DOCUMENT_CONTENT_TYPES: &[&str] = &[
    "application/pdf",
    "application/msword",
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "application/vnd.ms-excel",
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "application/vnd.ms-powerpoint",
    "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "application/rtf",
    "text/plain",
    "text/markdown",
];

#[derive(Clone)]
pub struct AdminDashboardService {
    posts: Arc<dyn PostsRepo>,
    pages: Arc<dyn PagesRepo>,
    tags: Arc<dyn TagsRepo>,
    navigation: Arc<dyn NavigationRepo>,
    uploads: Arc<dyn UploadsRepo>,
    api_keys: Arc<dyn ApiKeysRepo>,
}

pub struct AdminDashboardDeps {
    pub posts: Arc<dyn PostsRepo>,
    pub pages: Arc<dyn PagesRepo>,
    pub tags: Arc<dyn TagsRepo>,
    pub navigation: Arc<dyn NavigationRepo>,
    pub uploads: Arc<dyn UploadsRepo>,
    pub api_keys: Arc<dyn ApiKeysRepo>,
}

impl AdminDashboardService {
    pub fn new(deps: AdminDashboardDeps) -> Self {
        let AdminDashboardDeps {
            posts,
            pages,
            tags,
            navigation,
            uploads,
            api_keys,
        } = deps;

        Self {
            posts,
            pages,
            tags,
            navigation,
            uploads,
            api_keys,
        }
    }

    pub async fn overview(&self) -> Result<AdminDashboardView, HttpError> {
        let (posts_panel, pages_panel, tags_panel, navigation_panel, uploads_panel, api_keys_panel) =
            tokio::try_join!(
                self.collect_posts_panel(),
                self.collect_pages_panel(),
                self.collect_tags_panel(),
                self.collect_navigation_panel(),
                self.collect_uploads_panel(),
                self.collect_api_keys_panel(),
            )?;

        Ok(AdminDashboardView {
            title: "Dashboard".to_string(),
            panels: vec![
                posts_panel,
                pages_panel,
                tags_panel,
                navigation_panel,
                uploads_panel,
                api_keys_panel,
            ],
            empty_message: "No assets have been created yet.".to_string(),
        })
    }
}

fn repo_failure(message: &'static str, err: RepoError) -> HttpError {
    HttpError::new(
        SOURCE,
        StatusCode::INTERNAL_SERVER_ERROR,
        message,
        err.to_string(),
    )
}

fn is_document_content_type(content_type: &str) -> bool {
    DOCUMENT_CONTENT_TYPES
        .iter()
        .any(|ty| ty.eq_ignore_ascii_case(content_type))
}
