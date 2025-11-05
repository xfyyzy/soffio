mod data;

use std::collections::{BTreeMap, BTreeSet};

use time::{Date, format_description::FormatItem, macros::format_description};

pub use data::POSTS;

pub const HUMAN_DATE_FORMAT: &[FormatItem<'static>] =
    format_description!("[month repr:long] [day padding:none], [year]");
pub const MONTH_KEY_FORMAT: &[FormatItem<'static>] =
    format_description!("[year]-[month padding:zero]");
pub const MONTH_LABEL_FORMAT: &[FormatItem<'static>] =
    format_description!("[month repr:long] [year]");

#[derive(Clone)]
pub enum PostBlock {
    Paragraph(&'static str),
    Code {
        language: &'static str,
        code: &'static str,
    },
    List(&'static [&'static str]),
}

#[derive(Clone)]
pub struct PostSection {
    pub id: &'static str,
    pub title: &'static str,
    pub level: u8,
    pub blocks: &'static [PostBlock],
}

#[derive(Clone)]
pub struct Post {
    pub slug: &'static str,
    pub title: &'static str,
    pub excerpt: &'static str,
    pub date: Date,
    pub tags: &'static [&'static str],
    pub summary: Option<&'static [&'static str]>,
    pub sections: &'static [PostSection],
}

#[derive(Clone, Copy)]
pub enum PostFilter<'a> {
    All,
    Tag(&'a str),
    Month(&'a str),
}

#[derive(Clone)]
pub struct MonthCount {
    pub key: String,
    pub label: String,
    pub count: usize,
}

pub fn all() -> &'static [Post] {
    &POSTS
}

pub fn find_by_slug(slug: &str) -> Option<&'static Post> {
    POSTS.iter().find(|post| post.slug == slug)
}

pub fn collect(filter: PostFilter<'_>) -> Vec<&'static Post> {
    let mut posts: Vec<&Post> = POSTS.iter().collect();

    match filter {
        PostFilter::All => {}
        PostFilter::Tag(tag) => posts.retain(|post| post.tags.contains(&tag)),
        PostFilter::Month(month) => {
            posts.retain(|post| month == month_key_for(post.date));
        }
    }

    posts.sort_by(|a, b| b.date.cmp(&a.date));
    posts
}

pub fn visible_limit(posts: &[&Post], cursor: Option<&str>, page_size: usize) -> usize {
    if posts.is_empty() {
        return 0;
    }

    let base = page_size.min(posts.len());
    match cursor {
        None => base,
        Some(slug) => {
            if let Some(index) = posts.iter().position(|post| post.slug == slug) {
                ((index + 1) + page_size).min(posts.len())
            } else {
                base
            }
        }
    }
}

pub fn known_tags() -> BTreeSet<&'static str> {
    POSTS
        .iter()
        .flat_map(|post| post.tags.iter().copied())
        .collect()
}

pub fn is_known_tag(value: &str) -> bool {
    known_tags().contains(value)
}

pub fn compute_month_counts() -> Vec<MonthCount> {
    let mut map: BTreeMap<String, (String, usize)> = BTreeMap::new();

    for post in POSTS.iter() {
        let key = month_key_for(post.date);
        let label = month_label_for(post.date);
        map.entry(key)
            .and_modify(|entry| entry.1 += 1)
            .or_insert((label, 1));
    }

    let mut entries = map.into_iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| right.cmp(left));

    entries
        .into_iter()
        .map(|(key, (label, count))| MonthCount { key, label, count })
        .collect()
}

pub fn is_known_month(value: &str) -> bool {
    compute_month_counts()
        .iter()
        .any(|entry| entry.key == value)
}

pub fn month_key_for(date: Date) -> String {
    date.format(MONTH_KEY_FORMAT).expect("valid month key")
}

pub fn month_label_for(date: Date) -> String {
    date.format(MONTH_LABEL_FORMAT).expect("valid month label")
}

pub fn format_human_date(date: Date) -> String {
    date.format(HUMAN_DATE_FORMAT).expect("valid calendar date")
}

pub fn post_has_code_blocks(post: &Post) -> bool {
    post.sections.iter().any(|section| {
        section
            .blocks
            .iter()
            .any(|block| matches!(block, PostBlock::Code { .. }))
    })
}
