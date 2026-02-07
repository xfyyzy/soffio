use std::sync::Arc;

use crate::application::{error::HttpError, repos::PageQueryFilter};
use crate::domain::types::PageStatus;
use crate::presentation::admin::views::{AdminDashboardPanelView, AdminMetricView};

use super::{AdminDashboardService, PAGES_FAILURE_MESSAGE, repo_failure};

impl AdminDashboardService {
    pub(super) async fn collect_pages_panel(&self) -> Result<AdminDashboardPanelView, HttpError> {
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
}
