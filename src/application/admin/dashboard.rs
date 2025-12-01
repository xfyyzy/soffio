use std::sync::Arc;

use axum::http::StatusCode;

use crate::application::{
    error::HttpError,
    pagination::{NavigationCursor, PageRequest, PaginationError, UploadCursor},
    repos::{
        ApiKeyPageRequest, ApiKeyQueryFilter, ApiKeysRepo, NavigationQueryFilter, NavigationRepo,
        PageQueryFilter, PagesRepo, PostListScope, PostQueryFilter, PostsRepo, RepoError,
        TagQueryFilter, TagsRepo, UploadQueryFilter, UploadsRepo,
    },
};
use crate::domain::types::{NavigationDestinationType, PageStatus, PostStatus};
use crate::presentation::admin::views::{
    AdminDashboardPanelView, AdminDashboardView, AdminMetricView,
};
use crate::util::bytes::format_bytes;

const SOURCE: &str = "application::admin::dashboard::AdminDashboardService";
const POSTS_FAILURE_MESSAGE: &str = "Failed to compute post dashboard metrics";
const PAGES_FAILURE_MESSAGE: &str = "Failed to compute page dashboard metrics";
const TAGS_FAILURE_MESSAGE: &str = "Failed to compute tag dashboard metrics";
const TAGS_LIST_FAILURE_MESSAGE: &str = "Failed to enumerate tags for dashboard metrics";
const NAVIGATION_FAILURE_MESSAGE: &str = "Failed to compute navigation dashboard metrics";
const NAVIGATION_LIST_FAILURE_MESSAGE: &str = "Failed to enumerate navigation entries";
const NAVIGATION_CURSOR_FAILURE_MESSAGE: &str = "Failed to decode navigation cursor";
const UPLOADS_FAILURE_MESSAGE: &str = "Failed to compute upload dashboard metrics";
const UPLOADS_LIST_FAILURE_MESSAGE: &str = "Failed to enumerate uploads";
const UPLOADS_CURSOR_FAILURE_MESSAGE: &str = "Failed to decode upload cursor";
const UPLOAD_PAGE_LIMIT: u32 = 200;
const NAVIGATION_PAGE_LIMIT: u32 = 200;
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

