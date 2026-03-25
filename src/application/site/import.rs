use std::collections::HashMap;

use sqlx::{Postgres, Transaction, query};
use uuid::Uuid;

use crate::{
    application::error::AppError,
    domain::{
        api_keys::{ApiKeyStatus, ApiScope},
        types::{NavigationDestinationType, PageStatus, PostStatus},
    },
    infra::db::{PostgresRepositories, api_keys::duration_to_pg_interval},
};

use super::{
    SETTINGS_ROW_ID, map_sqlx_error,
    models::{MigrationEntry, MigrationSnapshot, SiteArchive},
};

pub(super) async fn import_archive(
    repositories: &PostgresRepositories,
    archive: SiteArchive,
) -> Result<(), AppError> {
    let mut tx = repositories.begin().await.map_err(map_sqlx_error)?;

    let db_migrations = fetch_migrations_from(&mut tx).await?;
    ensure_migrations_match(&db_migrations.entries, &archive.migrations.entries)?;

    query("SET CONSTRAINTS ALL DEFERRED")
        .execute(tx.as_mut())
        .await
        .map_err(map_sqlx_error)?;

    query(
        "TRUNCATE post_tags, navigation_items, tags, pages, posts, api_keys RESTART IDENTITY CASCADE",
    )
    .execute(tx.as_mut())
    .await
    .map_err(map_sqlx_error)?;

    for key in &archive.api_keys {
        let expires_in_pg = key.expires_in.map(duration_to_pg_interval);
        query!(
            r#"
            INSERT INTO api_keys (
                id,
                name,
                description,
                prefix,
                hashed_secret,
                scopes,
                status,
                expires_in,
                expires_at,
                revoked_at,
                last_used_at,
                created_by,
                created_at,
                updated_at
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6::api_scope[],
                $7::api_key_status,
                $8,
                $9,
                $10,
                $11,
                $12,
                $13,
                $14
            )
            "#,
            key.id,
            key.name,
            key.description,
            key.prefix,
            key.hashed_secret,
            key.scopes.clone() as Vec<ApiScope>,
            key.status as ApiKeyStatus,
            expires_in_pg,
            key.expires_at,
            key.revoked_at,
            key.last_used_at,
            key.created_by,
            key.created_at,
            key.updated_at,
        )
        .execute(tx.as_mut())
        .await
        .map_err(map_sqlx_error)?;
    }

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
