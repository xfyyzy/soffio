use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::admin::audit::AdminAuditService;
use crate::application::pagination::{AuditCursor, CursorPage, PageRequest, TagCursor};
use crate::application::repos::{
    AuditQueryFilter, AuditRepo, CreateTagParams, RepoError, TagListRecord, TagQueryFilter,
    TagWithCount, TagsRepo, TagsWriteRepo, UpdateTagParams,
};
use crate::domain::entities::{AuditLogRecord, TagRecord};
use crate::domain::posts::MonthCount;

use super::{AdminTagError, AdminTagService};

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
        self.deleted.lock().expect("record deleted ids").push(id);
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

    assert_eq!(
        writer_ref.deleted.lock().expect("deleted ids").as_slice(),
        &[id]
    );
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

    async fn count_filtered(&self, _filter: &AuditQueryFilter) -> Result<u64, RepoError> {
        Ok(0)
    }

    async fn list_entity_type_counts(
        &self,
        _filter: &AuditQueryFilter,
    ) -> Result<Vec<crate::application::repos::AuditEntityTypeCount>, RepoError> {
        Ok(Vec::new())
    }

    async fn list_distinct_actors(
        &self,
        _filter: &AuditQueryFilter,
    ) -> Result<Vec<crate::application::repos::AuditActorCount>, RepoError> {
        Ok(Vec::new())
    }

    async fn list_distinct_actions(
        &self,
        _filter: &AuditQueryFilter,
    ) -> Result<Vec<crate::application::repos::AuditActionCount>, RepoError> {
        Ok(Vec::new())
    }

    async fn find_by_id(&self, _id: Uuid) -> Result<Option<AuditLogRecord>, RepoError> {
        Ok(None)
    }
}
