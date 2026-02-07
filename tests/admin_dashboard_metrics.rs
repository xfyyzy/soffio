use soffio::application::repos::{
    CreateNavigationItemParams, NavigationQueryFilter, NavigationRepo, NavigationWriteRepo,
    UploadQueryFilter, UploadsRepo,
};
use soffio::domain::entities::UploadRecord;
use soffio::domain::types::NavigationDestinationType;
use soffio::domain::uploads::UploadMetadata;
use soffio::infra::db::PostgresRepositories;
use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn dashboard_navigation_external_count_matches_seed(pool: PgPool) {
    let repos = PostgresRepositories::new(pool);

    NavigationWriteRepo::create_navigation_item(
        &repos,
        CreateNavigationItemParams {
            label: "External first".to_string(),
            destination_type: NavigationDestinationType::External,
            destination_page_id: None,
            destination_url: Some("https://example.com/first".to_string()),
            sort_order: 1,
            visible: true,
            open_in_new_tab: false,
        },
    )
    .await
    .expect("create first external navigation item");

    NavigationWriteRepo::create_navigation_item(
        &repos,
        CreateNavigationItemParams {
            label: "External second".to_string(),
            destination_type: NavigationDestinationType::External,
            destination_page_id: None,
            destination_url: Some("https://example.com/second".to_string()),
            sort_order: 2,
            visible: false,
            open_in_new_tab: true,
        },
    )
    .await
    .expect("create second external navigation item");

    let all_external =
        NavigationRepo::count_external_navigation(&repos, None, &NavigationQueryFilter::default())
            .await
            .expect("count all external navigation items");
    assert_eq!(all_external, 2);

    let visible_external = NavigationRepo::count_external_navigation(
        &repos,
        Some(true),
        &NavigationQueryFilter::default(),
    )
    .await
    .expect("count visible external navigation items");
    assert_eq!(visible_external, 1);

    let filtered_external = NavigationRepo::count_external_navigation(
        &repos,
        None,
        &NavigationQueryFilter {
            search: Some("second".to_string()),
        },
    )
    .await
    .expect("count filtered external navigation items");
    assert_eq!(filtered_external, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn dashboard_upload_total_bytes_matches_seed(pool: PgPool) {
    let repos = PostgresRepositories::new(pool);
    let created_at = OffsetDateTime::now_utc();

    UploadsRepo::insert_upload(
        &repos,
        UploadRecord {
            id: Uuid::new_v4(),
            filename: "first.png".to_string(),
            content_type: "image/png".to_string(),
            size_bytes: 10,
            checksum: "1".repeat(64),
            stored_path: "uploads/first.png".to_string(),
            metadata: UploadMetadata::default(),
            created_at,
        },
    )
    .await
    .expect("insert first upload");

    UploadsRepo::insert_upload(
        &repos,
        UploadRecord {
            id: Uuid::new_v4(),
            filename: "second.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            size_bytes: 32,
            checksum: "2".repeat(64),
            stored_path: "uploads/second.pdf".to_string(),
            metadata: UploadMetadata::default(),
            created_at,
        },
    )
    .await
    .expect("insert second upload");

    let total_bytes = UploadsRepo::sum_upload_sizes(&repos, &UploadQueryFilter::default())
        .await
        .expect("sum all upload bytes");
    assert_eq!(total_bytes, 42);

    let image_bytes = UploadsRepo::sum_upload_sizes(
        &repos,
        &UploadQueryFilter {
            content_type: Some("image/png".to_string()),
            ..UploadQueryFilter::default()
        },
    )
    .await
    .expect("sum image upload bytes");
    assert_eq!(image_bytes, 10);
}
