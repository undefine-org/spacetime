//! Simple extractors — each matches 1-2 tokens and converts to CapturedValue.

use crate::syntax::conversions;
use crate::syntax::cst::SyntaxKind;
use crate::syntax::form_match::{CapturedValue, LengthValue};

use super::{CaptureExtractor, ExtractResult, TokenData};

/// Extract an identifier (IDENT token). Also accepts keywords as identifiers.
pub struct IdentExtractor;

impl CaptureExtractor for IdentExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        if tok.kind == SyntaxKind::IDENT || tok.kind.is_keyword() {
            let text = tok.text(source);
            // `as` is STRUCTURE, never a name. It introduces an alias binding
            // (`@on &.visible(600ms) as $reveal`), handled by the dedicated
            // `alias_capture` path — so a bare ident capture that swallows it
            // is claiming a keyword whose job is to separate two captures.
            //
            // This is BUG-328's rule one token earlier. There: `$name` is an
            // IDENT only when the name ENDS there, because a trailing accessor
            // means the author wrote an expression ABOUT the binding. Here a
            // LEADING `as` means the ident that follows belongs to the alias,
            // so `as` itself is not a name either.
            //
            // BUG-344: without this, a migration `%match` shaped
            // `@on &.visible $name:ident ( … )` matched the CURRENT form
            // `@on &.visible(600ms) as $reveal`, binding `as` as the animation
            // name and dropping `$reveal` entirely. `spacetime migrate .` would
            // have rewritten 31 working sites across 10 files to
            // `@on &.load(name: as, …)`.
            if text == "as" {
                return None;
            }
            Some((CapturedValue::Ident(text.to_string()), 1))
        } else if tok.kind == SyntaxKind::DOLLAR {
            // A `$`-prefixed binding name in an IDENT slot: strip the sigil and
            // capture the bare name (`$zone` -> `zone`). This harmonizes the
            // alias surfaces so the dollar form and the bare form are equivalent
            // ONE meaning for `as` everywhere: the in-paren binding
            // (`@each($items as $c)`) already takes `$`, and this lets the
            // post-paren alias (`@presence(...) as $name`, `@drop-zone(...) as
            // $zone`) take it too. Without this the `$` token fails the IDENT
            // match and the WHOLE form silently fails to expand (the macro just
            // does nothing). Captures the name WITHOUT the sigil, identical to
            // the bare form. Consumes 2 tokens ($ + name).
            let next = tokens.get(1)?;
            if next.kind == SyntaxKind::IDENT || next.kind.is_keyword() {
                // BUG-328: `$name` is an IDENT only when it ENDS there. A
                // following `.` makes it a MEMBER ACCESS on that binding
                // (`&$bounds.rect.width` — "the rect.width of the element in
                // $bounds"), not a name. Binding `bounds` here left `.rect.width`
                // trailing, where the sibling `$selector:selector` capture read it
                // as a CSS class selector and expanded `element-ref` against the
                // literal string ".rect.width" — emitting a second `const el` into
                // a scope that already had one. That is a bundle that does not
                // parse, i.e. every page importing stdlib was dead (stdlib/index.st
                // imports dnd, whose @drag macro %derives uses exactly this shape).
                //
                // Refusing here makes the whole form fail to match, which is
                // correct: `&$x.y` is not an element-ref declaration.
                // The general rule, not just the `.` case: `$name` is an IDENT
                // only when the name ENDS there. Any ACCESSOR immediately after
                // it means the author wrote an expression ABOUT that binding,
                // not a name:
                //
                //     &$bounds.rect.width   member access   (BUG-328)
                //     &$items[0]            index access
                //
                // Binding the head and leaving the accessor trailing lets a
                // sibling capture claim the remainder — `$selector:selector`
                // read `.rect.width` as a CSS class selector and expanded
                // `element-ref` against it, emitting a duplicate `const el` and
                // killing every page that imports stdlib.
                //
                // Fixing only `.` left `[` reachable with the identical failure,
                // so the check is on the CLASS of accessor tokens. `(` is
                // deliberately absent: a call is not an accessor on a name, and
                // no form pairs an ident capture with a following call.
                if tokens
                    .get(2)
                    .is_some_and(|t| matches!(t.kind, SyntaxKind::DOT | SyntaxKind::L_BRACKET))
                {
                    return None;
                }
                return Some((CapturedValue::Ident(next.text(source).to_string()), 2));
            }
            None
        } else if tok.kind == SyntaxKind::TILDE {
            // ~preset-name: TILDE followed by IDENT/keyword → combine into "~name"
            let next = tokens.get(1)?;
            if next.kind == SyntaxKind::IDENT || next.kind.is_keyword() {
                let name = format!("~{}", next.text(source));
                Some((CapturedValue::Ident(name), 2))
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Extract a DASHED-IDENT: an identifier that begins with `--`.
///
/// The `RawToken::Minus` ladder already merges `--rise` into ONE `IDENT` token
/// (the same ladder that yields `-42`, `-45deg`, `-webkit-*`), so a capture type
/// cannot select these by token kind — and it MUST be able to. SIP-001 declared
/// `%capture_type form_application { "--" $form:ident … }`, which can never match
/// because there is no separate `--` token for the literal to consume (BUG-242).
///
/// Discriminating here rather than in the lexer is deliberate. Introducing a
/// `DASHED_IDENT` SyntaxKind would change what 348 `SyntaxKind::IDENT` sites see
/// for `--custom-prop` — a large blast radius, in exchange for a distinction only
/// capture types need. The lexer keeps producing one IDENT (which is what CSS
/// compatibility requires); the EXTRACTOR decides whether this particular slot
/// demands the sigil.
///
/// Captures the name WITH its `--` prefix, since that is what the author wrote
/// and what downstream form lookup resolves against.
/// Extract an EVENT NAME for the legacy `@on` dispatchers: a bare ident,
/// never a `$`-sigiled binding. `IdentExtractor` deliberately strips the `$`
/// (alias harmonization), which let `@on $sig { … }` — the signal-arms
/// surface — fall through to the mutation dispatcher when the arms grammar
/// rejected the body (W3 review P2: a valid arm plus a non-arm statement
/// compiled through the WRONG dispatcher, silently). The event slot must not
/// inherit the alias leniency.
pub struct EventNameExtractor;

impl CaptureExtractor for EventNameExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        if tok.kind == SyntaxKind::IDENT || tok.kind.is_keyword() {
            let text = tok.text(source);
            if text.starts_with('$') || text.starts_with('&') || text.starts_with("--") {
                return None;
            }
            Some((CapturedValue::Ident(text.to_string()), 1))
        } else {
            None
        }
    }
}

