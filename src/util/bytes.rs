//! Utility helpers for representing byte counts in human-readable form.

/// Format a byte count into IEC units (KiB, MiB, GiB, TiB) with trimmed precision.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut value = bytes as f64;
    let mut unit_index = 0;

    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        return format!("{bytes} {}", UNITS[unit_index]);
    }

    let mut value_str = if value >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    };

    if value_str.contains('.') {
        while value_str.ends_with('0') {
            value_str.pop();
        }
        if value_str.ends_with('.') {
            value_str.pop();
        }
    }

    format!("{value_str} {}", UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn format_bytes_scales_units() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1), "1 B");
        assert_eq!(format_bytes(1024), "1 KiB");
        assert_eq!(format_bytes(10 * 1024), "10 KiB");
        assert_eq!(format_bytes(1536), "1.5 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1 MiB");
        assert_eq!(
            format_bytes((5 * 1024 * 1024 * 1024) + (512 * 1024 * 1024)),
            "5.5 GiB"
        );
    }
}
