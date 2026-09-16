//! Reference extractors — extract $binding, &element, ~preset, and selector references.

use crate::syntax::cst::SyntaxKind;
use crate::syntax::form_match::CapturedValue;

use super::{CaptureExtractor, ExtractResult, TokenData};

/// Extract a binding reference: `$` + IDENT → `$variable`.
/// Also handles dotted paths like `$data.items`.
pub struct BindingExtractor;

impl CaptureExtractor for BindingExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        if tok.kind == SyntaxKind::DOLLAR {
            // Must be followed by IDENT
            let ident = tokens.get(1)?;
            if ident.kind == SyntaxKind::IDENT || ident.kind.is_keyword() {
                let name = ident.text(source);
                // Check for dotted path: $data.items.name
                let mut consumed = 2;
                let mut full_name = format!("${}", name);
                while consumed + 1 < tokens.len()
                    && tokens[consumed].kind == SyntaxKind::DOT
                    && (tokens
                        .get(consumed + 1)
                        .is_some_and(|t| t.kind == SyntaxKind::IDENT || t.kind.is_keyword()))
                {
                    let dot_text = tokens[consumed].text(source);
                    let part_text = tokens[consumed + 1].text(source);
                    full_name.push_str(dot_text);
                    full_name.push_str(part_text);
                    consumed += 2;
                }
                Some((CapturedValue::Binding(full_name), consumed))
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Extract an element reference: `&` + IDENT → `element`.
pub struct ElementExtractor;

impl CaptureExtractor for ElementExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        if tok.kind == SyntaxKind::AMPERSAND {
            let ident = tokens.get(1)?;
            if ident.kind == SyntaxKind::IDENT {
                let name = ident.text(source);
                // FEAT-142 WAVE B: preserve a dotted FACET PATH
                // `&entity.component.facet` in full (mirrors BindingExtractor's
                // `$data.items` handling). The leading `&` is stripped (kept
                // implicit by the Element variant); the path segments are joined
                // with `.` so the Wave-C resolver can split entity/component/facet.
                let mut consumed = 2;
                let mut full_name = name.to_string();
                while consumed + 1 < tokens.len()
                    && tokens[consumed].kind == SyntaxKind::DOT
                    && tokens
                        .get(consumed + 1)
                        .is_some_and(|t| t.kind == SyntaxKind::IDENT || t.kind.is_keyword())
                {
                    let dot_text = tokens[consumed].text(source);
                    let part_text = tokens[consumed + 1].text(source);
                    full_name.push_str(dot_text);
                    full_name.push_str(part_text);
                    consumed += 2;
                }
                Some((CapturedValue::Element(full_name), consumed))
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Extract a preset reference (RETIRED — SIP-001c / BUG-263). The `~` preset
/// prefix is gone: `~name` errors at parse (the retirement diagnostic), so this
/// extractor never runs on parsed input. A `~` today is only CSS's
/// general-sibling combinator (handled by SelectorExtractor). Kept so
/// `CaptureType::Preset` stays a non-matching dormant path instead of silently
/// mis-extracting `a ~ b`.
pub struct PresetExtractor;

impl CaptureExtractor for PresetExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        if tok.kind == SyntaxKind::TILDE {
            let ident = tokens.get(1)?;
            if ident.kind == SyntaxKind::IDENT {
                let name = ident.text(source);
                Some((CapturedValue::Preset(name.to_string()), 2))
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Extract a CSS selector (`.class`, `#id`, `[attr]`, combined selectors).
/// Greedy: consumes tokens that form a valid selector.
pub struct SelectorExtractor;

impl CaptureExtractor for SelectorExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        if tokens.is_empty() {
            return None;
        }

        let first = &tokens[0];
        // Selectors start with `.`, `#`, `[`, `*`, `&`, `:`, CSS combinators
        // (`>`, `+`, `~`), or an element name (IDENT).
        //
        // `&` reached this list twice, from two directions, which is worth
        // recording because each half found a different failure:
        //
        //  - gh-18 (PLAN-137): `&` / `&self` is the SELF selector — a keyframes
        //    scope `& { prop: a -> b; }` targets the element itself, the reveal
        //    macro's own documented body shape. Before that fix, a `&` keyframes
        //    scope was silently DROPPED to an empty object.
        //
        //  - FUP-181 (this arc): the omission was invisible for as long as
        //    `capture_type_accepts` waved every builtin through. The moment W1
        //    made the check real it became eight FALSE REFUSALS across
        //    tests/fixtures/ — `@then &mixInst { … }` reported as
        //    `Expected $target:Selector, got token "AMPERSAND"`.
        //
        // Same omission, one silent and one loud: `&` is the CSS nesting
        // selector AND Spacetime's element-ref sigil, so a rule that refuses it
        // refuses ordinary authoring. A leading `:` is here for the same reason —
        // `:is(.a, .b)` and `:hover` are selectors, and the greedy loop below
        // already knows how to consume them; only this gate said no.
        let is_selector_start = matches!(
            first.kind,
            SyntaxKind::DOT
            | SyntaxKind::HASH
            | SyntaxKind::COLOR  // #id looks like color token
            | SyntaxKind::L_BRACKET
            | SyntaxKind::STAR
            | SyntaxKind::AMPERSAND // & nesting selector / element ref
            | SyntaxKind::COLON  // :is(…), :hover, ::before
            | SyntaxKind::GT     // > child combinator
            | SyntaxKind::PLUS   // + adjacent sibling
            | SyntaxKind::TILDE  // ~ general sibling
        ) || (first.kind == SyntaxKind::IDENT
            && is_html_element(first.text(source)));

        if !is_selector_start {
            return None;
        }

        // A leading `[]` is the COLLECTION MARKER of an element ref
        // (`&cards[] .cards { … }`), not an attribute selector. `L_BRACKET` is a
        // legitimate selector start (`[data-x="y"]`), but an EMPTY bracket pair
        // is not a selector at all — there is no CSS it could mean.
        //
        // Without this, the ident capture takes `cards`, the marker falls to the
        // selector, and the form emits `querySelector("[] .cards")`: invalid CSS
        // that throws `Expected name, found ]` out of the runtime's selector
        // parser, taking the block's `@each` with it. The convention elsewhere is
        // that the marker rides with the NAME (`r.ends_with("[]")` in the
        // template-invoke path), so refusing it here keeps it there.
        if first.kind == SyntaxKind::L_BRACKET
            && tokens
                .get(1)
                .is_some_and(|t| t.kind == SyntaxKind::R_BRACKET)
        {
            return None;
        }

        // `&name(` is a TEMPLATE INVOCATION, not a selector. Admitting `&` above
        // is what makes `&mixInst` and `&.hover` work, but it also made
        // `&card("Hello")` — a template call — look like a selector, and a
        // greedy `$target:selector` then swallowed the call site:
        //
        //     @template &card($label) { … }
        //     .host { &card("Hello"); }   <- consumed as a selector
        //
        // The functional-pseudo handling below deliberately treats a depth-0 `(`
        // as PART of the selector (BUG-065, `:not(.bar)`), so the paren cannot
        // disambiguate later — it has to be refused here, at the head.
        //
        // A selector's `(` only ever follows a pseudo-class (`:is(`), never an
        // element ref, so this costs no real selector.
        if first.kind == SyntaxKind::AMPERSAND {
            let mut rest = tokens[1..]
                .iter()
                .filter(|t| t.kind != SyntaxKind::WHITESPACE);
            if let Some(next) = rest.next()
                && next.kind == SyntaxKind::IDENT
                && rest.next().map(|t| t.kind) == Some(SyntaxKind::L_PAREN)
            {
                return None;
            }
        }

        // Greedily consume selector tokens. `[]` and `()` adjust nesting depth so an attribute
        // selector (`[data-x="y"]`) and a functional pseudo-class (`:not(.bar)`,
        // `:nth-child(2)`, `:is(.a,.b)`, `:has(.z)`) are consumed WHOLE — a depth-0 `(` is part
        // of the selector, not a terminator (BUG-065: truncating there leaked CSS blocks into
        // html / mis-flagged selector refs).
        let mut count = 0;
        let mut depth = 0i32;
        for tok in tokens {
            match tok.kind {
                SyntaxKind::L_BRACKET | SyntaxKind::L_PAREN => {
                    depth += 1;
                    count += 1;
                }
                SyntaxKind::R_BRACKET | SyntaxKind::R_PAREN => {
                    depth -= 1;
                    count += 1;
                }
                // Stop at delimiters that end a selector (only at depth 0).
                SyntaxKind::L_BRACE | SyntaxKind::SEMICOLON if depth == 0 => break,
                SyntaxKind::EOF => break,
                // Continue consuming selector parts. `AMPERSAND` belongs here as
                // well as in the start gate above: without it a selector that
                // BEGINS with `&` is admitted by the gate and then consumes zero
                // tokens, so `count == 0` returns None and the refusal comes back
                // by another route. Both lists describe the same alphabet.
                SyntaxKind::DOT
                | SyntaxKind::HASH
                | SyntaxKind::COLOR
                | SyntaxKind::IDENT
                | SyntaxKind::STAR
                | SyntaxKind::AMPERSAND
                | SyntaxKind::COLON
                | SyntaxKind::GT
                | SyntaxKind::PLUS
                | SyntaxKind::TILDE
                | SyntaxKind::WHITESPACE
                | SyntaxKind::COMMA => {
                    count += 1;
                }
                _ if depth > 0 => {
                    count += 1;
                } // inside [attr=...] or :pseudo(...)
                _ => break,
            }
        }

        if count == 0 {
            return None;
        }

        // Build selector text by concatenating individual token texts.
        // Using source byte ranges would include any gaps (e.g., body text)
        // between tokens that aren't part of the selector.
        let mut text = String::new();
        for tok in &tokens[..count] {
            text.push_str(tok.text(source));
        }
        let text = text.trim();
        Some((CapturedValue::Selector(text.to_string()), count))
    }
}

fn is_html_element(s: &str) -> bool {
    matches!(
        s,
        "div"
            | "span"
            | "p"
            | "a"
            | "button"
            | "input"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "ul"
            | "ol"
            | "li"
            | "table"
            | "tr"
            | "td"
            | "th"
            | "img"
            | "video"
            | "audio"
            | "canvas"
            | "svg"
            | "header"
            | "footer"
            | "nav"
            | "main"
            | "section"
            | "article"
            | "aside"
            | "body"
            | "html"
            | "head"
            | "title"
            | "link"
            | "meta"
            | "style"
            | "script"
            | "label"
            | "select"
            | "option"
            | "textarea"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tok(kind: SyntaxKind, start: usize, end: usize) -> TokenData {
        TokenData {
            kind,
            text_range: (start, end),
        }
    }

    #[test]
    fn binding_simple() {
        let source = "$count";
        let tokens = [tok(SyntaxKind::DOLLAR, 0, 1), tok(SyntaxKind::IDENT, 1, 6)];
        let result = BindingExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::Binding("$count".to_string()), 2))
        );
    }

    #[test]
    fn binding_dotted() {
        let source = "$data.items";
        let tokens = [
            tok(SyntaxKind::DOLLAR, 0, 1),
            tok(SyntaxKind::IDENT, 1, 5),
            tok(SyntaxKind::DOT, 5, 6),
            tok(SyntaxKind::IDENT, 6, 11),
        ];
        let result = BindingExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::Binding("$data.items".to_string()), 4))
        );
    }

    #[test]
    fn element_simple() {
        let source = "&button";
        let tokens = [
            tok(SyntaxKind::AMPERSAND, 0, 1),
            tok(SyntaxKind::IDENT, 1, 7),
        ];
        let result = ElementExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::Element("button".to_string()), 2))
        );
    }

    #[test]
    fn element_facet_path() {
        // FEAT-142 WAVE B: a dotted element ref `&entity.component.facet` must
        // preserve the FULL path in the capture (entity=evernet, component=peak,
        // facet=summit), mirroring how BindingExtractor keeps `$data.items`. The
        // Wave-C resolver splits the path into segments to resolve + type it.
        let source = "&evernet.peak.summit";
        let tokens = [
            tok(SyntaxKind::AMPERSAND, 0, 1),
            tok(SyntaxKind::IDENT, 1, 8), // evernet
            tok(SyntaxKind::DOT, 8, 9),
            tok(SyntaxKind::IDENT, 9, 13), // peak
            tok(SyntaxKind::DOT, 13, 14),
            tok(SyntaxKind::IDENT, 14, 20), // summit
        ];
        let result = ElementExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::Element("evernet.peak.summit".to_string()), 6))
        );
    }

    #[test]
    fn preset_simple() {
        let source = "~ease-out";
        let tokens = [tok(SyntaxKind::TILDE, 0, 1), tok(SyntaxKind::IDENT, 1, 9)];
        let result = PresetExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::Preset("ease-out".to_string()), 2))
        );
    }

    #[test]
    fn selector_class() {
        let source = ".hero";
        let tokens = [tok(SyntaxKind::DOT, 0, 1), tok(SyntaxKind::IDENT, 1, 5)];
        let result = SelectorExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::Selector(".hero".to_string()), 2))
        );
    }

    #[test]
    fn selector_stops_at_brace() {
        let source = ".hero {";
        let tokens = [
            tok(SyntaxKind::DOT, 0, 1),
            tok(SyntaxKind::IDENT, 1, 5),
            tok(SyntaxKind::WHITESPACE, 5, 6),
            tok(SyntaxKind::L_BRACE, 6, 7),
        ];
        let result = SelectorExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::Selector(".hero".to_string()), 3))
        );
    }
}
