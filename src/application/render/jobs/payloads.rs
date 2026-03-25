use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::render::RenderedSection;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPostJobPayload {
    pub slug: String,
    pub body_markdown: String,
    pub summary_markdown: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPostSectionsJobPayload {
    pub tracking_id: String,
    pub post_id: Uuid,
    pub slug: String,
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPostSectionJobPayload {
    pub tracking_id: String,
    pub post_id: Uuid,
    pub slug: String,
    pub section: RenderedSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderSummaryJobPayload {
    pub tracking_id: String,
    pub post_id: Uuid,
    pub slug: String,
    pub summary_markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPageJobPayload {
    pub slug: String,
    pub markdown: String,
}
