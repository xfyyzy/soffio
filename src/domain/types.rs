//! Shared domain enumerations aligned with persisted database enums.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "post_status", rename_all = "snake_case")]
pub enum PostStatus {
    Draft,
    Published,
    Archived,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "page_status", rename_all = "snake_case")]
pub enum PageStatus {
    Draft,
    Published,
    Archived,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "navigation_destination_type", rename_all = "snake_case")]
pub enum NavigationDestinationType {
    Internal,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Pending,
    Scheduled,
    Running,
    Done,
    Failed,
    Killed,
}

impl JobState {
    pub fn as_str(self) -> &'static str {
        match self {
            JobState::Pending => "Pending",
            JobState::Scheduled => "Scheduled",
            JobState::Running => "Running",
            JobState::Done => "Done",
            JobState::Failed => "Failed",
            JobState::Killed => "Killed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    RenderPost,
    RenderPostSections,
    RenderPostSection,
    RenderPage,
    RenderSummary,
    PublishPost,
    PublishPage,
}

/// Supported snapshot entity types (mirrors Postgres enum `snapshot_entity_type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "snapshot_entity_type", rename_all = "snake_case")]
pub enum SnapshotEntityType {
    Post,
    Page,
}

impl JobType {
    pub fn as_str(self) -> &'static str {
        match self {
            JobType::RenderPost => "render_post",
            JobType::RenderPostSections => "render_post_sections",
            JobType::RenderPostSection => "render_post_section",
            JobType::RenderPage => "render_page",
            JobType::RenderSummary => "render_summary",
            JobType::PublishPost => "publish_post",
            JobType::PublishPage => "publish_page",
        }
    }
}

impl TryFrom<&str> for JobType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "render_post" => Ok(JobType::RenderPost),
            "render_post_sections" => Ok(JobType::RenderPostSections),
            "render_post_section" => Ok(JobType::RenderPostSection),
            "render_page" => Ok(JobType::RenderPage),
            "render_summary" => Ok(JobType::RenderSummary),
            "publish_post" => Ok(JobType::PublishPost),
            "publish_page" => Ok(JobType::PublishPage),
            _ => Err(()),
        }
    }
}

impl TryFrom<&str> for JobState {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Pending" | "Latest" => Ok(JobState::Pending),
            "Scheduled" => Ok(JobState::Scheduled),
            "Running" => Ok(JobState::Running),
            "Done" => Ok(JobState::Done),
            "Failed" => Ok(JobState::Failed),
            "Killed" => Ok(JobState::Killed),
            _ => Err(()),
        }
    }
}
