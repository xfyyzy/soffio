use std::sync::OnceLock;

use url::Url;

use super::pages::{PageId, Slug};

#[derive(Clone, Debug)]
pub struct Navigation {
    entries: Vec<NavEntry>,
}

impl Navigation {
    pub fn entries(&self) -> &[NavEntry] {
        &self.entries
    }

    pub fn mock() -> Self {
        let mut entries = vec![
            NavEntry {
                order: 10,
                label: "About".to_string(),
                destination: NavDestination::Internal {
                    slug: Slug::new("about").expect("valid about slug"),
                    page_id: PageId::new("about"),
                },
            },
            NavEntry {
                order: 15,
                label: "Systems Handbook".to_string(),
                destination: NavDestination::Internal {
                    slug: Slug::new("systems-handbook").expect("valid systems-handbook slug"),
                    page_id: PageId::new("systems-handbook"),
                },
            },
            NavEntry {
                order: 20,
                label: "GitHub".to_string(),
                destination: NavDestination::External {
                    url: Url::parse("https://github.com/soffio").expect("valid url"),
                    target: LinkTarget::Blank,
                },
            },
            NavEntry {
                order: 30,
                label: "Playbook".to_string(),
                destination: NavDestination::External {
                    url: Url::parse("https://soffio.dev/playbook").expect("valid url"),
                    target: LinkTarget::Self_,
                },
            },
        ];

        entries.sort_by(|lhs, rhs| {
            lhs.order
                .cmp(&rhs.order)
                .then_with(|| lhs.label.cmp(&rhs.label))
        });

        Self { entries }
    }
}

static NAVIGATION: OnceLock<Navigation> = OnceLock::new();

pub fn navigation() -> &'static Navigation {
    NAVIGATION.get_or_init(Navigation::mock)
}

#[derive(Clone, Debug)]
pub struct NavEntry {
    pub order: u16,
    pub label: String,
    pub destination: NavDestination,
}

#[derive(Clone, Debug)]
pub enum NavDestination {
    Internal { slug: Slug, page_id: PageId },
    External { url: Url, target: LinkTarget },
}

#[derive(Clone, Debug)]
pub enum LinkTarget {
    Self_,
    Blank,
}

impl LinkTarget {
    pub fn as_html_target(&self) -> &'static str {
        match self {
            LinkTarget::Self_ => "_self",
            LinkTarget::Blank => "_blank",
        }
    }

    pub fn rel_attribute(&self) -> Option<&'static str> {
        match self {
            LinkTarget::Self_ => None,
            LinkTarget::Blank => Some("noopener noreferrer"),
        }
    }
}
