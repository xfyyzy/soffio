use std::collections::{BTreeSet, HashMap};

use axum::http::StatusCode;
use chrono_tz::Tz;
use uuid::Uuid;

use crate::{
    application::{
        admin::tags::AdminTagError, error::HttpError, feed::order_tags_with_pins,
        repos::TagWithCount,
    },
    domain::{entities::PostRecord, types::PostStatus},
    infra::http::admin::AdminState,
    presentation::admin::views as admin_views,
};

use super::status::post_status_options;

pub(super) async fn build_post_editor_view(
    state: &AdminState,
    post: &PostRecord,
    tz: Tz,
) -> Result<admin_views::AdminPostEditorView, HttpError> {
    let tags_with_counts = load_tag_counts(state).await?;
    let selected_records = load_selected_tags(state, post.id).await?;
    let selected_ids: Vec<Uuid> = selected_records.iter().map(|tag| tag.id).collect();

    let tag_picker = build_tag_picker_view(Some(post.id), &tags_with_counts, &selected_ids);

    Ok(admin_views::AdminPostEditorView {
        title: post.title.clone(),
        heading: format!("Edit Post: {}", post.title),
        excerpt: post.excerpt.clone(),
        body_markdown: post.body_markdown.clone(),
        summary_markdown: post.summary_markdown.clone(),
        status: post.status,
        status_options: post_status_options(post.status),
        published_at: post
            .published_at
            .map(|time| admin_views::format_timestamp(time, tz)),
        form_action: format!("/posts/{}/edit", post.id),
        submit_label: "Save Changes".to_string(),
        enable_live_submit: true,
        tag_picker,
        pinned: post.pinned,
    })
}

pub(super) async fn build_new_post_editor_view(
    state: &AdminState,
) -> Result<admin_views::AdminPostEditorView, HttpError> {
    let tags_with_counts = load_tag_counts(state).await?;
    let selected_ids = Vec::new();

    let tag_picker = build_tag_picker_view(None, &tags_with_counts, &selected_ids);

    Ok(admin_views::AdminPostEditorView {
        title: String::new(),
        heading: "Create Post".to_string(),
        excerpt: String::new(),
        body_markdown: String::new(),
        summary_markdown: None,
        status: PostStatus::Draft,
        status_options: post_status_options(PostStatus::Draft),
        published_at: None,
        form_action: "/posts/create".to_string(),
        submit_label: "Create Post".to_string(),
        enable_live_submit: true,
        tag_picker,
        pinned: false,
    })
}

pub(super) fn build_tag_picker_view(
    post_id: Option<Uuid>,
    tags_with_counts: &[TagWithCount],
    selected_ids: &[Uuid],
) -> admin_views::AdminPostTagPickerView {
    let toggle_action = match post_id {
        Some(id) => format!("/posts/{}/tags/toggle", id),
        None => "/posts/new/tags/toggle".to_string(),
    };

    let selected_set: BTreeSet<Uuid> = selected_ids.iter().copied().collect();
    let mut tag_lookup: HashMap<Uuid, &TagWithCount> = HashMap::new();
    for tag in tags_with_counts {
        tag_lookup.insert(tag.id, tag);
    }

    let ordered = order_tags_with_pins(tags_with_counts);

    let mut options = Vec::new();
    for tag in ordered {
        options.push(admin_views::AdminPostTagPickerOptionView {
            id: tag.id.to_string(),
            name: tag.name.clone(),
            slug: tag.slug.clone(),
            usage_count: tag.count,
            is_selected: selected_set.contains(&tag.id),
        });
    }

    let mut selected = Vec::new();
    let mut selected_tag_ids = Vec::new();

    for id in selected_ids {
        if let Some(tag) = tag_lookup.get(id) {
            selected_tag_ids.push(id.to_string());
            selected.push(admin_views::AdminPostSelectedTagView {
                id: id.to_string(),
                name: tag.name.clone(),
                slug: tag.slug.clone(),
            });
        }
    }

    admin_views::AdminPostTagPickerView {
        toggle_action,
        options,
        selected,
        selected_tag_ids,
    }
}

pub(super) async fn load_tag_counts(state: &AdminState) -> Result<Vec<TagWithCount>, HttpError> {
    state
        .tags
        .list_with_counts()
        .await
        .map_err(|err| map_tag_error("infra::http::admin::posts::sections::load_tag_counts", err))
}

pub(super) async fn load_selected_tags(
    state: &AdminState,
    post_id: Uuid,
) -> Result<Vec<crate::domain::entities::TagRecord>, HttpError> {
    state.tags.list_for_post(post_id).await.map_err(|err| {
        map_tag_error(
            "infra::http::admin::posts::sections::load_selected_tags",
            err,
        )
    })
}

fn map_tag_error(source: &'static str, err: AdminTagError) -> HttpError {
    HttpError::new(
        source,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Failed to load tag data",
        err.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn build_tag_picker_view_retains_all_selected_tags() {
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let third = Uuid::new_v4();

        let tags = vec![
            TagWithCount {
                id: first,
                slug: "rust".into(),
                name: "Rust".into(),
                pinned: false,
                count: 12,
            },
            TagWithCount {
                id: second,
                slug: "web".into(),
                name: "Web".into(),
                pinned: false,
                count: 8,
            },
            TagWithCount {
                id: third,
                slug: "systems".into(),
                name: "Systems".into(),
                pinned: false,
                count: 3,
            },
        ];

        let selected = vec![first, second];

        let view = build_tag_picker_view(Some(Uuid::new_v4()), &tags, &selected);

        assert_eq!(view.selected.len(), 2);
        assert_eq!(view.selected_tag_ids.len(), 2);
        assert!(
            view.selected
                .iter()
                .any(|tag| tag.slug == "rust" && tag.id == first.to_string())
        );
        assert!(
            view.selected
                .iter()
                .any(|tag| tag.slug == "web" && tag.id == second.to_string())
        );
    }

    #[test]
    fn build_tag_picker_view_prioritises_pinned_tags() {
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();

        let tags = vec![
            TagWithCount {
                id: first,
                slug: "alpha".into(),
                name: "Alpha".into(),
                pinned: false,
                count: 1,
            },
            TagWithCount {
                id: second,
                slug: "beta".into(),
                name: "Beta".into(),
                pinned: true,
                count: 10,
            },
        ];

        let view = build_tag_picker_view(Some(Uuid::new_v4()), &tags, &[]);
        let slugs: Vec<_> = view
            .options
            .iter()
            .map(|option| option.slug.as_str())
            .collect();

        assert_eq!(slugs, vec!["beta", "alpha"]);
    }

    #[test]
    fn build_tag_picker_view_includes_all_tags() {
        let ids: Vec<_> = (0..5).map(|_| Uuid::new_v4()).collect();
        let tags: Vec<_> = ids
            .iter()
            .enumerate()
            .map(|(index, id)| TagWithCount {
                id: *id,
                slug: format!("tag-{index}"),
                name: format!("Tag {index}"),
                pinned: index == 2,
                count: index as i64,
            })
            .collect();

        let view = build_tag_picker_view(Some(Uuid::new_v4()), &tags, &[]);

        assert_eq!(view.options.len(), tags.len());
        let option_ids: HashSet<_> = view
            .options
            .iter()
            .map(|option| option.id.clone())
            .collect();
        for id in ids {
            assert!(option_ids.contains(&id.to_string()));
        }
    }
}
