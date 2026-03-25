use super::*;

impl CacheConsumer {
    /// Invalidate L0 cache entries based on the plan.
    pub(super) fn invalidate_l0(&self, plan: &ConsumptionPlan) {
        for entity in &plan.invalidate_entities {
            match entity {
                EntityKey::SiteSettings => self.l0.invalidate_site_settings(),
                EntityKey::Navigation => self.l0.invalidate_navigation(),
                EntityKey::Post(id) => {
                    // Try to get the post to know its slug
                    if let Some(post) = self.l0.get_post_by_id(*id) {
                        self.l0.invalidate_post(*id, &post.slug);
                    }
                }
                EntityKey::PostSlug(slug) => {
                    if let Some(post) = self.l0.get_post_by_slug(slug) {
                        self.l0.invalidate_post(post.id, slug);
                    }
                }
                EntityKey::Page(id) => {
                    if let Some(page) = self.l0.get_page_by_id(*id) {
                        self.l0.invalidate_page(*id, &page.slug);
                    }
                }
                EntityKey::PageSlug(slug) => {
                    if let Some(page) = self.l0.get_page_by_slug(slug) {
                        self.l0.invalidate_page(page.id, slug);
                    }
                }
                EntityKey::ApiKey(prefix) => {
                    self.l0.invalidate_api_key(prefix);
                }
                EntityKey::PostsIndex => self.l0.invalidate_all_post_lists(),
                EntityKey::PostAggTags => self.l0.invalidate_tag_counts(),
                EntityKey::PostAggMonths => self.l0.invalidate_month_counts(),
                EntityKey::Feed | EntityKey::Sitemap => {
                    // These are L1-only, handled in invalidate_l1
                }
            }
        }
    }

    /// Invalidate L1 cache entries based on the plan.
    pub(super) fn invalidate_l1(&self, plan: &ConsumptionPlan) {
        for entity in &plan.invalidate_entities {
            let keys = self.registry.keys_for_entity(entity);
            for key in keys {
                if let CacheKey::L1(l1_key) = &key {
                    self.l1.invalidate(l1_key);
                }
                self.registry.unregister(&key);
            }
        }
    }
}
