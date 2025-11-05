use std::sync::Arc;

use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::application::admin::audit::AdminAuditService;
use crate::application::pagination::{CursorPage, PageRequest, TagCursor};
use crate::application::repos::{
    CreateTagParams, RepoError, TagListRecord, TagQueryFilter, TagWithCount, TagsRepo,
    TagsWriteRepo, UpdateTagParams,
};
use crate::domain::entities::TagRecord;
use crate::domain::posts::MonthCount;
use crate::domain::slug::{SlugAsyncError, SlugError, generate_unique_slug_async};

#[derive(Debug, Error)]
pub enum AdminTagError {
    #[error("{0}")]
    ConstraintViolation(&'static str),
    #[error("tag is referenced by {count} posts")]
    InUse { count: u64 },
    #[error(transparent)]
    Repo(#[from] RepoError),
}

#[derive(Debug, Clone)]
pub struct CreateTagCommand {
    pub name: String,
    pub description: Option<String>,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateTagCommand {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdminTagStatusCounts {
    pub total: u64,
    pub pinned: u64,
    pub unpinned: u64,
}

#[derive(Clone)]
pub struct AdminTagService {
    reader: Arc<dyn TagsRepo>,
    writer: Arc<dyn TagsWriteRepo>,
    audit: AdminAuditService,
}

impl AdminTagService {
    pub fn new(
        reader: Arc<dyn TagsRepo>,
        writer: Arc<dyn TagsWriteRepo>,
        audit: AdminAuditService,
    ) -> Self {
        Self {
            reader,
            writer,
            audit,
        }
    }

    pub async fn list_all(&self) -> Result<Vec<TagRecord>, AdminTagError> {
        self.reader.list_all().await.map_err(AdminTagError::from)
    }

    pub async fn list_with_counts(&self) -> Result<Vec<TagWithCount>, AdminTagError> {
        self.reader
            .list_with_counts()
            .await
            .map_err(AdminTagError::from)
    }

    pub async fn list_for_post(&self, post_id: Uuid) -> Result<Vec<TagRecord>, AdminTagError> {
        self.reader
            .list_for_post(post_id)
            .await
            .map_err(AdminTagError::from)
    }

    pub async fn list(
        &self,
        pinned: Option<bool>,
        filter: &TagQueryFilter,
        page: PageRequest<TagCursor>,
    ) -> Result<CursorPage<TagListRecord>, AdminTagError> {
        self.reader
            .list_admin_tags(pinned, filter, page)
            .await
            .map_err(AdminTagError::from)
    }

    pub async fn status_counts(
        &self,
        filter: &TagQueryFilter,
    ) -> Result<AdminTagStatusCounts, AdminTagError> {
        let total = self.reader.count_tags(None, filter).await?;
        let pinned = self.reader.count_tags(Some(true), filter).await?;
        let unpinned = self.reader.count_tags(Some(false), filter).await?;

        Ok(AdminTagStatusCounts {
            total,
            pinned,
            unpinned,
        })
    }

    pub async fn month_counts(
        &self,
        pinned: Option<bool>,
        filter: &TagQueryFilter,
    ) -> Result<Vec<MonthCount>, AdminTagError> {
        self.reader
            .month_counts(pinned, filter)
            .await
            .map_err(AdminTagError::from)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<TagRecord>, AdminTagError> {
        self.reader
            .find_by_id(id)
            .await
            .map_err(AdminTagError::from)
    }

    pub async fn create_tag(
        &self,
        actor: &str,
        command: CreateTagCommand,
    ) -> Result<TagRecord, AdminTagError> {
        ensure_non_empty(&command.name, "name")?;

        let CreateTagCommand {
            name,
            description,
            pinned,
        } = command;

        let name = name.trim().to_string();
        ensure_non_empty(&name, "name")?;

        let description = description.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

        let reader = self.reader.clone();
        let slug = match generate_unique_slug_async(&name, move |candidate| {
            let reader = reader.clone();
            let candidate = candidate.to_string();
            async move {
                reader
                    .find_by_slug(&candidate)
                    .await
                    .map(|existing| existing.is_none())
            }
        })
        .await
        {
            Ok(slug) => slug,
            Err(SlugAsyncError::Slug(err)) => match err {
                SlugError::EmptyInput | SlugError::Unrepresentable { .. } => {
                    return Err(AdminTagError::ConstraintViolation("name"));
                }
                SlugError::Exhausted { .. } => {
                    return Err(AdminTagError::ConstraintViolation("slug"));
                }
            },
            Err(SlugAsyncError::Predicate(err)) => return Err(AdminTagError::Repo(err)),
        };

        let params = CreateTagParams {
            slug,
            name,
            description,
            pinned,
        };

        let tag = self.writer.create_tag(params).await?;
        let snapshot = TagSnapshot {
            slug: tag.slug.as_str(),
            name: tag.name.as_str(),
        };
        self.audit
            .record(
                actor,
                "tag.create",
                "tag",
                Some(&tag.id.to_string()),
                Some(&snapshot),
            )
            .await?;
        Ok(tag)
    }

    pub async fn update_tag(
        &self,
        actor: &str,
        command: UpdateTagCommand,
    ) -> Result<TagRecord, AdminTagError> {
        ensure_non_empty(&command.name, "name")?;

        let UpdateTagCommand {
            id,
            name,
            description,
            pinned,
        } = command;

        let name = name.trim().to_string();
        ensure_non_empty(&name, "name")?;

        let description = description.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

        let existing = self
            .reader
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepoError::from_persistence("tag not found"))?;

        let params = UpdateTagParams {
            id,
            slug: existing.slug.clone(),
            name,
            description,
            pinned,
        };

        let tag = self.writer.update_tag(params).await?;
        let snapshot = TagSnapshot {
            slug: tag.slug.as_str(),
            name: tag.name.as_str(),
        };
        self.audit
            .record(
                actor,
                "tag.update",
                "tag",
                Some(&tag.id.to_string()),
                Some(&snapshot),
            )
            .await?;
        Ok(tag)
    }

    pub async fn update_tag_pinned(
        &self,
        actor: &str,
        id: Uuid,
        pinned: bool,
    ) -> Result<TagRecord, AdminTagError> {
        let existing = self
            .reader
            .find_by_id(id)
            .await?
            .ok_or_else(|| AdminTagError::Repo(RepoError::from_persistence("tag not found")))?;

        let command = UpdateTagCommand {
            id,
            name: existing.name.clone(),
            description: existing.description.clone(),
            pinned,
        };

        self.update_tag(actor, command).await
    }

    pub async fn delete_tag(&self, actor: &str, id: Uuid) -> Result<(), AdminTagError> {
        let usage = self.reader.count_usage(id).await?;
        if usage > 0 {
            return Err(AdminTagError::InUse { count: usage });
        }

        self.writer.delete_tag(id).await?;
        self.audit
            .record(
                actor,
                "tag.delete",
                "tag",
                Some(&id.to_string()),
                Option::<&TagSnapshot>::None,
            )
            .await?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct TagSnapshot<'a> {
    slug: &'a str,
    name: &'a str,
}

fn ensure_non_empty(value: &str, field: &'static str) -> Result<(), AdminTagError> {
    if value.trim().is_empty() {
        return Err(AdminTagError::ConstraintViolation(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};
    use time::OffsetDateTime;

    use crate::application::pagination::{AuditCursor, CursorPage, PageRequest, TagCursor};
    use crate::application::repos::{AuditQueryFilter, AuditRepo, TagListRecord, TagQueryFilter};
    use crate::domain::entities::AuditLogRecord;
    use crate::domain::posts::MonthCount;

    #[derive(Clone, Default)]
    struct StubTagsRepo {
        usage: u64,
        record: Option<TagRecord>,
    }

    #[async_trait]
    impl TagsRepo for StubTagsRepo {
        async fn list_all(&self) -> Result<Vec<TagRecord>, RepoError> {
            Ok(Vec::new())
        }

        async fn list_for_post(&self, _post_id: Uuid) -> Result<Vec<TagRecord>, RepoError> {
            Ok(Vec::new())
        }

        async fn list_with_counts(&self) -> Result<Vec<TagWithCount>, RepoError> {
            Ok(Vec::new())
        }

        async fn find_by_id(&self, id: Uuid) -> Result<Option<TagRecord>, RepoError> {
            let record = self.record.clone().filter(|tag| tag.id == id);
            Ok(record)
        }

        async fn find_by_slug(&self, slug: &str) -> Result<Option<TagRecord>, RepoError> {
            let record = self.record.clone().filter(|tag| tag.slug == slug);
            Ok(record)
        }

        async fn count_usage(&self, _id: Uuid) -> Result<u64, RepoError> {
            Ok(self.usage)
        }

        async fn list_admin_tags(
            &self,
            _pinned: Option<bool>,
            _filter: &TagQueryFilter,
            _page: PageRequest<TagCursor>,
        ) -> Result<CursorPage<TagListRecord>, RepoError> {
            Ok(CursorPage::empty())
        }

        async fn count_tags(
            &self,
            _pinned: Option<bool>,
            _filter: &TagQueryFilter,
        ) -> Result<u64, RepoError> {
            Ok(0)
        }

        async fn month_counts(
            &self,
            _pinned: Option<bool>,
            _filter: &TagQueryFilter,
        ) -> Result<Vec<MonthCount>, RepoError> {
            Ok(Vec::new())
        }
    }

    #[derive(Default)]
    struct RecordingTagsWriter {
        deleted: Mutex<Vec<Uuid>>,
    }

    #[async_trait]
    impl TagsWriteRepo for RecordingTagsWriter {
        async fn create_tag(&self, _params: CreateTagParams) -> Result<TagRecord, RepoError> {
            unreachable!("not used in these tests")
        }

        async fn update_tag(&self, _params: UpdateTagParams) -> Result<TagRecord, RepoError> {
            unreachable!("not used in these tests")
        }

        async fn delete_tag(&self, id: Uuid) -> Result<(), RepoError> {
            self.deleted.lock().unwrap().push(id);
            Ok(())
        }
    }

    fn sample_tag(id: Uuid) -> TagRecord {
        TagRecord {
            id,
            slug: "sample".into(),
            name: "Sample".into(),
            description: None,
            pinned: false,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    #[tokio::test]
    async fn delete_tag_rejects_when_in_use() {
        let id = Uuid::new_v4();
        let reader = StubTagsRepo {
            usage: 3,
            record: Some(sample_tag(id)),
        };
        let writer: Arc<dyn TagsWriteRepo> = Arc::new(RecordingTagsWriter::default());
        let audit_repo: Arc<dyn AuditRepo> = Arc::new(FakeAuditRepo);
        let audit = AdminAuditService::new(audit_repo);
        let service = AdminTagService::new(Arc::new(reader), writer, audit);

        let result = service.delete_tag("tester", id).await;
        match result {
            Err(AdminTagError::InUse { count }) => assert_eq!(count, 3),
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[tokio::test]
    async fn delete_tag_allows_when_unused() {
        let id = Uuid::new_v4();
        let reader = StubTagsRepo {
            usage: 0,
            record: Some(sample_tag(id)),
        };
        let writer_ref = Arc::new(RecordingTagsWriter::default());
        let writer: Arc<dyn TagsWriteRepo> = writer_ref.clone();
        let audit_repo: Arc<dyn AuditRepo> = Arc::new(FakeAuditRepo);
        let audit = AdminAuditService::new(audit_repo);
        let service = AdminTagService::new(Arc::new(reader), writer, audit);

        service
            .delete_tag("tester", id)
            .await
            .expect("delete succeeds");

        assert_eq!(writer_ref.deleted.lock().unwrap().as_slice(), &[id]);
    }

    #[derive(Default)]
    struct FakeAuditRepo;

    #[async_trait]
    impl AuditRepo for FakeAuditRepo {
        async fn append_log(&self, _record: AuditLogRecord) -> Result<(), RepoError> {
            Ok(())
        }

        async fn list_recent(&self, _limit: u32) -> Result<Vec<AuditLogRecord>, RepoError> {
            Ok(Vec::new())
        }

        async fn list_filtered(
            &self,
            _page: PageRequest<AuditCursor>,
            _filter: &AuditQueryFilter,
        ) -> Result<CursorPage<AuditLogRecord>, RepoError> {
            Ok(CursorPage::empty())
        }
    }
}