impl AdminDashboardService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        posts: Arc<dyn PostsRepo>,
        pages: Arc<dyn PagesRepo>,
        tags: Arc<dyn TagsRepo>,
        navigation: Arc<dyn NavigationRepo>,
        uploads: Arc<dyn UploadsRepo>,
        api_keys: Arc<dyn ApiKeysRepo>,
    ) -> Self {
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

    async fn collect_posts_panel(&self) -> Result<AdminDashboardPanelView, HttpError> {
        let repo = Arc::clone(&self.posts);
        let filter = PostQueryFilter::default();

        let total_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_posts(PostListScope::Admin { status: None }, &filter)
                    .await
                    .map_err(|err| repo_failure(POSTS_FAILURE_MESSAGE, err))
            }
        };
        let published_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_posts(
                    PostListScope::Admin {
                        status: Some(PostStatus::Published),
                    },
                    &filter,
                )
                .await
                .map_err(|err| repo_failure(POSTS_FAILURE_MESSAGE, err))
            }
        };
        let drafts_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_posts(
                    PostListScope::Admin {
                        status: Some(PostStatus::Draft),
                    },
                    &filter,
                )
                .await
                .map_err(|err| repo_failure(POSTS_FAILURE_MESSAGE, err))
            }
        };
        let archived_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_posts(
                    PostListScope::Admin {
                        status: Some(PostStatus::Archived),
                    },
                    &filter,
                )
                .await
                .map_err(|err| repo_failure(POSTS_FAILURE_MESSAGE, err))
            }
        };
        let errored_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_posts(
                    PostListScope::Admin {
                        status: Some(PostStatus::Error),
                    },
                    &filter,
                )
                .await
                .map_err(|err| repo_failure(POSTS_FAILURE_MESSAGE, err))
            }
        };

        let (total, published, drafts, archived, errored) = tokio::try_join!(
            total_future,
            published_future,
            drafts_future,
            archived_future,
            errored_future
        )?;

        let metrics = vec![
            AdminMetricView {
                label: "Total posts".to_string(),
                value: total,
                hint: None,
            },
            AdminMetricView {
                label: "Published".to_string(),
                value: published,
                hint: Some("Live on the public site".to_string()),
            },
            AdminMetricView {
                label: "Draft".to_string(),
                value: drafts,
                hint: Some("Awaiting editing or scheduling".to_string()),
            },
            AdminMetricView {
                label: "Archived".to_string(),
                value: archived,
                hint: Some("Hidden from public routes".to_string()),
            },
            AdminMetricView {
                label: "Error".to_string(),
                value: errored,
                hint: Some("Require intervention".to_string()),
            },
        ];

        Ok(AdminDashboardPanelView {
            title: "Posts".to_string(),
            caption: "Publication status overview".to_string(),
            metrics,
            empty_message: "No posts have been created yet.".to_string(),
        })
    }

    async fn collect_pages_panel(&self) -> Result<AdminDashboardPanelView, HttpError> {
        let repo = Arc::clone(&self.pages);
        let filter = PageQueryFilter::default();

        let total_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_pages(None, &filter)
                    .await
                    .map_err(|err| repo_failure(PAGES_FAILURE_MESSAGE, err))
            }
        };
        let published_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_pages(Some(PageStatus::Published), &filter)
                    .await
                    .map_err(|err| repo_failure(PAGES_FAILURE_MESSAGE, err))
            }
        };
        let drafts_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_pages(Some(PageStatus::Draft), &filter)
                    .await
                    .map_err(|err| repo_failure(PAGES_FAILURE_MESSAGE, err))
            }
        };
        let archived_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_pages(Some(PageStatus::Archived), &filter)
                    .await
                    .map_err(|err| repo_failure(PAGES_FAILURE_MESSAGE, err))
            }
        };
        let errored_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_pages(Some(PageStatus::Error), &filter)
                    .await
                    .map_err(|err| repo_failure(PAGES_FAILURE_MESSAGE, err))
            }
        };

        let (total, published, drafts, archived, errored) = tokio::try_join!(
            total_future,
            published_future,
            drafts_future,
            archived_future,
            errored_future
        )?;

        let metrics = vec![
            AdminMetricView {
                label: "Total pages".to_string(),
                value: total,
                hint: None,
            },
            AdminMetricView {
                label: "Published".to_string(),
                value: published,
                hint: Some("Linked from navigation and available publicly".to_string()),
            },
            AdminMetricView {
                label: "Draft".to_string(),
                value: drafts,
                hint: Some("Being edited or awaiting publication".to_string()),
            },
            AdminMetricView {
                label: "Archived".to_string(),
                value: archived,
                hint: Some("Removed from the public site".to_string()),
            },
            AdminMetricView {
                label: "Error".to_string(),
                value: errored,
                hint: Some("Require intervention".to_string()),
            },
        ];

        Ok(AdminDashboardPanelView {
            title: "Pages".to_string(),
            caption: "Standalone pages across the site".to_string(),
            metrics,
            empty_message: "No pages have been created yet.".to_string(),
        })
    }

    async fn collect_tags_panel(&self) -> Result<AdminDashboardPanelView, HttpError> {
        let repo = Arc::clone(&self.tags);
        let filter = TagQueryFilter::default();

        let total_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_tags(None, &filter)
                    .await
                    .map_err(|err| repo_failure(TAGS_FAILURE_MESSAGE, err))
            }
        };
        let pinned_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_tags(Some(true), &filter)
                    .await
                    .map_err(|err| repo_failure(TAGS_FAILURE_MESSAGE, err))
            }
        };
        let unpinned_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_tags(Some(false), &filter)
                    .await
                    .map_err(|err| repo_failure(TAGS_FAILURE_MESSAGE, err))
            }
        };
        let counts_future = {
            let repo = Arc::clone(&repo);
            async move {
                repo.list_with_counts()
                    .await
                    .map_err(|err| repo_failure(TAGS_LIST_FAILURE_MESSAGE, err))
            }
        };

        let (total, pinned, unpinned, tag_usage) =
            tokio::try_join!(total_future, pinned_future, unpinned_future, counts_future)?;

        let unused = tag_usage.iter().filter(|tag| tag.count == 0).count() as u64;

        let metrics = vec![
            AdminMetricView {
                label: "Total tags".to_string(),
                value: total,
                hint: None,
            },
            AdminMetricView {
                label: "Pinned".to_string(),
                value: pinned,
                hint: Some("Highlighted across the site".to_string()),
            },
            AdminMetricView {
                label: "Unpinned".to_string(),
                value: unpinned,
                hint: Some("Available but not highlighted".to_string()),
            },
            AdminMetricView {
                label: "Unused".to_string(),
                value: unused,
                hint: Some("Not assigned to any published post".to_string()),
            },
        ];

        Ok(AdminDashboardPanelView {
            title: "Tags".to_string(),
            caption: "Categorization across content inventory".to_string(),
            metrics,
            empty_message: "No tags have been created yet.".to_string(),
        })
    }

    async fn collect_navigation_panel(&self) -> Result<AdminDashboardPanelView, HttpError> {
        let repo = Arc::clone(&self.navigation);
        let filter = NavigationQueryFilter::default();

        let total_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_navigation(None, &filter)
                    .await
                    .map_err(|err| repo_failure(NAVIGATION_FAILURE_MESSAGE, err))
            }
        };
        let visible_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_navigation(Some(true), &filter)
                    .await
                    .map_err(|err| repo_failure(NAVIGATION_FAILURE_MESSAGE, err))
            }
        };
        let hidden_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_navigation(Some(false), &filter)
                    .await
                    .map_err(|err| repo_failure(NAVIGATION_FAILURE_MESSAGE, err))
            }
        };
        let external_future = count_external_navigation(Arc::clone(&repo), filter.clone());

        let (total, visible, hidden, external) =
            tokio::try_join!(total_future, visible_future, hidden_future, external_future)?;

        let metrics = vec![
            AdminMetricView {
                label: "Total items".to_string(),
                value: total,
                hint: None,
            },
            AdminMetricView {
                label: "Visible".to_string(),
                value: visible,
                hint: Some("Displayed in navigation menus".to_string()),
            },
            AdminMetricView {
                label: "Hidden".to_string(),
                value: hidden,
                hint: Some("Available for future publishing".to_string()),
            },
            AdminMetricView {
                label: "External links".to_string(),
                value: external,
                hint: Some("Link out to other sites".to_string()),
            },
        ];

        Ok(AdminDashboardPanelView {
            title: "Navigation".to_string(),
            caption: "Menu items and outbound links".to_string(),
            metrics,
            empty_message: "No navigation items have been created yet.".to_string(),
        })
    }

    async fn collect_uploads_panel(&self) -> Result<AdminDashboardPanelView, HttpError> {
        let repo = Arc::clone(&self.uploads);
        let filter = UploadQueryFilter::default();

        let total_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_uploads(&filter)
                    .await
                    .map_err(|err| repo_failure(UPLOADS_FAILURE_MESSAGE, err))
            }
        };
        let content_counts_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.content_type_counts(&filter)
                    .await
                    .map_err(|err| repo_failure(UPLOADS_FAILURE_MESSAGE, err))
            }
        };
        let total_bytes_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            sum_upload_sizes(repo, filter)
        };

        let (total, content_counts, total_bytes) =
            tokio::try_join!(total_future, content_counts_future, total_bytes_future)?;

        let mut images = 0_u64;
        let mut documents = 0_u64;
        let mut other = 0_u64;
        let mut categorized = 0_u64;

        for entry in content_counts {
            categorized = categorized.saturating_add(entry.count);
            if entry.content_type.starts_with("image/") {
                images = images.saturating_add(entry.count);
            } else if is_document_content_type(&entry.content_type) {
                documents = documents.saturating_add(entry.count);
            } else {
                other = other.saturating_add(entry.count);
            }
        }

        if categorized < total {
            other = other.saturating_add(total - categorized);
        }

        let metrics = vec![
            AdminMetricView {
                label: "Total uploads".to_string(),
                value: total,
                hint: Some(format!("Storage used: {}", format_bytes(total_bytes))),
            },
            AdminMetricView {
                label: "Images".to_string(),
                value: images,
                hint: Some("Content types starting with image/".to_string()),
            },
            AdminMetricView {
                label: "Documents".to_string(),
                value: documents,
                hint: Some("Common document formats (PDF, DOCX, Markdown, etc.)".to_string()),
            },
            AdminMetricView {
                label: "Other".to_string(),
                value: other,
                hint: Some("Media, archives, and miscellaneous uploads".to_string()),
            },
        ];

        Ok(AdminDashboardPanelView {
            title: "Uploads".to_string(),
            caption: "Stored media and documents".to_string(),
            metrics,
            empty_message: "No uploads have been stored yet.".to_string(),
        })
    }

    async fn collect_api_keys_panel(&self) -> Result<AdminDashboardPanelView, HttpError> {
        let repo = Arc::clone(&self.api_keys);

        let page = repo
            .list_keys(
                &ApiKeyQueryFilter::default(),
                ApiKeyPageRequest {
                    limit: 1,
                    cursor: None,
                },
            )
            .await
            .map_err(|err| repo_failure(API_KEYS_FAILURE_MESSAGE, err))?;

        let metrics = vec![
            AdminMetricView {
                label: "Total keys".to_string(),
                value: page.total,
                hint: Some("Issued across all scopes".to_string()),
            },
            AdminMetricView {
                label: "Active".to_string(),
                value: page.active,
                hint: Some("Valid and usable".to_string()),
            },
            AdminMetricView {
                label: "Revoked".to_string(),
                value: page.revoked,
                hint: Some("Explicitly disabled".to_string()),
            },
            AdminMetricView {
                label: "Expired".to_string(),
                value: page.expired,
                hint: Some("Past expiration timestamp".to_string()),
            },
        ];

        Ok(AdminDashboardPanelView {
            title: "API keys".to_string(),
            caption: "Lifecycle and validity".to_string(),
            metrics,
            empty_message: "No API keys have been issued yet.".to_string(),
        })
    }
}

