use sqlx::{PgPool, query};

use crate::{
    application::error::AppError,
    domain::{
        api_keys::{ApiKeyStatus, ApiScope},
        types::{NavigationDestinationType, PageStatus, PostStatus},
    },
    infra::db::api_keys::pg_interval_to_duration,
};

use super::{
    SETTINGS_ROW_ID, map_sqlx_error,
    models::{
        ApiKeySnapshot, MigrationEntry, MigrationSnapshot, NavigationSnapshot, PageSnapshot,
        PostSnapshot, PostTagLink, SiteArchive, SiteSettingsSnapshot, TagSnapshot,
    },
};

pub(super) async fn gather_archive(pool: &PgPool) -> Result<SiteArchive, AppError> {
    let migrations = fetch_migrations(pool).await?;
    let site_settings = fetch_site_settings(pool).await?;
    let posts = fetch_posts(pool).await?;
    let pages = fetch_pages(pool).await?;
    let tags = fetch_tags(pool).await?;
    let post_tags = fetch_post_tags(pool).await?;
    let navigation_items = fetch_navigation(pool).await?;
    let api_keys = fetch_api_keys(pool).await?;

    Ok(SiteArchive {
        migrations,
        site_settings,
        posts,
        pages,
        tags,
        post_tags,
        navigation_items,
        api_keys,
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

async fn fetch_api_keys(pool: &PgPool) -> Result<Vec<ApiKeySnapshot>, AppError> {
    let rows = query!(
        r#"
        SELECT
            id,
            name,
            description,
            prefix,
            hashed_secret,
            scopes as "scopes: Vec<ApiScope>",
            status as "status: ApiKeyStatus",
            expires_in,
            expires_at,
            revoked_at,
            last_used_at,
            created_by,
            created_at,
            updated_at
        FROM api_keys
        ORDER BY created_at
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_error)?;

    Ok(rows
        .into_iter()
        .map(|row| ApiKeySnapshot {
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
        .collect())
}