pub struct DashedIdentExtractor;

impl CaptureExtractor for DashedIdentExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        if tok.kind != SyntaxKind::IDENT {
            return None;
        }
        let text = tok.text(source);
        // `--` alone is not a name, and a single leading `-` is a vendor prefix
        // (`-webkit-transform`), not a form reference.
        if text.starts_with("--") && text.len() > 2 {
            Some((CapturedValue::Ident(text.to_string()), 1))
        } else {
            None
        }
    }
}

/// Extract a string literal (STRING token), stripping quotes.
pub struct StringExtractor;

impl CaptureExtractor for StringExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        if tok.kind == SyntaxKind::STRING {
            let text = tok.text(source);
            // Strip quotes
            if text.len() >= 2 {
                let inner = &text[1..text.len() - 1];
                Some((CapturedValue::String(inner.to_string()), 1))
            } else {
                Some((CapturedValue::String(String::new()), 1))
            }
        } else if (tok.kind == SyntaxKind::IDENT || tok.kind.is_keyword())
            && tok.text(source) != "_"
        {
            // FUP-079: accept a BARE identifier as a string literal so enum-ish
            // values read unquoted (`mode: pingpong`, `split: chars`, `tone: aces`).
            // This is the single chokepoint for every `:string` slot — leading-token
            // (`@object $shape:string`) AND named-arg (`@mm(mode: $mode:string)`) —
            // so one rule harmonizes both surfaces (the divergence FUP-079 reports).
            // Mirrors BoolExtractor, which already accepts bare `true`/`false` idents.
            //
            // EXCLUDE a lone `_`: it is the language-wide wildcard / placeholder
            // sigil (the `@match _ =>` fallback arm, `$_:skip_block`). The
            // `match_pat` grammar is `( $lit:string ) | ( $wild:ident )` and keys
            // the wildcard off the `wild` branch; capturing `_` as a string here
            // would route it to `lit`, turning the fallback arm into a literal that
            // only matches the string "_" (FEAT-073/124/126 regression). `_` is
            // never a meaningful enum value, so refusing it costs nothing.
            //
            // Refuse when the ident is merely the HEAD of a larger expression:
            //   `rgba(…)` (call), `a.b` (member), `a[…]` (index). Capturing just the
            // leading ident there would be wrong; such values must stay quoted (or
            // use `:expr`). A bare enum value is followed by a delimiter
            // (`)`/`,`/`;`/`}`/newline/EOF), never by `(`/`.`/`[`.
            // Skip trivia to find the next significant token (slices may carry
            // WHITESPACE between tokens).
            let next = tokens
                .iter()
                .skip(1)
                .find(|t| t.kind != SyntaxKind::WHITESPACE)
                .map(|t| t.kind);
            match next {
                Some(SyntaxKind::L_PAREN) | Some(SyntaxKind::DOT) | Some(SyntaxKind::L_BRACKET) => {
                    None
                }
                _ => Some((CapturedValue::String(tok.text(source).to_string()), 1)),
            }
        } else {
            None
        }
    }
}

