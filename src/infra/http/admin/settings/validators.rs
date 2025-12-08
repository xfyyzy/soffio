//! Validation helpers for settings form.

use super::forms::{AdminSettingsForm, AdminSettingsFormError};

pub(super) fn parse_positive_i32(
    value: &str,
    field: &'static str,
) -> Result<i32, AdminSettingsFormError> {
    let parsed = value
        .parse::<i32>()
        .map_err(|_| AdminSettingsFormError::InvalidInteger { field })?;
    if parsed <= 0 {
        return Err(AdminSettingsFormError::NonPositive { field });
    }
    Ok(parsed)
}

pub(super) fn validate_favicon_svg(value: &str) -> Result<(), AdminSettingsFormError> {
    if value.is_empty() {
        return Err(AdminSettingsFormError::InvalidFavicon {
            field: "favicon_svg",
            reason: "cannot be empty",
        });
    }
    if value.len() > AdminSettingsForm::MAX_FAVICON_SVG_LENGTH {
        return Err(AdminSettingsFormError::InvalidFavicon {
            field: "favicon_svg",
            reason: "exceeds maximum length",
        });
    }
    if !value.to_ascii_lowercase().contains("<svg") {
        return Err(AdminSettingsFormError::InvalidFavicon {
            field: "favicon_svg",
            reason: "missing <svg> element",
        });
    }
    if value.to_ascii_lowercase().contains("<script") {
        return Err(AdminSettingsFormError::InvalidFavicon {
            field: "favicon_svg",
            reason: "scripts are not allowed",
        });
    }
    Ok(())
}
