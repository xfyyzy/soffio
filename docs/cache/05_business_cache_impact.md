# Business → Cache Impact Matrix (Dec 2025)

Objective: enumerate business-side changes and the exact cache surfaces they influence, based on current code paths. Accuracy first; prefer conservative warms when uncertain, but encode the real rules so the planner can stay precise.

Legend
- Pages: `/`, `/tags/{tag}`, `/months/{yyyymm}`, `/posts/{slug}`, `/{page_slug}`, `/ui/posts` (partial), SEO: sitemap.xml, rss.xml, atom.xml, robots.txt.
- Settings fields from `SiteSettingsRecord`; navigation items from `navigation_items` (visible + internal destinations).
- Tag visibility on homepage uses `order_tags_with_pins` + `tag_filter_limit` in `application::feed::build_tag_summaries` when `show_tag_aggregations` is true.
- Month list uses `month_filter_limit` in `build_month_summaries` when `show_month_aggregations` is true.

## Post lifecycle
- **Create/Update Draft (status not Published)**
  - No public cache effect unless status flips to Published. (Post detail rejects non-published.)
- **Publish or Unpublish** (status crosses Published boundary)
  - Warm: `/posts/{slug}` (if published), homepage, `/ui/posts`, relevant `/tags/{tag}` for all tags on the post, relevant `/months/{yyyymm}` month archive (if `show_month_aggregations`), sitemap entry for the post, rss/atom feeds (post list includes published posts only), homepage tag list (if `show_tag_aggregations`), month list (if `show_month_aggregations`).
  - Evict: `/posts/{slug}` on unpublish/delete; remove from feeds/sitemap by rewarming them.
- **Edit published post title/excerpt/body/summary**
  - Warm: `/posts/{slug}`, homepage + `/ui/posts` card (title/excerpt/date), tag pages containing the post, month page for its published month (if `show_month_aggregations`), rss/atom entries (title/excerpt), sitemap lastmod for the post. Tag list/month list unaffected unless tag set changes.
- **Edit tags on a published post** (add/remove tags, change set)
  - Warm: same as publish, plus homepage tag list (tag counts), tag pages added/removed, rss/atom (post metadata), sitemap lastmod.
- **Edit pinned flag on post** (pinned affects ordering only; posts are ordered pinned desc then time)
  - Warm: homepage and `/ui/posts` (ordering), tag/month pages where the post appears (ordering), rss/atom ordering is time-only so unaffected.

## Tag changes
- **Name change**
  - Warm: `/tags/{slug}`, homepage + `/ui/posts` + `/months/*` where posts containing the tag appear (badges show name), homepage tag list if tag appears in displayed slice. RSS/Atom do **not** include tag names, so feeds unaffected.
- **Pinned flip**
  - Warm: same surfaces as name change; additionally homepage tag list because `order_tags_with_pins` prioritises pinned tags (only when `show_tag_aggregations` is enabled).
- **Description change**
  - Warm: `/tags/{slug}` (tag page renders description). Other pages unaffected.
- **Tag usage counts change** (driven by post publish/unpublish or retagging)
  - Warm: homepage tag list slice (when `show_tag_aggregations`) and tag pages whose counts changed enough to enter/leave the visible slice; month/tag pages containing affected posts already covered by post events.

## Navigation changes (navigation_items)
- Changes to label/destination/visibility/sort/open_in_new_tab for items with `destination_type=internal` and `visible=true` affect chrome rendered on **all public pages**.
  - Warm: homepage, `/posts/*`, `/tags/*`, `/months/*`, `/{page_slug}`. Also sitemap? (nav does not affect sitemap contents) → no sitemap change.
- If destination points to an internal page slug, ensure that page is warmed because users may click; but content doesn’t depend on nav—optional.

## Page changes
- **Publish/Unpublish page**
  - Warm: `/{slug}` (if published), sitemap entry, chrome-driven pages (because nav may include it), homepage chrome. Evict `/{slug}` on unpublish/delete.
- **Edit published page title/body**
  - Warm: `/{slug}`, sitemap lastmod for that page. Nav label change is handled via navigation events (see above) not page body.

## Settings (SiteSettingsRecord)
- **meta_title/meta_description/og_title/og_description/public_site_url/canonical**
  - Warm: all public pages, rss/atom (titles/descriptions), sitemap (base URL). robots.txt uses `public_site_url` → warm robots.
- **homepage_size**
  - Warm: homepage + `/ui/posts` (page size changes pagination slices), rss/atom unaffected.
- **show_tag_aggregations/tag_filter_limit**
  - Warm: homepage + tag pages (tag summaries visibility/slice), any page rendering `tags` block. If turned off, warming should clear cached versions that still render tags.
- **show_month_aggregations/month_filter_limit**
  - Warm: homepage + month pages (month summary block visibility/slice). If turned off, clear cached versions showing months.
- **timezone**
  - Warm: all pages showing dates (home, tag/month pages, post detail) and rss/atom because dates are rendered in new zone.
- **brand_title/brand_href/footer_copy/favicon_svg**
  - Warm: all public pages (chrome/footer/favicons served from settings). robots unaffected except via favicon not used there.

## Chrome / layout / settings aggregation
- **Chrome metadata (LayoutChrome) reload triggers** include settings above; if chrome reloads, warm all public pages + SEO feeds to propagate canonical/title/description.
- **SettingsChanged helper** should map the following to cache surfaces:
  - `homepage_size` → homepage, `/ui/posts`.
  - `show_tag_aggregations` / `tag_filter_limit` → homepage + tag lists; clear cached pages showing tags when disabled.
  - `show_month_aggregations` / `month_filter_limit` → homepage + month lists; clear cached pages showing months when disabled.
  - `timezone` → all pages with dates + RSS/Atom.
  - `brand_title`/`brand_href`/`footer_copy`/`favicon_svg`/`public_site_url`/`meta_title`/`meta_description`/`og_title`/`og_description`/`canonical` → all public pages; RSS/Atom (title/description); sitemap/robots via `public_site_url`.

## Snapshot rollback/publish (content rewind)
- Treat as bulk post/page content changes: warm affected post/page slugs plus aggregates (home, tag, month, feeds) for any published content touched.

## Robots.txt
- Affected only by `public_site_url` (sitemap link). Warm robots when that changes.

## Derived inclusion rules the planner must compute
- Tag visibility on homepage: include tag if `show_tag_aggregations` is true **and** tag is within the first `tag_filter_limit` entries after sorting by (pinned desc, count desc, name asc, slug asc). Counts come from `tags.list_with_counts()` (published posts).
- Month visibility: include month if `show_month_aggregations` is true **and** it is within the first `month_filter_limit` entries from `posts.list_month_counts` (published posts only).
- Post listing/order: homepage/tag/month pages order posts by pinned desc then published_at desc (via `PostCursor` ordering). Any change to `pinned`, `published_at`, or status affects ordering.
- Feeds: rss/atom include up to 100 published posts ordered by published_at/updated_at; content = title + excerpt. Tags not rendered explicitly, but tag name changes still matter for HTML pages, not feeds.
- Sitemap: includes home (lastmod = settings.updated_at), each published post (lastmod published_at or updated_at), each published page (lastmod published_at or updated_at). Tag or month lists are **not** in sitemap.

## Conservatism guidance
- When precise inclusion (e.g., tag slice membership) cannot be computed cheaply, choose conservative warm set: homepage + affected tag pages + sitemap/rss/atom as applicable. Document when using conservative mode.
- If planner cannot cheaply recompute tag/month slices, fallback should still respect toggles: only warm tag/month blocks when their `show_*_aggregations` setting is enabled.