/// Extract a number (NUMBER token) as f64.
pub struct NumberExtractor;

impl CaptureExtractor for NumberExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        if tok.kind == SyntaxKind::NUMBER {
            let text = tok.text(source);
            let value: f64 = text.parse().ok()?;
            Some((CapturedValue::Number(value), 1))
        } else {
            None
        }
    }
}

/// Extract a boolean (KW_TRUE or KW_FALSE).
pub struct BoolExtractor;

impl CaptureExtractor for BoolExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        match tok.kind {
            SyntaxKind::KW_TRUE => Some((CapturedValue::Bool(true), 1)),
            SyntaxKind::KW_FALSE => Some((CapturedValue::Bool(false), 1)),
            SyntaxKind::IDENT => {
                let text = tok.text(source);
                match text {
                    "true" => Some((CapturedValue::Bool(true), 1)),
                    "false" => Some((CapturedValue::Bool(false), 1)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// Extract a time/duration value (NUMBER_WITH_UNIT with time unit: ms, s, us, m, fps).
pub struct TimeExtractor;

impl CaptureExtractor for TimeExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        if tok.kind == SyntaxKind::NUMBER_WITH_UNIT {
            let text = tok.text(source);
            let (ms, _) = conversions::parse_duration_with_unit(text)?;
            Some((CapturedValue::Time(ms), 1))
        } else {
            None
        }
    }
}

/// Extract a length value (NUMBER_WITH_UNIT with length unit: px, %, em, rem, etc.).
pub struct LengthExtractor;

impl CaptureExtractor for LengthExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        if tok.kind == SyntaxKind::NUMBER_WITH_UNIT {
            let text = tok.text(source);
            let (value, unit) = conversions::parse_length_with_unit(text)?;
            Some((CapturedValue::Length(LengthValue::new(value, unit)), 1))
        } else {
            None
        }
    }
}

/// Extract an event name (IDENT from the known event set).
pub struct EventExtractor;

const KNOWN_EVENTS: &[&str] = &[
    "hover",
    "click",
    "focus",
    "blur",
    "submit",
    "input",
    "change",
    "mouseenter",
    "mouseleave",
    "mousedown",
    "mouseup",
    "mousemove",
    "keydown",
    "keyup",
    "keypress",
    "scroll",
    "resize",
    "load",
    "unload",
    "touchstart",
    "touchend",
    "touchmove",
    "pointerdown",
    "pointerup",
    "pointermove",
    "pointerenter",
    "pointerleave",
    "dragstart",
    "dragend",
    "drag",
    "dragover",
    "drop",
    "wheel",
    "contextmenu",
    "dblclick",
    "focusin",
    "focusout",
    "animationend",
    "transitionend",
    "mutation",
    "intersection",
    "visible",
    "progress",
];

