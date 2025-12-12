use std::sync::Arc;

use serde_json::Value;
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::error::AppError;
use crate::application::jobs::invalidate_and_enqueue_warm;
use crate::application::pagination::{CursorPage, PageRequest, SnapshotCursor};
use crate::application::repos::{
    JobsRepo, RepoError, SnapshotFilter, SnapshotRecord, SnapshotsRepo,
};
use crate::domain::snapshots::{SnapshotError, Snapshotable};
use crate::infra::cache::{CacheWarmDebouncer, ResponseCache};

use futures::future::Future;

#[derive(Debug, Error)]
pub enum SnapshotServiceError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Snapshot(#[from] SnapshotError),
    #[error(transparent)]
    App(#[from] AppError),
    #[error("snapshot not found")]
    NotFound,
}

#[derive(Clone)]
pub struct AdminSnapshotService {
    repo: Arc<dyn SnapshotsRepo>,
    jobs: Arc<dyn JobsRepo>,
    cache: Arc<ResponseCache>,
    debouncer: Arc<CacheWarmDebouncer>,
}

impl AdminSnapshotService {
    pub fn new(
        repo: Arc<dyn SnapshotsRepo>,
        jobs: Arc<dyn JobsRepo>,
        cache: Arc<ResponseCache>,
        debouncer: Arc<CacheWarmDebouncer>,
    ) -> Self {
        Self {
            repo,
            jobs,
            cache,
            debouncer,
        }
    }

    pub async fn create<E: Snapshotable<Id = Uuid>>(
        &self,
        _actor: &str,
        entity: &E,
        description: Option<String>,
    ) -> Result<SnapshotRecord, SnapshotServiceError> {
        let payload = entity.to_snapshot()?;
        E::validate_snapshot(&payload)?;
        let content: Value =
            serde_json::to_value(&payload).map_err(|e| AppError::unexpected(e.to_string()))?;

        let entity_type = E::ENTITY_TYPE;
        let entity_id = *entity.id();

        let schema_version = self.repo.current_schema_version().await?;

        let version = self.next_version(entity_type, entity_id).await?;

        let record = SnapshotRecord {
            id: Uuid::new_v4(),
            entity_type,
            entity_id,
            version,
            description,
            schema_version,
            content,
            created_at: OffsetDateTime::now_utc(),
        };

        self.repo.create(record.clone()).await?;
        Ok(record)
    }

    pub async fn rollback<E, ApplyFn, Fut>(
        &self,
        actor: &str,
        snapshot_id: Uuid,
        apply: ApplyFn,
    ) -> Result<SnapshotRecord, SnapshotServiceError>
    where
        E: Snapshotable<Id = Uuid>,
        ApplyFn: FnOnce(E::Payload) -> Fut,
        Fut: Future<Output = Result<(), SnapshotServiceError>>,
    {
        let snapshot = self
            .repo
            .find_snapshot(snapshot_id)
            .await?
            .ok_or(SnapshotServiceError::NotFound)?;

        if snapshot.entity_type != E::ENTITY_TYPE {
            return Err(SnapshotServiceError::Snapshot(SnapshotError::Validation(
                "entity type mismatch".to_string(),
            )));
        }

        let payload: E::Payload = serde_json::from_value(snapshot.content.clone())
            .map_err(|e| AppError::unexpected(e.to_string()))?;
        E::validate_snapshot(&payload)?;

        apply(payload).await?;

        invalidate_and_enqueue_warm(
            self.cache.as_ref(),
            self.debouncer.as_ref(),
            self.jobs.as_ref(),
            Some(format!("snapshot.rollback:{}:{}", actor, snapshot.id)),
        )
        .await?;

        Ok(snapshot)
    }

    pub async fn list(
        &self,
        filter: &SnapshotFilter,
        page: PageRequest<SnapshotCursor>,
    ) -> Result<CursorPage<SnapshotRecord>, SnapshotServiceError> {
        Ok(self.repo.list_snapshots(filter, page).await?)
    }

    pub async fn count(&self, filter: &SnapshotFilter) -> Result<u64, SnapshotServiceError> {
        Ok(self.repo.count_snapshots(filter).await?)
    }

    pub async fn month_counts(
        &self,
        filter: &SnapshotFilter,
    ) -> Result<Vec<crate::application::repos::SnapshotMonthCount>, SnapshotServiceError> {
        Ok(self.repo.month_counts(filter).await?)
    }

    pub async fn find(&self, id: Uuid) -> Result<Option<SnapshotRecord>, SnapshotServiceError> {
        Ok(self.repo.find_snapshot(id).await?)
    }

    pub async fn next_version(
        &self,
        entity_type: crate::domain::types::SnapshotEntityType,
        entity_id: Uuid,
    ) -> Result<i32, SnapshotServiceError> {
        let next = self
            .repo
            .latest_snapshot(entity_type, entity_id)
            .await?
            .map(|s| s.version + 1)
            .unwrap_or(1);

        Ok(next)
    }
}
