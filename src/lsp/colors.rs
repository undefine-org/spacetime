//! Document color provider for color picker support.
//!
//! Provides LSP color features for hex color literals like #fff, #ff0088, #rgba.

use tower_lsp::lsp_types::{
    Color, ColorInformation, ColorPresentation, ColorPresentationParams, Range,
};

use super::document::DocumentState;

/// Find all color literals in a document.
/// Report every colour literal in the document so the editor can draw a swatch.
///
/// # One parser, not four (FUP-164 / PLAN-122)
///
/// This used to byte-scan for `#`, then `rgba(`, then `rgb(`, with a hand-written
/// parser behind each — three partial answers to "what is a colour", none of
/// which agreed with the compiler's. `oklch()`, `hsl()`, `color-mix()` and every
/// named colour got NO swatch, because nobody had written a fourth scanner.
///
/// Now the candidate is found positionally and handed to lightningcss, which is
/// the same parser the declaration validator uses. Coverage becomes whatever CSS
/// defines rather than whatever we remembered to implement, and it grows when the
/// dependency is bumped rather than when someone notices a gap.
pub fn provide_document_colors(doc: &DocumentState) -> Vec<ColorInformation> {
    let content = &doc.content;
    let mapper = &doc.position_mapper;
    let bytes = content.as_bytes();
    let mut colors = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        // gh-33: a `#hex`/colour-function-looking string inside a comment or a
        // string literal is PROSE, not a colour. Skip those regions wholesale so
        // the position scan below never reports them. (A scanner that judged
        // from the characters alone would colour `#abc` in "use the #abc hex"
        // and mislead the author.)
        match skip_prose_region(bytes, i) {
            Some(next) => {
                i = next;
                continue;
            }
            None => {}
        }

        let Some((start, end)) = colour_candidate_at(content, bytes, i) else {
            i += 1;
            continue;
        };
        if let Some(color) = parse_css_color(&content[start..end]) {
            colors.push(ColorInformation {
                range: Range {
                    start: mapper.position_from_offset(start),
                    end: mapper.position_from_offset(end),
                },
                color,
            });
            i = end;
        } else {
            i = start + 1;
        }
    }

    colors
}

/// If `bytes[i]` begins a `//` line comment, a `/* … */` block comment, or a
/// `"…"` string literal, return the byte index just past that region; otherwise
/// `None`. This keeps comment/string text out of colour detection (gh-33).
fn skip_prose_region(bytes: &[u8], i: usize) -> Option<usize> {
    // `//` line comment → skip to (but not past) the newline.
    if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
        let mut j = i + 2;
        while j < bytes.len() && bytes[j] != b'\n' {
            j += 1;
        }
        return Some(j);
    }
    // `/* … */` block comment.
    if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
        let mut j = i + 2;
        while j + 1 < bytes.len() && !(bytes[j] == b'*' && bytes[j + 1] == b'/') {
            j += 1;
        }
        return Some((j + 2).min(bytes.len()));
    }
    // `"…"` string literal (backslash escapes a quote).
    if bytes[i] == b'"' {
        let mut j = i + 1;
        while j < bytes.len() {
            if bytes[j] == b'\\' && j + 1 < bytes.len() {
                j += 2;
            } else if bytes[j] == b'"' {
                return Some(j + 1);
            } else {
                j += 1;
            }
        }
        return Some(bytes.len());
    }
    None
}

