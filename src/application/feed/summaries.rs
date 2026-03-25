use super::*;

pub(super) fn order_tags_with_pins(counts: &[TagWithCount]) -> Vec<&TagWithCount> {
    let mut ordered: Vec<&TagWithCount> = counts.iter().collect();
    ordered.sort_by(|left, right| {
        right
            .pinned
            .cmp(&left.pinned)
            .then(right.count.cmp(&left.count))
            .then(left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then(left.slug.to_lowercase().cmp(&right.slug.to_lowercase()))
    });
    ordered
}

pub(super) fn build_tag_summaries(
    counts: &[TagWithCount],
    active_tag: Option<&str>,
    total_posts: u64,
    settings: &SiteSettingsRecord,
) -> Vec<views::TagSummary> {
    let mut summaries = Vec::with_capacity(counts.len() + 1);
    summaries.push(views::TagSummary {
        label: "All tags".to_string(),
        path: "/".to_string(),
        count: usize::try_from(total_posts).unwrap_or(usize::MAX),
        is_active: active_tag.is_none(),
    });

    let ordered = order_tags_with_pins(counts);
    let limit = settings.tag_filter_limit.max(0) as usize;
    let mut non_pinned_added = 0;

    for entry in ordered {
        if !entry.pinned && non_pinned_added >= limit {
            continue;
        }

        if !entry.pinned {
            non_pinned_added += 1;
        }

        summaries.push(views::TagSummary {
            label: format!("#{}", entry.name),
            path: format!("/tags/{}", entry.slug),
            count: usize::try_from(entry.count).unwrap_or(usize::MAX),
            is_active: active_tag.map(|tag| tag == entry.slug).unwrap_or(false),
        });
    }

    summaries
}

pub(super) fn build_month_summaries(
    counts: &[posts::MonthCount],
    active: Option<&str>,
    total_posts: u64,
    limit: i32,
) -> Vec<views::MonthSummary> {
    let mut summaries = Vec::with_capacity(counts.len() + 1);
    summaries.push(views::MonthSummary {
        label: "All months".to_string(),
        path: "/".to_string(),
        count: usize::try_from(total_posts).unwrap_or(usize::MAX),
        is_active: active.is_none(),
    });

    let quota = limit.max(0) as usize;
    for entry in counts.iter().take(quota) {
        summaries.push(views::MonthSummary {
            label: entry.label.clone(),
            path: format!("/months/{}", entry.key),
            count: entry.count,
            is_active: active.map(|value| value == entry.key).unwrap_or(false),
        });
    }

    summaries
}
