use super::*;

pub(super) fn build_post_section_events(nodes: &[PostSectionNode]) -> Vec<PostSectionEvent> {
    let mut events = Vec::new();
    for node in nodes {
        append_section_events(node, &mut events);
    }
    events
}

fn append_section_events(node: &PostSectionNode, events: &mut Vec<PostSectionEvent>) {
    events.push(PostSectionEvent::StartSection {
        anchor: node.anchor_slug.clone(),
        level: node.level,
        heading_html: node.heading_html.clone(),
        body_html: node.body_html.clone(),
    });

    if !node.children.is_empty() {
        events.push(PostSectionEvent::StartChildren);
        for child in &node.children {
            append_section_events(child, events);
        }
        events.push(PostSectionEvent::EndChildren);
    }

    events.push(PostSectionEvent::EndSection);
}

pub(super) fn build_post_toc_view(nodes: &[PostSectionNode]) -> Option<PostTocView> {
    if nodes.is_empty() {
        return None;
    }

    let mut events = Vec::new();
    append_toc_events(nodes, &mut events);
    Some(PostTocView { events })
}

fn append_toc_events(nodes: &[PostSectionNode], events: &mut Vec<PostTocEvent>) {
    events.push(PostTocEvent::StartList);

    for node in nodes {
        let title = node.heading_text.trim().to_string();
        events.push(PostTocEvent::StartItem {
            anchor: node.anchor_slug.clone(),
            title,
            level: node.level,
        });

        if !node.children.is_empty() {
            append_toc_events(&node.children, events);
        }

        events.push(PostTocEvent::EndItem);
    }

    events.push(PostTocEvent::EndList);
}
