use std::sync::Arc;

use crate::application::{error::HttpError, repos::TagQueryFilter};
use crate::presentation::admin::views::{AdminDashboardPanelView, AdminMetricView};

use super::{AdminDashboardService, TAGS_FAILURE_MESSAGE, TAGS_LIST_FAILURE_MESSAGE, repo_failure};

impl AdminDashboardService {
    pub(super) async fn collect_tags_panel(&self) -> Result<AdminDashboardPanelView, HttpError> {
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
}
