use std::sync::Arc;

use crate::application::{error::HttpError, repos::UploadQueryFilter};
use crate::presentation::admin::views::{AdminDashboardPanelView, AdminMetricView};
use crate::util::bytes::format_bytes;

use super::{
    AdminDashboardService, UPLOADS_FAILURE_MESSAGE, is_document_content_type, repo_failure,
};

impl AdminDashboardService {
    pub(super) async fn collect_uploads_panel(&self) -> Result<AdminDashboardPanelView, HttpError> {
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
            async move {
                repo.sum_upload_sizes(&filter)
                    .await
                    .map_err(|err| repo_failure(UPLOADS_FAILURE_MESSAGE, err))
            }
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
}
