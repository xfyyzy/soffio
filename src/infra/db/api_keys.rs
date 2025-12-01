use sqlx::postgres::types::PgInterval;
use sqlx::query;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::pagination::ApiKeyCursor;
use crate::application::repos::{
    ApiKeyListPage, ApiKeyPageRequest, ApiKeyQueryFilter, ApiKeyStatusFilter, ApiKeysRepo,
    CreateApiKeyParams, RepoError, UpdateApiKeyMetadataParams, UpdateApiKeySecretParams,
};
use crate::domain::api_keys::{ApiKeyRecord, ApiKeyStatus, ApiScope};

use super::{PostgresRepositories, map_sqlx_error};

#[derive(Debug, sqlx::FromRow)]
struct ApiKeyRow {
    id: Uuid,
    name: String,
    description: Option<String>,
    prefix: String,
    hashed_secret: Vec<u8>,
    scopes: Vec<ApiScope>,
    status: ApiKeyStatus,
    expires_in: Option<PgInterval>,
    expires_at: Option<OffsetDateTime>,
    revoked_at: Option<OffsetDateTime>,
    last_used_at: Option<OffsetDateTime>,
    created_by: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

pub(crate) fn pg_interval_to_duration(interval: PgInterval) -> time::Duration {
    // PgInterval stores months, days, and microseconds
    // For simplicity, we convert assuming 30 days per month
    let total_days = (interval.months as i64) * 30 + (interval.days as i64);
    let total_seconds = total_days * 86400 + (interval.microseconds / 1_000_000);
    time::Duration::seconds(total_seconds)
}

pub(crate) fn duration_to_pg_interval(duration: time::Duration) -> PgInterval {
    let total_seconds = duration.whole_seconds();
    let days = (total_seconds / 86400) as i32;
    let remaining_microseconds = (total_seconds % 86400) * 1_000_000;
    PgInterval {
        months: 0,
        days,
        microseconds: remaining_microseconds,
    }
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
            status: row.status,
            expires_in: row.expires_in.map(pg_interval_to_duration),
            expires_at: row.expires_at,
            revoked_at: row.revoked_at,
            last_used_at: row.last_used_at,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[async_trait::async_trait]
impl ApiKeysRepo for PostgresRepositories {
    async fn create_key(&self, params: CreateApiKeyParams) -> Result<ApiKeyRecord, RepoError> {
        let now = OffsetDateTime::now_utc();
        let expires_in_pg = params.expires_in.map(duration_to_pg_interval);
        let row = sqlx::query_as!(
            ApiKeyRow,
            r#"
            INSERT INTO api_keys (id, name, description, prefix, hashed_secret, scopes, status, expires_in, expires_at, created_by, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6::api_scope[], 'active', $7, $8, $9, $10, $10)
            RETURNING id, name, description, prefix, hashed_secret,
                      scopes as "scopes: Vec<ApiScope>",
                      status as "status: ApiKeyStatus",
                      expires_in, expires_at, revoked_at, last_used_at, created_by, created_at, updated_at
            "#,
            Uuid::new_v4(),
            params.name,
            params.description,
            params.prefix,
            params.hashed_secret,
            params.scopes as Vec<ApiScope>,
            expires_in_pg,
            params.expires_at,
            params.created_by,
            now,
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
        let status_filter = filter.status.map(|s| match s {
            ApiKeyStatusFilter::Active => "active",
            ApiKeyStatusFilter::Revoked => "revoked",
            ApiKeyStatusFilter::Expired => "expired",
        });

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

        // Use effective_status to handle keys that expired between cron runs
        let rows = sqlx::query_as!(
            ApiKeyRow,
            r#"
            SELECT id, name, description, prefix, hashed_secret,
                   scopes as "scopes: Vec<ApiScope>",
                   CASE
                       WHEN status = 'active' AND expires_at IS NOT NULL AND expires_at < now()
                       THEN 'expired'::api_key_status
                       ELSE status
                   END as "status!: ApiKeyStatus",
                   expires_in, expires_at, revoked_at, last_used_at, created_by, created_at, updated_at
            FROM api_keys
            WHERE
                ($1::text IS NULL OR
                    CASE
                        WHEN status = 'active' AND expires_at IS NOT NULL AND expires_at < now()
                        THEN 'expired'
                        ELSE status::text
                    END = $1)
                AND ($2::text IS NULL OR LOWER(name) LIKE $2 OR LOWER(prefix) LIKE $2 OR LOWER(COALESCE(description, '')) LIKE $2)
                AND ($3::text IS NULL OR $3::api_scope = ANY(scopes))
                AND ($4::timestamptz IS NULL OR (created_at, id) < ($4, $5))
            ORDER BY created_at DESC, id DESC
            LIMIT $6
            "#,
            status_filter,
            search,
            scope_filter,
            cursor_created_at,
            cursor_id,
            limit + 1,
        )
        .fetch_all(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        let mut records: Vec<ApiKeyRecord> = rows
            .into_iter()
            .map(ApiKeyRecord::try_from)
            .collect::<Result<_, _>>()?;

        let has_more = (records.len() as i64) > limit;
        if has_more {
            // drop the extra item used to detect presence of a next page
            records.pop();
        }

        let next_cursor = if has_more {
            records
                .last()
                .map(|last| ApiKeyCursor::new(last.created_at, last.id))
        } else {
            None
        };

        // Get counts by status (with effective status calculation)
        // Note: counts are calculated WITHOUT status filter, only with search/scope filters
        // This matches posts page behavior where tab counts show totals across all statuses
        let count_row = sqlx::query!(
            r#"
            SELECT
              COUNT(*)::bigint AS "total!: i64",
              COUNT(*) FILTER (WHERE
                  CASE
                      WHEN status = 'active' AND expires_at IS NOT NULL AND expires_at < now()
                      THEN 'expired'
                      ELSE status::text
                  END = 'active'
              )::bigint AS "active!: i64",
              COUNT(*) FILTER (WHERE status = 'revoked')::bigint AS "revoked!: i64",
              COUNT(*) FILTER (WHERE
                  status = 'expired' OR (status = 'active' AND expires_at IS NOT NULL AND expires_at < now())
              )::bigint AS "expired!: i64"
            FROM api_keys
            WHERE
                ($1::text IS NULL OR LOWER(name) LIKE $1 OR LOWER(prefix) LIKE $1 OR LOWER(COALESCE(description, '')) LIKE $1)
                AND ($2::text IS NULL OR $2::api_scope = ANY(scopes))
            "#,
            search,
            scope_filter,
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

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
        .map_err(map_sqlx_error)?;

        let scope_counts = scope_rows
            .into_iter()
            .map(|r| (r.scope, r.count as u64))
            .collect();

        Ok(ApiKeyListPage {
            items: records,
            total: count_row.total as u64,
            active: count_row.active as u64,
            revoked: count_row.revoked as u64,
            expired: count_row.expired as u64,
            next_cursor,
            scope_counts,
        })
    }

    async fn find_by_prefix(&self, prefix: &str) -> Result<Option<ApiKeyRecord>, RepoError> {
        let row = sqlx::query_as!(
            ApiKeyRow,
            r#"
            SELECT id, name, description, prefix, hashed_secret,
                   scopes as "scopes: Vec<ApiScope>",
                   status as "status: ApiKeyStatus",
                   expires_in, expires_at, revoked_at, last_used_at, created_by, created_at, updated_at
            FROM api_keys
            WHERE prefix = $1
            "#,
            prefix
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        row.map(ApiKeyRecord::try_from).transpose()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ApiKeyRecord>, RepoError> {
        let row = sqlx::query_as!(
            ApiKeyRow,
            r#"
            SELECT id, name, description, prefix, hashed_secret,
                   scopes as "scopes: Vec<ApiScope>",
                   status as "status: ApiKeyStatus",
                   expires_in, expires_at, revoked_at, last_used_at, created_by, created_at, updated_at
            FROM api_keys
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        row.map(ApiKeyRecord::try_from).transpose()
    }

    async fn revoke_key(&self, id: Uuid, revoked_at: OffsetDateTime) -> Result<(), RepoError> {
        query!(
            r#"
            UPDATE api_keys
            SET status = 'revoked',
                revoked_at = $1,
                updated_at = now()
            WHERE id = $2
            "#,
            revoked_at,
            id
        )
        .execute(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(())
    }

    async fn delete_key(&self, id: Uuid) -> Result<bool, RepoError> {
        let result = query!(
            r#"
            DELETE FROM api_keys
            WHERE id = $1
            "#,
            id
        )
        .execute(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(result.rows_affected() > 0)
    }

    async fn expire_keys(&self) -> Result<u64, RepoError> {
        let result = query!(
            r#"
            UPDATE api_keys
            SET status = 'expired',
                updated_at = now()
            WHERE status = 'active'
              AND expires_at IS NOT NULL
              AND expires_at < now()
            "#
        )
        .execute(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        Ok(result.rows_affected())
    }

    async fn update_secret(
        &self,
        params: UpdateApiKeySecretParams,
    ) -> Result<ApiKeyRecord, RepoError> {
        // Rotate key: update secret, reactivate if revoked/expired, recalculate expires_at
        let row = sqlx::query_as!(
            ApiKeyRow,
            r#"
            UPDATE api_keys
            SET prefix = $1,
                hashed_secret = $2,
                status = 'active',
                expires_at = CASE WHEN expires_in IS NOT NULL THEN now() + expires_in ELSE NULL END,
                revoked_at = NULL,
                last_used_at = NULL,
                updated_at = now()
            WHERE id = $3
            RETURNING id, name, description, prefix, hashed_secret,
                      scopes as "scopes: Vec<ApiScope>",
                      status as "status: ApiKeyStatus",
                      expires_in, expires_at, revoked_at, last_used_at, created_by, created_at, updated_at
            "#,
            params.new_prefix,
            params.new_hashed_secret,
            params.id
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

        ApiKeyRecord::try_from(row)
    }

    async fn update_metadata(
        &self,
        params: UpdateApiKeyMetadataParams,
    ) -> Result<ApiKeyRecord, RepoError> {
        let row = sqlx::query_as!(
            ApiKeyRow,
            r#"
            UPDATE api_keys
            SET name = $1,
                description = $2,
                scopes = $3::api_scope[],
                updated_at = now()
            WHERE id = $4
            RETURNING id, name, description, prefix, hashed_secret,
                      scopes as "scopes: Vec<ApiScope>",
                      status as "status: ApiKeyStatus",
                      expires_in, expires_at, revoked_at, last_used_at, created_by, created_at, updated_at
            "#,
            params.name,
            params.description,
            params.scopes as Vec<ApiScope>,
            params.id
        )
        .fetch_one(self.pool())
        .await
        .map_err(map_sqlx_error)?;

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
        .map_err(map_sqlx_error)?;

        Ok(())
    }
}
