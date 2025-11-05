use std::collections::HashMap;

use thiserror::Error;
use uuid::Uuid;

use crate::domain::entities::PostSectionRecord;

pub const MAX_SECTION_DEPTH: u8 = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct PostSectionNode {
    pub id: Uuid,
    pub anchor_slug: String,
    pub heading_html: String,
    pub heading_text: String,
    pub body_html: String,
    pub level: u8,
    pub position: u32,
    pub contains_code: bool,
    pub contains_math: bool,
    pub contains_mermaid: bool,
    pub children: Vec<PostSectionNode>,
}

impl PostSectionNode {
    pub fn any_contains_code(nodes: &[PostSectionNode]) -> bool {
        nodes
            .iter()
            .any(|node| node.contains_code || PostSectionNode::any_contains_code(&node.children))
    }

    pub fn any_contains_math(nodes: &[PostSectionNode]) -> bool {
        nodes
            .iter()
            .any(|node| node.contains_math || PostSectionNode::any_contains_math(&node.children))
    }

    pub fn any_contains_mermaid(nodes: &[PostSectionNode]) -> bool {
        nodes.iter().any(|node| {
            node.contains_mermaid || PostSectionNode::any_contains_mermaid(&node.children)
        })
    }
}

#[derive(Debug, Error)]
pub enum SectionTreeError {
    #[error("section `{id}` uses invalid position `{position}`")]
    InvalidPosition { id: Uuid, position: i32 },
    #[error("section `{id}` uses invalid level `{level}`")]
    InvalidLevel { id: Uuid, level: i16 },
    #[error("section `{id}` references itself as a parent")]
    SelfParent { id: Uuid },
    #[error("section `{child}` references missing parent `{parent}`")]
    MissingParent { child: Uuid, parent: Uuid },
    #[error("section `{id}` exceeds maximum depth {max_depth}")]
    DepthExceeded { id: Uuid, max_depth: u8 },
    #[error("duplicate section id `{id}` detected")]
    DuplicateId { id: Uuid },
    #[error("section `{id}` is disconnected from any root")]
    Disconnected { id: Uuid },
    #[error("section `{id}` could not be materialised while building tree")]
    MissingNode { id: Uuid },
}

pub fn build_section_tree(
    records: Vec<PostSectionRecord>,
) -> Result<Vec<PostSectionNode>, SectionTreeError> {
    let mut nodes: HashMap<Uuid, PostSectionNode> = HashMap::with_capacity(records.len());
    let mut children: HashMap<Option<Uuid>, Vec<Uuid>> = HashMap::new();
    let mut parents: HashMap<Uuid, Option<Uuid>> = HashMap::with_capacity(records.len());

    for record in records {
        if record.parent_id.is_some_and(|parent| parent == record.id) {
            return Err(SectionTreeError::SelfParent { id: record.id });
        }

        let position =
            u32::try_from(record.position).map_err(|_| SectionTreeError::InvalidPosition {
                id: record.id,
                position: record.position,
            })?;

        let level = u8::try_from(record.level).map_err(|_| SectionTreeError::InvalidLevel {
            id: record.id,
            level: record.level,
        })?;

        if nodes.contains_key(&record.id) {
            return Err(SectionTreeError::DuplicateId { id: record.id });
        }

        let node = PostSectionNode {
            id: record.id,
            anchor_slug: record.anchor_slug,
            heading_html: record.heading_html,
            heading_text: record.heading_text,
            body_html: record.body_html,
            level,
            position,
            contains_code: record.contains_code,
            contains_math: record.contains_math,
            contains_mermaid: record.contains_mermaid,
            children: Vec::new(),
        };

        parents.insert(node.id, record.parent_id);
        children.entry(record.parent_id).or_default().push(node.id);
        nodes.insert(node.id, node);
    }

    for (&child_id, parent_id_opt) in &parents {
        match *parent_id_opt {
            Some(parent_id) if !nodes.contains_key(&parent_id) => {
                return Err(SectionTreeError::MissingParent {
                    child: child_id,
                    parent: parent_id,
                });
            }
            _ => {}
        }
    }

    for ids in children.values_mut() {
        ids.sort_by_key(|id| nodes.get(id).map(|node| node.position).unwrap_or(0));
    }

    let mut working_nodes = nodes;
    let mut roots = Vec::new();

    if let Some(root_ids) = children.get(&None) {
        for &root_id in root_ids {
            roots.push(assemble(root_id, 1, &mut working_nodes, &children)?);
        }
    }

    if !working_nodes.is_empty() {
        let id = *working_nodes.keys().next().unwrap();
        let parent = parents.get(&id).copied().flatten().unwrap_or(Uuid::nil());
        if parent.is_nil() {
            return Err(SectionTreeError::Disconnected { id });
        } else {
            return Err(SectionTreeError::MissingParent { child: id, parent });
        }
    }

    Ok(roots)
}

fn assemble(
    id: Uuid,
    depth: u8,
    nodes: &mut HashMap<Uuid, PostSectionNode>,
    children: &HashMap<Option<Uuid>, Vec<Uuid>>,
) -> Result<PostSectionNode, SectionTreeError> {
    if depth > MAX_SECTION_DEPTH {
        return Err(SectionTreeError::DepthExceeded {
            id,
            max_depth: MAX_SECTION_DEPTH,
        });
    }

    let mut node = nodes
        .remove(&id)
        .ok_or(SectionTreeError::MissingNode { id })?;

    if let Some(child_ids) = children.get(&Some(id)) {
        for &child_id in child_ids {
            let child = assemble(child_id, depth + 1, nodes, children)?;
            node.children.push(child);
        }
    }

    Ok(node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    fn make_record(
        id: Uuid,
        parent_id: Option<Uuid>,
        position: i32,
        level: i16,
    ) -> PostSectionRecord {
        PostSectionRecord {
            id,
            post_id: Uuid::nil(),
            position,
            level,
            parent_id,
            heading_html: format!("<h{level}>Heading</h{level}>", level = level),
            heading_text: format!("Heading {level}"),
            body_html: format!("<p>Body {position}</p>"),
            contains_code: false,
            contains_math: false,
            contains_mermaid: false,
            anchor_slug: format!("slug-{position}"),
            created_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn build_tree_assigns_children() {
        let root_id = Uuid::from_u128(1);
        let child_id = Uuid::from_u128(2);
        let records = vec![
            make_record(root_id, None, 1, 1),
            make_record(child_id, Some(root_id), 1, 2),
        ];

        let tree = build_section_tree(records).expect("tree");
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].id, root_id);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].id, child_id);
        assert_eq!(tree[0].children[0].position, 1);
    }

    #[test]
    fn build_tree_rejects_missing_parent() {
        let missing_parent = Uuid::from_u128(42);
        let child_id = Uuid::from_u128(43);
        let records = vec![make_record(child_id, Some(missing_parent), 1, 2)];

        let err = build_section_tree(records).expect_err("missing parent error");
        matches!(
            err,
            SectionTreeError::MissingParent { child, parent }
                if child == child_id && parent == missing_parent
        );
    }

    #[test]
    fn build_tree_rejects_depth_overflow() {
        let mut records = Vec::new();
        let mut parent = None;
        for i in 0..=MAX_SECTION_DEPTH {
            let id = Uuid::from_u128((i + 1) as u128);
            records.push(make_record(id, parent, (i + 1) as i32, (i + 1) as i16));
            parent = Some(id);
        }

        let err = build_section_tree(records).expect_err("depth overflow");
        assert!(matches!(err, SectionTreeError::DepthExceeded { .. }));
    }
}