impl CaptureExtractor for EventExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        if tok.kind == SyntaxKind::IDENT {
            let text = tok.text(source);
            // Accept any identifier as event name (forms validate correctness)
            if KNOWN_EVENTS.contains(&text)
                || text.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            {
                Some((CapturedValue::Ident(text.to_string()), 1))
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Extract an easing preset name (IDENT like "ease-in-out", "linear").
pub struct EasingExtractor;

impl CaptureExtractor for EasingExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        let tok = tokens.first()?;
        if tok.kind == SyntaxKind::IDENT {
            let text = tok.text(source);
            Some((CapturedValue::Ident(text.to_string()), 1))
        } else if tok.kind == SyntaxKind::TILDE {
            // ~preset-name: TILDE followed by IDENT → combine into "~name"
            let next = tokens.get(1)?;
            if next.kind == SyntaxKind::IDENT || next.kind.is_keyword() {
                let name = format!("~{}", next.text(source));
                Some((CapturedValue::Ident(name), 2))
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Extract a type reference (IDENT, possibly with [] suffix).
pub struct TyperefExtractor;

impl CaptureExtractor for TyperefExtractor {
    /// Match a type reference: an IDENT/STRING term (with optional `[]` / `?`
    /// suffix), optionally a `|`-union of such terms.
    ///
    /// Grammar: `term ( "|" term )*`  where  `term = IDENT ( "[]" )? ( "?" )? | STRING`.
    ///
    /// The first token may be a STRING literal, which is what lets a
    /// string-literal-led union annotate a `param_list` param
    /// (`$size "S" | "M" | "L"`) — parity with the parenthesized `%primitive`
    /// union form (`axis: ("x" | "y")`, handled by `parse_meta_param_type`).
    /// Source spelling is preserved in `TypeRef(String)` (same as `Product[]`).
    ///
    /// The token slice carries interleaved WHITESPACE trivia (the pattern matcher
    /// only strips it at capture *boundaries*); the union fold skips it between
    /// the term, the `|`, and the next term.
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        // Consume the leading term; if it is not a valid type term, no match.
        let mut full = String::new();
        let mut consumed = consume_type_term(tokens, source, &mut full)?;

        // Greedily fold `| term` while the next non-trivia token is a PIPE.
        loop {
            let after_term = skip_ws(tokens, consumed);
            let Some(pipe) = tokens.get(after_term) else {
                break;
            };
            if pipe.kind != SyntaxKind::PIPE {
                break;
            }
            let after_pipe = skip_ws(tokens, after_term + 1);
            // A `|` with no valid term after it is NOT part of this type — stop
            // before the pipe and leave it for the surrounding pattern.
            let mut term = String::new();
            let Some(term_len) = consume_type_term(&tokens[after_pipe..], source, &mut term) else {
                break;
            };
            full.push_str(" | ");
            full.push_str(&term);
            consumed = after_pipe + term_len;
        }

        Some((CapturedValue::TypeRef(full), consumed))
    }
}

/// Index of the first non-WHITESPACE token at or after `start`.
fn skip_ws(tokens: &[TokenData], start: usize) -> usize {
    let mut i = start;
    while i < tokens.len() && tokens[i].kind == SyntaxKind::WHITESPACE {
        i += 1;
    }
    i
}

/// Consume a single type term from the front of `tokens`, appending its source
/// text to `out`. Returns the token count consumed (counting from index 0,
/// including any leading WHITESPACE skipped), or `None` if no valid term.
///
/// A term is `IDENT ( "[]" )? ( "?" )?` (array/optional suffixes) or a bare
/// `STRING` literal (a string-enum alternative like `"S"`). Array/optional
/// suffixes must be adjacent to the IDENT (no trivia between), matching the
/// lexer's emission for `Type[]` / `Type?`.
fn consume_type_term(tokens: &[TokenData], source: &str, out: &mut String) -> Option<usize> {
    let start = skip_ws(tokens, 0);
    let tok = tokens.get(start)?;
    match tok.kind {
        SyntaxKind::IDENT => {
            let mut consumed = start + 1;
            out.push_str(tok.text(source));
            // Array suffix(es): `Type[]`, `Type[][]` (nested arrays).
            while tokens.len() >= consumed + 2
                && tokens[consumed].kind == SyntaxKind::L_BRACKET
                && tokens[consumed + 1].kind == SyntaxKind::R_BRACKET
            {
                out.push_str("[]");
                consumed += 2;
            }
            // Optional suffix: `Type?`
            if tokens.len() > consumed && tokens[consumed].kind == SyntaxKind::QUESTION {
                out.push('?');
                consumed += 1;
            }
            Some(consumed)
        }
        // A string literal is a valid term ONLY as a union alternative; its
        // source text (incl. quotes) is preserved verbatim.
        SyntaxKind::STRING => {
            out.push_str(tok.text(source));
            Some(start + 1)
        }
        _ => None,
    }
}

/// Greedy expression extractor — consumes tokens up to a boundary.
/// Boundary tokens: `;`, `)`, `}`, `,`, `->`.
pub struct ExprExtractor;

impl CaptureExtractor for ExprExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        if tokens.is_empty() {
            return None;
        }

        let mut count = 0;
        let mut depth = 0i32; // track parens/braces/brackets nesting

        for tok in tokens {
            match tok.kind {
                SyntaxKind::L_PAREN | SyntaxKind::L_BRACE | SyntaxKind::L_BRACKET => {
                    depth += 1;
                    count += 1;
                }
                SyntaxKind::R_PAREN | SyntaxKind::R_BRACE | SyntaxKind::R_BRACKET => {
                    if depth <= 0 {
                        break; // closing delimiter not ours
                    }
                    depth -= 1;
                    count += 1;
                }
                SyntaxKind::SEMICOLON
                | SyntaxKind::ARROW
                | SyntaxKind::FAT_ARROW
                | SyntaxKind::COMMA
                    if depth == 0 =>
                {
                    break; // boundary token
                }
                SyntaxKind::EOF => break,
                _ => {
                    count += 1;
                }
            }
        }

        if count == 0 {
            return None;
        }

        // Build the expression text. End at the last NON-TRIVIA token consumed: a
        // trailing whitespace/comment token can carry a text range that extends well
        // past the expression (e.g. a `\n` whose span reaches a following body), which
        // would otherwise splice unrelated source into the captured value (BUG-088).
        let start = tokens[0].text_range.0;
        let end = tokens[..count]
            .iter()
            .rev()
            .find(|t| !t.kind.is_trivia())
            .map(|t| t.text_range.1)
            .unwrap_or(tokens[count - 1].text_range.1);
        let text = source[start..end].trim();
        Some((CapturedValue::Expr(text.to_string()), count))
    }
}

/// Color extractor — works like ExprExtractor but produces CapturedValue::Color.
/// CSS colors can be named idents (cyan), hex (#E85D4A), or function calls (rgb(), oklch()).
pub struct ColorExtractor;

impl CaptureExtractor for ColorExtractor {
    fn extract(&self, tokens: &[TokenData], source: &str) -> ExtractResult {
        if tokens.is_empty() {
            return None;
        }

        let mut count = 0;
        let mut depth = 0i32;

        for tok in tokens {
            match tok.kind {
                SyntaxKind::L_PAREN | SyntaxKind::L_BRACE | SyntaxKind::L_BRACKET => {
                    depth += 1;
                    count += 1;
                }
                SyntaxKind::R_PAREN | SyntaxKind::R_BRACE | SyntaxKind::R_BRACKET => {
                    if depth <= 0 {
                        break;
                    }
                    depth -= 1;
                    count += 1;
                }
                SyntaxKind::SEMICOLON
                | SyntaxKind::ARROW
                | SyntaxKind::FAT_ARROW
                | SyntaxKind::COMMA
                    if depth == 0 =>
                {
                    break;
                }
                SyntaxKind::EOF => break,
                _ => {
                    count += 1;
                }
            }
        }

        if count == 0 {
            return None;
        }

        let start = tokens[0].text_range.0;
        let end = tokens[count - 1].text_range.1;
        let text = source[start..end].trim();
        Some((CapturedValue::Color(text.to_string()), count))
    }
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
    fn ident_extractor() {
        let source = "hover";
        let tokens = [tok(SyntaxKind::IDENT, 0, 5)];
        let result = IdentExtractor.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Ident("hover".to_string()), 1)));
    }

    #[test]
    fn ident_extractor_rejects_number() {
        let source = "42";
        let tokens = [tok(SyntaxKind::NUMBER, 0, 2)];
        assert_eq!(IdentExtractor.extract(&tokens, source), None);
    }

    #[test]
    fn string_extractor() {
        let source = "\"hello world\"";
        let tokens = [tok(SyntaxKind::STRING, 0, 13)];
        let result = StringExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::String("hello world".to_string()), 1))
        );
    }

    #[test]
    fn string_extractor_accepts_bare_ident() {
        // FUP-079: a bare identifier is captured as a string literal so enum-ish
        // values read unquoted (`mode: pingpong`). One rule for every `:string`
        // slot — leading-token and named-arg alike.
        let source = "pingpong";
        let tokens = [tok(SyntaxKind::IDENT, 0, 8)];
        assert_eq!(
            StringExtractor.extract(&tokens, source),
            Some((CapturedValue::String("pingpong".to_string()), 1))
        );
    }

    #[test]
    fn string_extractor_accepts_keyword_ident() {
        // Keyword-spelled values (`from`, `to`, `on`) are valid bare string values.
        let source = "from";
        let tokens = [tok(SyntaxKind::KW_FROM, 0, 4)];
        assert_eq!(
            StringExtractor.extract(&tokens, source),
            Some((CapturedValue::String("from".to_string()), 1))
        );
    }

    #[test]
    fn string_extractor_refuses_lone_underscore() {
        // `_` is the language-wide wildcard sigil (`@match _ =>`); it must NOT be
        // captured as the string "_", or the `match_pat` alternation routes the
        // fallback arm to the literal branch (FEAT-073/124/126 regression).
        let source = "_";
        let tokens = [tok(SyntaxKind::IDENT, 0, 1)];
        assert_eq!(StringExtractor.extract(&tokens, source), None);
    }

    #[test]
    fn string_extractor_refuses_expression_head_ident() {
        // A bare ident that is the HEAD of a larger expression must stay quoted
        // (or use `:expr`). Capturing just the leading ident would be wrong.
        // call head: `rgba(...)`
        let call = "rgba(0,0,0,0.1)";
        let call_tokens = [tok(SyntaxKind::IDENT, 0, 4), tok(SyntaxKind::L_PAREN, 4, 5)];
        assert_eq!(StringExtractor.extract(&call_tokens, call), None);
        // member head: `a.b`
        let member = "a.b";
        let member_tokens = [
            tok(SyntaxKind::IDENT, 0, 1),
            tok(SyntaxKind::DOT, 1, 2),
            tok(SyntaxKind::IDENT, 2, 3),
        ];
        assert_eq!(StringExtractor.extract(&member_tokens, member), None);
        // index head: `a[0]`
        let index = "a[0]";
        let index_tokens = [
            tok(SyntaxKind::IDENT, 0, 1),
            tok(SyntaxKind::L_BRACKET, 1, 2),
        ];
        assert_eq!(StringExtractor.extract(&index_tokens, index), None);
    }

    #[test]
    fn string_extractor_quoted_still_wins_and_strips() {
        // Regression guard: the original quoted-string behaviour is unchanged —
        // quotes stripped, single token consumed.
        let source = "\"glass\"";
        let tokens = [tok(SyntaxKind::STRING, 0, 7)];
        assert_eq!(
            StringExtractor.extract(&tokens, source),
            Some((CapturedValue::String("glass".to_string()), 1))
        );
    }

    // 3.14 below is an arbitrary decimal test fixture (verifying number
    // extraction), not an intended pi approximation --
    // clippy::approx_constant is a false positive on this test.
    #[allow(clippy::approx_constant)]
    #[test]
    fn number_extractor() {
        let source = "3.14";
        let tokens = [tok(SyntaxKind::NUMBER, 0, 4)];
        let result = NumberExtractor.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Number(3.14), 1)));
    }

    #[test]
    fn bool_extractor_true() {
        let source = "true";
        let tokens = [tok(SyntaxKind::KW_TRUE, 0, 4)];
        let result = BoolExtractor.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Bool(true), 1)));
    }

    #[test]
    fn time_extractor() {
        let source = "300ms";
        let tokens = [tok(SyntaxKind::NUMBER_WITH_UNIT, 0, 5)];
        let result = TimeExtractor.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Time(300), 1)));
    }

    #[test]
    fn time_extractor_seconds() {
        let source = "2s";
        let tokens = [tok(SyntaxKind::NUMBER_WITH_UNIT, 0, 2)];
        let result = TimeExtractor.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Time(2000), 1)));
    }

    #[test]
    fn length_extractor() {
        let source = "100px";
        let tokens = [tok(SyntaxKind::NUMBER_WITH_UNIT, 0, 5)];
        let result = LengthExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::Length(LengthValue::px(100.0)), 1))
        );
    }

    #[test]
    fn event_extractor() {
        let source = "hover";
        let tokens = [tok(SyntaxKind::IDENT, 0, 5)];
        let result = EventExtractor.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Ident("hover".to_string()), 1)));
    }

    #[test]
    fn typeref_extractor_simple() {
        let source = "string";
        let tokens = [tok(SyntaxKind::IDENT, 0, 6)];
        let result = TyperefExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::TypeRef("string".to_string()), 1))
        );
    }

    #[test]
    fn typeref_extractor_array() {
        let source = "Product[]";
        let tokens = [
            tok(SyntaxKind::IDENT, 0, 7),
            tok(SyntaxKind::L_BRACKET, 7, 8),
            tok(SyntaxKind::R_BRACKET, 8, 9),
        ];
        let result = TyperefExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::TypeRef("Product[]".to_string()), 3))
        );
    }

    #[test]
    fn typeref_extractor_nested_array() {
        // `Card[][]` — a 2-D array type. A single `[]` consume left a trailing
        // `[]` that broke the enclosing `@data inline` pattern and silently
        // dropped the whole directive (signal never seeded -> derive crash).
        let source = "Card[][]";
        let tokens = [
            tok(SyntaxKind::IDENT, 0, 4),
            tok(SyntaxKind::L_BRACKET, 4, 5),
            tok(SyntaxKind::R_BRACKET, 5, 6),
            tok(SyntaxKind::L_BRACKET, 6, 7),
            tok(SyntaxKind::R_BRACKET, 7, 8),
        ];
        let result = TyperefExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::TypeRef("Card[][]".to_string()), 5))
        );
    }

    #[test]
    fn typeref_extractor_string_led_union() {
        // FUP-043: `"S" | "M" | "L"` — a string-literal-led union annotation
        // (parity with the parenthesized `%primitive` union form).
        let source = "\"S\" | \"M\" | \"L\"";
        let tokens = [
            tok(SyntaxKind::STRING, 0, 3),
            tok(SyntaxKind::PIPE, 4, 5),
            tok(SyntaxKind::STRING, 6, 9),
            tok(SyntaxKind::PIPE, 10, 11),
            tok(SyntaxKind::STRING, 12, 15),
        ];
        let result = TyperefExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((
                CapturedValue::TypeRef("\"S\" | \"M\" | \"L\"".to_string()),
                5
            ))
        );
    }

    #[test]
    fn typeref_extractor_ident_led_union() {
        // `string | null` — ident-led union still works.
        let source = "string | null";
        let tokens = [
            tok(SyntaxKind::IDENT, 0, 6),
            tok(SyntaxKind::PIPE, 7, 8),
            tok(SyntaxKind::IDENT, 9, 13),
        ];
        let result = TyperefExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::TypeRef("string | null".to_string()), 3))
        );
    }

    #[test]
    fn typeref_extractor_trailing_pipe_not_consumed() {
        // A `|` with no valid term after it is left for the surrounding pattern.
        let source = "string |";
        let tokens = [tok(SyntaxKind::IDENT, 0, 6), tok(SyntaxKind::PIPE, 7, 8)];
        let result = TyperefExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::TypeRef("string".to_string()), 1))
        );
    }

    #[test]
    fn ident_extractor_with_tilde_prefix() {
        let source = "~ora-ease";
        let tokens = [tok(SyntaxKind::TILDE, 0, 1), tok(SyntaxKind::IDENT, 1, 9)];
        let result = IdentExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::Ident("~ora-ease".to_string()), 2))
        );
    }

    #[test]
    fn ident_extractor_tilde_only_rejects() {
        let source = "~";
        let tokens = [tok(SyntaxKind::TILDE, 0, 1)];
        assert_eq!(IdentExtractor.extract(&tokens, source), None);
    }

    #[test]
    fn easing_extractor_with_tilde_prefix() {
        let source = "~spring-gentle";
        let tokens = [tok(SyntaxKind::TILDE, 0, 1), tok(SyntaxKind::IDENT, 1, 14)];
        let result = EasingExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::Ident("~spring-gentle".to_string()), 2))
        );
    }

    #[test]
    fn expr_extractor_simple() {
        let source = "x + 1;";
        let tokens = [
            tok(SyntaxKind::IDENT, 0, 1),
            tok(SyntaxKind::WHITESPACE, 1, 2),
            tok(SyntaxKind::PLUS, 2, 3),
            tok(SyntaxKind::WHITESPACE, 3, 4),
            tok(SyntaxKind::NUMBER, 4, 5),
            tok(SyntaxKind::SEMICOLON, 5, 6),
        ];
        let result = ExprExtractor.extract(&tokens, source);
        assert_eq!(result, Some((CapturedValue::Expr("x + 1".to_string()), 5)));
    }

    #[test]
    fn expr_extractor_stops_at_fat_arrow() {
        let source = "$a && $b => Blue;";
        let tokens = [
            tok(SyntaxKind::DOLLAR, 0, 1),
            tok(SyntaxKind::IDENT, 1, 2),
            tok(SyntaxKind::WHITESPACE, 2, 3),
            tok(SyntaxKind::AND_AND, 3, 5),
            tok(SyntaxKind::WHITESPACE, 5, 6),
            tok(SyntaxKind::DOLLAR, 6, 7),
            tok(SyntaxKind::IDENT, 7, 8),
            tok(SyntaxKind::WHITESPACE, 8, 9),
            tok(SyntaxKind::FAT_ARROW, 9, 11),
            tok(SyntaxKind::WHITESPACE, 11, 12),
            tok(SyntaxKind::IDENT, 12, 16),
            tok(SyntaxKind::SEMICOLON, 16, 17),
        ];
        let result = ExprExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::Expr("$a && $b".to_string()), 8))
        );
    }

    #[test]
    fn expr_extractor_with_parens() {
        let source = "(a + b) * c;";
        let tokens = [
            tok(SyntaxKind::L_PAREN, 0, 1),
            tok(SyntaxKind::IDENT, 1, 2),
            tok(SyntaxKind::WHITESPACE, 2, 3),
            tok(SyntaxKind::PLUS, 3, 4),
            tok(SyntaxKind::WHITESPACE, 4, 5),
            tok(SyntaxKind::IDENT, 5, 6),
            tok(SyntaxKind::R_PAREN, 6, 7),
            tok(SyntaxKind::WHITESPACE, 7, 8),
            tok(SyntaxKind::STAR, 8, 9),
            tok(SyntaxKind::WHITESPACE, 9, 10),
            tok(SyntaxKind::IDENT, 10, 11),
            tok(SyntaxKind::SEMICOLON, 11, 12),
        ];
        let result = ExprExtractor.extract(&tokens, source);
        assert_eq!(
            result,
            Some((CapturedValue::Expr("(a + b) * c".to_string()), 11))
        );
    }
}

