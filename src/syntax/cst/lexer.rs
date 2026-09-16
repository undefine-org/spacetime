//! Lexer for Spacetime DSL
//!
//! Tokenizes Spacetime source code into a sequence of tokens with SyntaxKind.
//! The lexer is lossless - all input characters are represented in the output.
//!
//! Built on logos for the core DFA tokenization, with a post-processing pass
//! for context-dependent decisions (negative numbers, vendor-prefix identifiers,
//! keyword recognition, number+unit merging, color literals).

use logos::Logos;

use super::SyntaxKind;

/// A token produced by the lexer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The kind of token
    pub kind: SyntaxKind,
    /// The text of the token
    pub text: String,
    /// Byte offset in the source
    pub offset: usize,
}

impl Token {
    /// Returns the length of this token in bytes
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Returns true if this token is empty (shouldn't happen in practice)
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// Raw token kinds from logos DFA.
///
/// These are refined into `SyntaxKind` by the post-processing pass in `Lexer`.
#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"")] // Don't skip anything — we're lossless
enum RawToken {
    // Whitespace
    #[regex(r"[ \t\n\r]+")]
    Whitespace,

    // Comments
    #[regex(r"//[^\n]*")]
    LineComment,

    // Block comments handled by callback for error recovery
    #[token("/*", block_comment_callback)]
    BlockComment,

    // String literals — callback handles unterminated strings (error recovery)
    #[token("\"", double_string_callback)]
    DoubleString,
    #[token("'", single_string_callback)]
    SingleString,

    // Numbers (digits with optional decimal)
    #[regex(r"[0-9]+(\.[0-9]+)?")]
    Number,

    // Leading-decimal numbers (.5, .123)
    #[regex(r"\.[0-9]+")]
    LeadingDecimalNumber,

    // Identifiers (letters/underscore, then alphanumeric/underscore/hyphen)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_-]*")]
    Ident,

    // Color literals (#hex)
    #[regex(r"#[0-9a-fA-F]+")]
    Color,

    // Multi-character operators (higher priority via longer match)
    #[token("->")]
    Arrow,
    #[token("<-")]
    LeftArrow,
    #[token("=>")]
    FatArrow,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("===")]
    EqEqEq,
    #[token("!==")]
    NotEqEq,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("??")]
    QuestionQuestion,

    // Single-character tokens
    #[token("$")]
    Dollar,
    // The HOLE sigil. One char, non-consuming, exactly like `$`/`&`/`@`/`%`.
    // It does NOT scan to a closing backtick: the parser decides what a hole
    // spans (`parse_html_element`'s byte-scan, `parse_hole_inner`). A
    // span-consuming version was tried and swallowed all 4,934 corpus holes
    // (FUP-046 attempt 1, tests/template_literal_token_test.rs).
    #[token("`")]
    Backtick,

    #[token("&")]
    Ampersand,
    #[token("~")]
    Tilde,
    #[token("@")]
    AtSign,
    #[token("%")]
    Percent,
    #[token("#")]
    Hash,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("?")]
    Question,
    #[token("!")]
    Exclaim,
    #[token("=")]
    Equals,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,

    // A byte logos did not recognise. Never produced by a `#[token]`/`#[regex]`
    // rule — `raw_tokenize` synthesises it for the `Err(())` case so an unknown
    // byte cannot borrow another token's identity and inherit its merge rules.
    Unknown,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("|")]
    Pipe,
    #[token("^")]
    Caret,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
}

/// Callback for block comments — scans to `*/`.
///
/// `/*` also occurs in ordinary HTML text — a glob in prose (`process/*.md`,
/// `src/**/*.rs`), a footnote marker. An UNTERMINATED one must NOT swallow the
/// rest of the file (markup, CSS, directives) to EOF: that silently deletes every
/// rule after it, and since the page still compiles and the text still renders,
/// the loss is invisible until some later style is simply missing (BUG-224). Same
/// failure mode, and same remedy, as the lone quote in HTML text (BUG-072).
///
/// The blank-line heuristic distinguishes the two, but it applies ONLY when the
/// comment is unterminated. BUG-224 originally let a blank line win even when a
/// `*/` followed, on the theory that "a real block comment never crosses a blank
/// line" — which is false. A documentation banner routinely has paragraphs, and
/// this repo's own `stdlib/__mcp__/workbench/styles.st` is the counterexample: its
/// PLAN-064 banner broke, the workbench stopped compiling, and 7 MCP tests failed
/// (BUG-225).
///
/// So: a CLOSED `/* … */` is always a comment, however many blank lines it spans.
/// Only when there is no `*/` at all does a blank line mean "this was prose
/// punctuation" — consume just the two bytes logos already took and let the rest
/// re-lex. That keeps BUG-224's protection (an unterminated `/*` cannot eat the
/// file) without penalising a correctly-closed comment. The two goals are
/// independent; the original ordering traded one for the other.
fn block_comment_callback(lex: &mut logos::Lexer<RawToken>) -> bool {
    let remainder = lex.remainder();
    let terminator = remainder.find("*/");
    match terminator {
        // CLOSED: a real comment, regardless of blank lines inside it.
        Some(end) => {
            lex.bump(end + 2); // consume through "*/"
            true
        }
        None => {
            // UNTERMINATED. A blank line means this `/*` was prose punctuation
            // rather than the start of a comment, so stand as a lone token and
            // keep the following source alive (BUG-224).
            let has_blank_line = remainder.contains("\n\n") || remainder.contains("\n\r\n");
            if has_blank_line {
                lex.bump(0);
            } else {
                // Unterminated with no blank line — consume the rest (recovery).
                lex.bump(remainder.len());
            }
            true
        }
    }
}

/// Callback for double-quoted strings — scans to closing `"` or EOF (error recovery).
fn double_string_callback(lex: &mut logos::Lexer<RawToken>) -> bool {
    string_callback_impl(lex, b'"')
}

/// Callback for single-quoted strings — scans to closing `'` or EOF (error recovery).
fn single_string_callback(lex: &mut logos::Lexer<RawToken>) -> bool {
    string_callback_impl(lex, b'\'')
}

/// Shared string scanning logic: advance past escapes until the closing quote.
/// A string never crosses a NEWLINE: Spacetime string literals are single-line,
/// and a lone quote in HTML text (an apostrophe in prose like `compiler's`, or a
/// `"`-quote in a sentence) must NOT start a string that swallows the rest of the
/// file (newlines, markup, CSS, directives) up to the next quote/EOF (BUG-072).
/// On hitting a newline before the closing quote we treat the opening quote as a
/// lone punctuation token: consume just that one byte and let the rest re-lex
/// normally.
fn string_callback_impl(lex: &mut logos::Lexer<RawToken>, quote: u8) -> bool {
    let remainder = lex.remainder().as_bytes();
    let mut i = 0;
    while i < remainder.len() {
        if remainder[i] == b'\n' {
            // Newline before a closing quote → not a string. Consume nothing past
            // the opening quote (already consumed by logos); the lone quote stands
            // as its own token.
            lex.bump(0);
            return true;
        } else if remainder[i] == b'\\' && i + 1 < remainder.len() {
            i += 2; // skip escape sequence
        } else if remainder[i] == quote {
            lex.bump(i + 1); // consume through closing quote
            return true;
        } else {
            i += 1;
        }
    }
    // Unterminated string at EOF — treat the opening quote as lone punctuation,
    // exactly as the newline case above does (BUG-072's rule, applied to the other
    // end of the input). Consuming the remainder instead would make the quote
    // swallow every following token, which is how a quote inside a FOREIGN-CODE
    // island (a JS regex char class `/[&<>"']/g`, a backtick in `/(['"`])/g`)
    // ate its block's closing brace and produced "expected R_BRACE, found EOF"
    // nowhere near the offending regex (BUG-040, BUG-041). A newline already
    // rescued the multi-line form; a body whose last line held the regex, or a
    // single-line `%emit`, still lost everything after the quote.
    //
    // NB this is recovery for genuinely malformed input either way: neither
    // behaviour produces a valid string token. Standing as punctuation keeps the
    // following source alive and lets the surrounding grammar report the real
    // error, instead of reporting a missing brace hundreds of bytes downstream.
    lex.bump(0);
    true
}

