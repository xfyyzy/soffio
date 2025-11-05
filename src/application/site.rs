//! Import/export of site content and configuration.

use std::{collections::HashMap, fs, path::Path};

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction, query};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    application::error::AppError,
    domain::types::{NavigationDestinationType, PageStatus, PostStatus},
    infra::{db::PostgresRepositories, error::InfraError},
};

const SETTINGS_ROW_ID: i16 = 1;

/// Export the current site data to the provided path as a TOML archive.
pub async fn export_site(repositories: &PostgresRepositories, path: &Path) -> Result<(), AppError> {
    let archive = gather_archive(repositories.pool()).await?;
    let encoded = toml::to_string_pretty(&archive)
        .map_err(|err| AppError::unexpected(format!("failed to encode archive: {err}")))?;
    fs::write(path, encoded).map_err(|err| AppError::from(InfraError::Io(err)))?;
    Ok(())
}

/// Import site data from the provided TOML archive path.
pub async fn import_site(repositories: &PostgresRepositories, path: &Path) -> Result<(), AppError> {
    let data = fs::read_to_string(path).map_err(|err| AppError::from(InfraError::Io(err)))?;
    let mut archive: SiteArchive = toml::from_str(&data)
        .map_err(|err| AppError::validation(format!("invalid archive: {err}")))?;
    archive.normalize();

    let mut tx = repositories.begin().await.map_err(map_sqlx_error)?;

    let db_migrations = fetch_migrations_from(&mut tx).await?;
    ensure_migrations_match(&db_migrations.entries, &archive.migrations.entries)?;

    query("SET CONSTRAINTS ALL DEFERRED")
        .execute(tx.as_mut())
        .await
        .map_err(map_sqlx_error)?;

    // Clear existing content in dependency-safe order.
    query("TRUNCATE post_tags, navigation_items, tags, pages, posts RESTART IDENTITY CASCADE")
        .execute(tx.as_mut())
        .await
        .map_err(map_sqlx_error)?;

    let mut tag_ids = HashMap::new();
    for tag in &archive.tags {
        let id = Uuid::new_v4();
        query!(
            r#"
            INSERT INTO tags (id, slug, name, description, pinned)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            id,
            tag.slug,
            tag.name,
            tag.description,
            tag.pinned,
        )
        .execute(tx.as_mut())
        .await
        .map_err(map_sqlx_error)?;
        tag_ids.insert(tag.slug.clone(), id);
    }

    let mut post_ids = HashMap::new();
    for post in &archive.posts {
        let id = Uuid::new_v4();
        query!(
            r#"
            INSERT INTO posts (
                id,
                slug,
                title,
                excerpt,
                body_markdown,
                summary_markdown,
                status,
                pinned,
                scheduled_at,
                published_at,
                archived_at
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
            id,
            post.slug,
            post.title,
            post.excerpt,
            post.body_markdown,
            post.summary_markdown,
            post.status as PostStatus,
            post.pinned,
            post.scheduled_at,
            post.published_at,
            post.archived_at,
        )
        .execute(tx.as_mut())
        .await
        .map_err(map_sqlx_error)?;
        post_ids.insert(post.slug.clone(), id);
    }

    let mut page_ids = HashMap::new();
    for page in &archive.pages {
        let id = Uuid::new_v4();
        query!(
            r#"
            INSERT INTO pages (
                id,
                slug,
                title,
                body_markdown,
                rendered_html,
                status,
                scheduled_at,
                published_at,
                archived_at
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
            "#,
            id,
            page.slug,
            page.title,
            page.body_markdown,
            "", // rendered_html intentionally blank; renderall will repopulate
            page.status as PageStatus,
            page.scheduled_at,
            page.published_at,
            page.archived_at,
        )
        .execute(tx.as_mut())
        .await
        .map_err(map_sqlx_error)?;
        page_ids.insert(page.slug.clone(), id);
    }

    for link in &archive.post_tags {
        let post_id = post_ids.get(&link.post_slug).ok_or_else(|| {
            AppError::validation(format!("unknown post slug `{}`", link.post_slug))
        })?;
        let tag_id = tag_ids
            .get(&link.tag_slug)
            .ok_or_else(|| AppError::validation(format!("unknown tag slug `{}`", link.tag_slug)))?;

        query!(
            r#"
            INSERT INTO post_tags (post_id, tag_id)
            VALUES ($1, $2)
            "#,
            post_id,
            tag_id,
        )
        .execute(tx.as_mut())
        .await
        .map_err(map_sqlx_error)?;
    }

    for item in &archive.navigation_items {
        let destination_page_id = match item.destination_type {
            NavigationDestinationType::Internal => {
                let slug = item.destination_page_slug.as_ref().ok_or_else(|| {
                    AppError::validation(format!(
                        "navigation item `{}` missing destination_page_slug",
                        item.label
                    ))
                })?;
                Some(*page_ids.get(slug).ok_or_else(|| {
                    AppError::validation(format!(
                        "navigation item `{}` references unknown page slug `{slug}`",
                        item.label
                    ))
                })?)
            }
            NavigationDestinationType::External => None,
        };

        query!(
            r#"
            INSERT INTO navigation_items (
                id,
                label,
                destination_type,
                destination_url,
                destination_page_id,
                sort_order,
                open_in_new_tab,
                visible
            )
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
            Uuid::new_v4(),
            item.label,
            item.destination_type as NavigationDestinationType,
            item.destination_url,
            destination_page_id,
            item.sort_order,
            item.open_in_new_tab,
            item.visible,
        )
        .execute(tx.as_mut())
        .await
        .map_err(map_sqlx_error)?;
    }

    let settings = &archive.site_settings;
    query!(
        r#"
        UPDATE site_settings
        SET
            homepage_size = $1,
            admin_page_size = $2,
            show_tag_aggregations = $3,
            show_month_aggregations = $4,
            tag_filter_limit = $5,
            month_filter_limit = $6,
            global_toc_enabled = $7,
            brand_title = $8,
            brand_href = $9,
            footer_copy = $10,
            public_site_url = $11,
            favicon_svg = $12,
            timezone = $13,
            meta_title = $14,
            meta_description = $15,
            og_title = $16,
            og_description = $17,
            updated_at = now()
        WHERE id = $18
        "#,
        settings.homepage_size,
        settings.admin_page_size,
        settings.show_tag_aggregations,
        settings.show_month_aggregations,
        settings.tag_filter_limit,
        settings.month_filter_limit,
        settings.global_toc_enabled,
        settings.brand_title,
        settings.brand_href,
        settings.footer_copy,
        settings.public_site_url,
        settings.favicon_svg,
        settings.timezone,
        settings.meta_title,
        settings.meta_description,
        settings.og_title,
        settings.og_description,
        SETTINGS_ROW_ID,
    )
    .execute(tx.as_mut())
    .await
    .map_err(map_sqlx_error)?;

    tx.commit().await.map_err(map_sqlx_error)?;
    Ok(())
}

fn ensure_migrations_match(
    db: &[MigrationEntry],
    archive: &[MigrationEntry],
) -> Result<(), AppError> {
    if db.len() != archive.len() {
        return Err(AppError::validation(
            "database migration history does not match archive",
        ));
    }

    for (db_entry, archive_entry) in db.iter().zip(archive.iter()) {
        if db_entry != archive_entry {
            return Err(AppError::validation(format!(
                "migration mismatch: database has version {} (checksum {}), archive has checksum {}",
                db_entry.version, db_entry.checksum, archive_entry.checksum
            )));
        }
    }

    Ok(())
}

fn map_sqlx_error(err: sqlx::Error) -> AppError {
    AppError::from(InfraError::database(err.to_string()))
}

async fn gather_archive(pool: &PgPool) -> Result<SiteArchive, AppError> {
    let migrations = fetch_migrations(pool).await?;
    let site_settings = fetch_site_settings(pool).await?;
    let posts = fetch_posts(pool).await?;
    let pages = fetch_pages(pool).await?;
    let tags = fetch_tags(pool).await?;
    let post_tags = fetch_post_tags(pool).await?;
    let navigation_items = fetch_navigation(pool).await?;

    Ok(SiteArchive {
        migrations,
        site_settings,
        posts,
        pages,
        tags,
        post_tags,
        navigation_items,
    })
}

async fn fetch_migrations(pool: &PgPool) -> Result<MigrationSnapshot, AppError> {
    let rows = query!(
        r#"SELECT version, encode(checksum, 'hex') AS "checksum!" FROM _sqlx_migrations ORDER BY version"#
    )
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_error)?;

    let entries = rows
        .into_iter()
        .map(|row| MigrationEntry {
            version: row.version,
            checksum: row.checksum,
        })
        .collect();

    Ok(MigrationSnapshot { entries })
}

async fn fetch_migrations_from(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<MigrationSnapshot, AppError> {
    let rows = query!(
        r#"SELECT version, encode(checksum, 'hex') AS "checksum!" FROM _sqlx_migrations ORDER BY version"#
    )
    .fetch_all(tx.as_mut())
    .await
    .map_err(map_sqlx_error)?;

    let entries = rows
        .into_iter()
        .map(|row| MigrationEntry {
            version: row.version,
            checksum: row.checksum,
        })
        .collect();

    Ok(MigrationSnapshot { entries })
}

async fn fetch_site_settings(pool: &PgPool) -> Result<SiteSettingsSnapshot, AppError> {
    let row = query!(
        r#"
        SELECT
            homepage_size,
            admin_page_size,
            show_tag_aggregations,
            show_month_aggregations,
            tag_filter_limit,
            month_filter_limit,
            global_toc_enabled,
            brand_title,
            brand_href,
            footer_copy,
            public_site_url,
            favicon_svg,
            timezone,
            meta_title,
            meta_description,
            og_title,
            og_description
        FROM site_settings
        WHERE id = $1
        "#,
        SETTINGS_ROW_ID,
    )
    .fetch_one(pool)
    .await
    .map_err(map_sqlx_error)?;

    Ok(SiteSettingsSnapshot {
        homepage_size: row.homepage_size,
        admin_page_size: row.admin_page_size,
        show_tag_aggregations: row.show_tag_aggregations,
        show_month_aggregations: row.show_month_aggregations,
        tag_filter_limit: row.tag_filter_limit,
        month_filter_limit: row.month_filter_limit,
        global_toc_enabled: row.global_toc_enabled,
        brand_title: row.brand_title,
        brand_href: row.brand_href,
        footer_copy: row.footer_copy,
        public_site_url: row.public_site_url,
        favicon_svg: row.favicon_svg,
        timezone: row.timezone,
        meta_title: row.meta_title,
        meta_description: row.meta_description,
        og_title: row.og_title,
        og_description: row.og_description,
    })
}

async fn fetch_posts(pool: &PgPool) -> Result<Vec<PostSnapshot>, AppError> {
    let rows = query!(
        r#"
        SELECT
            slug,
            title,
            excerpt,
            body_markdown,
            summary_markdown,
            status AS "status: PostStatus",
            pinned,
            scheduled_at,
            published_at,
            archived_at
        FROM posts
        ORDER BY slug
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_error)?;

    Ok(rows
        .into_iter()
        .map(|row| PostSnapshot {
            slug: row.slug,
            title: row.title,
            excerpt: row.excerpt,
            body_markdown: row.body_markdown,
            summary_markdown: row.summary_markdown,
            status: row.status,
            pinned: row.pinned,
            scheduled_at: row.scheduled_at,
            published_at: row.published_at,
            archived_at: row.archived_at,
        })
        .collect())
}

async fn fetch_pages(pool: &PgPool) -> Result<Vec<PageSnapshot>, AppError> {
    let rows = query!(
        r#"
        SELECT
            slug,
            title,
            body_markdown,
            status AS "status: PageStatus",
            scheduled_at,
            published_at,
            archived_at
        FROM pages
        ORDER BY slug
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_error)?;

    Ok(rows
        .into_iter()
        .map(|row| PageSnapshot {
            slug: row.slug,
            title: row.title,
            body_markdown: row.body_markdown,
            status: row.status,
            scheduled_at: row.scheduled_at,
            published_at: row.published_at,
            archived_at: row.archived_at,
        })
        .collect())
}

async fn fetch_tags(pool: &PgPool) -> Result<Vec<TagSnapshot>, AppError> {
    let rows = query!(
        r#"
        SELECT slug, name, description, pinned
        FROM tags
        ORDER BY slug
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_error)?;

    Ok(rows
        .into_iter()
        .map(|row| TagSnapshot {
            slug: row.slug,
            name: row.name,
            description: row.description,
            pinned: row.pinned,
        })
        .collect())
}

async fn fetch_post_tags(pool: &PgPool) -> Result<Vec<PostTagLink>, AppError> {
    let rows = query!(
        r#"
        SELECT
            p.slug AS "post_slug!",
            t.slug AS "tag_slug!"
        FROM post_tags pt
        INNER JOIN posts p ON p.id = pt.post_id
        INNER JOIN tags t ON t.id = pt.tag_id
        ORDER BY p.slug, t.slug
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_error)?;

    Ok(rows
        .into_iter()
        .map(|row| PostTagLink {
            post_slug: row.post_slug,
            tag_slug: row.tag_slug,
        })
        .collect())
}

async fn fetch_navigation(pool: &PgPool) -> Result<Vec<NavigationSnapshot>, AppError> {
    let rows = query!(
        r#"
        SELECT
            ni.label,
            ni.destination_type AS "destination_type: NavigationDestinationType",
            ni.destination_url,
            p.slug AS "destination_page_slug?",
            ni.sort_order,
            ni.open_in_new_tab,
            ni.visible
        FROM navigation_items ni
        LEFT JOIN pages p ON p.id = ni.destination_page_id
        ORDER BY ni.sort_order ASC, ni.label ASC
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_error)?;

    Ok(rows
        .into_iter()
        .map(|row| NavigationSnapshot {
            label: row.label,
            destination_type: row.destination_type,
            destination_url: row.destination_url,
            destination_page_slug: row.destination_page_slug,
            sort_order: row.sort_order,
            open_in_new_tab: row.open_in_new_tab,
            visible: row.visible,
        })
        .collect())
}

#[derive(Debug, Serialize, Deserialize)]
struct SiteArchive {
    migrations: MigrationSnapshot,
    site_settings: SiteSettingsSnapshot,
    posts: Vec<PostSnapshot>,
    pages: Vec<PageSnapshot>,
    tags: Vec<TagSnapshot>,
    #[serde(rename = "post_tags")]
    post_tags: Vec<PostTagLink>,
    navigation_items: Vec<NavigationSnapshot>,
}

impl SiteArchive {
    fn normalize(&mut self) {
        self.posts.sort_by(|a, b| a.slug.cmp(&b.slug));
        self.pages.sort_by(|a, b| a.slug.cmp(&b.slug));
        self.tags.sort_by(|a, b| a.slug.cmp(&b.slug));
        self.post_tags.sort_by(|a, b| {
            a.post_slug
                .cmp(&b.post_slug)
                .then(a.tag_slug.cmp(&b.tag_slug))
        });
        self.navigation_items
            .sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.label.cmp(&b.label)));
        self.migrations.entries.sort_by_key(|entry| entry.version);
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct MigrationSnapshot {
    entries: Vec<MigrationEntry>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct MigrationEntry {
    version: i64,
    checksum: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SiteSettingsSnapshot {
    homepage_size: i32,
    admin_page_size: i32,
    show_tag_aggregations: bool,
    show_month_aggregations: bool,
    tag_filter_limit: i32,
    month_filter_limit: i32,
    global_toc_enabled: bool,
    brand_title: String,
    brand_href: String,
    footer_copy: String,
    public_site_url: String,
    favicon_svg: String,
    timezone: String,
    meta_title: String,
    meta_description: String,
    og_title: String,
    og_description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PostSnapshot {
    slug: String,
    title: String,
    excerpt: String,
    body_markdown: String,
    summary_markdown: Option<String>,
    status: PostStatus,
    pinned: bool,
    scheduled_at: Option<OffsetDateTime>,
    published_at: Option<OffsetDateTime>,
    archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PageSnapshot {
    slug: String,
    title: String,
    body_markdown: String,
    status: PageStatus,
    scheduled_at: Option<OffsetDateTime>,
    published_at: Option<OffsetDateTime>,
    archived_at: Option<OffsetDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TagSnapshot {
    slug: String,
    name: String,
    description: Option<String>,
    pinned: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct PostTagLink {
    post_slug: String,
    tag_slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NavigationSnapshot {
    label: String,
    destination_type: NavigationDestinationType,
    destination_url: Option<String>,
    destination_page_slug: Option<String>,
    sort_order: i32,
    open_in_new_tab: bool,
    visible: bool,
}
