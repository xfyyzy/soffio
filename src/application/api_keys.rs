use std::sync::Arc;

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::repos::{
    ApiKeyPageRequest, ApiKeyQueryFilter, ApiKeysRepo, CreateApiKeyParams, RepoError,
    UpdateApiKeyMetadataParams, UpdateApiKeySecretParams,
};
use crate::domain::api_keys::{ApiKeyRecord, ApiKeyStatus, ApiScope};

const TOKEN_PREFIX: &str = "sk";
const MIN_SECRET_LEN: usize = 32;

#[derive(Debug, Error)]
pub enum ApiKeyError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error("invalid scope set")]
    InvalidScopes,
    #[error("key not found")]
    NotFound,
}

#[derive(Debug, Error)]
pub enum ApiAuthError {
    #[error("missing api key")]
    Missing,
    #[error("invalid api key")]
    Invalid,
    #[error("expired api key")]
    Expired,
    #[error("revoked api key")]
    Revoked,
}

#[derive(Debug, Clone)]
pub struct IssueApiKeyCommand {
    pub name: String,
    pub description: Option<String>,
    pub scopes: Vec<ApiScope>,
    pub expires_in: Option<time::Duration>,
    pub created_by: String,
}

#[derive(Debug, Clone)]
pub struct UpdateApiKeyCommand {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub scopes: Vec<ApiScope>,
}

#[derive(Debug, Clone)]
pub struct ApiKeyIssued {
    pub record: ApiKeyRecord,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct ApiPrincipal {
    pub key_id: Uuid,
    pub name: String,
    pub prefix: String,
    pub scopes: Vec<ApiScope>,
}

impl ApiPrincipal {
    pub fn requires(&self, needed: ApiScope) -> Result<(), ApiAuthError> {
        if self.scopes.contains(&needed) {
            Ok(())
        } else {
            Err(ApiAuthError::Invalid)
        }
    }
}

#[derive(Clone)]
pub struct ApiKeyService {
    repo: Arc<dyn ApiKeysRepo>,
}

impl ApiKeyService {
    pub fn new(repo: Arc<dyn ApiKeysRepo>) -> Self {
        Self { repo }
    }

    pub async fn issue(&self, cmd: IssueApiKeyCommand) -> Result<ApiKeyIssued, ApiKeyError> {
        if cmd.scopes.is_empty() {
            return Err(ApiKeyError::InvalidScopes);
        }

        let description = Self::normalize_description(cmd.description);

        let prefix = Self::generate_prefix();
        let secret = Self::generate_secret();
        let token = format!("{TOKEN_PREFIX}_{prefix}_{secret}");
        let hashed_secret = Self::hash_secret(&secret);

        // Compute expires_at from expires_in if set
        let now = OffsetDateTime::now_utc();
        let expires_at = cmd.expires_in.map(|d| now + d);

        let record = self
            .repo
            .create_key(CreateApiKeyParams {
                name: cmd.name,
                description,
                prefix: prefix.clone(),
                hashed_secret,
                scopes: cmd.scopes,
                expires_in: cmd.expires_in,
                expires_at,
                created_by: cmd.created_by,
            })
            .await?;

        Ok(ApiKeyIssued { record, token })
    }

    pub async fn rotate(&self, id: Uuid) -> Result<ApiKeyIssued, ApiKeyError> {
        // Verify key exists
        let _current = self
            .repo
            .find_by_id(id)
            .await?
            .ok_or(ApiKeyError::NotFound)?;

        // Generate new credentials - repo layer handles reactivation and expires_at recalculation
        let prefix = Self::generate_prefix();
        let secret = Self::generate_secret();
        let token = format!("{TOKEN_PREFIX}_{prefix}_{secret}");
        let hashed_secret = Self::hash_secret(&secret);

        let record = self
            .repo
            .update_secret(UpdateApiKeySecretParams {
                id,
                new_prefix: prefix.clone(),
                new_hashed_secret: hashed_secret,
            })
            .await?;

        Ok(ApiKeyIssued { record, token })
    }

    pub async fn update(&self, cmd: UpdateApiKeyCommand) -> Result<ApiKeyRecord, ApiKeyError> {
        if cmd.scopes.is_empty() {
            return Err(ApiKeyError::InvalidScopes);
        }

        let description = Self::normalize_description(cmd.description);

        let record = self
            .repo
            .update_metadata(UpdateApiKeyMetadataParams {
                id: cmd.id,
                name: cmd.name,
                description,
                scopes: cmd.scopes,
            })
            .await?;

        Ok(record)
    }

    pub async fn load(&self, id: Uuid) -> Result<Option<ApiKeyRecord>, ApiKeyError> {
        let record = self.repo.find_by_id(id).await?;
        Ok(record)
    }