#[cfg(test)]
mod bug328_tests {
    use super::*;

    fn tok(kind: SyntaxKind, range: (usize, usize)) -> TokenData {
        TokenData {
            kind,
            text_range: range,
        }
    }

    /// BUG-328: `$name` followed by `.` is a MEMBER ACCESS, not an ident.
    ///
    /// Binding `bounds` out of `&$bounds.rect.width` left `.rect.width` trailing,
    /// where the sibling `$selector:selector` capture read it as a CSS class
    /// selector and expanded `element-ref` against the literal ".rect.width" —
    /// emitting a second `const el` into a scope that already had one. Every page
    /// importing stdlib died, because stdlib imports dnd and `@drag` %derives uses
    /// exactly this shape.
    #[test]
    fn a_dollar_ident_followed_by_a_dot_is_not_an_ident() {
        let src = "$bounds.rect.width";
        let toks = vec![
            tok(SyntaxKind::DOLLAR, (0, 1)),
            tok(SyntaxKind::IDENT, (1, 7)),
            tok(SyntaxKind::DOT, (7, 8)),
            tok(SyntaxKind::IDENT, (8, 12)),
        ];
        let out = IdentExtractor.extract(&toks, src);
        assert!(
            out.is_none(),
            "`$bounds.rect` is a member access; binding `bounds` leaves `.rect.width` \
             to be misread as a selector (BUG-328). got: {out:?}"
        );
    }