/// Lexer for Spacetime source code
pub struct Lexer<'a> {
    /// The input source
    input: &'a str,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input.
    ///
    /// PLAN-122 W1 moved the CSS value domain out of the lexer and into the stdlib
    /// grammars in `stdlib/capture-types/css-values.st`. The lexer used to fuse
    /// `8px` into one `NUMBER_WITH_UNIT` token and promote `#e8eef7` to one
    /// `COLOR` token in a value position — it decided what a length and a colour
    /// ARE, in Rust, before any grammar had spoken. It no longer does: `8px` lexes
    /// as `NUMBER` + `IDENT`, `50%` as `NUMBER` + `PERCENT`, `#e8eef7` as `HASH` +
    /// `IDENT`, the shapes a stdlib value grammar can match and validate.
    ///
    /// The cutover was not separable from the grammars. A capture extractor
    /// consumes a whole number of TOKENS, so a `%capture_type length` of
    /// `( $n:number $u:length_unit )` cannot match a fused `NUMBER_WITH_UNIT`
    /// token — with the grammars live and the lexer still fusing, every
    /// `distance: 40px` failed its own form. Name resolution and tokenization
    /// were ONE cutover, and the demotion is byte-for-byte lossless: it only ever
    /// SPLITS a token, never rewrites or drops text.
    pub fn new(input: &'a str) -> Self {
        Lexer { input }
    }

    /// Tokenize the entire input
    pub fn tokenize(self) -> Vec<Token> {
        // Phase 1: logos raw tokenization
        let raw_tokens = self.raw_tokenize();

        // Phase 2: post-processing (merge, disambiguate, keyword recognition)
        self.refine(raw_tokens)
    }

    /// Phase 1: Run logos to get raw tokens.
    fn raw_tokenize(&self) -> Vec<(RawToken, usize, usize)> {
        let mut result = Vec::new();
        let mut lexer = RawToken::lexer(self.input);

        while let Some(token_result) = lexer.next() {
            let span = lexer.span();
            match token_result {
                Ok(tok) => result.push((tok, span.start, span.end)),
                Err(()) => {
                    // Unknown byte. It gets its OWN category — `Unknown` — and not,
                    // as it did before, `RawToken::Minus /* placeholder */`.
                    //
                    // That placeholder was the FUP-046 leak. A byte the lexer did
                    // not recognise kept its span but wore a minus sign's identity,
                    // so `refine`'s vendor-prefix rule (MINUS + IDENT fuse for
                    // `-webkit-*`) applied to it. A backtick before an identifier
                    // was therefore absorbed INTO that identifier:
                    //
                    //     "a `b` c"  ->  IDENT("a")  IDENT("`b")  MINUS("`")  IDENT("c")
                    //
                    // Spacetime's hole sigil, arriving downstream as arithmetic.
                    // Every consumer that balances braces or reads an identifier
                    // was reasoning about a token that never existed.
                    //
                    // An unknown byte now refines to ERROR: visible, one byte wide,
                    // fusing with nothing.
                    result.push((RawToken::Unknown, span.start, span.end));
                }
            }
        }
        result
    }

    /// Phase 2: Refine raw tokens into SyntaxKind tokens.
    ///
    /// Handles:
    /// - Keyword recognition (ident text → keyword kind)
    /// - Negative number merging (MINUS + NUMBER → NUMBER when in value position)
    /// - Vendor prefix idents (MINUS + IDENT → IDENT for `-webkit-*`)
    fn refine(&self, raw: Vec<(RawToken, usize, usize)>) -> Vec<Token> {
        let mut tokens = Vec::with_capacity(raw.len() + 1);
        let mut i = 0;

        while i < raw.len() {
            let (tok, start, end) = raw[i];
            let text = &self.input[start..end];

            match tok {
                RawToken::Whitespace => {
                    tokens.push(Token {
                        kind: SyntaxKind::WHITESPACE,
                        text: text.to_string(),
                        offset: start,
                    });
                }
                RawToken::LineComment | RawToken::BlockComment => {
                    tokens.push(Token {
                        kind: SyntaxKind::COMMENT,
                        text: text.to_string(),
                        offset: start,
                    });
                }
                RawToken::DoubleString | RawToken::SingleString => {
                    tokens.push(Token {
                        kind: SyntaxKind::STRING,
                        text: text.to_string(),
                        offset: start,
                    });
                }
                RawToken::Number | RawToken::LeadingDecimalNumber => {
                    tokens.push(Token {
                        kind: SyntaxKind::NUMBER,
                        text: text.to_string(),
                        offset: start,
                    });
                }
                RawToken::Backtick => {
                    tokens.push(Token {
                        kind: SyntaxKind::BACKTICK,
                        text: text.to_string(),
                        offset: start,
                    });
                }
                RawToken::Unknown => {
                    tokens.push(Token {
                        kind: SyntaxKind::ERROR,
                        text: text.to_string(),
                        offset: start,
                    });
                }
                RawToken::Ident => {
                    let kind = keyword_or_ident(text);
                    tokens.push(Token {
                        kind,
                        text: text.to_string(),
                        offset: start,
                    });
                }
                RawToken::Color => {
                    let mut merged_end = end;
                    let mut j = i + 1;

                    while let Some(&(next_tok, next_start, next_end)) = raw.get(j) {
                        if next_start != merged_end {
                            break;
                        }
                        if !matches!(next_tok, RawToken::Ident | RawToken::Minus) {
                            break;
                        }
                        merged_end = next_end;
                        j += 1;
                    }

                    if j > i + 1 {
                        push_hash_and_ident(&mut tokens, self.input, start, merged_end);
                        i = j;
                        continue;
                    }

                    // The lexer used to promote `#e8eef7` in a value position to
                    // one COLOR token; that CSS domain now lives in the stdlib
                    // colour grammar (`"#" [0-9a-fA-F]{3|4|6|8}`), so a hex stays
                    // HASH + <run> — which is what lets the grammar both match
                    // AND reject `#e8ee1`. Previously a 5-digit hex passed `check`
                    // and landed verbatim in the emitted stylesheet, where the
                    // browser silently dropped the declaration.
                    push_hash_and_ident(&mut tokens, self.input, start, end);
                }
                RawToken::Minus => {
                    // Disambiguation: MINUS vs negative number vs vendor prefix
                    if let Some(&(next_tok, next_start, next_end)) = raw.get(i + 1) {
                        // Immediately adjacent (no whitespace between)?
                        if next_start == end {
                            // -42, -0.3 → negative number
                            if matches!(next_tok, RawToken::Number | RawToken::LeadingDecimalNumber)
                            {
                                // -42 → NUMBER
                                let merged = &self.input[start..next_end];
                                tokens.push(Token {
                                    kind: SyntaxKind::NUMBER,
                                    text: merged.to_string(),
                                    offset: start,
                                });
                                i += 2;
                                continue;
                            }
                            // -webkit-transform → IDENT (vendor prefix)
                            if next_tok == RawToken::Ident {
                                let merged = &self.input[start..next_end];
                                tokens.push(Token {
                                    kind: SyntaxKind::IDENT,
                                    text: merged.to_string(),
                                    offset: start,
                                });
                                i += 2;
                                continue;
                            }
                            // --custom-prop → IDENT (CSS custom property)
                            if next_tok == RawToken::Minus
                                && let Some(&(after_tok, after_start, after_end)) = raw.get(i + 2)
                                && after_start == next_end
                                && after_tok == RawToken::Ident
                            {
                                let merged = &self.input[start..after_end];
                                tokens.push(Token {
                                    kind: SyntaxKind::IDENT,
                                    text: merged.to_string(),
                                    offset: start,
                                });
                                i += 3;
                                continue;
                            }
                            // -> is handled by Arrow, but -MINUS → just MINUS
                        }
                    }
                    // Plain minus
                    tokens.push(Token {
                        kind: SyntaxKind::MINUS,
                        text: text.to_string(),
                        offset: start,
                    });
                }
                RawToken::Hash => {
                    tokens.push(Token {
                        kind: SyntaxKind::HASH,
                        text: text.to_string(),
                        offset: start,
                    });
                }
                // Multi-char operators
                RawToken::Arrow => tokens.push(Token {
                    kind: SyntaxKind::ARROW,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::LeftArrow => tokens.push(Token {
                    kind: SyntaxKind::LEFT_ARROW,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::FatArrow => tokens.push(Token {
                    kind: SyntaxKind::FAT_ARROW,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::AndAnd => tokens.push(Token {
                    kind: SyntaxKind::AND_AND,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::OrOr => tokens.push(Token {
                    kind: SyntaxKind::OR_OR,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::EqEqEq => tokens.push(Token {
                    kind: SyntaxKind::EQ_EQ_EQ,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::NotEqEq => tokens.push(Token {
                    kind: SyntaxKind::NOT_EQ_EQ,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::EqEq => tokens.push(Token {
                    kind: SyntaxKind::EQ_EQ,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::NotEq => tokens.push(Token {
                    kind: SyntaxKind::NOT_EQ,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::LtEq => tokens.push(Token {
                    kind: SyntaxKind::LT_EQ,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::GtEq => tokens.push(Token {
                    kind: SyntaxKind::GT_EQ,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::QuestionQuestion => tokens.push(Token {
                    kind: SyntaxKind::QUESTION_QUESTION,
                    text: text.to_string(),
                    offset: start,
                }),

                // Single-char tokens
                RawToken::Dollar => tokens.push(Token {
                    kind: SyntaxKind::DOLLAR,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Ampersand => tokens.push(Token {
                    kind: SyntaxKind::AMPERSAND,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Tilde => tokens.push(Token {
                    kind: SyntaxKind::TILDE,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::AtSign => tokens.push(Token {
                    kind: SyntaxKind::AT_SIGN,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Percent => tokens.push(Token {
                    kind: SyntaxKind::PERCENT,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Colon => tokens.push(Token {
                    kind: SyntaxKind::COLON,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Semicolon => tokens.push(Token {
                    kind: SyntaxKind::SEMICOLON,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Comma => tokens.push(Token {
                    kind: SyntaxKind::COMMA,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Dot => tokens.push(Token {
                    kind: SyntaxKind::DOT,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Question => tokens.push(Token {
                    kind: SyntaxKind::QUESTION,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Exclaim => tokens.push(Token {
                    kind: SyntaxKind::EXCLAIM,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Equals => tokens.push(Token {
                    kind: SyntaxKind::EQUALS,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Plus => tokens.push(Token {
                    kind: SyntaxKind::PLUS,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Star => tokens.push(Token {
                    kind: SyntaxKind::STAR,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Slash => tokens.push(Token {
                    kind: SyntaxKind::SLASH,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Pipe => tokens.push(Token {
                    kind: SyntaxKind::PIPE,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Caret => tokens.push(Token {
                    kind: SyntaxKind::CARET,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Lt => tokens.push(Token {
                    kind: SyntaxKind::LT,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::Gt => tokens.push(Token {
                    kind: SyntaxKind::GT,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::LParen => tokens.push(Token {
                    kind: SyntaxKind::L_PAREN,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::RParen => tokens.push(Token {
                    kind: SyntaxKind::R_PAREN,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::LBrace => tokens.push(Token {
                    kind: SyntaxKind::L_BRACE,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::RBrace => tokens.push(Token {
                    kind: SyntaxKind::R_BRACE,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::LBracket => tokens.push(Token {
                    kind: SyntaxKind::L_BRACKET,
                    text: text.to_string(),
                    offset: start,
                }),
                RawToken::RBracket => tokens.push(Token {
                    kind: SyntaxKind::R_BRACKET,
                    text: text.to_string(),
                    offset: start,
                }),
            }
            i += 1;
        }

        // EOF token
        tokens.push(Token {
            kind: SyntaxKind::EOF,
            text: String::new(),
            offset: self.input.len(),
        });

        tokens
    }
}

impl Iterator for Lexer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        // Note: Iterator interface is preserved for compatibility but
        // tokenize() is preferred for efficiency.
        if self.input.is_empty() {
            None
        } else {
            let tokens = Lexer::new(self.input).tokenize();
            if tokens.len() <= 1 {
                // Only EOF
                self.input = "";
                None
            } else {
                let tok = tokens[0].clone();
                self.input = &self.input[tok.len()..];
                Some(tok)
            }
        }
    }
}

/// Classify an identifier as a keyword or plain IDENT.
fn keyword_or_ident(text: &str) -> SyntaxKind {
    match text {
        "as" => SyntaxKind::KW_AS,
        "if" => SyntaxKind::KW_IF,
        "else" => SyntaxKind::KW_ELSE,
        "true" => SyntaxKind::KW_TRUE,
        "false" => SyntaxKind::KW_FALSE,
        "in" => SyntaxKind::KW_IN,
        "with" => SyntaxKind::KW_WITH,
        "on" => SyntaxKind::KW_ON,
        "from" => SyntaxKind::KW_FROM,
        "to" => SyntaxKind::KW_TO,
        _ => SyntaxKind::IDENT,
    }
}


fn push_hash_and_ident(tokens: &mut Vec<Token>, input: &str, start: usize, end: usize) {
    tokens.push(Token {
        kind: SyntaxKind::HASH,
        text: "#".to_string(),
        offset: start,
    });
    tokens.push(Token {
        kind: SyntaxKind::IDENT,
        text: input[start + 1..end].to_string(),
        offset: start + 1,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(input: &str) -> Vec<(SyntaxKind, &str)> {
        let lexer = Lexer::new(input);
        let tokens = lexer.tokenize();
        tokens
            .iter()
            .map(|t| (t.kind, &input[t.offset..t.offset + t.text.len()]))
            .collect()
    }

    fn significant(input: &str) -> Vec<(SyntaxKind, &str)> {
        lex(input)
            .into_iter()
            .filter(|(kind, _)| {
                !matches!(
                    kind,
                    SyntaxKind::WHITESPACE | SyntaxKind::COMMENT | SyntaxKind::EOF
                )
            })
            .collect()
    }

    fn lex_single(input: &str) -> (SyntaxKind, String) {
        let tokens = Lexer::new(input).tokenize();
        assert!(
            tokens.len() >= 1,
            "Expected at least one token, got {:?}",
            tokens
        );
        (tokens[0].kind, tokens[0].text.clone())
    }

    mod whitespace_tests {
        use super::*;

        #[test]
        fn spaces() {
            let (kind, text) = lex_single("   ");
            assert_eq!(kind, SyntaxKind::WHITESPACE);
            assert_eq!(text, "   ");
        }

        #[test]
        fn tabs() {
            let (kind, text) = lex_single("\t\t");
            assert_eq!(kind, SyntaxKind::WHITESPACE);
            assert_eq!(text, "\t\t");
        }

        #[test]
        fn newlines() {
            let (kind, text) = lex_single("\n\n");
            assert_eq!(kind, SyntaxKind::WHITESPACE);
            assert_eq!(text, "\n\n");
        }

        #[test]
        fn mixed() {
            let (kind, text) = lex_single("  \t\n  ");
            assert_eq!(kind, SyntaxKind::WHITESPACE);
            assert_eq!(text, "  \t\n  ");
        }
    }

    mod comment_tests {
        use super::*;

        #[test]
        fn line_comment() {
            let (kind, text) = lex_single("// this is a comment");
            assert_eq!(kind, SyntaxKind::COMMENT);
            assert_eq!(text, "// this is a comment");
        }

        #[test]
        fn line_comment_with_newline() {
            let tokens = lex("// comment\ncode");
            assert_eq!(tokens[0], (SyntaxKind::COMMENT, "// comment"));
            assert_eq!(tokens[1], (SyntaxKind::WHITESPACE, "\n"));
            assert_eq!(tokens[2], (SyntaxKind::IDENT, "code"));
        }

        #[test]
        fn block_comment() {
            let (kind, text) = lex_single("/* block comment */");
            assert_eq!(kind, SyntaxKind::COMMENT);
            assert_eq!(text, "/* block comment */");
        }

        #[test]
        fn block_comment_multiline() {
            let (kind, text) = lex_single("/* line1\nline2\nline3 */");
            assert_eq!(kind, SyntaxKind::COMMENT);
            assert_eq!(text, "/* line1\nline2\nline3 */");
        }

        #[test]
        fn unterminated_block_comment() {
            // Should still tokenize as comment (error recovery)
            let (kind, text) = lex_single("/* unterminated");
            assert_eq!(kind, SyntaxKind::COMMENT);
            assert_eq!(text, "/* unterminated");
        }

        /// BUG-224: `/*` in HTML text (a glob in prose) must not swallow the
        /// rest of the file. The blank line after the paragraph ends it, so the
        /// CSS rule below survives — before the fix, everything from `/*` to EOF
        /// was one COMMENT token and the rule silently vanished.
        #[test]
        fn glob_in_html_text_does_not_swallow_following_source() {
            let src = "<p>rendered from process/*.md by md</p>\n\n.box { opacity: 0; }\n";
            let tokens = lex(src);
            // The `/*` may still stand as a lone 2-byte token; what must NOT
            // happen is a comment that spans past it into the following source.
            for (kind, text) in &tokens {
                assert!(
                    *kind != SyntaxKind::COMMENT || text.len() <= 2,
                    "a glob in prose swallowed source into a comment: {text:?}"
                );
            }
            assert!(
                tokens.iter().any(|(_, t)| *t == "opacity"),
                "the rule after the prose glob was swallowed: {tokens:?}"
            );
        }

        /// The blank line is the boundary — a genuine block comment spanning
        /// several CONTIGUOUS lines still lexes as one comment.
        #[test]
        fn multiline_banner_comment_still_lexes_as_one_comment() {
            let src = "/* banner\n   line two\n   line three */\n.box { opacity: 0; }\n";
            let tokens = lex(src);
            assert_eq!(tokens[0].0, SyntaxKind::COMMENT);
            assert!(
                tokens[0].1.ends_with("line three */"),
                "banner comment truncated: {:?}",
                tokens[0].1
            );
        }

        /// A `/* … */` that closes BEFORE any blank line is unaffected.
        #[test]
        fn terminated_comment_before_blank_line_is_a_comment() {
            let src = "/* c */\n\n.box { opacity: 0; }\n";
            let tokens = lex(src);
            assert_eq!(tokens[0], (SyntaxKind::COMMENT, "/* c */"));
        }
    }

    mod identifier_tests {
        use super::*;

        #[test]
        fn simple() {
            let (kind, text) = lex_single("foo");
            assert_eq!(kind, SyntaxKind::IDENT);
            assert_eq!(text, "foo");
        }

        #[test]
        fn with_underscore() {
            let (kind, text) = lex_single("_bar");
            assert_eq!(kind, SyntaxKind::IDENT);
            assert_eq!(text, "_bar");
        }

        #[test]
        fn camel_case() {
            let (kind, text) = lex_single("camelCase");
            assert_eq!(kind, SyntaxKind::IDENT);
            assert_eq!(text, "camelCase");
        }

        #[test]
        fn snake_case() {
            let (kind, text) = lex_single("snake_case");
            assert_eq!(kind, SyntaxKind::IDENT);
            assert_eq!(text, "snake_case");
        }

        #[test]
        fn kebab_case() {
            let (kind, text) = lex_single("kebab-case");
            assert_eq!(kind, SyntaxKind::IDENT);
            assert_eq!(text, "kebab-case");
        }

        #[test]
        fn with_digits() {
            let (kind, text) = lex_single("item123");
            assert_eq!(kind, SyntaxKind::IDENT);
            assert_eq!(text, "item123");
        }

        #[test]
        fn hyphen_prefix() {
            let (kind, text) = lex_single("-webkit-transform");
            assert_eq!(kind, SyntaxKind::IDENT);
            assert_eq!(text, "-webkit-transform");
        }
    }

    mod keyword_tests {
        use super::*;

        #[test]
        fn kw_as() {
            let (kind, _) = lex_single("as");
            assert_eq!(kind, SyntaxKind::KW_AS);
        }

        #[test]
        fn kw_if() {
            let (kind, _) = lex_single("if");
            assert_eq!(kind, SyntaxKind::KW_IF);
        }

        #[test]
        fn kw_else() {
            let (kind, _) = lex_single("else");
            assert_eq!(kind, SyntaxKind::KW_ELSE);
        }

        #[test]
        fn kw_true() {
            let (kind, _) = lex_single("true");
            assert_eq!(kind, SyntaxKind::KW_TRUE);
        }

        #[test]
        fn kw_false() {
            let (kind, _) = lex_single("false");
            assert_eq!(kind, SyntaxKind::KW_FALSE);
        }

        #[test]
        fn kw_in() {
            let (kind, _) = lex_single("in");
            assert_eq!(kind, SyntaxKind::KW_IN);
        }

        #[test]
        fn kw_with() {
            let (kind, _) = lex_single("with");
            assert_eq!(kind, SyntaxKind::KW_WITH);
        }

        #[test]
        fn kw_on() {
            let (kind, _) = lex_single("on");
            assert_eq!(kind, SyntaxKind::KW_ON);
        }

        #[test]
        fn kw_from() {
            let (kind, _) = lex_single("from");
            assert_eq!(kind, SyntaxKind::KW_FROM);
        }

        #[test]
        fn kw_to() {
            let (kind, _) = lex_single("to");
            assert_eq!(kind, SyntaxKind::KW_TO);
        }
    }

    mod string_tests {
        use super::*;

        #[test]
        fn double_quoted() {
            let (kind, text) = lex_single("\"hello world\"");
            assert_eq!(kind, SyntaxKind::STRING);
            assert_eq!(text, "\"hello world\"");
        }

        #[test]
        fn single_quoted() {
            let (kind, text) = lex_single("'single quotes'");
            assert_eq!(kind, SyntaxKind::STRING);
            assert_eq!(text, "'single quotes'");
        }

        #[test]
        fn with_escapes() {
            let (kind, text) = lex_single("\"with \\\"escapes\\\"\"");
            assert_eq!(kind, SyntaxKind::STRING);
            assert_eq!(text, "\"with \\\"escapes\\\"\"");
        }

        #[test]
        fn with_newline_escape() {
            let (kind, text) = lex_single("\"line1\\nline2\"");
            assert_eq!(kind, SyntaxKind::STRING);
            assert_eq!(text, "\"line1\\nline2\"");
        }

        #[test]
        fn empty() {
            let (kind, text) = lex_single("\"\"");
            assert_eq!(kind, SyntaxKind::STRING);
            assert_eq!(text, "\"\"");
        }

        #[test]
        fn unterminated() {
            // Error recovery: the opening quote stands as a LONE token and the rest
            // of the input re-lexes normally — it is not absorbed into a string that
            // runs to EOF.
            //
            // This matches the newline case (`a quote never crosses a newline`,
            // BUG-072) rather than contradicting it: both ends of the input now
            // recover the same way. The old behaviour (swallow to EOF) let a quote
            // inside a foreign-code island — a JS regex char class `/[&<>"']/g` —
            // eat its block's closing brace, reporting "expected R_BRACE" far from
            // the real fault (BUG-040/BUG-041). Neither behaviour yields a valid
            // string; standing as punctuation keeps the following source alive so
            // the surrounding grammar can report the actual error.
            let (kind, text) = lex_single("\"unterminated");
            assert_eq!(kind, SyntaxKind::STRING);
            assert_eq!(text, "\"");
        }
    }

    mod number_tests {
        use super::*;

        #[test]
        fn integer() {
            let (kind, text) = lex_single("42");
            assert_eq!(kind, SyntaxKind::NUMBER);
            assert_eq!(text, "42");
        }

        #[test]
        fn negative() {
            let (kind, text) = lex_single("-17");
            assert_eq!(kind, SyntaxKind::NUMBER);
            assert_eq!(text, "-17");
        }

        #[test]
        fn float() {
            let (kind, text) = lex_single("3.14");
            assert_eq!(kind, SyntaxKind::NUMBER);
            assert_eq!(text, "3.14");
        }

        #[test]
        fn leading_decimal() {
            let (kind, text) = lex_single(".5");
            assert_eq!(kind, SyntaxKind::NUMBER);
            assert_eq!(text, ".5");
        }

        #[test]
        fn negative_float() {
            let (kind, text) = lex_single("-0.123");
            assert_eq!(kind, SyntaxKind::NUMBER);
            assert_eq!(text, "-0.123");
        }
    }

    mod number_with_unit_tests {
        use super::*;

        // PLAN-122 W1.2/W3: the CSS unit/colour domain left the lexer and moved to
        // stdlib grammars (`stdlib/capture-types/css-values.st`). The lexer used to
        // fuse a dimension into a single `NUMBER_WITH_UNIT` token; it no longer
        // recognises a unit list, so `8px` lexes as `NUMBER("8") IDENT("px")` and
        // `50%` as `NUMBER("50") PERCENT`. The old fused-shape assertions in this
        // module retired with that cutover (PLAN-122). What these tests keep is the
        // question they always asked: is a dimension lexed LOSSLESSLY? The stdlib
        // value grammar re-fuses the pieces at match time; the lexer must hand it
        // each number and unit unmodified.

        /// Every non-negative dimension lexes as exactly `NUMBER` + unit, and the
        /// concatenation of all token texts reproduces the source exactly.
        #[test]
        fn dimensions_lex_losslessly_as_number_plus_unit() {
            // (input, number text, unit kind, unit text)
            let cases: &[(&str, &str, SyntaxKind, &str)] = &[
                ("100ms", "100", SyntaxKind::IDENT, "ms"),
                ("2s", "2", SyntaxKind::IDENT, "s"),
                ("500us", "500", SyntaxKind::IDENT, "us"),
                ("100px", "100", SyntaxKind::IDENT, "px"),
                ("1.5em", "1.5", SyntaxKind::IDENT, "em"),
                ("2rem", "2", SyntaxKind::IDENT, "rem"),
                ("50%", "50", SyntaxKind::PERCENT, "%"), // unit is PERCENT, not IDENT
                ("100vh", "100", SyntaxKind::IDENT, "vh"),
                ("100vw", "100", SyntaxKind::IDENT, "vw"),
                ("50vmin", "50", SyntaxKind::IDENT, "vmin"),
                ("50vmax", "50", SyntaxKind::IDENT, "vmax"),
                ("45deg", "45", SyntaxKind::IDENT, "deg"),
                ("3.14rad", "3.14", SyntaxKind::IDENT, "rad"),
                ("0.5turn", "0.5", SyntaxKind::IDENT, "turn"),
                // Spacetime extensions, not CSS — still lexed losslessly.
                ("60fps", "60", SyntaxKind::IDENT, "fps"),
                ("5m", "5", SyntaxKind::IDENT, "m"),
                ("1fr", "1", SyntaxKind::IDENT, "fr"),
                ("10ch", "10", SyntaxKind::IDENT, "ch"),
            ];

            for (input, number, unit_kind, unit) in cases {
                let tokens = significant(input);
                assert_eq!(
                    tokens.len(),
                    2,
                    "dimension {input:?} must lex as exactly NUMBER + unit (got {tokens:?})"
                );
                assert_eq!(
                    tokens[0].0,
                    SyntaxKind::NUMBER,
                    "first token of {input:?} must be NUMBER"
                );
                assert_eq!(tokens[0].1, *number, "number text of {input:?}");
                assert_eq!(tokens[1].0, *unit_kind, "unit kind of {input:?}");
                assert_eq!(tokens[1].1, *unit, "unit text of {input:?}");
                let rebuilt: String = tokens.iter().map(|(_, t)| *t).collect();
                assert_eq!(
                    rebuilt, *input,
                    "concatenating {input:?}'s tokens must reproduce it exactly"
                );
            }
        }

        /// `-45deg`: Kernel still merges the leading MINUS into a negative NUMBER
        /// (`-45`), so the sign travels with the number, not the unit. Lossless.
        #[test]
        fn negative_dimension_lexes_losslessly() {
            let tokens = significant("-45deg");
            assert_eq!(
                tokens,
                vec![(SyntaxKind::NUMBER, "-45"), (SyntaxKind::IDENT, "deg")]
            );
            let rebuilt: String = tokens.iter().map(|(_, t)| *t).collect();
            assert_eq!(rebuilt, "-45deg");
        }
    }

    mod color_tests {
        use super::*;

        // PLAN-122 W1.2/W3: the CSS colour domain left the lexer and moved to
        // stdlib grammars (`stdlib/capture-types/css-values.st`). A hex colour is no
        // longer promoted to a single COLOR token in a value position: `#e8eef7`
        // lexes uniformly as `HASH("#") IDENT("e8eef7")` in every position, and the
        // stdlib value grammar re-fuses the pieces at match time. The old "is this
        // classified as a COLOR?" assertions retired with that cutover (PLAN-122) —
        // the lexer no longer does colour classification.
        //
        // The positional promotion being removed was ALREADY inconsistent
        // (BUG-257): `color: #6b7280` promoted, but `border: 1px solid #232634`
        // did NOT, because `solid` was not a recognised value position — 77 real
        // `solid #hex` sites across the corpus were invisible to every COLOR
        // consumer. Removing positional promotion fixes that class of blind spot.
        //
        // What these tests keep is the question they always asked: is a hex
        // colour in a value context lexed LOSSLESSLY? The stdlib grammar
        // re-fuses the pieces at match time; the lexer must hand it `#` and the
        // hex digits unmodified.

        /// Every value-context hex colour lexes as exactly `HASH` + `IDENT`, and
        /// concatenating all token texts reproduces the source exactly.
        #[test]
        fn hex_colours_lex_losslessly_in_value_contexts() {
            // (input, hash text, hex-digit text)
            let cases: &[(&str, &str, &str)] = &[
                ("color: #fff;", "#", "fff"),            // three digit
                ("color: #aabbcc;", "#", "aabbcc"),     // six digit
                ("color: #11223344;", "#", "11223344"), // eight digit
                ("color: #AABBCC;", "#", "AABBCC"),     // uppercase
                ("color: #AaBbCc;", "#", "AaBbCc"),     // mixed case
            ];

            for (input, hash, digits) in cases {
                let tokens = lex(input);
                let pos = tokens
                    .iter()
                    .position(|(kind, _)| *kind == SyntaxKind::HASH)
                    .unwrap_or_else(|| panic!("{input:?} must lex a HASH"));
                assert_eq!(tokens[pos], (SyntaxKind::HASH, *hash), "hash of {input:?}");
                assert_eq!(
                    tokens[pos + 1],
                    (SyntaxKind::IDENT, *digits),
                    "hex digits of {input:?}"
                );
                let rebuilt: String = tokens.iter().map(|(_, t)| *t).collect();
                assert_eq!(
                    rebuilt, *input,
                    "concatenating {input:?}'s tokens must reproduce it exactly"
                );
            }
        }

        /// Multi-value and value-function positions hand every hex colour to the
        /// grammar as `HASH` + `IDENT`, unmodified — no promotion, no swallowing.
        #[test]
        fn hex_colours_survive_in_multi_value_and_function_args() {
            let multi = lex(".swatch { border-color: #cafe #beef; }");
            let pairs: Vec<(&str, &str)> = multi
                .windows(2)
                .filter(|w| w[0].0 == SyntaxKind::HASH && w[1].0 == SyntaxKind::IDENT)
                .map(|w| (w[0].1, w[1].1))
                .collect();
            assert_eq!(pairs, vec![("#", "cafe"), ("#", "beef")]);
            let rebuilt: String = multi.iter().map(|(_, t)| *t).collect();
            assert_eq!(rebuilt, ".swatch { border-color: #cafe #beef; }");

            let gradient = lex(".swatch { background: linear-gradient(#cafe, #beef); }");
            let pairs: Vec<(&str, &str)> = gradient
                .windows(2)
                .filter(|w| w[0].0 == SyntaxKind::HASH && w[1].0 == SyntaxKind::IDENT)
                .map(|w| (w[0].1, w[1].1))
                .collect();
            assert_eq!(pairs, vec![("#", "cafe"), ("#", "beef")]);
            let rebuilt: String = gradient.iter().map(|(_, t)| *t).collect();
            assert_eq!(
                rebuilt,
                ".swatch { background: linear-gradient(#cafe, #beef); }"
            );
        }
    }

    mod hex_id_ambiguity_tests {
        use super::*;

        #[test]
        fn plain_hash_hex_defaults_to_id_selector() {
            let tokens = lex("#fff");
            assert_eq!(tokens[0], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "fff"));
        }

        #[test]
        fn hash_hex_selector_head_uses_hash_ident() {
            let tokens = significant("#cafe {}");
            assert_eq!(tokens[0], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "cafe"));
            assert_eq!(tokens[2], (SyntaxKind::L_BRACE, "{"));
        }

        #[test]
        fn hash_hex_descendant_selector_uses_hash_ident() {
            let tokens = significant("div #cafe {}");
            assert_eq!(tokens[0], (SyntaxKind::IDENT, "div"));
            assert_eq!(tokens[1], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[2], (SyntaxKind::IDENT, "cafe"));
        }

        #[test]
        fn hash_hex_selector_list_uses_hash_ident() {
            let tokens = significant("#cafe, #beef {}");
            assert_eq!(tokens[0], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "cafe"));
            assert_eq!(tokens[2], (SyntaxKind::COMMA, ","));
            assert_eq!(tokens[3], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[4], (SyntaxKind::IDENT, "beef"));
        }

        #[test]
        fn adjacent_minus_ident_continuation_merges_into_id_selector() {
            let tokens = lex("#abc-def");
            assert_eq!(tokens[0], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "abc-def"));
        }

        #[test]
        fn adjacent_ident_continuation_merges_into_id_selector() {
            let tokens = lex("#abcfoo");
            assert_eq!(tokens[0], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "abcfoo"));
        }

        #[test]
        fn adjacent_continuation_case_cosmetics() {
            let tokens = lex("#case-cosmetics");
            assert_eq!(tokens[0], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "case-cosmetics"));
        }

        #[test]
        fn adjacent_continuation_beef_section() {
            let tokens = lex("#beef-section");
            assert_eq!(tokens[0], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "beef-section"));
        }

        #[test]
        fn adjacent_continuation_digit_prefix() {
            let tokens = lex("#1st-item");
            assert_eq!(tokens[0], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "1st-item"));
        }

        // PLAN-122 W1.2/W3: the `#id`-selector vs `#hex` colour ambiguity
        // DISSOLVES. `#fff` used to promote to one COLOR token
        // inside a value function, while a selector stayed HASH + IDENT — the
        // lexer decided which was which by position. Now both are the same
        // `HASH("#") IDENT("fff")` shape in every position, and the consumer
        // (a stdlib grammar, not the lexer) decides. This test asserts the two
        // positions agree instead of silently deleting the coverage.
        #[test]
        fn value_function_colour_lexes_same_shape_as_id_selector() {
            let value = significant("background: linear-gradient(#fff);");
            let selector = significant("#fff");

            let hash_pos = value
                .iter()
                .position(|(kind, _)| *kind == SyntaxKind::HASH)
                .expect("value function must contain a HASH");
            assert_eq!(value[hash_pos], (SyntaxKind::HASH, "#"));
            assert_eq!(value[hash_pos + 1], (SyntaxKind::IDENT, "fff"));
            // Byte-identical token shape to a bare `#fff` id selector.
            assert_eq!(&value[hash_pos..hash_pos + 2], &selector[..]);
        }

        #[test]
        fn pseudo_not_argument_uses_hash_ident() {
            let tokens = lex(":not(#cafe)");
            assert_eq!(tokens[0], (SyntaxKind::COLON, ":"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "not"));
            assert_eq!(tokens[2], (SyntaxKind::L_PAREN, "("));
            assert_eq!(tokens[3], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[4], (SyntaxKind::IDENT, "cafe"));
            assert_eq!(tokens[5], (SyntaxKind::R_PAREN, ")"));
        }

        #[test]
        fn pseudo_is_argument_uses_hash_ident() {
            let tokens = significant(":is(.foo, #cafe)");
            assert_eq!(tokens[0], (SyntaxKind::COLON, ":"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "is"));
            assert_eq!(tokens[2], (SyntaxKind::L_PAREN, "("));
            assert_eq!(tokens[3], (SyntaxKind::DOT, "."));
            assert_eq!(tokens[4], (SyntaxKind::IDENT, "foo"));
            assert_eq!(tokens[5], (SyntaxKind::COMMA, ","));
            assert_eq!(tokens[6], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[7], (SyntaxKind::IDENT, "cafe"));
            assert_eq!(tokens[8], (SyntaxKind::R_PAREN, ")"));
        }

        #[test]
        fn pseudo_has_argument_uses_hash_ident() {
            let tokens = lex(":has(#123abc)");
            assert_eq!(tokens[3], (SyntaxKind::HASH, "#"));
            assert_eq!(tokens[4], (SyntaxKind::IDENT, "123abc"));
        }

        #[test]
        fn hex_prefixed_id_scope_parses() {
            let parsed = crate::syntax::cst::parse("#cafe { color: red; }");
            assert!(
                parsed.errors.is_empty(),
                "Parse errors: {:?}",
                parsed.errors
            );
        }
    }

    mod operator_tests {
        use super::*;

        #[test]
        fn arrow() {
            let (kind, text) = lex_single("->");
            assert_eq!(kind, SyntaxKind::ARROW);
            assert_eq!(text, "->");
        }

        #[test]
        fn fat_arrow() {
            let (kind, text) = lex_single("=>");
            assert_eq!(kind, SyntaxKind::FAT_ARROW);
            assert_eq!(text, "=>");
        }

        #[test]
        fn and_and() {
            let (kind, text) = lex_single("&&");
            assert_eq!(kind, SyntaxKind::AND_AND);
            assert_eq!(text, "&&");
        }

        #[test]
        fn or_or() {
            let (kind, text) = lex_single("||");
            assert_eq!(kind, SyntaxKind::OR_OR);
            assert_eq!(text, "||");
        }

        #[test]
        fn eq_eq() {
            let (kind, text) = lex_single("==");
            assert_eq!(kind, SyntaxKind::EQ_EQ);
            assert_eq!(text, "==");
        }

        #[test]
        fn not_eq() {
            let (kind, text) = lex_single("!=");
            assert_eq!(kind, SyntaxKind::NOT_EQ);
            assert_eq!(text, "!=");
        }

        #[test]
        fn lt_eq() {
            let (kind, text) = lex_single("<=");
            assert_eq!(kind, SyntaxKind::LT_EQ);
            assert_eq!(text, "<=");
        }

        #[test]
        fn gt_eq() {
            let (kind, text) = lex_single(">=");
            assert_eq!(kind, SyntaxKind::GT_EQ);
            assert_eq!(text, ">=");
        }

        #[test]
        fn eq_eq_eq() {
            let (kind, text) = lex_single("===");
            assert_eq!(kind, SyntaxKind::EQ_EQ_EQ);
            assert_eq!(text, "===");
        }

        #[test]
        fn not_eq_eq() {
            let (kind, text) = lex_single("!==");
            assert_eq!(kind, SyntaxKind::NOT_EQ_EQ);
            assert_eq!(text, "!==");
        }

        #[test]
        fn eq_eq_not_eq_eq_eq() {
            // "==" should still lex as EQ_EQ (not partial EQ_EQ_EQ)
            let (kind, text) = lex_single("==");
            assert_eq!(kind, SyntaxKind::EQ_EQ);
            assert_eq!(text, "==");
        }
    }

    mod prefix_tests {
        use super::*;

        #[test]
        fn dollar() {
            let (kind, _) = lex_single("$");
            assert_eq!(kind, SyntaxKind::DOLLAR);
        }

        #[test]
        fn ampersand() {
            let (kind, _) = lex_single("&");
            assert_eq!(kind, SyntaxKind::AMPERSAND);
        }

        #[test]
        fn tilde() {
            let (kind, _) = lex_single("~");
            assert_eq!(kind, SyntaxKind::TILDE);
        }

        #[test]
        fn at_sign() {
            let (kind, _) = lex_single("@");
            assert_eq!(kind, SyntaxKind::AT_SIGN);
        }

        #[test]
        fn percent() {
            let (kind, _) = lex_single("%");
            assert_eq!(kind, SyntaxKind::PERCENT);
        }

        #[test]
        fn hash_without_hex() {
            // Hash not followed by hex digit
            let (kind, _) = lex_single("#");
            assert_eq!(kind, SyntaxKind::HASH);
        }
    }

    mod punctuation_tests {
        use super::*;

        #[test]
        fn colon() {
            let (kind, _) = lex_single(":");
            assert_eq!(kind, SyntaxKind::COLON);
        }

        #[test]
        fn semicolon() {
            let (kind, _) = lex_single(";");
            assert_eq!(kind, SyntaxKind::SEMICOLON);
        }

        #[test]
        fn comma() {
            let (kind, _) = lex_single(",");
            assert_eq!(kind, SyntaxKind::COMMA);
        }

        #[test]
        fn dot() {
            let (kind, _) = lex_single(".");
            assert_eq!(kind, SyntaxKind::DOT);
        }

        #[test]
        fn question() {
            let (kind, _) = lex_single("?");
            assert_eq!(kind, SyntaxKind::QUESTION);
        }

        #[test]
        fn exclaim() {
            let (kind, _) = lex_single("!");
            assert_eq!(kind, SyntaxKind::EXCLAIM);
        }

        #[test]
        fn equals() {
            let (kind, _) = lex_single("=");
            assert_eq!(kind, SyntaxKind::EQUALS);
        }

        #[test]
        fn plus() {
            let (kind, _) = lex_single("+");
            assert_eq!(kind, SyntaxKind::PLUS);
        }

        #[test]
        fn minus() {
            let (kind, _) = lex_single("-");
            assert_eq!(kind, SyntaxKind::MINUS);
        }

        #[test]
        fn star() {
            let (kind, _) = lex_single("*");
            assert_eq!(kind, SyntaxKind::STAR);
        }

        #[test]
        fn slash() {
            let (kind, _) = lex_single("/");
            assert_eq!(kind, SyntaxKind::SLASH);
        }

        #[test]
        fn pipe() {
            let (kind, _) = lex_single("|");
            assert_eq!(kind, SyntaxKind::PIPE);
        }

        #[test]
        fn caret() {
            let (kind, _) = lex_single("^");
            assert_eq!(kind, SyntaxKind::CARET);
        }

        #[test]
        fn lt() {
            let (kind, _) = lex_single("<");
            assert_eq!(kind, SyntaxKind::LT);
        }

        #[test]
        fn gt() {
            let (kind, _) = lex_single(">");
            assert_eq!(kind, SyntaxKind::GT);
        }
    }

    mod delimiter_tests {
        use super::*;

        #[test]
        fn parens() {
            let tokens = lex("()");
            assert_eq!(tokens[0], (SyntaxKind::L_PAREN, "("));
            assert_eq!(tokens[1], (SyntaxKind::R_PAREN, ")"));
        }

        #[test]
        fn braces() {
            let tokens = lex("{}");
            assert_eq!(tokens[0], (SyntaxKind::L_BRACE, "{"));
            assert_eq!(tokens[1], (SyntaxKind::R_BRACE, "}"));
        }

        #[test]
        fn brackets() {
            let tokens = lex("[]");
            assert_eq!(tokens[0], (SyntaxKind::L_BRACKET, "["));
            assert_eq!(tokens[1], (SyntaxKind::R_BRACKET, "]"));
        }
    }

    mod integration_tests {
        use super::*;

        #[test]
        fn variable_reference() {
            let tokens = lex("$myVar");
            assert_eq!(tokens[0], (SyntaxKind::DOLLAR, "$"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "myVar"));
        }

        #[test]
        fn element_reference() {
            let tokens = lex("&button.opacity");
            assert_eq!(tokens[0], (SyntaxKind::AMPERSAND, "&"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "button"));
            assert_eq!(tokens[2], (SyntaxKind::DOT, "."));
            assert_eq!(tokens[3], (SyntaxKind::IDENT, "opacity"));
        }

        #[test]
        fn preset_reference() {
            let tokens = lex("~smooth");
            assert_eq!(tokens[0], (SyntaxKind::TILDE, "~"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "smooth"));
        }

        #[test]
        fn directive() {
            let tokens = lex("@animate(fade-in, 500ms)");
            assert_eq!(tokens[0], (SyntaxKind::AT_SIGN, "@"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "animate"));
            assert_eq!(tokens[2], (SyntaxKind::L_PAREN, "("));
            assert_eq!(tokens[3], (SyntaxKind::IDENT, "fade-in"));
            assert_eq!(tokens[4], (SyntaxKind::COMMA, ","));
            assert_eq!(tokens[5], (SyntaxKind::WHITESPACE, " "));
            // A dimension is two tokens at Kernel tier (PLAN-122 W1.2/W3): the
            // unit is its own IDENT, and `%capture_type duration` in stdlib is
            // what decides `ms` is a legal one.
            assert_eq!(tokens[6], (SyntaxKind::NUMBER, "500"));
            assert_eq!(tokens[7], (SyntaxKind::IDENT, "ms"));
            assert_eq!(tokens[8], (SyntaxKind::R_PAREN, ")"));
        }

        #[test]
        fn css_property() {
            let tokens = lex("opacity: 0 -> 1;");
            assert_eq!(tokens[0], (SyntaxKind::IDENT, "opacity"));
            assert_eq!(tokens[1], (SyntaxKind::COLON, ":"));
            assert_eq!(tokens[2], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[3], (SyntaxKind::NUMBER, "0"));
            assert_eq!(tokens[4], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[5], (SyntaxKind::ARROW, "->"));
            assert_eq!(tokens[6], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[7], (SyntaxKind::NUMBER, "1"));
            assert_eq!(tokens[8], (SyntaxKind::SEMICOLON, ";"));
        }

        #[test]
        fn scope_block() {
            let tokens = lex(".hero { }");
            assert_eq!(tokens[0], (SyntaxKind::DOT, "."));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "hero"));
            assert_eq!(tokens[2], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[3], (SyntaxKind::L_BRACE, "{"));
            assert_eq!(tokens[4], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[5], (SyntaxKind::R_BRACE, "}"));
        }

        #[test]
        fn meta_def() {
            let tokens = lex("%macro { }");
            assert_eq!(tokens[0], (SyntaxKind::PERCENT, "%"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "macro"));
            assert_eq!(tokens[2], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[3], (SyntaxKind::L_BRACE, "{"));
            assert_eq!(tokens[4], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[5], (SyntaxKind::R_BRACE, "}"));
        }

        #[test]
        fn expression() {
            let tokens = lex("$a + $b * 2");
            assert_eq!(tokens[0], (SyntaxKind::DOLLAR, "$"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "a"));
            assert_eq!(tokens[2], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[3], (SyntaxKind::PLUS, "+"));
            assert_eq!(tokens[4], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[5], (SyntaxKind::DOLLAR, "$"));
            assert_eq!(tokens[6], (SyntaxKind::IDENT, "b"));
            assert_eq!(tokens[7], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[8], (SyntaxKind::STAR, "*"));
            assert_eq!(tokens[9], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[10], (SyntaxKind::NUMBER, "2"));
        }

        #[test]
        fn comparison() {
            let tokens = lex("$x >= 10 && $x <= 100");
            assert_eq!(tokens[0], (SyntaxKind::DOLLAR, "$"));
            assert_eq!(tokens[1], (SyntaxKind::IDENT, "x"));
            assert_eq!(tokens[2], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[3], (SyntaxKind::GT_EQ, ">="));
            assert_eq!(tokens[4], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[5], (SyntaxKind::NUMBER, "10"));
            assert_eq!(tokens[6], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[7], (SyntaxKind::AND_AND, "&&"));
        }
    }

    mod css_custom_property_tests {
        use super::*;

        #[test]
        fn test_css_custom_property_double_dash() {
            let lexer = Lexer::new("--st-bg: #FAF7F2;");
            let tokens = lexer.tokenize();
            // First token should be IDENT("--st-bg"), not MINUS + IDENT("-st-bg")
            assert_eq!(tokens[0].kind, SyntaxKind::IDENT);
            assert_eq!(tokens[0].text, "--st-bg");
        }

        #[test]
        fn test_css_custom_property_simple() {
            let (kind, text) = lex_single("--color");
            assert_eq!(kind, SyntaxKind::IDENT);
            assert_eq!(text, "--color");
        }

        #[test]
        fn test_css_custom_property_multi_hyphen() {
            let (kind, text) = lex_single("--my-var");
            assert_eq!(kind, SyntaxKind::IDENT);
            assert_eq!(text, "--my-var");
        }

        #[test]
        fn test_css_custom_property_in_declaration() {
            let tokens = lex("--st-bg: red;");
            assert_eq!(tokens[0], (SyntaxKind::IDENT, "--st-bg"));
            assert_eq!(tokens[1], (SyntaxKind::COLON, ":"));
        }

        #[test]
        fn test_vendor_prefix_still_works() {
            // Ensure vendor prefixes still merge correctly
            let (kind, text) = lex_single("-webkit-transform");
            assert_eq!(kind, SyntaxKind::IDENT);
            assert_eq!(text, "-webkit-transform");
        }

        #[test]
        fn test_double_dash_with_space_does_not_merge() {
            // "-- foo" should NOT merge into a custom property
            let tokens = lex("-- foo");
            assert_eq!(tokens[0], (SyntaxKind::MINUS, "-"));
            assert_eq!(tokens[1], (SyntaxKind::MINUS, "-"));
            assert_eq!(tokens[2], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[3], (SyntaxKind::IDENT, "foo"));
        }

        #[test]
        fn test_dash_space_dash_does_not_merge() {
            // "- -foo" should NOT merge
            let tokens = lex("- -foo");
            assert_eq!(tokens[0], (SyntaxKind::MINUS, "-"));
            assert_eq!(tokens[1], (SyntaxKind::WHITESPACE, " "));
            assert_eq!(tokens[2], (SyntaxKind::IDENT, "-foo"));
        }

        #[test]
        fn test_css_custom_property_var_reference() {
            // var(--my-color) usage
            let tokens = lex("var(--my-color)");
            assert_eq!(tokens[0], (SyntaxKind::IDENT, "var"));
            assert_eq!(tokens[1], (SyntaxKind::L_PAREN, "("));
            assert_eq!(tokens[2], (SyntaxKind::IDENT, "--my-color"));
            assert_eq!(tokens[3], (SyntaxKind::R_PAREN, ")"));
        }
    }

    mod corpus_lossless_tests {
        use super::*;
        use std::path::{Path, PathBuf};

        /// Every `.st` file under a directory (recursive).
        fn st_files(root: &Path, out: &mut Vec<PathBuf>) {
            let Ok(entries) = std::fs::read_dir(root) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    st_files(&path, out);
                } else if path.extension().map(|e| e == "st").unwrap_or(false) {
                    out.push(path);
                }
            }
        }

        /// The corpus: everything the compiler is expected to lex.
        fn corpus() -> Vec<PathBuf> {
            let mut files = Vec::new();
            for dir in ["stdlib", "demos", "examples", "tests/fixtures"] {
                st_files(Path::new(dir), &mut files);
            }
            files.sort();
            assert!(
                files.len() > 300,
                "corpus collapsed to {} files — the walker is looking in the wrong place, \
                 and a losslessness test over an empty corpus proves nothing",
                files.len()
            );
            files
        }

        /// Lexing reproduces the source byte-for-byte over the whole corpus.
        ///
        /// The load-bearing invariant inherited from the tier cutover (PLAN-122):
        /// the CSS value domain moved out of the lexer and into stdlib grammars, so
        /// a hex stays `HASH` + `IDENT` and a dimension `NUMBER` + `IDENT` — the
        /// lexer SPLITS tokens it used to fuse. Splitting must never rewrite, drop,
        /// or reorder a byte; otherwise every stdlib value grammar built on the
        /// split shapes would inherit the damage. Checked over the real corpus
        /// because hand-written samples systematically miss the shapes real code
        /// contains.
        #[test]
        fn lexing_is_byte_lossless_over_the_corpus() {
            let mut failures = Vec::new();

            for file in corpus() {
                let Ok(source) = std::fs::read_to_string(&file) else {
                    continue;
                };
                let rebuilt: String = Lexer::new(&source)
                    .tokenize()
                    .into_iter()
                    .filter(|t| t.kind != SyntaxKind::EOF)
                    .map(|t| t.text.clone())
                    .collect();
                if rebuilt != source {
                    failures.push(file.display().to_string());
                }
            }

            assert!(
                failures.is_empty(),
                "lexing is not byte-lossless in {} case(s):\n  {}\n\n\
                 A lexer that loses or rewrites text cannot support value grammars — \
                 every capture built on it would inherit the damage.",
                failures.len(),
                failures.join("\n  ")
            );
        }

        /// The demoted shapes are the ones a value grammar can consume, and their
        /// kinds are honest.
        ///
        /// `#e8eef7` and `#123456` are the same KIND of value but lex differently
        /// once split (IDENT vs a run that is still one IDENT), because the lexer's
        /// regexes are about characters, not about CSS. That is why the FEAT-164
        /// counted terminal counts characters rather than matching token kinds —
        /// this test pins the shapes it has to cope with.
        #[test]
        fn demoted_shapes_are_what_a_value_grammar_can_match() {
            let cases: &[(&str, &[(SyntaxKind, &str)])] = &[
                // A hex colour in value position: the case the whole cutover existed
                // for.
                (
                    "a { color: #e8eef7; }",
                    &[
                        (SyntaxKind::HASH, "#"),
                        (SyntaxKind::IDENT, "e8eef7"),
                        (SyntaxKind::SEMICOLON, ";"),
                    ],
                ),
                // An all-digit hex demotes to HASH + IDENT too, NOT to HASH + NUMBER.
                // The `#…` run is lexed by the COLOR regex as one unit before any
                // splitting, so its remainder is one span of characters whatever they
                // happen to be. Re-lexing that span so all-digit runs became NUMBER
                // was tried and REVERTED: it widened the tier delta beyond the two
                // fused kinds, and nothing needs it, because the counted-class
                // terminal scans bytes from a token start.
                (
                    "a { color: #123456; }",
                    &[
                        (SyntaxKind::HASH, "#"),
                        (SyntaxKind::IDENT, "123456"),
                        (SyntaxKind::SEMICOLON, ";"),
                    ],
                ),
                // A length: two tokens the grammar can pair, instead of one fused
                // atom.
                (
                    "a { width: 8px; }",
                    &[
                        (SyntaxKind::NUMBER, "8"),
                        (SyntaxKind::IDENT, "px"),
                        (SyntaxKind::SEMICOLON, ";"),
                    ],
                ),
                // A percentage keeps its own punctuation rather than being absorbed.
                (
                    "a { width: 50%; }",
                    &[
                        (SyntaxKind::NUMBER, "50"),
                        (SyntaxKind::PERCENT, "%"),
                        (SyntaxKind::SEMICOLON, ";"),
                    ],
                ),
                // A 5-digit hex — illegal in CSS in every spelling. It used to be
                // promoted to a COLOR token and sail through to the emitted
                // stylesheet. Now it is an ordinary run of characters a counted
                // grammar can measure and REFUSE. This is the diagnostic the cutover
                // buys.
                (
                    "a { color: #e8ee1; }",
                    &[
                        (SyntaxKind::HASH, "#"),
                        (SyntaxKind::IDENT, "e8ee1"),
                        (SyntaxKind::SEMICOLON, ";"),
                    ],
                ),
            ];

            for (source, expected_tail) in cases {
                let flat = significant(source);
                let found = flat.windows(expected_tail.len()).any(|w| w == *expected_tail);
                assert!(
                    found,
                    "{source:?} did not contain {expected_tail:?}\n  got: {flat:?}"
                );
            }
        }
    }

}

#[cfg(test)]
mod block_comment_tests {
    //! BUG-225 (and BUG-224's own protection, which shipped untested).
    //!
    //! Two goals that were originally traded against each other:
    //!   - an UNTERMINATED `/*` in prose must not swallow the rest of the file
    //!   - a CLOSED `/* … */` must remain a comment however many blank lines it spans
    //!
    //! The first rule applied unconditionally, so a documentation banner with
    //! paragraphs stopped being a comment. That broke
    //! `stdlib/__mcp__/workbench/styles.st`, and with it 7 MCP workbench tests.

    /// A closed banner comment spanning a blank line is STILL a comment, and the
    /// CSS that follows it must survive. This is the BUG-225 regression.
    #[test]
    fn a_closed_block_comment_may_span_a_blank_line() {
        let src = "/* Banner headline\n   first paragraph.\n\n   second paragraph after a blank line. */\n.after { color: red; }\n";
        let file = crate::parser::parse(src).expect("a closed banner comment must parse");
        assert_eq!(
            file.scopes.len(),
            1,
            "the rule after a multi-paragraph banner must survive — treating the \
             banner as prose punctuation is what broke the MCP workbench"
        );
    }

    /// The real file that regressed. Guards the exact source, not just its shape.
    #[test]
    fn the_mcp_workbench_stylesheet_parses() {
        let path = std::path::Path::new("stdlib/__mcp__/workbench/styles.st");
        if !path.exists() {
            return; // embedded-stdlib builds run without the source tree
        }
        let src = std::fs::read_to_string(path).expect("read styles.st");
        assert!(
            crate::parser::parse(&src).is_ok(),
            "stdlib/__mcp__/workbench/styles.st must parse — when it does not, the \
             workbench silently fails to mount and 7 MCP tests fail with a null \
             function name"
        );
    }

    /// BUG-224's protection, which must not regress: an UNTERMINATED `/*` in prose
    /// stands as punctuation and the following rules stay alive.
    /// BUG-224's protection, which must not regress: an UNTERMINATED `/*` followed
    /// by a blank line stands as punctuation and the following rules stay alive.
    ///
    /// NOTE on the input. An earlier version of this test used `/*` inside HTML
    /// TEXT (`<p>see process/*.md</p>`) — which never reaches this callback at all,
    /// because HTML text is lexed separately. It therefore passed with the rule
    /// disabled and proved nothing. The inputs below are ones where the `/*` is
    /// genuinely lexed as a block-comment start.
    #[test]
    fn an_unterminated_block_comment_before_a_blank_line_does_not_eat_the_file() {
        // Bare, at statement position, unterminated, blank line before the rule.
        let file = crate::parser::parse("/*\n\n.after { color: red; }\n").expect("must parse");
        assert_eq!(
            file.scopes.len(),
            1,
            "the rule after an unterminated `/*` must survive — swallowing to EOF \
             deletes every style after it with no error (BUG-224)"
        );
        assert_eq!(
            file.scopes[0].selector, ".after",
            "and it must be the REAL scope, not a recovery artifact"
        );

        // Same, following a complete rule.
        let file2 = crate::parser::parse(".a { color: blue; }\n/*\n\n.after { color: red; }\n")
            .expect("must parse");
        assert_eq!(
            file2.scopes.len(),
            2,
            "both the preceding and following rules must survive"
        );
    }

    /// The remaining case: unterminated AND no blank line. Consume to EOF as error
    /// recovery, which is the pre-BUG-224 behaviour for a genuinely broken comment.
    #[test]
    fn an_unterminated_comment_without_a_blank_line_is_error_recovery() {
        let src = "/* never closed\n.after { color: red; }\n";
        let file = crate::parser::parse(src).expect("must parse via recovery");
        assert_eq!(
            file.scopes.len(),
            0,
            "with no blank line to mark it as prose, an unterminated comment is \
             treated as a comment to EOF"
        );
    }
}
