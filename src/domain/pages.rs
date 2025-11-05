use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PageId(String);

impl PageId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Slug(String);

impl Slug {
    pub fn new(value: impl Into<String>) -> Result<Self, SlugError> {
        let raw = value.into();
        if raw.trim().is_empty() {
            return Err(SlugError::Empty);
        }
        if raw.contains('/') || raw.contains(' ') {
            return Err(SlugError::InvalidCharacter);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug)]
pub enum SlugError {
    Empty,
    InvalidCharacter,
}

#[derive(Clone, Debug)]
pub struct Page {
    pub id: PageId,
    pub slug: Slug,
    pub content_html: String,
}

impl Page {
    pub fn new(id: PageId, slug: Slug, content_html: impl Into<String>) -> Self {
        Self {
            id,
            slug,
            content_html: content_html.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PageRepository {
    by_slug: HashMap<String, Page>,
    by_id: HashMap<PageId, String>,
}

impl PageRepository {
    pub fn mock() -> Self {
        let about = Page::new(
            PageId::new("about"),
            Slug::new("about").expect("valid about slug"),
            r#"
              <section data-role="page-section">
                <p>
                  Soffio Blog 记录我们在工具链迭代、工作流调优、协作机制等方面的实践。
                  我们坚持以真实案例拆解问题，关注如何将抽象的工程原则落地成可复用的系统。
                </p>
                <p>
                  架构层面我们偏向最小可行的高内聚组件；流程层面，我们鼓励持续验证和自动化审计；
                  团队层面，我们追求透明与高信任的合作节奏。欢迎与我们交流，或在 GitHub 上关注最新进展。
                </p>
              </section>
            "#,
        );
        let systems_handbook = Page::new(
            PageId::new("systems-handbook"),
            Slug::new("systems-handbook").expect("valid systems-handbook slug"),
            r#"
              <section style="display:flex; flex-direction:column; gap:1.5rem;">
                <header style="background:linear-gradient(135deg, #0b57d0 0%, #304ffe 100%); color:#ffffff; padding:2.5rem 2rem; border-radius:16px;">
                  <h2 style="margin:0; font-size:2.1rem; letter-spacing:-0.01em;">Systems Handbook</h2>
                  <p style="margin:0.75rem 0 0; max-width:46ch; font-size:1.05rem; line-height:1.6;">
                    A living reference for Soffio engineers covering operational guardrails, observability contracts, and rollout playbooks.
                  </p>
                </header>
                <section style="display:grid; grid-template-columns:repeat(auto-fit, minmax(280px, 1fr)); gap:1.5rem;">
                  <article style="border:1px solid #dfe3e8; border-radius:12px; padding:1.5rem; background:#ffffff; box-shadow:0 10px 30px rgba(15, 23, 42, 0.08);">
                    <h3 style="margin:0 0 0.75rem; font-size:1.15rem;">Release Checklist</h3>
                    <ol style="margin:0; padding-left:1.3rem; line-height:1.55; color:#4b5563;">
                      <li>Run targeted regression suite with feature toggles mirrored to production.</li>
                      <li>Verify telemetry schema diffs in staging and publish the schema hash.</li>
                      <li>Schedule rollout window and share incident fallback owner in #launchpad.</li>
                    </ol>
                  </article>
                  <article style="border:1px solid #dfe3e8; border-radius:12px; padding:1.5rem; background:#ffffff; box-shadow:0 10px 30px rgba(15, 23, 42, 0.08);">
                    <h3 style="margin:0 0 0.75rem; font-size:1.15rem;">Incident Response Tiers</h3>
                    <table style="width:100%; border-collapse:collapse; font-size:0.95rem; color:#4b5563;">
                      <thead>
                        <tr>
                          <th style="text-align:left; padding:0.5rem; border-bottom:1px solid #e5e7eb;">Tier</th>
                          <th style="text-align:left; padding:0.5rem; border-bottom:1px solid #e5e7eb;">Trigger</th>
                          <th style="text-align:left; padding:0.5rem; border-bottom:1px solid #e5e7eb;">Response</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr>
                          <td style="padding:0.5rem; border-bottom:1px solid #f3f4f6;">SEV-1</td>
                          <td style="padding:0.5rem; border-bottom:1px solid #f3f4f6;">System wide outage</td>
                          <td style="padding:0.5rem; border-bottom:1px solid #f3f4f6;">Page incident commander, freeze deploy pipeline, enable dark launch chart.</td>
                        </tr>
                        <tr>
                          <td style="padding:0.5rem; border-bottom:1px solid #f3f4f6;">SEV-2</td>
                          <td style="padding:0.5rem; border-bottom:1px solid #f3f4f6;">Degraded core workflows</td>
                          <td style="padding:0.5rem; border-bottom:1px solid #f3f4f6;">Spin up analysis bridge, gather logs, communicate every 30 minutes.</td>
                        </tr>
                        <tr>
                          <td style="padding:0.5rem;">SEV-3</td>
                          <td style="padding:0.5rem;">Isolated regression</td>
                          <td style="padding:0.5rem;">Triage asynchronously, attach remediation proposal within 24 hours.</td>
                        </tr>
                      </tbody>
                    </table>
                  </article>
                  <article style="border:1px solid #dfe3e8; border-radius:12px; padding:1.5rem; background:#ffffff; box-shadow:0 10px 30px rgba(15, 23, 42, 0.08);">
                    <h3 style="margin:0 0 0.75rem; font-size:1.15rem;">Capability Directory</h3>
                    <div style="display:flex; flex-direction:column; gap:0.85rem;">
                      <div style="display:flex; justify-content:space-between; align-items:center;">
                        <span style="font-weight:600; color:#111827;">Build Graph</span>
                        <span style="font-size:0.9rem; color:#6b7280;">Owner: Pipeline Guild</span>
                      </div>
                      <p style="margin:0; color:#4b5563; line-height:1.55;">
                        Maintains the incremental build orchestrator, cache heuristics, and artifact traceability contract.
                      </p>
                      <div style="display:flex; justify-content:space-between; align-items:center;">
                        <span style="font-weight:600; color:#111827;">Observability</span>
                        <span style="font-size:0.9rem; color:#6b7280;">Owner: Signals Team</span>
                      </div>
                      <p style="margin:0; color:#4b5563; line-height:1.55;">
                        Curates unified logging pipeline, alert playbooks, and service level artefacts.
                      </p>
                    </div>
                  </article>
                </section>
                <footer style="display:flex; flex-wrap:wrap; gap:1rem; align-items:center; border:1px solid #dfe3e8; border-radius:12px; padding:1.25rem 1.5rem; background:#f8f8f8;">
                  <span style="font-weight:600; color:#111827;">Need to propose a change?</span>
                  <a href="mailto:handbook@soffio.dev" style="color:#0b57d0; text-decoration:none; font-weight:500;">Email the Handbook maintainers</a>
                  <a href="https://soffio.dev/handbook/changelog" style="color:#0b57d0; text-decoration:none; font-weight:500;">View change log</a>
                </footer>
              </section>
            "#,
        );

        let mut by_slug = HashMap::new();
        let mut by_id = HashMap::new();

        fn insert_page(
            by_slug: &mut HashMap<String, Page>,
            by_id: &mut HashMap<PageId, String>,
            page: Page,
        ) {
            let slug_key = page.slug.as_str().to_string();
            let page_id = page.id.clone();
            by_id.insert(page_id, slug_key.clone());
            by_slug.insert(slug_key, page);
        }

        insert_page(&mut by_slug, &mut by_id, about);
        insert_page(&mut by_slug, &mut by_id, systems_handbook);

        Self { by_slug, by_id }
    }

    pub fn find_by_slug(&self, slug: &str) -> Option<&Page> {
        self.by_slug.get(slug)
    }

    pub fn find_by_id(&self, page_id: &PageId) -> Option<&Page> {
        self.by_id
            .get(page_id)
            .and_then(|slug| self.by_slug.get(slug))
    }
}

static PAGES: OnceLock<PageRepository> = OnceLock::new();

pub fn pages() -> &'static PageRepository {
    PAGES.get_or_init(PageRepository::mock)
}
