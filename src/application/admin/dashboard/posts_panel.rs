use std::sync::Arc;

use crate::application::{
    error::HttpError,
    repos::{PostListScope, PostQueryFilter},
};
use crate::domain::types::PostStatus;
use crate::presentation::admin::views::{AdminDashboardPanelView, AdminMetricView};

use super::{AdminDashboardService, POSTS_FAILURE_MESSAGE, repo_failure};

impl AdminDashboardService {
    pub(super) async fn collect_posts_panel(&self) -> Result<AdminDashboardPanelView, HttpError> {
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
}
