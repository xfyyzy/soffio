use async_trait::async_trait;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::application::repos::{RepoError, SectionsRepo};
use crate::domain::entities::PostSectionRecord;

use super::PostgresRepositories;
use super::types::{PersistedPostSection, PersistedPostSectionOwned, PostSectionRow};

impl PostgresRepositories {
    pub async fn find_post_id_by_slug_immediate(
        &self,
        slug: &str,
    ) -> Result<Option<Uuid>, RepoError> {
        let row = sqlx::query!(
            r#"
            SELECT id
            FROM posts
            WHERE slug = $1
            "#,
            slug
        )
        .fetch_optional(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(row.map(|record| record.id))
    }

    pub async fn find_post_id_by_slug(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        slug: &str,
    ) -> Result<Option<Uuid>, RepoError> {
        let row = sqlx::query!(
            r#"
            SELECT id
            FROM posts
            WHERE slug = $1
            "#,
            slug
        )
        .fetch_optional(tx.as_mut())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(row.map(|record| record.id))
    }

    pub async fn replace_post_sections(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        post_id: Uuid,
        sections: &[PersistedPostSection<'_>],
    ) -> Result<(), RepoError> {
        let owned: Vec<PersistedPostSectionOwned> = sections
            .iter()
            .map(PersistedPostSectionOwned::from)
            .collect();
        self.replace_post_sections_bulk(tx, post_id, &owned).await
    }

    pub async fn replace_post_sections_bulk(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        post_id: Uuid,
        sections: &[PersistedPostSectionOwned],
    ) -> Result<(), RepoError> {
        sqlx::query!(
            r#"
            DELETE FROM post_sections
            WHERE post_id = $1
            "#,
            post_id
        )
        .execute(tx.as_mut())
        .await
        .map_err(RepoError::from_persistence)?;

        if sections.is_empty() {
            return Ok(());
        }

        let mut copy = tx
            .copy_in_raw(
                r#"COPY post_sections (id, post_id, parent_id, position, level, heading_html, heading_text, body_html, contains_code, contains_math, contains_mermaid, anchor_slug) FROM STDIN WITH (FORMAT CSV, DELIMITER ',', NULL '', QUOTE '"')"#,
            )
            .await
            .map_err(RepoError::from_persistence)?;

        let mut buffer = String::with_capacity(64 * 1024);
        let post_id_str = post_id.to_string();

        for section in sections {
            append_csv_row(&mut buffer, &post_id_str, section);

            if buffer.len() >= 32 * 1024 {
                copy.send(buffer.as_bytes())
                    .await
                    .map_err(RepoError::from_persistence)?;
                buffer.clear();
            }
        }

        if !buffer.is_empty() {
            copy.send(buffer.as_bytes())
                .await
                .map_err(RepoError::from_persistence)?;
            buffer.clear();
        }

        copy.finish().await.map_err(RepoError::from_persistence)?;

        Ok(())
    }

    pub async fn update_post_summary_html(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        post_id: Uuid,
        summary_html: &str,
    ) -> Result<(), RepoError> {
        sqlx::query!(
            r#"
            UPDATE posts
            SET summary_html = $2,
                updated_at = now()
            WHERE id = $1
            "#,
            post_id,
            summary_html
        )
        .execute(tx.as_mut())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(())
    }

    pub async fn update_post_updated_at(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        post_id: Uuid,
    ) -> Result<(), RepoError> {
        sqlx::query!(
            r#"
            UPDATE posts
            SET updated_at = now()
            WHERE id = $1
            "#,
            post_id
        )
        .execute(tx.as_mut())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(())
    }
}

#[async_trait]
impl SectionsRepo for PostgresRepositories {
    async fn list_sections(&self, post_id: Uuid) -> Result<Vec<PostSectionRecord>, RepoError> {
        let rows = sqlx::query_as!(
            PostSectionRow,
            r#"
            SELECT id, post_id, parent_id, position, level, heading_html, heading_text, body_html,
                   contains_code, contains_math, contains_mermaid, anchor_slug, created_at
            FROM post_sections
            WHERE post_id = $1
            ORDER BY parent_id NULLS FIRST, position ASC
            "#,
            post_id
        )
        .fetch_all(self.pool())
        .await
        .map_err(RepoError::from_persistence)?;

        Ok(rows.into_iter().map(PostSectionRecord::from).collect())
    }
}

fn append_csv_row(buffer: &mut String, post_id_str: &str, section: &PersistedPostSectionOwned) {
    let mut first = true;

    let id = section.id.to_string();
    append_csv_field(buffer, &mut first, &id);
    append_csv_field(buffer, &mut first, post_id_str);

    let parent = section.parent_id.map(|id| id.to_string());
    append_csv_field_option(buffer, &mut first, parent.as_deref());

    let position = section.position.to_string();
    append_csv_field(buffer, &mut first, &position);

    let level = section.level.to_string();
    append_csv_field(buffer, &mut first, &level);

    append_csv_field(buffer, &mut first, section.heading_html.as_str());
    append_csv_field(buffer, &mut first, section.heading_text.as_str());
    append_csv_field(buffer, &mut first, section.body_html.as_str());
    append_csv_field(
        buffer,
        &mut first,
        if section.contains_code {
            "true"
        } else {
            "false"
        },
    );
    append_csv_field(
        buffer,
        &mut first,
        if section.contains_math {
            "true"
        } else {
            "false"
        },
    );
    append_csv_field(
        buffer,
        &mut first,
        if section.contains_mermaid {
            "true"
        } else {
            "false"
        },
    );
    append_csv_field(buffer, &mut first, section.anchor_slug.as_str());

    buffer.push('\n');
}

fn append_csv_field(buffer: &mut String, first: &mut bool, value: &str) {
    if !*first {
        buffer.push(',');
    } else {
        *first = false;
    }

    append_csv_value(buffer, value);
}

fn append_csv_field_option(buffer: &mut String, first: &mut bool, value: Option<&str>) {
    if !*first {
        buffer.push(',');
    } else {
        *first = false;
    }

    if let Some(value) = value {
        append_csv_value(buffer, value);
    }
}

fn append_csv_value(buffer: &mut String, value: &str) {
    if value.is_empty() {
        return;
    }

    if value
        .bytes()
        .any(|b| matches!(b, b',' | b'"' | b'\n' | b'\r'))
    {
        buffer.push('"');
        for ch in value.chars() {
            if ch == '"' {
                buffer.push('"');
            }
            buffer.push(ch);
        }
        buffer.push('"');
    } else {
        buffer.push_str(value);
    }
}
