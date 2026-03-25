use super::*;

pub(super) fn record_to_card(
    record: &PostRecord,
    tags: &[TagRecord],
    timezone: chrono_tz::Tz,
) -> PostCard {
    let published_at = record.published_at.unwrap_or(record.created_at);
    let localized = timezone::localized_datetime(published_at, timezone);
    let date = timezone::localized_date(published_at, timezone);

    PostCard {
        slug: record.slug.clone(),
        title: record.title.clone(),
        excerpt: record.excerpt.clone(),
        iso_date: localized.to_rfc3339(),
        published: posts::format_human_date(date),
        badges: build_tag_badges(
            tags.iter()
                .map(|tag| (tag.slug.as_str(), tag.name.as_str())),
        ),
        is_pinned: record.pinned,
    }
}

pub(super) fn build_posts_ld_json(
    cards: &[PostCard],
    filter: &FeedFilter,
    public_site_url: &str,
    blog_name: &str,
) -> Option<String> {
    if cards.is_empty() {
        return None;
    }

    let site_url = normalize_public_site_url(public_site_url);
    let blog_url = format!("{site_url}{}", filter.base_path());

    let blog_posts = cards
        .iter()
        .map(|card| {
            json!({
                "@type": "BlogPosting",
                "headline": card.title,
                "description": card.excerpt,
                "datePublished": card.iso_date,
                "url": format!("{site_url}posts/{}", card.slug),
            })
        })
        .collect::<Vec<_>>();

    serde_json::to_string(&json!({
        "@context": "https://schema.org",
        "@type": "Blog",
        "name": blog_name,
        "url": blog_url,
        "blogPost": blog_posts,
    }))
    .ok()
}

pub(super) fn build_datastar_append_response(
    payload: AppendPayload,
    load_more_query: String,
) -> Result<Response, HttpError> {
    let AppendPayload {
        offset,
        cards,
        next_cursor,
        total_visible,
    } = payload;

    let appended_count = cards.len();

    let cards_html = if appended_count > 0 {
        let template = PostCardsAppendTemplate {
            posts: cards,
            offset,
        };
        Some(template.render().map_err(|err| {
            HttpError::from(TemplateRenderError::new(
                "application::feed::build_datastar_append_response",
                "Template rendering failed",
                err,
            ))
        })?)
    } else {
        None
    };

    let loader_html = FeedLoaderTemplate {
        view: FeedLoaderContext {
            has_results: total_visible > 0,
            next_cursor,
            load_more_query,
        },
    }
    .render()
    .map_err(|err| {
        HttpError::from(TemplateRenderError::new(
            "application::feed::build_datastar_append_response",
            "Template rendering failed",
            err,
        ))
    })?;

    let mut stream = StreamBuilder::new();

    if let Some(html) = cards_html {
        stream.push_patch(html, "#post-grid", ElementPatchMode::Append);
    }

    stream.push_patch(
        loader_html,
        "#feed-sentinel-container",
        ElementPatchMode::Inner,
    );

    let script = format!(
        "(function() {{ const grid = document.querySelector('#post-grid'); if (grid) {{ grid.setAttribute('data-count', '{}'); }} }})();",
        total_visible
    );
    stream.push_script(script);

    stream.push_signals(r#"{"feedLoading": false}"#);

    Ok(stream.into_response())
}

pub(super) fn homepage_page_limit(settings: &SiteSettingsRecord) -> u32 {
    let clamped = settings.homepage_size.clamp(1, 48) as u32;
    if clamped == 0 {
        DEFAULT_PAGE_SIZE as u32
    } else {
        clamped
    }
}

fn normalize_public_site_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    format!("{trimmed}/")
}
