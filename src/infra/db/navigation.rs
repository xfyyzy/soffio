use async_trait::async_trait;
use sqlx::QueryBuilder;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::{
        pagination::{CursorPage, NavigationCursor, PageRequest},
        repos::{
            CreateNavigationItemParams, NavigationQueryFilter, NavigationRepo, NavigationWriteRepo,
            RepoError, UpdateNavigationItemParams,
        },
    },
    domain::{entities::NavigationItemRecord, types::NavigationDestinationType},
};

use super::PostgresRepositories;

fn map_sqlx_error(err: sqlx::Error) -> RepoError {
    match err {
        sqlx::Error::RowNotFound => RepoError::NotFound,
        sqlx::Error::Database(db) if db.message().contains("duplicate key") => {
            RepoError::Duplicate {
                constraint: db.constraint().unwrap_or("unknown").to_string(),
            }
        }
        sqlx::Error::Database(db)
            if db.message().contains("violates foreign key constraint")
                || db.message().contains("invalid input syntax") =>
        {
            RepoError::InvalidInput {
                message: db.message().to_string(),
            }
        }
        sqlx::Error::Database(db) if db.message().contains("violates") => RepoError::Integrity {
            message: db.message().to_string(),
        },
        sqlx::Error::Database(db)
            if db
                .message()
                .contains("canceling statement due to user request") =>
        {
            RepoError::Timeout
        }
        other => RepoError::from_persistence(other),
    }
}

