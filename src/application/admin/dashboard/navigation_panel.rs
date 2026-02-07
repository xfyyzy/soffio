use std::sync::Arc;

use crate::application::{error::HttpError, repos::NavigationQueryFilter};
use crate::presentation::admin::views::{AdminDashboardPanelView, AdminMetricView};

use super::{
    AdminDashboardService, NAVIGATION_FAILURE_MESSAGE, NAVIGATION_LIST_FAILURE_MESSAGE,
    repo_failure,
};

impl AdminDashboardService {
    pub(super) async fn collect_navigation_panel(
        &self,
    ) -> Result<AdminDashboardPanelView, HttpError> {
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
        let external_future = {
            let repo = Arc::clone(&repo);
            let filter = filter.clone();
            async move {
                repo.count_external_navigation(None, &filter)
                    .await
                    .map_err(|err| repo_failure(NAVIGATION_LIST_FAILURE_MESSAGE, err))
            }
        };

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
}
