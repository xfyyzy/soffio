//! Form definitions for settings admin handlers.

use chrono_tz::Tz;
use serde::Deserialize;
use thiserror::Error;

use crate::application::admin::settings::UpdateSettingsCommand;
use crate::presentation::admin::views as admin_views;

use super::validators::{parse_positive_i32, validate_favicon_svg};
use super::views::{EditFieldValues, build_edit_view};

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AdminSettingsForm {
    pub(super) homepage_size: String,
    pub(super) admin_page_size: String,
    pub(super) show_tag_aggregations: Option<String>,
    pub(super) show_month_aggregations: Option<String>,
    pub(super) tag_filter_limit: String,
    pub(super) month_filter_limit: String,
    pub(super) global_toc_enabled: Option<String>,
    pub(super) brand_title: String,
    pub(super) brand_href: String,
    pub(super) footer_copy: String,
    pub(super) public_site_url: String,
    pub(super) favicon_svg: String,
    pub(super) timezone: String,
    pub(super) meta_title: String,
    pub(super) meta_description: String,
    pub(super) og_title: String,
    pub(super) og_description: String,
}

#[derive(Debug, Error)]
pub(super) enum AdminSettingsFormError {
    #[error("`{field}` must be a positive integer")]
    InvalidInteger { field: &'static str },
    #[error("`{field}` must be greater than zero")]
    NonPositive { field: &'static str },
    #[error("`{value}` is not a recognised timezone")]
    InvalidTimezone { value: String },
    #[error("`{field}` must be an SVG document")]
    InvalidFavicon {
        field: &'static str,
        reason: &'static str,
    },
}

impl AdminSettingsForm {
    pub(super) const MAX_FAVICON_SVG_LENGTH: usize = 8 * 1024;

    pub(super) fn to_command(&self) -> Result<UpdateSettingsCommand, AdminSettingsFormError> {
        let homepage_size = parse_positive_i32(self.homepage_size.trim(), "homepage_size")?;
        let admin_page_size = parse_positive_i32(self.admin_page_size.trim(), "admin_page_size")?;
        let tag_filter_limit =
            parse_positive_i32(self.tag_filter_limit.trim(), "tag_filter_limit")?;
        let month_filter_limit =
            parse_positive_i32(self.month_filter_limit.trim(), "month_filter_limit")?;

        let timezone = self.timezone.trim().parse::<Tz>().map_err(|_| {
            AdminSettingsFormError::InvalidTimezone {
                value: self.timezone.trim().to_string(),
            }
        })?;

        let favicon_svg = self.favicon_svg.trim();
        validate_favicon_svg(favicon_svg)?;

        Ok(UpdateSettingsCommand {
            homepage_size,
            admin_page_size,
            show_tag_aggregations: self.show_tag_aggregations.is_some(),
            show_month_aggregations: self.show_month_aggregations.is_some(),
            tag_filter_limit,
            month_filter_limit,
            global_toc_enabled: self.global_toc_enabled.is_some(),
            brand_title: self.brand_title.trim().to_string(),
            brand_href: self.brand_href.trim().to_string(),
            footer_copy: self.footer_copy.trim().to_string(),
            public_site_url: self.public_site_url.trim().to_string(),
            favicon_svg: favicon_svg.to_string(),
            timezone,
            meta_title: self.meta_title.trim().to_string(),
            meta_description: self.meta_description.trim().to_string(),
            og_title: self.og_title.trim().to_string(),
            og_description: self.og_description.trim().to_string(),
        })
    }

    pub(super) fn to_edit_view(&self, updated_at: String) -> admin_views::AdminSettingsEditView {
        build_edit_view(EditFieldValues {
            homepage_size: self.homepage_size.trim().to_string(),
            admin_page_size: self.admin_page_size.trim().to_string(),
            tag_filter_limit: self.tag_filter_limit.trim().to_string(),
            month_filter_limit: self.month_filter_limit.trim().to_string(),
            timezone: self.timezone.trim().to_string(),
            show_tag_aggregations: self.show_tag_aggregations.is_some(),
            show_month_aggregations: self.show_month_aggregations.is_some(),
            global_toc_enabled: self.global_toc_enabled.is_some(),
            brand_title: self.brand_title.trim().to_string(),
            brand_href: self.brand_href.trim().to_string(),
            footer_copy: self.footer_copy.trim().to_string(),
            public_site_url: self.public_site_url.trim().to_string(),
            favicon_svg: self.favicon_svg.trim().to_string(),
            meta_title: self.meta_title.trim().to_string(),
            meta_description: self.meta_description.trim().to_string(),
            og_title: self.og_title.trim().to_string(),
            og_description: self.og_description.trim().to_string(),
            updated_at,
        })
    }
}