#[derive(sqlx::FromRow)]
struct NavigationItemRow {
    id: Uuid,
    label: String,
    destination_type: NavigationDestinationType,
    destination_page_id: Option<Uuid>,
    destination_page_slug: Option<String>,
    destination_url: Option<String>,
    sort_order: i32,
    visible: bool,
    open_in_new_tab: bool,
    created_at: OffsetDateTime,
    primary_time: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl From<NavigationItemRow> for NavigationItemRecord {
    fn from(row: NavigationItemRow) -> Self {
        Self {
            id: row.id,
            label: row.label,
            destination_type: row.destination_type,
            destination_page_id: row.destination_page_id,
            destination_page_slug: row.destination_page_slug,
            destination_url: row.destination_url,
            sort_order: row.sort_order,
            visible: row.visible,
            open_in_new_tab: row.open_in_new_tab,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl NavigationRepo for PostgresRepositories {
    async fn list_navigation(
        &self,
        visibility: Option<bool>,
        filter: &NavigationQueryFilter,
        page: PageRequest<NavigationCursor>,
    ) -> Result<CursorPage<NavigationItemRecord>, RepoError> {
        let mut qb = QueryBuilder::new(
            "SELECT ni.id, ni.label, ni.destination_type, ni.destination_page_id, \
             p.slug AS destination_page_slug, ni.destination_url, \
             ni.sort_order, ni.visible, ni.open_in_new_tab, ni.created_at, \
             COALESCE(ni.updated_at, ni.created_at) AS primary_time, ni.updated_at \
             FROM navigation_items ni \
             LEFT JOIN pages p ON p.id = ni.destination_page_id \
             WHERE 1=1 ",
        );

        if let Some(visibility) = visibility {
            qb.push("AND ni.visible = ");
            qb.push_bind(visibility);
            qb.push(" ");
        }

        if let Some(search) = filter.search.as_ref().and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }) {
            let pattern = format!("%{}%", search);
            qb.push(" AND (");
            qb.push("ni.label ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(" OR (p.slug IS NOT NULL AND p.slug ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(") OR (ni.destination_url IS NOT NULL AND ni.destination_url ILIKE ");
            qb.push_bind(pattern);
            qb.push(")) ");
        }

        qb.push(
            " AND (ni.destination_type <> 'internal'::navigation_destination_type \
                 OR (p.status = 'published'::page_status AND p.published_at IS NOT NULL))",
        );

        if let Some(cursor) = page.cursor {
            qb.push(" AND (");
            qb.push("ni.sort_order > ");
            qb.push_bind(cursor.sort_order());
            qb.push(" OR (ni.sort_order = ");
            qb.push_bind(cursor.sort_order());
            qb.push(" AND COALESCE(ni.updated_at, ni.created_at) < ");
            qb.push_bind(cursor.primary_time());
            qb.push(") OR (ni.sort_order = ");
            qb.push_bind(cursor.sort_order());
            qb.push(" AND COALESCE(ni.updated_at, ni.created_at) = ");
            qb.push_bind(cursor.primary_time());
            qb.push(" AND ni.id > ");
            qb.push_bind(cursor.id());
            qb.push("))");
        }

        qb.push(" ORDER BY ni.sort_order ASC, primary_time DESC, ni.id ASC ");
        qb.push(" LIMIT ");
        qb.push_bind((page.limit + 1) as i64);

        let mut rows = qb
            .build_query_as::<NavigationItemRow>()
            .fetch_all(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        let has_more = (rows.len() as u32) > page.limit;
        let next_cursor = if has_more {
            let extra = rows
                .pop()
                .expect("navigation query should have at least one extra row when truncated");
            let cursor = NavigationCursor::new(extra.sort_order, extra.primary_time, extra.id);
            Some(cursor.encode())
        } else {
            None
        };

        let records = rows.into_iter().map(NavigationItemRecord::from).collect();

        Ok(CursorPage::new(records, next_cursor))
    }

    async fn count_navigation(
        &self,
        visibility: Option<bool>,
        filter: &NavigationQueryFilter,
    ) -> Result<u64, RepoError> {
        let mut qb = QueryBuilder::new(
            "SELECT COUNT(*) AS count \
             FROM navigation_items ni \
             LEFT JOIN pages p ON p.id = ni.destination_page_id \
             WHERE 1=1 ",
        );

        if let Some(visibility) = visibility {
            qb.push("AND ni.visible = ");
            qb.push_bind(visibility);
            qb.push(" ");
        }

        if let Some(search) = filter.search.as_ref().and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }) {
            let pattern = format!("%{}%", search);
            qb.push(" AND (");
            qb.push("ni.label ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(" OR (p.slug IS NOT NULL AND p.slug ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(") OR (ni.destination_url IS NOT NULL AND ni.destination_url ILIKE ");
            qb.push_bind(pattern);
            qb.push(")) ");
        }

        qb.push(
            " AND (ni.destination_type <> 'internal'::navigation_destination_type \
                 OR (p.status = 'published'::page_status AND p.published_at IS NOT NULL))",
        );

        let count: i64 = qb
            .build_query_scalar()
            .fetch_one(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        Ok(u64::try_from(count).map_err(|e| RepoError::InvalidInput {
            message: e.to_string(),
        })?)
    }

    async fn count_external_navigation(
        &self,
        visibility: Option<bool>,
        filter: &NavigationQueryFilter,
    ) -> Result<u64, RepoError> {
        let mut qb = QueryBuilder::new(
            "SELECT COUNT(*) AS count \
             FROM navigation_items ni \
             LEFT JOIN pages p ON p.id = ni.destination_page_id \
             WHERE 1=1 ",
        );

        if let Some(visibility) = visibility {
            qb.push("AND ni.visible = ");
            qb.push_bind(visibility);
            qb.push(" ");
        }

        if let Some(search) = filter.search.as_ref().and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }) {
            let pattern = format!("%{}%", search);
            qb.push(" AND (");
            qb.push("ni.label ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(" OR (p.slug IS NOT NULL AND p.slug ILIKE ");
            qb.push_bind(pattern.clone());
            qb.push(") OR (ni.destination_url IS NOT NULL AND ni.destination_url ILIKE ");
            qb.push_bind(pattern);
            qb.push(")) ");
        }

        qb.push(" AND ni.destination_type = 'external'::navigation_destination_type ");
        qb.push(
            " AND (ni.destination_type <> 'internal'::navigation_destination_type \
                 OR (p.status = 'published'::page_status AND p.published_at IS NOT NULL))",
        );

        let count: i64 = qb
            .build_query_scalar()
            .fetch_one(self.pool())
            .await
            .map_err(map_sqlx_error)?;

        Ok(u64::try_from(count).map_err(|e| RepoError::InvalidInput {
            message: e.to_string(),
        })?)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<NavigationItemRecord>, RepoError> {
        let row = sqlx::query_as!(
            NavigationItemRow,
            r#"
            SELECT ni.id,
                   ni.label,
                   ni.destination_type AS "destination_type: NavigationDestinationType",
                   ni.destination_page_id,
                   p.slug AS "destination_page_slug?",
                    ni.destination_url,
                    ni.sort_order,
                    ni.visible,
                    ni.open_in_new_tab,
                    ni.created_at,
                    COALESCE(ni.updated_at, ni.created_at) AS "primary_time!",
                    ni.updated_at
            FROM navigation_items ni
            LEFT JOIN pages p ON p.id = ni.destination_page_id
            WHERE ni.id = $1
            "#,
            id
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(row.map(NavigationItemRecord::from))
    }
}

#[async_trait]
impl NavigationWriteRepo for PostgresRepositories {
    async fn create_navigation_item(
        &self,
        params: CreateNavigationItemParams,
    ) -> Result<NavigationItemRecord, RepoError> {
        let CreateNavigationItemParams {
            label,
            destination_type,
            destination_page_id,
            destination_url,
            sort_order,
            visible,
            open_in_new_tab,
        } = params;

        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc();

        let row = sqlx::query_as!(
            NavigationItemRow,
            r#"
            INSERT INTO navigation_items (
                id, label, destination_type, destination_page_id, destination_url,
                sort_order, visible, open_in_new_tab, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9)
            RETURNING id, label, destination_type AS "destination_type: NavigationDestinationType",
                     destination_page_id,
                     (SELECT slug FROM pages WHERE id = destination_page_id) AS "destination_page_slug?",
                     destination_url, sort_order, visible, open_in_new_tab,
                     created_at,
                     COALESCE(updated_at, created_at) AS "primary_time!",
                     updated_at
            "#,
            id,
            label,
            destination_type as NavigationDestinationType,
            destination_page_id,
            destination_url,
            sort_order,
            visible,
            open_in_new_tab,
            now
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(NavigationItemRecord::from(row))
    }

    async fn update_navigation_item(
        &self,
        params: UpdateNavigationItemParams,
    ) -> Result<NavigationItemRecord, RepoError> {
        let UpdateNavigationItemParams {
            id,
            label,
            destination_type,
            destination_page_id,
            destination_url,
            sort_order,
            visible,
            open_in_new_tab,
        } = params;

        let row = sqlx::query_as!(
            NavigationItemRow,
            r#"
            UPDATE navigation_items
            SET label = $2,
                destination_type = $3,
                destination_page_id = $4,
                destination_url = $5,
                sort_order = $6,
                visible = $7,
                open_in_new_tab = $8,
                updated_at = now()
            WHERE id = $1
            RETURNING id, label, destination_type AS "destination_type: NavigationDestinationType",
                     destination_page_id,
                     (SELECT slug FROM pages WHERE id = destination_page_id) AS "destination_page_slug?",
                     destination_url, sort_order, visible, open_in_new_tab,
                     created_at,
                     COALESCE(updated_at, created_at) AS "primary_time!",
                     updated_at
            "#,
            id,
            label,
            destination_type as NavigationDestinationType,
            destination_page_id,
            destination_url,
            sort_order,
            visible,
            open_in_new_tab
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(NavigationItemRecord::from(row))
    }

    async fn delete_navigation_item(&self, id: Uuid) -> Result<(), RepoError> {
        sqlx::query!(
            r#"
            DELETE FROM navigation_items
            WHERE id = $1
            "#,
            id
        )
        .execute(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }
}
