//! Value conversions for the event-based parser extractors.

/// Parse a duration string preserving the unit.
/// Returns (value_in_ms, original_unit)
pub fn parse_duration_with_unit(s: &str) -> Option<(u32, &str)> {
    let s = s.trim();

    // Find where the number ends and unit begins
    let num_end = s
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s.len());
    let (num_str, unit) = s.split_at(num_end);

    let value: f64 = num_str.parse().ok()?;

    let ms = match unit {
        "ms" => value as u32,
        "s" => (value * 1000.0) as u32,
        "us" | "μs" => (value / 1000.0) as u32,
        "m" => (value * 60_000.0) as u32,
        // Hours. A polling `refresh:` is the only place this appears
        // (`refresh: 1h`), and it was accepted by the grammar's unit set while
        // this converter returned None — the unit has to be known in BOTH or a
        // legal duration silently becomes no duration at all.
        "h" => (value * 3_600_000.0) as u32,
        // Frames. A score/film duration is authored in frames (`--dissolve(6f)`),
        // and 60fps is the project's reference rate. Added HERE rather than kept
        // in a second parser: `pipeline::drivers` had its own `parse_duration_ms`
        // that knew `ms`/`s`/`f` and NOTHING else, so `2m` meant 120000ms through
        // this function and 300ms (the fallback) through that one — one literal,
        // two durations, nothing failing. Same shape as GH-11's three filter
        // tables. One parser, every unit it needs.
        "f" => ((value / 60.0) * 1000.0) as u32,
        "fps" => {
            // Convert fps to ms per frame
            if value > 0.0 {
                (1000.0 / value) as u32
            } else {
                0
            }
        }
        _ => return None,
    };

    Some((ms, unit))
}

/// Parse a length value string preserving the unit.
/// Returns (value, unit) for CSS length units like px, %, em, rem, vw, vh, etc.
pub fn parse_length_with_unit(s: &str) -> Option<(f64, &str)> {
    let s = s.trim();

    // Find where the number ends and unit begins
    let num_end = s
        .find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .unwrap_or(s.len());

    // Handle negative numbers: the '-' might be at position 0
    let num_end = if s.starts_with('-') && num_end == 0 {
        s[1..]
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .map(|i| i + 1)
            .unwrap_or(s.len())
    } else {
        num_end
    };

    let (num_str, unit) = s.split_at(num_end);

    let value: f64 = num_str.parse().ok()?;

    // Validate known CSS length units
    let valid_units = [
        "px", "%", "em", "rem", "vw", "vh", "vmin", "vmax", "ch", "ex", "cm", "mm", "in", "pt",
        "pc", "fr", "deg", "rad", "turn", "grad",
    ];

    if valid_units.contains(&unit) {
        Some((value, unit))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_with_unit() {
        assert_eq!(parse_duration_with_unit("500ms"), Some((500, "ms")));
        assert_eq!(parse_duration_with_unit("2s"), Some((2000, "s")));
        assert_eq!(parse_duration_with_unit("1000us"), Some((1, "us")));
        assert_eq!(parse_duration_with_unit("1m"), Some((60000, "m")));
        assert_eq!(parse_duration_with_unit("60fps"), Some((16, "fps"))); // ~16.67ms
        assert_eq!(parse_duration_with_unit("invalid"), None);
    }

    #[test]
    fn test_parse_length_with_unit() {
        assert_eq!(parse_length_with_unit("100px"), Some((100.0, "px")));
        assert_eq!(parse_length_with_unit("50%"), Some((50.0, "%")));
        assert_eq!(parse_length_with_unit("2em"), Some((2.0, "em")));
        assert_eq!(parse_length_with_unit("1.5rem"), Some((1.5, "rem")));
        assert_eq!(parse_length_with_unit("-10px"), Some((-10.0, "px")));
        assert_eq!(parse_length_with_unit("100vw"), Some((100.0, "vw")));
        assert_eq!(parse_length_with_unit("90deg"), Some((90.0, "deg")));
        assert_eq!(parse_length_with_unit("invalid"), None);
        assert_eq!(parse_length_with_unit("100xyz"), None);
    }
}
