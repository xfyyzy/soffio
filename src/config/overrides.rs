use super::cli::{DatabaseOverride, RenderAllOverrides, RenderOverrides, ServeOverrides};
use super::loading::RawSettings;

impl RawSettings {
    pub(super) fn apply_serve_overrides(&mut self, overrides: &ServeOverrides) {
        if let Some(host) = overrides.server_host.as_ref() {
            self.server.host = Some(host.clone());
        }
        if let Some(host) = overrides.server_admin_host.as_ref() {
            self.server.admin_host = Some(host.clone());
        }
        if let Some(port) = overrides.public_port {
            self.server.public_port = Some(port);
        }
        if let Some(port) = overrides.admin_port {
            self.server.admin_port = Some(port);
        }
        if let Some(seconds) = overrides.server_graceful_shutdown_seconds {
            self.server.graceful_shutdown_seconds = Some(seconds);
        }
        if let Some(level) = overrides.log_level.as_ref() {
            self.logging.level = Some(level.clone());
        }
        if let Some(json) = overrides.log_json {
            self.logging.json = Some(json);
        }
        if let Some(url) = overrides.database_url.as_ref() {
            self.database.url = Some(url.clone());
        }
        if let Some(max) = overrides.database_http_max_connections {
            self.database.http_max_connections = Some(max);
        }
        if let Some(max) = overrides.database_jobs_max_connections {
            self.database.jobs_max_connections = Some(max);
        }
        if let Some(directory) = overrides.uploads_directory.as_ref() {
            self.uploads.directory = Some(directory.clone());
        }
        if let Some(limit) = overrides.uploads_max_request_bytes {
            self.uploads.max_request_bytes = Some(limit);
        }
        if let Some(window) = overrides.rate_limit_window_seconds {
            self.rate_limit.window_seconds = Some(window);
        }
        if let Some(max) = overrides.rate_limit_max_requests {
            self.rate_limit.max_requests = Some(max);
        }
        if let Some(window) = overrides.api_rate_limit_window_seconds {
            self.api_rate_limit.window_seconds = Some(window);
        }
        if let Some(max) = overrides.api_rate_limit_max_requests {
            self.api_rate_limit.max_requests = Some(max);
        }
        if let Some(cadence) = overrides.scheduler_cadence_seconds {
            self.scheduler.cadence_seconds = Some(cadence);
        }
        if let Some(value) = overrides.jobs_render_post_concurrency {
            self.jobs.render_post_concurrency = Some(value);
        }
        if let Some(value) = overrides.jobs_render_summary_concurrency {
            self.jobs.render_summary_concurrency = Some(value);
        }
        if let Some(value) = overrides.jobs_render_page_concurrency {
            self.jobs.render_page_concurrency = Some(value);
        }
        if let Some(value) = overrides.jobs_publish_post_concurrency {
            self.jobs.publish_post_concurrency = Some(value);
        }
        if let Some(value) = overrides.jobs_publish_page_concurrency {
            self.jobs.publish_page_concurrency = Some(value);
        }

        self.apply_render_overrides(&overrides.render);
        self.apply_cache_overrides(overrides);
    }

    fn apply_cache_overrides(&mut self, overrides: &ServeOverrides) {
        if let Some(v) = overrides.cache_enable_l0_cache {
            self.cache.enable_l0_cache = Some(v);
        }
        if let Some(v) = overrides.cache_enable_l1_cache {
            self.cache.enable_l1_cache = Some(v);
        }
        if let Some(v) = overrides.cache_l0_post_limit {
            self.cache.l0_post_limit = Some(v);
        }
        if let Some(v) = overrides.cache_l0_page_limit {
            self.cache.l0_page_limit = Some(v);
        }
        if let Some(v) = overrides.cache_l0_api_key_limit {
            self.cache.l0_api_key_limit = Some(v);
        }
        if let Some(v) = overrides.cache_l0_post_list_limit {
            self.cache.l0_post_list_limit = Some(v);
        }
        if let Some(v) = overrides.cache_l1_response_limit {
            self.cache.l1_response_limit = Some(v);
        }
        if let Some(v) = overrides.cache_l1_response_body_limit_bytes {
            self.cache.l1_response_body_limit_bytes = Some(v);
        }
        if let Some(v) = overrides.cache_auto_consume_interval_ms {
            self.cache.auto_consume_interval_ms = Some(v);
        }
        if let Some(v) = overrides.cache_consume_batch_limit {
            self.cache.consume_batch_limit = Some(v);
        }
        if let Some(v) = overrides.cache_max_event_queue_len {
            self.cache.max_event_queue_len = Some(v);
        }
    }

    pub(super) fn apply_renderall_overrides(&mut self, overrides: &RenderAllOverrides) {
        self.apply_database_override(&overrides.database);
        self.apply_render_overrides(&overrides.render);
    }

    pub(super) fn apply_database_override(&mut self, overrides: &DatabaseOverride) {
        if let Some(url) = overrides.database_url.as_ref() {
            self.database.url = Some(url.clone());
        }
    }

    fn apply_render_overrides(&mut self, overrides: &RenderOverrides) {
        if let Some(path) = overrides.mermaid_cli_path.as_ref() {
            self.render.mermaid_cli_path = Some(path.clone());
        }
        if let Some(dir) = overrides.mermaid_cache_dir.as_ref() {
            self.render.mermaid_cache_dir = Some(dir.clone());
        }
    }
}
