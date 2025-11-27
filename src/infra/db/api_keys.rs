use sqlx::query;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::pagination::ApiKeyCursor;
use crate::application::repos::{
    ApiKeyListPage, ApiKeyPageRequest, ApiKeyQueryFilter, ApiKeyStatusFilter, ApiKeysRepo,
    CreateApiKeyParams, RepoError, UpdateApiKeySecretParams,
};
use crate::domain::api_keys::{ApiKeyRecord, ApiScope};

use super::PostgresRepositories;

#[derive(Debug, sqlx::FromRow)]
struct ApiKeyRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    prefix: String,
    hashed_secret: Vec<u8>,
    scopes: Vec<ApiScope>,
    expires_at: Option<OffsetDateTime>,
    revoked_at: Option<OffsetDateTime>,
    last_used_at: Option<OffsetDateTime>,
    created_by: String,
    created_at: OffsetDateTime,
}

impl TryFrom<ApiKeyRow> for ApiKeyRecord {
    type Error = RepoError;

    fn try_from(row: ApiKeyRow) -> Result<Self, Self::Error> {
        Ok(ApiKeyRecord {
            id: row.id,
            name: row.name,
            description: row.description,
            prefix: row.prefix,
            hashed_secret: row.hashed_secret,
            scopes: row.scopes,
            expires_at: row.expires_at,
            revoked_at: row.revoked_at,
            last_used_at: row.last_used_at,
            created_by: row.created_by,
            created_at: row.created_at,
        })
    }
}

#[async_trait::async_trait]
impl ApiKeysRepo for PostgresRepositories {
    async fn create_key(&self, params: CreateApiKeyParams) -> Result<ApiKeyRecord, RepoError> {
        let row = sqlx::query_as!(
            ApiKeyRow,
            r#"
            INSERT INTO api_keys (id, name, description, prefix, hashed_secret, scopes, expires_at, created_by, created_at)
            VALUES ($1, $2, $3, $4, $5, $6::api_scope[], $7, $8, $9)
            RETURNING id, name, description, prefix, hashed_secret, scopes as "scopes: Vec<ApiScope>", expires_at, revoked_at, last_used_at, created_by, created_at
            "#,
            Uuid::new_v4(),
            params.name,
            params.description,
            params.prefix,
            params.hashed_secret,
            params.scopes as Vec<ApiScope>,
            params.expires_at,
            params.created_by,
            OffsetDateTime::now_utc(),
        )
        .fetch_one(self.pool())
        .await
        .map_err(|err| match err {
            sqlx::Error::Database(db_err) => {
                if let Some(constraint) = db_err.constraint() {
                    RepoError::Duplicate {
                        constraint: constraint.to_owned(),
                    }
                } else {
                    RepoError::from_persistence(db_err)
                }
            }
            other => RepoError::from_persistence(other),
        })?;

        ApiKeyRecord::try_from(row)
    }