/// The byte range of a possible colour beginning at `i`, or `None`.
///
/// POSITION only — this decides where a candidate STARTS and ENDS, never whether
/// it is a colour. That question belongs to the parser, which is the whole point:
/// a scanner that also judges is a second answer waiting to drift.
fn colour_candidate_at(content: &str, bytes: &[u8], i: usize) -> Option<(usize, usize)> {
    // `#` followed by hex digits.
    if bytes[i] == b'#' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_hexdigit() {
        let mut end = i + 1;
        while end < bytes.len() && bytes[end].is_ascii_hexdigit() {
            end += 1;
        }
        return Some((i, end));
    }

    // A colour FUNCTION: an identifier immediately followed by `(`. The name is
    // not checked against a list — an unknown function simply fails to parse,
    // which is one place fewer for a list to go stale.
    if bytes[i].is_ascii_alphabetic() && (i == 0 || !is_ident_byte(bytes[i - 1])) {
        let mut name_end = i;
        while name_end < bytes.len() && is_ident_byte(bytes[name_end]) {
            name_end += 1;
        }
        if name_end < bytes.len() && bytes[name_end] == b'(' {
            // Balance the parens so nested functions stay whole
            // (`color-mix(in oklch, rgb(1 2 3), #fff)`).
            let mut depth = 0i32;
            for (offset, b) in content[name_end..].bytes().enumerate() {
                match b {
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some((i, name_end + offset + 1));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    None
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// Parse any CSS colour into an LSP `Color`, via the compiler's own parser.
///
/// Named colours are deliberately NOT reported: `red` is a valid colour, but so
/// is the word `red` in a class name, a comment, or a string, and a swatch on
/// prose is worse than no swatch. Hex and function forms are unambiguous.
fn parse_css_color(text: &str) -> Option<Color> {
    // FEAT-109 Tier 0 — see `crate::color::parse_color_to_normalized_rgba`. This
    // was a verbatim twin of `src/analysis/visual_lint.rs::parse_css_color`,
    // differing only in the float width of the colour it built.
    let [r, g, b, a] = crate::color::parse_color_to_normalized_rgba(text).ok()?;
    Some(Color {
        red: r as f32,
        green: g as f32,
        blue: b as f32,
        alpha: a as f32,
    })
}

pub fn provide_color_presentations(params: &ColorPresentationParams) -> Vec<ColorPresentation> {
    let color = &params.color;
    let r = (color.red * 255.0).round() as u8;
    let g = (color.green * 255.0).round() as u8;
    let b = (color.blue * 255.0).round() as u8;
    let a = (color.alpha * 255.0).round() as u8;

    let mut presentations = Vec::new();

    // 6-digit hex (if fully opaque)
    if a == 255 {
        presentations.push(ColorPresentation {
            label: format!("#{:02x}{:02x}{:02x}", r, g, b),
            text_edit: None,
            additional_text_edits: None,
        });

        // 3-digit shorthand if possible
        if r.is_multiple_of(17) && g.is_multiple_of(17) && b.is_multiple_of(17) {
            presentations.push(ColorPresentation {
                label: format!("#{:x}{:x}{:x}", r / 17, g / 17, b / 17),
                text_edit: None,
                additional_text_edits: None,
            });
        }
    } else {
        // 8-digit hex with alpha
        presentations.push(ColorPresentation {
            label: format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a),
            text_edit: None,
            additional_text_edits: None,
        });

        // 4-digit shorthand if possible
        if r.is_multiple_of(17)
            && g.is_multiple_of(17)
            && b.is_multiple_of(17)
            && a.is_multiple_of(17)
        {
            presentations.push(ColorPresentation {
                label: format!("#{:x}{:x}{:x}{:x}", r / 17, g / 17, b / 17, a / 17),
                text_edit: None,
                additional_text_edits: None,
            });
        }
    }

    presentations
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(content: &str) -> DocumentState {
        DocumentState::new(content.to_string(), 0)
    }

    /// The swatches a document reports, as `(text, alpha)` pairs.
    fn swatches(content: &str) -> Vec<(String, f32)> {
        let d = doc(content);
        provide_document_colors(&d)
            .into_iter()
            .map(|c| {
                let start = d.position_mapper.offset_from_position(c.range.start);
                let end = d.position_mapper.offset_from_position(c.range.end);
                (content[start..end].to_string(), c.color.alpha)
            })
            .collect()
    }

    /// The permitted hex lengths are 3/4/6/8 — the same DATA the grammar uses
    /// (`{3|4|6|8}`), not a "3 or 6" rule that would drop the alpha forms.
    #[test]
    fn color_hex_in_comment_yields_no_swatch() {
        // A color-looking hex inside a comment must NOT get a swatch.
        let s = swatches("a { /* the #abc color */ color: red; }");
        assert!(s.is_empty(), "hex inside comment got a swatch: {s:?}");
    }

    #[test]
    fn color_hex_in_string_yields_no_swatch() {
        let s = swatches("a { content: \"use #abc here\"; }");
        assert!(s.is_empty(), "hex inside string got a swatch: {s:?}");
    }

    #[test]
    fn real_css_hex_still_gets_a_swatch() {
        let s = swatches("a { color: #123; }");
        assert_eq!(s.len(), 1, "real CSS hex must get a swatch: {s:?}");
    }

    #[test]
    fn every_legal_hex_length_gets_a_swatch() {
        for hex in ["#fff", "#fffa", "#ff0088", "#ff008880"] {
            let src = format!("a {{ color: {hex}; }}");
            assert_eq!(
                swatches(&src).len(),
                1,
                "{hex} should produce exactly one swatch"
            );
        }
    }

    #[test]
    fn an_illegal_hex_length_gets_no_swatch() {
        for hex in ["#ff", "#fffff", "#fffffff"] {
            let src = format!("a {{ color: {hex}; }}");
            assert!(
                swatches(&src).is_empty(),
                "{hex} is not a colour and must not be given a swatch"
            );
        }
    }

    #[test]
    fn alpha_is_carried_through() {
        let s = swatches("a { color: #ff008880; }");
        assert_eq!(s.len(), 1);
        assert!(
            (s[0].1 - 0.502).abs() < 0.01,
            "8-digit hex carries its alpha, got {}",
            s[0].1
        );
    }

    /// THE POINT OF THE CUTOVER. Each of these was invisible to the editor
    /// before, because the old scanner knew only `#`, `rgb(` and `rgba(` — three
    /// hand-written parsers, and no fourth for anything else.
    #[test]
    fn colour_functions_the_old_scanner_could_not_see() {
        for value in [
            "rgb(255, 0, 136)",
            "rgba(255, 0, 136, 0.5)",
            "hsl(210, 50%, 40%)",
            "oklch(70% 0.1 200)",
            "color-mix(in oklch, #fff, #000)",
        ] {
            let src = format!("a {{ color: {value}; }}");
            assert_eq!(
                swatches(&src).len(),
                1,
                "{value} should produce exactly one swatch"
            );
        }
    }

    /// A nested function is ONE colour, not two — the range must cover the whole
    /// expression or the editor draws a swatch over half of it.
    #[test]
    fn a_nested_colour_function_is_one_swatch_covering_all_of_it() {
        let src = "a { color: color-mix(in oklch, rgb(1 2 3), #fff); }";
        let s = swatches(src);
        assert_eq!(s.len(), 1, "expected one swatch, got {s:?}");
        assert_eq!(s[0].0, "color-mix(in oklch, rgb(1 2 3), #fff)");
    }

    /// A named colour is a valid CSS colour, but `red` is also a plausible class
    /// name, comment word or string. A swatch on prose is worse than none.
    #[test]
    fn a_bare_word_is_never_given_a_swatch() {
        assert!(swatches("a { color: red; }").is_empty());
        assert!(swatches(".red-banner { padding: 4px; }").is_empty());
        assert!(swatches("// the red one\n").is_empty());
    }

    #[test]
    fn test_color_presentations_opaque() {
        let params = ColorPresentationParams {
            text_document: tower_lsp::lsp_types::TextDocumentIdentifier {
                uri: "file:///test.st".parse().unwrap(),
            },
            color: Color {
                red: 1.0,
                green: 0.0,
                blue: 0.533,
                alpha: 1.0,
            },
            range: Range::default(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let presentations = provide_color_presentations(&params);
        assert!(!presentations.is_empty());
        assert!(presentations[0].label.starts_with('#'));
    }

    #[test]
    fn test_color_presentations_with_alpha() {
        let params = ColorPresentationParams {
            text_document: tower_lsp::lsp_types::TextDocumentIdentifier {
                uri: "file:///test.st".parse().unwrap(),
            },
            color: Color {
                red: 1.0,
                green: 0.0,
                blue: 0.0,
                alpha: 0.5,
            },
            range: Range::default(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let presentations = provide_color_presentations(&params);
        assert!(!presentations.is_empty());
        // Should have 8-digit hex
        assert!(presentations[0].label.len() == 9); // #rrggbbaa
    }
}