async fn count_external_navigation(
    repo: Arc<dyn NavigationRepo>,
    filter: NavigationQueryFilter,
) -> Result<u64, HttpError> {
    let mut cursor: Option<NavigationCursor> = None;
    let mut external_total = 0_u64;

    loop {
        let page = repo
            .list_navigation(
                None,
                &filter,
                PageRequest::new(NAVIGATION_PAGE_LIMIT, cursor),
            )
            .await
            .map_err(|err| repo_failure(NAVIGATION_LIST_FAILURE_MESSAGE, err))?;

        external_total = external_total.saturating_add(
            page.items
                .iter()
                .filter(|item| item.destination_type == NavigationDestinationType::External)
                .count() as u64,
        );

        cursor = match page.next_cursor {
            Some(next) => {
                let decoded = NavigationCursor::decode(&next)
                    .map_err(|err| pagination_failure(NAVIGATION_CURSOR_FAILURE_MESSAGE, err))?;
                Some(decoded)
            }
            None => break,
        };
    }

    Ok(external_total)
}

async fn sum_upload_sizes(
    repo: Arc<dyn UploadsRepo>,
    filter: UploadQueryFilter,
) -> Result<u64, HttpError> {
    let mut cursor: Option<UploadCursor> = None;
    let mut total = 0_u64;

    loop {
        let page = repo
            .list_uploads(&filter, PageRequest::new(UPLOAD_PAGE_LIMIT, cursor))
            .await
            .map_err(|err| repo_failure(UPLOADS_LIST_FAILURE_MESSAGE, err))?;

        for record in &page.items {
            let size = u64::try_from(record.size_bytes).map_err(|_| {
                HttpError::new(
                    SOURCE,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    UPLOADS_FAILURE_MESSAGE,
                    format!(
                        "Upload `{}` reported invalid size {}",
                        record.id, record.size_bytes
                    ),
                )
            })?;

            total = total.checked_add(size).ok_or_else(|| {
                HttpError::new(
                    SOURCE,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    UPLOADS_FAILURE_MESSAGE,
                    "Summing upload sizes overflowed u64",
                )
            })?;
        }

        cursor = match page.next_cursor {
            Some(next) => {
                let decoded = UploadCursor::decode(&next)
                    .map_err(|err| pagination_failure(UPLOADS_CURSOR_FAILURE_MESSAGE, err))?;
                Some(decoded)
            }
            None => break,
        };
    }

    Ok(total)
}

fn repo_failure(message: &'static str, err: RepoError) -> HttpError {
    HttpError::new(
        SOURCE,
        StatusCode::INTERNAL_SERVER_ERROR,
        message,
        err.to_string(),
    )
}

fn pagination_failure(message: &'static str, err: PaginationError) -> HttpError {
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
