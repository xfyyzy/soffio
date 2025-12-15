# Cache Candidates

This document details objects and data structures within the codebase that are candidates for caching. They are categorized by their source and access patterns.

## 1. Global Configuration & Navigation (High Read, Low Write)

These objects are accessed on almost every request (e.g., for headers, footers, metadata) and rarely change.

*   **Site Settings**
    *   **Representation**: `crate::domain::entities::SiteSettingsRecord`
    *   **Source**: `src/infra/db/settings.rs`
    *   **Access Pattern**: Fetched by `load_site_settings` on nearly every page load to populate `base.html` context (branding, footer copy, meta tags).
    *   **Cache strategy**: Singleton. Invalidation on admin update.

*   **Navigation Menu**
    *   **Representation**: `crate::domain::navigation::Navigation` (or `Vec<NavigationItemRecord>`)
    *   **Source**: `src/infra/db/navigation.rs`
    *   **Access Pattern**: Fetched on every page load for the header menu.
    *   **Cache strategy**: Singleton. Invalidation on admin update.
    *   **Existing Mechanism**: `src/domain/navigation.rs` uses `OnceLock` for a static mock. The system seems to separate the DB implementation from this mock.

## 2. Content Entities (High Read, Low Write)

Primary content requires database hits and potentially markdown rendering.

*   **Posts (Single)**
    *   **Representation**: `crate::domain::entities::PostRecord`
    *   **Source**: `src/infra/db/posts/read.rs` (`find_by_slug`, `find_by_id`)
    *   **Access Pattern**: Detailed view of a post.
    *   **Cache strategy**: Key-Value (`slug -> Post`, `id -> Post`). Invalidation on edit/publish.
    *   **Existing Mechanism**: `src/domain/posts/data.rs` defines a `static POSTS` array, effectively a compile-time cache for hardcoded content.

*   **Pages (Single)**
    *   **Representation**: `crate::domain::entities::PageRecord`
    *   **Source**: `src/infra/db/pages.rs` (`find_by_slug`, `find_by_id`)
    *   **Access Pattern**: Serving static-like pages (About, Privacy Policy).
    *   **Cache strategy**: Key-Value (`slug -> Page`). Invalidation on edit.
    *   **Existing Mechanism**: `src/domain/pages.rs` uses `OnceLock` for a mock repository.

## 3. Lists & Aggregations (Expensive Computations)

Complex queries involving filtering, sorting, and counting.

*   **Post Lists (Pagination)**
    *   **Representation**: `CursorPage<PostRecord>`
    *   **Source**: `src/infra/db/posts/read.rs` (`list_posts`)
    *   **Access Pattern**: Homepage, Archives, Admin Post List.
    *   **Cache strategy**: Keyed by `scope` + `filter` + `cursor`. High cardinality.

*   **Page Lists (Pagination)**
    *   **Representation**: `CursorPage<PageRecord>`
    *   **Source**: `src/infra/db/pages.rs` (`list_pages`)
    *   **Access Pattern**: Admin Page List, Sitemap generation.

*   **Aggregations (Analytics/Filters)**
    *   **Representation**:
        *   `Vec<MonthCount>` (Archive sidebar)
        *   `Vec<PostTagCount>` (Tag cloud/sidebar)
    *   **Source**: `src/infra/db/posts/read.rs` (`list_month_counts`, `list_tag_counts`)
    *   **Access Pattern**: Computed on many pages (sidebar).
    *   **Cache strategy**: Global/singleton for public view. Invalidation on any post add/remove/update.

## 4. Security & Auth (High Throughput)

*   **API Keys**
    *   **Representation**: `crate::domain::api_keys::ApiKeyRecord`
    *   **Source**: `src/infra/db/api_keys.rs` (`find_by_prefix`)
    *   **Access Pattern**: Checked on **every** API request to validate authentication.
    *   **Cache strategy**: Key-Value (`prefix -> ApiKeyRecord`). Critical for API latency. High hit rate expected.

## 5. Other Potential Candidates

*   **RSS/Atom Feeds**: Generated XML strings. Can be cached as a blob.
*   **Sitemaps**: `sitemap.xml`. Generated from iterating all posts/pages.
*   **Rendered Markdown**: If markdown rendering becomes a bottleneck, the `rendered_html` field in DB acts as a persistent cache, but an in-memory layer could save DB bandwidth.