    pub async fn revoke(&self, id: Uuid) -> Result<(), ApiKeyError> {
        let now = OffsetDateTime::now_utc();
        self.repo.revoke_key(id, now).await?;
        Ok(())
    }

    pub async fn delete(&self, id: Uuid) -> Result<bool, ApiKeyError> {
        let deleted = self.repo.delete_key(id).await?;
        Ok(deleted)
    }

    pub async fn expire_keys(&self) -> Result<u64, ApiKeyError> {
        let count = self.repo.expire_keys().await?;
        Ok(count)
    }

    pub async fn list(&self) -> Result<Vec<ApiKeyRecord>, ApiKeyError> {
        let page = self
            .repo
            .list_keys(
                &ApiKeyQueryFilter::default(),
                ApiKeyPageRequest {
                    limit: 100,
                    cursor: None,
                },
            )
            .await
            .map_err(ApiKeyError::from)?;
        Ok(page.items)
    }

    pub async fn list_page(
        &self,
        filter: &ApiKeyQueryFilter,
        page: ApiKeyPageRequest,
    ) -> Result<crate::application::repos::ApiKeyListPage, ApiKeyError> {
        self.repo
            .list_keys(filter, page)
            .await
            .map_err(ApiKeyError::from)
    }

    pub async fn authenticate(&self, token: &str) -> Result<ApiPrincipal, ApiAuthError> {
        let parsed = Self::parse_token(token).ok_or(ApiAuthError::Invalid)?;
        let record = self
            .repo
            .find_by_prefix(&parsed.prefix)
            .await
            .map_err(|_| ApiAuthError::Invalid)?
            .ok_or(ApiAuthError::Invalid)?;

        // Check status field first
        match record.status {
            ApiKeyStatus::Revoked => return Err(ApiAuthError::Revoked),
            ApiKeyStatus::Expired => return Err(ApiAuthError::Expired),
            ApiKeyStatus::Active => { /* continue */ }
        }

        // Fallback: check for keys that expired between cron runs
        let now = OffsetDateTime::now_utc();
        if let Some(expires_at) = record.expires_at
            && expires_at <= now
        {
            return Err(ApiAuthError::Expired);
        }

        // Verify secret
        let hashed_input = Self::hash_secret(&parsed.secret);
        if record.hashed_secret.ct_eq(&hashed_input).unwrap_u8() == 0 {
            return Err(ApiAuthError::Invalid);
        }

        // best-effort last_used update; do not block auth
        let repo = self.repo.clone();
        tokio::spawn(async move {
            let _ = repo.update_last_used(record.id, now).await;
        });

        Ok(ApiPrincipal {
            key_id: record.id,
            name: record.name,
            prefix: record.prefix,
            scopes: record.scopes,
        })
    }

    /// Normalize optional descriptions coming from external inputs.
    /// Treat empty or whitespace-only strings as absent to align with UI placeholder logic.
    fn normalize_description(desc: Option<String>) -> Option<String> {
        desc.and_then(|d| if d.trim().is_empty() { None } else { Some(d) })
    }

    fn hash_secret(secret: &str) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hasher.finalize().to_vec()
    }

    fn generate_prefix() -> String {
        Uuid::new_v4().simple().to_string()[..12].to_string()
    }

    fn generate_secret() -> String {
        let raw = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        raw
    }

    fn parse_token(token: &str) -> Option<ParsedToken> {
        let mut parts = token.splitn(3, '_');
        let prefix_tag = parts.next()?;
        if prefix_tag != TOKEN_PREFIX {
            return None;
        }
        let prefix = parts.next()?;
        let secret = parts.next()?;
        if secret.len() < MIN_SECRET_LEN || prefix.is_empty() {
            return None;
        }
        Some(ParsedToken {
            prefix: prefix.to_string(),
            secret: secret.to_string(),
        })
    }
}

struct ParsedToken {
    prefix: String,
    secret: String,
}

#[cfg(test)]
mod tests {
    use super::ApiKeyService;

    #[test]
    fn normalize_description_drops_empty_and_whitespace() {
        assert_eq!(ApiKeyService::normalize_description(None), None);
        assert_eq!(
            ApiKeyService::normalize_description(Some(String::new())),
            None
        );
        assert_eq!(
            ApiKeyService::normalize_description(Some("   ".into())),
            None
        );
    }

    #[test]
    fn normalize_description_preserves_non_empty() {
        assert_eq!(
            ApiKeyService::normalize_description(Some("desc".into())),
            Some("desc".into())
        );
        // Leading/trailing whitespace is preserved; only empty strings are stripped.
        assert_eq!(
            ApiKeyService::normalize_description(Some(" spaced ".into())),
            Some(" spaced ".into())
        );
    }
}
