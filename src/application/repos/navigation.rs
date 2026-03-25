use async_trait::async_trait;
use uuid::Uuid;

use crate::application::pagination::{CursorPage, NavigationCursor, PageRequest};
use crate::domain::entities::NavigationItemRecord;
use crate::domain::types::NavigationDestinationType;

use super::RepoError;

#[derive(Debug, Clone, Default)]
pub struct NavigationQueryFilter {
    pub search: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateNavigationItemParams {
    pub label: String,
    pub destination_type: NavigationDestinationType,
    pub destination_page_id: Option<Uuid>,
    pub destination_url: Option<String>,
    pub sort_order: i32,
    pub visible: bool,
    pub open_in_new_tab: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateNavigationItemParams {
    pub id: Uuid,
    pub label: String,
    pub destination_type: NavigationDestinationType,
    pub destination_page_id: Option<Uuid>,
    pub destination_url: Option<String>,
    pub sort_order: i32,
    pub visible: bool,
    pub open_in_new_tab: bool,
}

#[async_trait]
pub trait NavigationRepo: Send + Sync {
    async fn list_navigation(
        &self,
        visibility: Option<bool>,
        filter: &NavigationQueryFilter,
        page: PageRequest<NavigationCursor>,
    ) -> Result<CursorPage<NavigationItemRecord>, RepoError>;
    async fn count_navigation(
        &self,
        visibility: Option<bool>,
        filter: &NavigationQueryFilter,
    ) -> Result<u64, RepoError>;
    async fn count_external_navigation(
        &self,
        visibility: Option<bool>,
        filter: &NavigationQueryFilter,
    ) -> Result<u64, RepoError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<NavigationItemRecord>, RepoError>;
}

#[async_trait]
pub trait NavigationWriteRepo: Send + Sync {
    async fn create_navigation_item(
        &self,
        params: CreateNavigationItemParams,
    ) -> Result<NavigationItemRecord, RepoError>;

    async fn update_navigation_item(
        &self,
        params: UpdateNavigationItemParams,
    ) -> Result<NavigationItemRecord, RepoError>;

    async fn delete_navigation_item(&self, id: Uuid) -> Result<(), RepoError>;
}
