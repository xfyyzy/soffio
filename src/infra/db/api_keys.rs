use sqlx::query;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::repos::{
    ApiKeysRepo, CreateApiKeyParams, RepoError, UpdateApiKeySecretParams,
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

    async fn list_keys(&self) -> Result<Vec<ApiKeyRecord>, RepoError> {
        let rows = sqlx::query_as!(
            ApiKeyRow,
            r#"
            SELECT id, name, description, prefix, hashed_secret, scopes as "scopes: Vec<ApiScope>", expires_at, revoked_at, last_used_at, created_by, created_at
            FROM api_keys
            ORDER BY created_at DESC, id DESC
            "#,
        )
        .fetch_all(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        rows.into_iter().map(ApiKeyRecord::try_from).collect()
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
