use uuid::Uuid;

use crate::application::pagination::{CursorPage, PageRequest, PostCursor};
use crate::application::repos::{PostListScope, PostQueryFilter, PostTagCount};
use crate::domain::entities::{PostRecord, PostSectionRecord};
use crate::domain::types::PostStatus;

use super::service::AdminPostService;
use super::types::{AdminPostError, AdminPostStatusCounts};

impl AdminPostService {
    pub async fn list(
        &self,
        status: Option<PostStatus>,
        filter: &PostQueryFilter,
        page: PageRequest<PostCursor>,
    ) -> Result<CursorPage<PostRecord>, AdminPostError> {
        self.reader
            .list_posts(PostListScope::Admin { status }, filter, page)
            .await
            .map_err(AdminPostError::from)
    }

    pub async fn load_post(&self, id: Uuid) -> Result<Option<PostRecord>, AdminPostError> {
        self.reader
            .find_by_id(id)
            .await
            .map_err(AdminPostError::from)
    }

    pub async fn load_sections(
        &self,
        post_id: Uuid,
    ) -> Result<Vec<PostSectionRecord>, AdminPostError> {
        self.sections
            .list_sections(post_id)
            .await
            .map_err(AdminPostError::from)
    }

    pub async fn status_counts(
        &self,
        filter: &PostQueryFilter,
    ) -> Result<AdminPostStatusCounts, AdminPostError> {
        let total_filter = filter.clone();
        let draft_filter = filter.clone();
        let published_filter = filter.clone();
        let archived_filter = filter.clone();
        let error_filter = filter.clone();

        let total_fut = self
            .reader
            .count_posts(PostListScope::Admin { status: None }, &total_filter);
        let draft_fut = self.reader.count_posts(
            PostListScope::Admin {
                status: Some(PostStatus::Draft),
            },
            &draft_filter,
        );
        let published_fut = self.reader.count_posts(
            PostListScope::Admin {
                status: Some(PostStatus::Published),
            },
            &published_filter,
        );
        let archived_fut = self.reader.count_posts(
            PostListScope::Admin {
                status: Some(PostStatus::Archived),
            },
            &archived_filter,
        );
        let error_fut = self.reader.count_posts(
            PostListScope::Admin {
                status: Some(PostStatus::Error),
            },
            &error_filter,
        );

        let (total, draft, published, archived, error) =
            tokio::try_join!(total_fut, draft_fut, published_fut, archived_fut, error_fut)?;

        Ok(AdminPostStatusCounts {
            total,
            draft,
            published,
            archived,
            error,
        })
    }

    pub async fn month_counts(
        &self,
        status: Option<PostStatus>,
        filter: &PostQueryFilter,
    ) -> Result<Vec<crate::domain::posts::MonthCount>, AdminPostError> {
        self.reader
            .list_month_counts(PostListScope::Admin { status }, filter)
            .await
            .map_err(AdminPostError::from)
    }

    pub async fn tag_counts(
        &self,
        status: Option<PostStatus>,
        filter: &PostQueryFilter,
    ) -> Result<Vec<PostTagCount>, AdminPostError> {
        self.reader
            .list_tag_counts(PostListScope::Admin { status }, filter)
            .await
            .map_err(AdminPostError::from)
    }
}
