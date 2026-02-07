use std::sync::Arc;

use crate::application::{
    error::HttpError,
    repos::{ApiKeyPageRequest, ApiKeyQueryFilter},
};
use crate::presentation::admin::views::{AdminDashboardPanelView, AdminMetricView};

use super::{API_KEYS_FAILURE_MESSAGE, AdminDashboardService, repo_failure};

impl AdminDashboardService {
    pub(super) async fn collect_api_keys_panel(
        &self,
    ) -> Result<AdminDashboardPanelView, HttpError> {
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