    async fn list_keys(
        &self,
        filter: &ApiKeyQueryFilter,
        page: ApiKeyPageRequest,
    ) -> Result<ApiKeyListPage, RepoError> {
        let (status_active, status_revoked) = match filter.status {
            Some(ApiKeyStatusFilter::Active) => (true, false),
            Some(ApiKeyStatusFilter::Revoked) => (false, true),
            None => (false, false),
        };

        let scope_filter = filter.scope.as_ref().map(|s| s.as_str().to_string());
        let search = filter
            .search
            .as_ref()
            .map(|s| format!("%{}%", s.to_lowercase()));

        let (cursor_created_at, cursor_id) = page
            .cursor
            .map(|c| (Some(c.created_at()), Some(c.id())))
            .unwrap_or((None, None));

        let limit = (page.limit as i64).clamp(1, 200);

        let rows = sqlx::query_as!(
            ApiKeyRow,
            r#"
            SELECT id, name, description, prefix, hashed_secret, scopes as "scopes: Vec<ApiScope>", expires_at, revoked_at, last_used_at, created_by, created_at
            FROM api_keys
            WHERE
                (($3::bool = FALSE AND $4::bool = FALSE) OR ($3 = TRUE AND revoked_at IS NULL) OR ($4 = TRUE AND revoked_at IS NOT NULL))
                AND ($1::text IS NULL OR LOWER(name) LIKE $1 OR LOWER(prefix) LIKE $1 OR LOWER(COALESCE(description, '')) LIKE $1)
                AND ($2::text IS NULL OR $2::api_scope = ANY(scopes))
                AND ($5::timestamptz IS NULL OR (created_at, id) < ($5, $6))
            ORDER BY created_at DESC, id DESC
            LIMIT $7
            "#,
            search,
            scope_filter,
            status_active,
            status_revoked,
            cursor_created_at,
            cursor_id,
            limit + 1,
        )
        .fetch_all(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        let mut records: Vec<ApiKeyRecord> = rows
            .into_iter()
            .map(ApiKeyRecord::try_from)
            .collect::<Result<_, _>>()?;

        let next_cursor = if records.len() as i64 > limit {
            let last = records.pop().expect("exists");
            Some(ApiKeyCursor::new(last.created_at, last.id))
        } else {
            None
        };

        let scope_filter_for_counts = scope_filter.clone();

        let total_row = sqlx::query!(
            r#"
            SELECT
              COUNT(*)::bigint AS "total!: i64",
              COUNT(*) FILTER (WHERE revoked_at IS NOT NULL)::bigint AS "revoked!: i64"
            FROM api_keys
            WHERE
                (($3::bool = FALSE AND $4::bool = FALSE) OR ($3 = TRUE AND revoked_at IS NULL) OR ($4 = TRUE AND revoked_at IS NOT NULL))
                AND ($1::text IS NULL OR LOWER(name) LIKE $1 OR LOWER(prefix) LIKE $1 OR LOWER(COALESCE(description, '')) LIKE $1)
                AND ($2::text IS NULL OR $2::api_scope = ANY(scopes))
            "#,
            search,
            scope_filter_for_counts,
            status_active,
            status_revoked,
        )
        .fetch_one(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        let scope_rows = sqlx::query!(
            r#"
            SELECT unnest(scopes) as "scope!: ApiScope", COUNT(*)::bigint as "count!: i64"
            FROM api_keys
            GROUP BY 1
            ORDER BY 1
            "#
        )
        .fetch_all(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        let scope_counts = scope_rows
            .into_iter()
            .map(|r| (r.scope, r.count as u64))
            .collect();

        Ok(ApiKeyListPage {
            items: records,
            total: total_row.total as u64,
            revoked: total_row.revoked as u64,
            next_cursor,
            scope_counts,
        })
    }

    async fn find_by_prefix(&self, prefix: &str) -> Result<Option<ApiKeyRecord>, RepoError> {
        let row = sqlx::query_as!(
            ApiKeyRow,
            r#"
            SELECT id, name, description, prefix, hashed_secret, scopes as "scopes: Vec<ApiScope>", expires_at, revoked_at, last_used_at, created_by, created_at
            FROM api_keys
            WHERE prefix = $1
            "#,
            prefix
        )
        .fetch_optional(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        row.map(ApiKeyRecord::try_from).transpose()
    }

    async fn revoke_key(&self, id: Uuid, revoked_at: OffsetDateTime) -> Result<(), RepoError> {
        query!(
            r#"
            UPDATE api_keys
            SET revoked_at = $1
            WHERE id = $2
            "#,
            revoked_at,
            id
        )
        .execute(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(())
    }

    async fn update_secret(
        &self,
        params: UpdateApiKeySecretParams,
    ) -> Result<ApiKeyRecord, RepoError> {
        let row = sqlx::query_as!(
            ApiKeyRow,
            r#"
            UPDATE api_keys
            SET prefix = $1,
                hashed_secret = $2,
                revoked_at = NULL,
                expires_at = NULL,
                last_used_at = NULL,
                created_at = now()
            WHERE id = $3
            RETURNING id, name, description, prefix, hashed_secret, scopes as "scopes: Vec<ApiScope>", expires_at, revoked_at, last_used_at, created_by, created_at
            "#,
            params.new_prefix,
            params.new_hashed_secret,
            params.id
        )
        .fetch_one(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        ApiKeyRecord::try_from(row)
    }

    async fn update_last_used(
        &self,
        id: Uuid,
        last_used_at: OffsetDateTime,
    ) -> Result<(), RepoError> {
        query!(
            r#"
            UPDATE api_keys
            SET last_used_at = $1
            WHERE id = $2
            "#,
            last_used_at,
            id
        )
        .execute(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(())
    }
}
