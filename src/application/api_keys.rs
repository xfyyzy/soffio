use std::sync::Arc;

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::application::repos::{
    ApiKeysRepo, CreateApiKeyParams, RepoError, UpdateApiKeySecretParams,
};
use crate::domain::api_keys::{ApiKeyRecord, ApiScope};

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
    pub expires_at: Option<OffsetDateTime>,
    pub created_by: String,
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

        let prefix = Self::generate_prefix();
        let secret = Self::generate_secret();
        let token = format!("{TOKEN_PREFIX}_{prefix}_{secret}");
        let hashed_secret = Self::hash_secret(&secret);

        let record = self
            .repo
            .create_key(CreateApiKeyParams {
                name: cmd.name,
                description: cmd.description,
                prefix: prefix.clone(),
                hashed_secret,
                scopes: cmd.scopes,
                expires_at: cmd.expires_at,
                created_by: cmd.created_by,
            })
            .await?;

        Ok(ApiKeyIssued { record, token })
    }

    pub async fn rotate(&self, id: Uuid) -> Result<ApiKeyIssued, ApiKeyError> {
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

    pub async fn revoke(&self, id: Uuid) -> Result<(), ApiKeyError> {
        let now = OffsetDateTime::now_utc();
        self.repo.revoke_key(id, now).await?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<ApiKeyRecord>, ApiKeyError> {
        self.repo.list_keys().await.map_err(ApiKeyError::from)
    }

    pub async fn authenticate(&self, token: &str) -> Result<ApiPrincipal, ApiAuthError> {
        let parsed = Self::parse_token(token).ok_or(ApiAuthError::Invalid)?;
        let record = self
            .repo
            .find_by_prefix(&parsed.prefix)
            .await
            .map_err(|_| ApiAuthError::Invalid)?
            .ok_or(ApiAuthError::Invalid)?;

        let now = OffsetDateTime::now_utc();
        if let Some(revoked_at) = record.revoked_at
            && revoked_at <= now
        {
            return Err(ApiAuthError::Revoked);
        }
        if let Some(expires_at) = record.expires_at
            && expires_at <= now
        {
            return Err(ApiAuthError::Expired);
        }

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
