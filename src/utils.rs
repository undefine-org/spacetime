//! Shared utility functions

/// Escape a string for use in JavaScript string literals.
///
/// Handles both single and double quote contexts by escaping both quote types,
/// plus common control characters.
pub fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Unescape a JS string that was previously escaped with `escape_js_string`.
///
/// Processes escape sequences left-to-right so that `\\n` (literal backslash-n)
/// is correctly handled: `\\` → `\`, then the remaining `n` stays as `n`.
pub fn unescape_js_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('\'') => result.push('\''),
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Parse a duration literal to milliseconds.
///
/// Delegates to `syntax::conversions::parse_duration_with_unit`, which is THE
/// duration converter — it knows the full unit set (`ms`, `s`, `us`, `m` for
/// minutes, `h` for hours, `fps`) and is what `%capture_type duration`'s unit
/// grammar is written against.
///
/// This function used to carry its own two-unit implementation (`ms` and `s`
/// only), which was invisible while durations arrived here pre-converted as
/// `CapturedValue::Time`. PLAN-122 made a scalar's captured value its SOURCE
/// TEXT, so `refresh: 5m` began arriving as the string `"5m"` — and the weaker
/// parser returned `None`, silently dropping a polling interval rather than
/// converting it. Two answers to "how long is this?" is one too many.
pub fn parse_time_to_ms(s: &str) -> Option<f64> {
    crate::syntax::conversions::parse_duration_with_unit(s.trim()).map(|(ms, _)| ms as f64)
}

/// Generate a unified diff between original and migrated source
pub fn generate_diff(original: &str, migrated: &str, filename: &str) -> String {
    let mut result = String::new();

    result.push_str(&format!("--- {} (original)\n", filename));
    result.push_str(&format!("+++ {} (migrated)\n", filename));

    let orig_lines: Vec<&str> = original.lines().collect();
    let mig_lines: Vec<&str> = migrated.lines().collect();

    // Simple line-by-line diff
    let mut i = 0;
    let mut j = 0;
    let mut changes = Vec::new();

    while i < orig_lines.len() || j < mig_lines.len() {
        if i < orig_lines.len() && j < mig_lines.len() && orig_lines[i] == mig_lines[j] {
            i += 1;
            j += 1;
        } else {
            // Found a difference
            let mut removed = Vec::new();
            let mut added = Vec::new();

            // Collect removed lines
            while i < orig_lines.len() && (j >= mig_lines.len() || orig_lines[i] != mig_lines[j]) {
                removed.push(orig_lines[i]);
                i += 1;
                if i >= orig_lines.len() || (j < mig_lines.len() && orig_lines[i] == mig_lines[j]) {
                    break;
                }
            }

            // Collect added lines
            while j < mig_lines.len() && (i >= orig_lines.len() || mig_lines[j] != orig_lines[i]) {
                added.push(mig_lines[j]);
                j += 1;
                if j >= mig_lines.len() || (i < orig_lines.len() && mig_lines[j] == orig_lines[i]) {
                    break;
                }
            }

            if !removed.is_empty() || !added.is_empty() {
                changes.push((i, removed, added));
            }
        }
    }

    // Format changes
    for (line_num, removed, added) in changes {
        result.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            line_num.saturating_sub(removed.len()),
            removed.len(),
            line_num.saturating_sub(added.len()),
            added.len()
        ));

        for line in removed {
            result.push_str(&format!("-{}\n", line));
        }

        for line in added {
            result.push_str(&format!("+{}\n", line));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_js_string() {
        assert_eq!(escape_js_string("hello"), "hello");
        assert_eq!(escape_js_string("he\"llo"), "he\\\"llo");
        assert_eq!(escape_js_string("he'llo"), "he\\'llo");
        assert_eq!(escape_js_string("he\\llo"), "he\\\\llo");
        assert_eq!(escape_js_string("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_js_string("tab\there"), "tab\\there");
    }

    #[test]
    fn test_unescape_js_string() {
        assert_eq!(unescape_js_string("hello"), "hello");
        assert_eq!(unescape_js_string("he\\\"llo"), "he\"llo");
        assert_eq!(unescape_js_string("he\\'llo"), "he'llo");
        assert_eq!(unescape_js_string("he\\\\llo"), "he\\llo");
        assert_eq!(unescape_js_string("line1\\nline2"), "line1\nline2");
        assert_eq!(unescape_js_string("tab\\there"), "tab\there");
    }

    #[test]
    fn test_escape_unescape_roundtrip() {
        let cases = [
            "hello world",
            "quotes: \"double\" and 'single'",
            "newline\nand\ttab",
            "backslash: \\ end",
            "<a href=\"#work\">Work</a>\n<a href=\"#growth\">Growth</a>",
        ];
        for original in &cases {
            let escaped = escape_js_string(original);
            let unescaped = unescape_js_string(&escaped);
            assert_eq!(&unescaped, original, "Round-trip failed for {:?}", original);
        }
    }

    #[test]
    fn test_unescape_literal_backslash_n() {
        // "\\n" in source = backslash then n → should unescape to backslash then n? No!
        // Actually, "\\n" is two JS escape sequences: \\ (backslash) and then literal n
        // So unescape("\\\\n") → backslash + n (literal)
        assert_eq!(unescape_js_string("\\\\n"), "\\n");
        // While "\\n" → newline
        assert_eq!(unescape_js_string("\\n"), "\n");
    }
}
