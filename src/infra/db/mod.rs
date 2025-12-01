//! Postgres-backed repository implementations.

pub(crate) mod api_keys;
mod audit;
mod jobs;
mod navigation;
mod pages;
mod posts;
mod settings;
mod tags;
mod timezone;
mod uploads;
mod util;

pub use posts::{PersistedPostSection, PersistedPostSectionOwned};
pub use timezone::DbTimeZone;
pub use util::map_sqlx_error;

use std::sync::Arc;

use sqlx::{
    Postgres, QueryBuilder, Transaction,
    postgres::{PgPool, PgPoolOptions},
    query,
};

use crate::application::repos::{PostListScope, PostQueryFilter, RepoError};
use crate::domain::types::PostStatus;

const POSTS_PRIMARY_TIME_EXPR: &str = "CASE \
    WHEN p.status = 'published'::post_status THEN \
        COALESCE(p.published_at, p.updated_at, p.created_at) \
    ELSE \
        COALESCE(p.updated_at, p.created_at) \
END";

#[derive(Clone)]
pub struct PostgresRepositories {
    pool: Arc<PgPool>,
}

impl PostgresRepositories {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn begin(&self) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
        self.pool.begin().await
    }

    pub async fn connect(url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
        PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(url)
            .await
    }

    pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::migrate!("./migrations")
            .run(pool)
            .await
            .map_err(Into::into)
    }

    pub async fn health_check(&self) -> Result<(), sqlx::Error> {
        query("SELECT 1").execute(self.pool()).await.map(|_| ())
    }

    fn push_primary_time_expr<'q>(qb: &mut QueryBuilder<'q, Postgres>) {
        qb.push(POSTS_PRIMARY_TIME_EXPR);
    }

    fn apply_scope_conditions<'q>(qb: &mut QueryBuilder<'q, Postgres>, scope: PostListScope) {
        match scope {
            PostListScope::Public => {
                qb.push(" AND p.status = ");
                qb.push_bind(PostStatus::Published);
                qb.push(" AND p.published_at IS NOT NULL ");
            }
            PostListScope::Admin { status } => {
                if let Some(status) = status {
                    qb.push(" AND p.status = ");
                    qb.push_bind(status);
                }
            }
        }
    }

    fn apply_feed_filter<'q>(qb: &mut QueryBuilder<'q, Postgres>, filter: &'q PostQueryFilter) {
        if let Some(tag) = filter.tag.as_ref() {
            qb.push(
                " AND EXISTS (SELECT 1 FROM post_tags pt INNER JOIN tags t ON t.id = pt.tag_id WHERE pt.post_id = p.id AND t.slug = ",
            );
            qb.push_bind(tag);
            qb.push(")");
        }

        if let Some(month) = filter.month.as_ref() {
            qb.push(" AND to_char(");
            Self::push_primary_time_expr(qb);
            qb.push(", 'YYYY-MM') = ");
            qb.push_bind(month);
            qb.push(" ");
        }

        if let Some(search) = filter.search.as_ref() {
            qb.push(" AND (");
            qb.push("p.title ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(" OR p.slug ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(" OR p.excerpt ILIKE ");
            qb.push_bind(format!("%{}%", search));
            qb.push(")");
        }
    }

    fn convert_count(value: i64) -> Result<u64, RepoError> {
        value
            .try_into()
            .map_err(|_| RepoError::from_persistence("count exceeds supported range"))
    }
}
