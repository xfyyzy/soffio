use std::collections::HashSet;

use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn upload_indexes_exist(pool: PgPool) {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT indexname FROM pg_indexes WHERE schemaname = 'public' AND tablename = 'uploads'",
    )
    .fetch_all(&pool)
    .await
    .expect("fetch upload indexes");

    let indexes: HashSet<String> = rows.into_iter().collect();

    assert!(
        indexes.contains("uploads_created_at_id_idx"),
        "missing uploads_created_at_id_idx"
    );
    assert!(
        indexes.contains("uploads_content_type_created_at_id_idx"),
        "missing uploads_content_type_created_at_id_idx"
    );
}