    /// The `.` case was fixed first; `[` had the IDENTICAL failure and was still
    /// reachable, which is why the check is on the CLASS of accessor tokens.
    #[test]
    fn a_dollar_ident_followed_by_a_bracket_is_not_an_ident() {
        let src = "$items[0]";
        let toks = vec![
            tok(SyntaxKind::DOLLAR, (0, 1)),
            tok(SyntaxKind::IDENT, (1, 6)),
            tok(SyntaxKind::L_BRACKET, (6, 7)),
            tok(SyntaxKind::NUMBER, (7, 8)),
        ];
        let out = IdentExtractor.extract(&toks, src);
        assert!(
            out.is_none(),
            "`$items[0]` is an index access; binding `items` leaves `[0]` for a \
             sibling capture to misclaim (same class as BUG-328). got: {out:?}"
        );
    }

    /// The harmonized alias surface must keep working: `$name` alone IS an ident.
    /// `@drop-zone(...) as $zone` depends on it.
    #[test]
    fn a_bare_dollar_ident_still_binds() {
        let src = "$zone";
        let toks = vec![
            tok(SyntaxKind::DOLLAR, (0, 1)),
            tok(SyntaxKind::IDENT, (1, 5)),
        ];
        let out = IdentExtractor.extract(&toks, src);
        assert!(
            matches!(&out, Some((CapturedValue::Ident(n), 2)) if n == "zone"),
            "the post-paren alias form must still bind. got: {out:?}"
        );
    }
}
