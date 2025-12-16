use std::sync::Arc;

use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::application::{
    admin::audit::AdminAuditService,
    pagination::{CursorPage, NavigationCursor, PageRequest},
    repos::{
        CreateNavigationItemParams, NavigationQueryFilter, NavigationRepo, NavigationWriteRepo,
        PageQueryFilter, PagesRepo, RepoError, UpdateNavigationItemParams,
    },
};
use crate::cache::CacheTrigger;
use crate::domain::entities::NavigationItemRecord;
use crate::domain::types::{NavigationDestinationType, PageStatus};

#[derive(Debug, Error)]
pub enum AdminNavigationError {
    #[error("{0}")]
    ConstraintViolation(&'static str),
    #[error(transparent)]
    Repo(#[from] RepoError),
}

#[derive(Debug, Clone)]
pub struct CreateNavigationItemCommand {
    pub label: String,
    pub destination_type: NavigationDestinationType,
    pub destination_page_id: Option<Uuid>,
    pub destination_url: Option<String>,
    pub sort_order: i32,
    pub visible: bool,
    pub open_in_new_tab: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateNavigationItemCommand {
    pub id: Uuid,
    pub label: String,
    pub destination_type: NavigationDestinationType,
    pub destination_page_id: Option<Uuid>,
    pub destination_url: Option<String>,
    pub sort_order: i32,
    pub visible: bool,
    pub open_in_new_tab: bool,
}

#[derive(Debug, Clone)]
pub struct NavigationStatusCounts {
    pub total: u64,
    pub visible: u64,
    pub hidden: u64,
}

#[derive(Clone)]
pub struct AdminNavigationService {
    reader: Arc<dyn NavigationRepo>,
    writer: Arc<dyn NavigationWriteRepo>,
    pages: Arc<dyn PagesRepo>,
    audit: AdminAuditService,
    cache_trigger: Option<Arc<CacheTrigger>>,
}

impl AdminNavigationService {
    pub fn new(
        reader: Arc<dyn NavigationRepo>,
        writer: Arc<dyn NavigationWriteRepo>,
        pages: Arc<dyn PagesRepo>,
        audit: AdminAuditService,
    ) -> Self {
        Self {
            reader,
            writer,
            pages,
            audit,
            cache_trigger: None,
        }
    }

    /// Set the cache trigger for this service.
    pub fn with_cache_trigger(mut self, trigger: Arc<CacheTrigger>) -> Self {
        self.cache_trigger = Some(trigger);
        self
    }

    /// Set the cache trigger for this service (optional).
    pub fn with_cache_trigger_opt(mut self, trigger: Option<Arc<CacheTrigger>>) -> Self {
        self.cache_trigger = trigger;
        self
    }

    pub async fn list(
        &self,
        visibility: Option<bool>,
        filter: &NavigationQueryFilter,
        page: PageRequest<NavigationCursor>,
    ) -> Result<CursorPage<NavigationItemRecord>, AdminNavigationError> {
        self.reader
            .list_navigation(visibility, filter, page)
            .await
            .map_err(AdminNavigationError::from)
    }

    pub async fn status_counts(
        &self,
        filter: &NavigationQueryFilter,
    ) -> Result<NavigationStatusCounts, AdminNavigationError> {
        let (total, visible, hidden) = tokio::try_join!(
            self.reader.count_navigation(None, filter),
            self.reader.count_navigation(Some(true), filter),
            self.reader.count_navigation(Some(false), filter)
        )
        .map_err(AdminNavigationError::from)?;

        Ok(NavigationStatusCounts {
            total,
            visible,
            hidden,
        })
    }

    pub async fn published_pages(
        &self,
    ) -> Result<Vec<crate::domain::entities::PageRecord>, AdminNavigationError> {
        let filter = PageQueryFilter::default();
        let pages = self
            .pages
            .list_pages(Some(PageStatus::Published), 200, None, &filter)
            .await
            .map_err(AdminNavigationError::from)?;
        Ok(pages.items)
    }

    pub async fn create_item(
        &self,
        actor: &str,
        command: CreateNavigationItemCommand,
    ) -> Result<NavigationItemRecord, AdminNavigationError> {
        ensure_non_empty(&command.label, "label")?;
        let (destination_page_id, destination_url) = match command.destination_type {
            NavigationDestinationType::Internal => {
                let page_id = command.destination_page_id.ok_or(
                    AdminNavigationError::ConstraintViolation("destination_page_id"),
                )?;
                let page_id = self.ensure_published_page(page_id).await?;
                (Some(page_id), None)
            }
            NavigationDestinationType::External => {
                let url = command
                    .destination_url
                    .as_ref()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .ok_or(AdminNavigationError::ConstraintViolation("destination_url"))?;
                (None, Some(url.to_string()))
            }
        };

        let params = CreateNavigationItemParams {
            label: command.label,
            destination_type: command.destination_type,
            destination_page_id,
            destination_url,
            sort_order: command.sort_order,
            visible: command.visible,
            open_in_new_tab: command.open_in_new_tab,
        };

        let item = self.writer.create_navigation_item(params).await?;
        let snapshot = NavigationSnapshot::from(&item);
        self.audit
            .record(
                actor,
                "navigation.create",
                "navigation",
                Some(&item.id.to_string()),
                Some(&snapshot),
            )
            .await?;

        // Trigger cache invalidation
        if let Some(trigger) = &self.cache_trigger {
            trigger.navigation_updated().await;
        }

        Ok(item)
    }

    pub async fn find_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<NavigationItemRecord>, AdminNavigationError> {
        self.reader
            .find_by_id(id)
            .await
            .map_err(AdminNavigationError::from)
    }

    pub async fn update_item(
        &self,
        actor: &str,
        command: UpdateNavigationItemCommand,
    ) -> Result<NavigationItemRecord, AdminNavigationError> {
        ensure_non_empty(&command.label, "label")?;
        let (destination_page_id, destination_url) = match command.destination_type {
            NavigationDestinationType::Internal => {
                let page_id = command.destination_page_id.ok_or(
                    AdminNavigationError::ConstraintViolation("destination_page_id"),
                )?;
                let page_id = self.ensure_published_page(page_id).await?;
                (Some(page_id), None)
            }
            NavigationDestinationType::External => {
                let url = command
                    .destination_url
                    .as_ref()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .ok_or(AdminNavigationError::ConstraintViolation("destination_url"))?;
                (None, Some(url.to_string()))
            }
        };

        let params = UpdateNavigationItemParams {
            id: command.id,
            label: command.label,
            destination_type: command.destination_type,
            destination_page_id,
            destination_url,
            sort_order: command.sort_order,
            visible: command.visible,
            open_in_new_tab: command.open_in_new_tab,
        };

        let item = self.writer.update_navigation_item(params).await?;
        let snapshot = NavigationSnapshot::from(&item);
        self.audit
            .record(
                actor,
                "navigation.update",
                "navigation",
                Some(&item.id.to_string()),
                Some(&snapshot),
            )
            .await?;

        // Trigger cache invalidation
        if let Some(trigger) = &self.cache_trigger {
            trigger.navigation_updated().await;
        }

        Ok(item)
    }

    pub async fn delete_item(&self, actor: &str, id: Uuid) -> Result<(), AdminNavigationError> {
        self.writer.delete_navigation_item(id).await?;
        self.audit
            .record(
                actor,
                "navigation.delete",
                "navigation",
                Some(&id.to_string()),
                Option::<&NavigationSnapshot>::None,
            )
            .await?;

        // Trigger cache invalidation
        if let Some(trigger) = &self.cache_trigger {
            trigger.navigation_updated().await;
        }

        Ok(())
    }
}

impl AdminNavigationService {
    async fn ensure_published_page(&self, page_id: Uuid) -> Result<Uuid, AdminNavigationError> {
        let page = self.pages.find_by_id(page_id).await?.ok_or(
            AdminNavigationError::ConstraintViolation("destination_page_id"),
        )?;

        if page.status != PageStatus::Published || page.published_at.is_none() {
            return Err(AdminNavigationError::ConstraintViolation(
                "destination_page_id",
            ));
        }

        Ok(page.id)
    }
}

#[derive(Debug, Serialize)]
struct NavigationSnapshot<'a> {
    label: &'a str,
    destination_type: NavigationDestinationType,
    destination: NavigationDestination<'a>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum NavigationDestination<'a> {
    Internal { slug: &'a str },
    External { url: &'a str },
}

impl<'a> From<&'a NavigationItemRecord> for NavigationSnapshot<'a> {
    fn from(item: &'a NavigationItemRecord) -> Self {
        let destination = match item.destination_type {
            NavigationDestinationType::Internal => NavigationDestination::Internal {
                slug: item.destination_page_slug.as_deref().unwrap_or_default(),
            },
            NavigationDestinationType::External => NavigationDestination::External {
                url: item.destination_url.as_deref().unwrap_or_default(),
            },
        };

        Self {
            label: item.label.as_str(),
            destination_type: item.destination_type,
            destination,
        }
    }
}

fn ensure_non_empty(value: &str, field: &'static str) -> Result<(), AdminNavigationError> {
    if value.trim().is_empty() {
        return Err(AdminNavigationError::ConstraintViolation(field));
    }
    Ok(())
}
