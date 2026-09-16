//! Rowan-based parser for Spacetime DSL
//!
//! This parser builds a lossless CST using Rowan's green tree infrastructure.
//! It supports error recovery by wrapping invalid syntax in ERROR nodes.

use rowan::GreenNodeBuilder;

use super::lexer::{Lexer, Token};
use super::{SyntaxKind, SyntaxNode};

/// Parse result containing the syntax tree and any errors
#[derive(Debug)]
pub struct ParseResult {
    /// The root syntax node
    pub root: SyntaxNode,
    /// Parse errors encountered
    pub errors: Vec<ParseError>,
}

/// A parse error with location information
#[derive(Debug, Clone)]
pub struct ParseError {
    /// Error message
    pub message: String,
    /// Byte offset in the source
    pub offset: usize,
    /// Length of the error span
    pub len: usize,
}

/// Parser state
struct Parser {
    /// Tokens from the lexer
    tokens: Vec<Token>,
    /// Raw source text (needed to capture exact HTML spans for html5ever; PLAN-023 W0)
    source: String,
    /// Current position in the token stream
    pos: usize,
    /// Green tree builder
    builder: GreenNodeBuilder<'static>,
    /// Collected errors
    errors: Vec<ParseError>,
}

impl Parser {
    fn new(tokens: Vec<Token>, source: String) -> Self {
        Parser {
            tokens,
            source,
            pos: 0,
            builder: GreenNodeBuilder::new(),
            errors: Vec::new(),
        }
    }

    /// Returns the current token kind (EOF if at end)
    fn current(&self) -> SyntaxKind {
        self.tokens
            .get(self.pos)
            .map(|t| t.kind)
            .unwrap_or(SyntaxKind::EOF)
    }

    /// Returns the current token (or None if at end)
    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    /// Text of the current token, if any.
    fn current_text(&self) -> Option<&str> {
        self.current_token().map(|t| t.text.as_str())
    }

    /// Peek at the nth token ahead (0 = current)
    fn peek(&self, n: usize) -> SyntaxKind {
        self.tokens
            .get(self.pos + n)
            .map(|t| t.kind)
            .unwrap_or(SyntaxKind::EOF)
    }

    /// Kind of the nearest non-trivia token BEFORE the current position
    /// (lookback sibling of `peek_non_trivia`).
    fn prev_non_trivia(&self) -> SyntaxKind {
        let mut i = self.pos;
        while i > 0 {
            i -= 1;
            let kind = self
                .tokens
                .get(i)
                .map(|t| t.kind)
                .unwrap_or(SyntaxKind::EOF);
            if kind == SyntaxKind::EOF {
                return SyntaxKind::EOF;
            }
            if !kind.is_trivia() {
                return kind;
            }
        }
        SyntaxKind::EOF
    }

    /// Kind of the `n`-th NON-TRIVIA token at or after the current position
    /// (n = 0 is the current non-trivia token). Skips whitespace/comments so a
    /// lookahead like "is the token after `{` a STRING" is not defeated by the
    /// space in `{ "key"`.
    fn peek_non_trivia(&self, n: usize) -> SyntaxKind {
        let mut seen = 0;
        let mut i = self.pos;
        while let Some(t) = self.tokens.get(i) {
            if !t.kind.is_trivia() {
                if seen == n {
                    return t.kind;
                }
                seen += 1;
            }
            i += 1;
        }
        SyntaxKind::EOF
    }

    /// Returns true if at end of file
    fn at_eof(&self) -> bool {
        self.current() == SyntaxKind::EOF
    }

    /// Returns true if current token is the given kind
    fn at(&self, kind: SyntaxKind) -> bool {
        self.current() == kind
    }

    /// Returns true if current token is any of the given kinds
    fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        kinds.contains(&self.current())
    }

    /// Consume the current token and add it to the tree
    fn bump(&mut self) {
        if let Some(token) = self.tokens.get(self.pos) {
            self.builder
                .token(rowan::SyntaxKind(token.kind.into()), &token.text);
            self.pos += 1;
        }
    }

    /// Consume a value atom, keeping a dimension's parts together.
    ///
    /// Adjacency is what distinguishes a dimension from two separate values:
    /// `40px` is one value, `40 px` is a number followed by an identifier. The
    /// old lexer answered this by fusing the pair into `NUMBER_WITH_UNIT` and
    /// keeping a hardcoded unit list; at Kernel tier (PLAN-122 W1.2) the pair
    /// stays split, so the question moves here — as pure POSITION, with no
    /// opinion about which units exist. That opinion now lives in
    /// `stdlib/capture-types/css-values.st`, once.
    ///
    /// At Kernel tier `300ms` arrives as `NUMBER("300") IDENT("ms")` and `50%`
    /// as `NUMBER("50") PERCENT`. Every position that used to accept a single
    /// `NUMBER_WITH_UNIT` token must now accept that pair, or it stops at the
    /// number and reports the unit as unexpected — which is exactly how
    /// `duration: time = 300ms` began failing with "expected R_PAREN, found
    /// IDENT".
    ///
    /// Only an ADJACENT suffix is absorbed, so `1 solid` (two values) is
    /// unaffected while `1px` stays whole. A hex colour needs no equivalent:
    /// `#e8eef7` is `HASH IDENT`, and the positions that accept a colour
    /// already accept a leading `HASH`.
    fn bump_value_atom(&mut self) {
        self.bump();
        if self.next_is_adjacent_unit() {
            self.bump();
        }
    }

    /// True when the CURRENT token is an adjacent unit suffix for the value just
    /// consumed: an identifier (`px`, `ms`, `deg`) or a percent sign.
    ///
    /// Checked against the PREVIOUS token's end, since `bump_value_atom` calls
    /// this after consuming the number.
    fn next_is_adjacent_unit(&self) -> bool {
        if self.pos == 0 {
            return false;
        }
        let Some(prev) = self.tokens.get(self.pos - 1) else {
            return false;
        };
        let Some(cur) = self.tokens.get(self.pos) else {
            return false;
        };
        prev.offset + prev.len() == cur.offset
            && matches!(cur.kind, SyntaxKind::IDENT | SyntaxKind::PERCENT)
    }

    /// Consume the current token if it matches, return true if consumed
    fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Expect and consume a token of the given kind, or record an error
    fn expect(&mut self, kind: SyntaxKind) {
        if !self.eat(kind) {
            let (offset, len) = self
                .current_token()
                .map(|t| (t.offset, t.text.len()))
                .unwrap_or((0, 0));
            self.errors.push(ParseError {
                message: format!("expected {:?}, found {:?}", kind, self.current()),
                offset,
                len,
            });
        }
    }

    /// Skip trivia (whitespace and comments)
    fn skip_trivia(&mut self) {
        while self.current().is_trivia() {
            self.bump();
        }
    }

    /// Skip trivia and return true if a newline was encountered
    fn skip_trivia_has_newline(&mut self) -> bool {
        let mut has_newline = false;
        while self.current().is_trivia() {
            if let Some(token) = self.current_token()
                && token.kind == SyntaxKind::WHITESPACE
                && token.text.contains('\n')
            {
                has_newline = true;
            }
            self.bump();
        }
        has_newline
    }

    /// Check if upcoming trivia contains a newline followed by a property pattern.
    /// Patterns detected:
    /// - IDENT + COLON (e.g., `opacity:`)
    /// - DOLLAR + IDENT + COLON (e.g., `$rawX:`)
    /// This is used to detect property boundaries in CSS-like blocks without semicolons.
    /// Returns true if we should stop parsing the current value because a new property starts.
    fn at_newline_property_start(&self) -> bool {
        let mut offset = 0;
        let mut found_newline = false;

        // Skip over trivia, tracking newlines
        while let Some(token) = self.tokens.get(self.pos + offset) {
            if token.kind.is_trivia() {
                if token.kind == SyntaxKind::WHITESPACE && token.text.contains('\n') {
                    found_newline = true;
                }
                offset += 1;
            } else {
                break;
            }
        }

        // If we found a newline, check if next non-trivia tokens form a property start
        if found_newline {
            let first = self.tokens.get(self.pos + offset).map(|t| t.kind);

            // Check for IDENT/keyword + COLON pattern (e.g., `from:`, `opacity:`)
            if first == Some(SyntaxKind::IDENT) || first.is_some_and(|k| k.is_keyword()) {
                // After IDENT/keyword, skip any trivia to find COLON
                let mut colon_offset = offset + 1;
                while let Some(token) = self.tokens.get(self.pos + colon_offset) {
                    if token.kind.is_trivia() {
                        colon_offset += 1;
                    } else {
                        break;
                    }
                }
                let second = self.tokens.get(self.pos + colon_offset).map(|t| t.kind);

                if second == Some(SyntaxKind::COLON) {
                    return true;
                }
            }

            // Check for DOLLAR + IDENT + COLON pattern (e.g., $rawX:)
            if first == Some(SyntaxKind::DOLLAR) {
                // After DOLLAR, check for IDENT
                let ident_offset = offset + 1;
                let second = self.tokens.get(self.pos + ident_offset).map(|t| t.kind);

                if second == Some(SyntaxKind::IDENT) {
                    // After IDENT, skip any trivia to find COLON
                    let mut colon_offset = ident_offset + 1;
                    while let Some(token) = self.tokens.get(self.pos + colon_offset) {
                        if token.kind.is_trivia() {
                            colon_offset += 1;
                        } else {
                            break;
                        }
                    }
                    let third = self.tokens.get(self.pos + colon_offset).map(|t| t.kind);

                    if third == Some(SyntaxKind::COLON) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Start a new node
    fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(rowan::SyntaxKind(kind.into()));
    }

    /// Finish the current node
    fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    /// Start an error recovery node
    fn start_error(&mut self) {
        self.start_node(SyntaxKind::ERROR);
    }

    /// Record an error and wrap tokens in ERROR node until recovery point
    fn error_recover(&mut self, message: &str, recovery: &[SyntaxKind]) {
        let (offset, len) = self
            .current_token()
            .map(|t| (t.offset, t.text.len()))
            .unwrap_or((0, 0));
        self.errors.push(ParseError {
            message: message.to_string(),
            offset,
            len,
        });

        self.start_error();
        // If we're already at a recovery point, consume at least one token to avoid infinite loops
        if self.at_any(recovery) {
            self.bump();
        } else {
            // Consume tokens until we hit a recovery point
            while !self.at_eof() && !self.at_any(recovery) {
                self.bump();
            }
        }
        self.finish_node();
    }

    /// Parse the entire file
    fn parse_root(&mut self) {
        self.start_node(SyntaxKind::ROOT);

        while !self.at_eof() {
            self.skip_trivia();
            if self.at_eof() {
                break;
            }

            match self.current() {
                // Scope block: .selector { ... } or #id { ... } or [attr] { ... }
                // or a selector LED BY A PSEUDO: `:where(...) { ... }` / `:is(...)`
                // / `:hover {}`. A leading `:` is unambiguous at top level (no
                // construct begins with a bare colon; a declaration is `ident:`),
                // so it always opens a scope-block selector. parse_selector_part
                // already handles `:name(balanced-parens)`, incl. a nested selector
                // list inside `:where()/:is()` — only this dispatch was missing it.
                SyntaxKind::DOT
                | SyntaxKind::HASH
                | SyntaxKind::L_BRACKET
                | SyntaxKind::STAR
                | SyntaxKind::COLON => {
                    self.parse_scope_block();
                }
                // Element selector scope block: body { ... } or div { ... }
                SyntaxKind::IDENT => {
                    if self.is_element_scope_block() {
                        self.parse_scope_block();
                    } else if self.at_dashed_form_ref() {
                        // BUG-241: a file-ROOT form splice (`--name;`). Without
                        // this arm it hit error_recover, which check COUNTED
                        // but never rendered — a file with a root-level splice
                        // failed with no diagnostic at all. Recognized here and
                        // validated like any other splice (unknown = E0947).
                        self.parse_form_ref();
                    } else {
                        // Unknown identifier at top level - error recovery
                        self.error_recover(
                            "expected scope block, directive, or declaration",
                            &[
                                SyntaxKind::DOT,
                                SyntaxKind::HASH,
                                SyntaxKind::AT_SIGN,
                                SyntaxKind::PERCENT,
                                SyntaxKind::DOLLAR,
                                SyntaxKind::R_BRACE,
                            ],
                        );
                    }
                }
                // Directive: @name(args) { body }
                SyntaxKind::AT_SIGN => {
                    self.parse_directive();
                }
                // Meta definition: %name { ... }
                SyntaxKind::PERCENT => {
                    self.parse_meta_def();
                }
                // Variable declaration at top level: $name type : value ;
                SyntaxKind::DOLLAR => {
                    self.parse_variable_decl();
                }
                // Preset reference at top level: ~name
                SyntaxKind::TILDE => {
                    self.preset_ref_retired();
                }
                // Element reference at top level: &name, &name .selector;, &name { body }
                SyntaxKind::AMPERSAND => {
                    self.parse_element_ref_stmt();
                }
                // HTML element literal at top level: <tag ...>...</tag> (PLAN-023 W0)
                // At top level `<` is unambiguously HTML (a comparison can't begin a
                // top-level construct). Capture the raw source span and extract holes.
                SyntaxKind::LT => {
                    self.parse_html_element();
                }
                // Import: @import "path"
                // (handled as directive)
                _ => {
                    // Unknown top-level construct - error recovery
                    self.error_recover(
                        "expected scope block, directive, or declaration",
                        &[
                            SyntaxKind::DOT,
                            SyntaxKind::HASH,
                            SyntaxKind::AT_SIGN,
                            SyntaxKind::PERCENT,
                            SyntaxKind::DOLLAR,
                            SyntaxKind::R_BRACE,
                        ],
                    );
                }
            }
        }

        // Consume EOF token
        if self.at(SyntaxKind::EOF) {
            self.bump();
        }

        self.finish_node();
    }

    /// Parse a scope block: .selector { ... }
    fn parse_scope_block(&mut self) {
        self.start_node(SyntaxKind::SCOPE_BLOCK);

        // Parse selector
        self.parse_selector();

        self.skip_trivia();

        // Parse body
        if self.at(SyntaxKind::L_BRACE) {
            self.parse_body();
        } else {
            self.errors.push(ParseError {
                message: "expected '{' after selector".to_string(),
                offset: self.current_token().map(|t| t.offset).unwrap_or(0),
                len: 1,
            });
        }

        self.finish_node();
    }

    /// Parse a selector (can be complex with combinators)
    fn parse_selector(&mut self) {
        self.start_node(SyntaxKind::SELECTOR);

        // Parse first selector part
        self.parse_selector_part();

        // Parse additional selectors separated by combinators or commas
        loop {
            self.skip_trivia();

            match self.current() {
                // Comma-separated selectors
                SyntaxKind::COMMA => {
                    self.bump();
                    self.skip_trivia();
                    self.parse_selector_part();
                }
                // Descendant combinator (whitespace before another selector part)
                // Child combinator (>)
                SyntaxKind::GT => {
                    self.bump();
                    self.skip_trivia();
                    self.parse_selector_part();
                }
                // Adjacent sibling (+)
                SyntaxKind::PLUS => {
                    self.bump();
                    self.skip_trivia();
                    self.parse_selector_part();
                }
                // General sibling (~)
                SyntaxKind::TILDE => {
                    self.bump();
                    self.skip_trivia();
                    self.parse_selector_part();
                }
                // Another selector part immediately following (compound or descendant selector)
                SyntaxKind::DOT
                | SyntaxKind::HASH
                | SyntaxKind::L_BRACKET
                | SyntaxKind::COLON
                | SyntaxKind::IDENT
                | SyntaxKind::STAR
                | SyntaxKind::AMPERSAND => {
                    self.parse_selector_part();
                }
                _ => break,
            }
        }

        self.finish_node();
    }

    /// Parse a single selector part (.class, #id, [attr], :pseudo, etc.)
    fn parse_selector_part(&mut self) {
        self.start_node(SyntaxKind::SELECTOR_PART);

        match self.current() {
            // Class selector: .class
            SyntaxKind::DOT => {
                self.bump();
                if self.at(SyntaxKind::IDENT) {
                    self.bump();
                }
            }
            // ID selector: #id
            SyntaxKind::HASH => {
                self.bump();
                if self.at(SyntaxKind::IDENT) {
                    self.bump();
                }
            }
            // Attribute selector: [attr] or [attr=value]
            SyntaxKind::L_BRACKET => {
                self.bump();
                // Consume everything until ]
                while !self.at_eof() && !self.at(SyntaxKind::R_BRACKET) {
                    self.bump();
                }
                if self.at(SyntaxKind::R_BRACKET) {
                    self.bump();
                }
            }
            // Pseudo-class or pseudo-element: :hover, ::before
            SyntaxKind::COLON => {
                self.bump();
                // Double colon for pseudo-element
                if self.at(SyntaxKind::COLON) {
                    self.bump();
                }
                if self.at(SyntaxKind::IDENT) {
                    self.bump();
                }
                // Handle functional pseudo-classes: :nth-child(2n+1), :not(.class)
                // CSS pseudo-class arguments have their own syntax (2n+1, odd, even, etc.)
                // so we just consume tokens until the matching close paren
                if self.at(SyntaxKind::L_PAREN) {
                    self.bump(); // consume (
                    let mut depth = 1;
                    while !self.at_eof() && depth > 0 {
                        if self.at(SyntaxKind::L_PAREN) {
                            depth += 1;
                        } else if self.at(SyntaxKind::R_PAREN) {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        self.bump();
                    }
                    if self.at(SyntaxKind::R_PAREN) {
                        self.bump(); // consume )
                    }
                }
            }
            // Universal selector: *
            SyntaxKind::STAR => {
                self.bump();
            }
            // Element type selector: div, span
            SyntaxKind::IDENT => {
                self.bump();
            }
            // Ampersand (parent reference in nested selectors)
            SyntaxKind::AMPERSAND => {
                self.bump();
            }
            _ => {
                // Empty selector part (error will be caught by caller)
            }
        }

        self.finish_node();
    }

    /// Consume `@name … { … }` or `@name …;` as one opaque value ARG when it
    /// immediately follows a value colon. This is the CST twin of the events
    /// grammar's directive_value_atom: the nested directive must not become an
    /// independently matched sibling before its outer form captures the value.
    fn try_parse_directive_value_atom(&mut self) -> bool {
        if !self.at(SyntaxKind::AT_SIGN)
            || !(self.peek_non_trivia(1) == SyntaxKind::IDENT
                || self.peek_non_trivia(1).is_keyword())
            || self.prev_non_trivia() != SyntaxKind::COLON
        {
            return false;
        }

        self.start_node(SyntaxKind::ARG);
        self.bump(); // '@'
        self.skip_trivia();
        self.bump(); // directive name
        self.skip_trivia();

        while !self.at_eof() {
            if self.at(SyntaxKind::L_BRACE) {
                // A following `;` belongs to the outer directive.
                self.consume_balanced_brackets();
                self.finish_node();
                return true;
            }
            if self.at(SyntaxKind::SEMICOLON) {
                // Bodyless directive values own their terminator.
                self.bump();
                self.finish_node();
                return true;
            }
            if self.at(SyntaxKind::AT_SIGN) || self.at(SyntaxKind::R_BRACE) {
                self.errors.push(ParseError {
                    message: "expected `{ … }` body or `;` after directive value".to_string(),
                    offset: self.current_token().map(|t| t.offset).unwrap_or(0),
                    len: 1,
                });
                self.finish_node();
                return true;
            }
            if self.at(SyntaxKind::L_PAREN) || self.at(SyntaxKind::L_BRACKET) {
                self.consume_balanced_brackets();
            } else {
                self.bump();
            }
            self.skip_trivia();
        }

        self.errors.push(ParseError {
            message: "expected `{ … }` body or `;` after directive value".to_string(),
            offset: self.current_token().map(|t| t.offset).unwrap_or(0),
            len: 1,
        });
        self.finish_node();
        true
    }

    /// Parse a directive: @name(args) { body } or @name(args);
    fn parse_directive(&mut self) {
        self.start_node(SyntaxKind::DIRECTIVE);

        // @
        self.expect(SyntaxKind::AT_SIGN);
        self.skip_trivia();

        // Name - can be IDENT or keywords like if, else, etc.
        // Capture the directive name text so the body decision below can route
        // DSL-bodied directives (e.g. `@data signal`) to a RAW body (the inner
        // `send`/`receive`/`policy` clauses are a capture-grammar DSL, not CST
        // statements — PLAN-038 W1).
        let mut directive_name: Option<String> = None;
        if self.at(SyntaxKind::IDENT) || self.current().is_keyword() {
            directive_name = self.current_text().map(|s| s.to_string());
            self.bump();
            // FEAT-118: a namespace-QUALIFIED directive name `@scene/camera` or
            // `@alias/name`. Consume `/IDENT` segments ONLY when TIGHTLY adjacent
            // (no trivia around the `/`), so CSS shorthand in value position
            // (`12px / 1.5`, `grid: 1 / 3`) is never mistaken for a qualifier —
            // those `/`s are spaced and/or not in directive-NAME position. The
            // segments join into the DIRECTIVE node text; `name_text()`
            // reconstructs the full `scene/camera` (see ast.rs).
            while self.at(SyntaxKind::SLASH)
                && (self.peek(1) == SyntaxKind::IDENT
                    || self
                        .tokens
                        .get(self.pos + 1)
                        .is_some_and(|t| t.kind.is_keyword()))
            {
                self.bump(); // '/'
                self.bump(); // segment ident
            }

            // BUG-232 / PLAN-117 W3: a COLON where the qualifier slash belongs
            // (`@b:badge`). The name run ends at `:`, so the directive matches
            // no `%form` and is DROPPED IN SILENCE — proven by differential
            // build: `@b/badge` emits `.badge::before`, `@b:badge` compiles
            // "clean" and emits an empty stylesheet. A silent drop is the worst
            // outcome for an addressing mistake, so refuse it here, at the only
            // point where both halves of the name are still visible.
            //
            // Same tight-adjacency discipline as the slash rule above, which is
            // what keeps a legitimate `@directive prop: value` (spaced `:` in
            // VALUE position) out of the way.
            if self.at(SyntaxKind::COLON) && self.peek(1) == SyntaxKind::IDENT {
                let head = directive_name.clone().unwrap_or_default();
                let leaf = self
                    .tokens
                    .get(self.pos + 1)
                    .and_then(|t| self.source.get(t.offset..t.offset + t.len()))
                    .unwrap_or("")
                    .to_string();
                self.errors.push(ParseError {
                    message: format!(
                        "`@{head}:{leaf}` uses `:` where a module qualifier needs `/` \
                         — write `@{head}/{leaf}`. `:` belongs inside a module address \
                         in an import string (`@use \"local:{head}\"`); in a reference, \
                         `/` addresses within a registry"
                    ),
                    offset: self.current_token().map(|t| t.offset).unwrap_or(0),
                    len: 1,
                });
            }
        } else {
            self.errors.push(ParseError {
                message: "expected directive name".to_string(),
                offset: self.current_token().map(|t| t.offset).unwrap_or(0),
                len: 1,
            });
        }

        let head_crossed_newline = self.skip_trivia_has_newline();

        // Directives whose BODY is a capture-grammar DSL (not CST statements) must
        // be captured as a RAW body so the %capture_type engine parses the clauses:
        //   `@data signal` / `@data stream` — send/receive/policy/optimistic clauses
        //   `@handle`                       — optimistic/receive/final clauses
        // Structural descent would mis-read `receive { Variant(b) => { … } }` arms
        // as nested scope blocks. PLAN-038 W1.
        let is_data_signal = directive_name.as_deref() == Some("data")
            && self.at(SyntaxKind::IDENT)
            && matches!(self.current_text(), Some("signal") | Some("stream"));
        let is_handle = directive_name.as_deref() == Some("handle");
        if is_data_signal || is_handle {
            // Consume the inline head (`signal $name(params) to $host`, or
            // `$signal` for @handle) token-by-token up to the body `{`, then
            // capture the body as RAW text.
            while !self.at_eof()
                && !self.at(SyntaxKind::L_BRACE)
                && !self.at(SyntaxKind::SEMICOLON)
                && !self.at(SyntaxKind::R_BRACE)
            {
                if self.at(SyntaxKind::L_PAREN) {
                    self.parse_arg_list(); // the (params) signature
                } else {
                    self.parse_inline_arg();
                }
                self.skip_trivia();
            }
            if self.at(SyntaxKind::L_BRACE) {
                self.parse_raw_body();
            } else if self.at(SyntaxKind::SEMICOLON) {
                self.bump();
            }
            self.finish_node();
            return;
        }

        // Optional inline arguments (before parentheses)
        // This includes: identifiers, type annotations (: Type), arrays ([]), comparisons,
        // CSS selectors (.class, #id), etc.
        let mut inline_crossed_newline = head_crossed_newline;
        // Tracks whether the most-recently consumed inline arg was a value-introducing
        // COLON (`@data derive $x : <expr>`). When true, a following `(` begins a
        // VALUE expression (`($a).length`, `($a || [])`), NOT a directive arg-list —
        // so it must be consumed as a balanced inline expression. Without this, a
        // paren-led derive/fetch value is mis-parsed as an arg-list and the trailing
        // `.member` is read as a new `.class` scope (BUG-077).
        let mut last_was_value_colon = false;
        // Whether a value-introducing COLON has been seen at all in this
        // directive's inline head. Distinct from `last_was_value_colon`, which
        // asks the narrower "was the PREVIOUS token that colon".
        let mut in_value_position = false;
        while self.at_any(&[
            SyntaxKind::IDENT,
            SyntaxKind::STRING,
            SyntaxKind::NUMBER,
            SyntaxKind::NUMBER_WITH_UNIT,
            SyntaxKind::COLOR,
            SyntaxKind::DOLLAR,
            SyntaxKind::AMPERSAND,
            SyntaxKind::TILDE,
            SyntaxKind::COLON,     // Type annotation: @data name: Type
            SyntaxKind::L_BRACKET, // Array type: Type[] or attribute selector [data-x]
            SyntaxKind::R_BRACKET,
            SyntaxKind::DOT,  // CSS class selector: .class-name
            SyntaxKind::HASH, // CSS ID selector: #element-id
            // Keywords used as literal tokens in form patterns
            SyntaxKind::KW_FROM,
            SyntaxKind::KW_TO,
            SyntaxKind::KW_ON,
            SyntaxKind::KW_IN,
            // Comparison operators (for @if conditions)
            SyntaxKind::EQ_EQ,
            SyntaxKind::EQ_EQ_EQ,
            SyntaxKind::NOT_EQ,
            SyntaxKind::NOT_EQ_EQ,
            SyntaxKind::LT,
            SyntaxKind::GT,
            SyntaxKind::LT_EQ,
            SyntaxKind::GT_EQ,
            SyntaxKind::EXCLAIM, // For !$var style negations
            // Arithmetic operators so a directive's inline VALUE expression
            // (`@data fold $t from $c : acc + item.qty`) is consumed whole rather
            // than the loop stopping at `+` and leaving `item.qty` to be mis-parsed
            // as a `.class` selector. STAR is intentionally NOT included — it doubles
            // as the universal selector and the ZeroOrMore modifier; multiplication in
            // a directive value is rare and can be parenthesized. (PLAN-023 W5.)
            SyntaxKind::PLUS,
            SyntaxKind::MINUS,
            SyntaxKind::SLASH,
        ]) {
            // A `.`/`#` after a newline begins a NEW scope statement, not a
            // continuation of this directive's inline args. Without this guard a
            // bodyless directive like `@import "x"` (no trailing `;`) followed by
            // `\n.box { … }` would swallow `.box` as an arg and `{ … }` as its
            // body. Stdlib's 98 bare `@import`s only escaped this via the flat
            // loader; directory-module imports rely on correct parsing here.
            //
            // A newline-led `<` is the same case for HTML: `@import "x"` followed
            // by `\n<main>…` begins a top-level HTML element literal, not a `<`
            // comparison continuing the directive's inline args (comparisons stay
            // on one line). Without this, the directive swallows `<main>` and the
            // file-scope markup never reaches body_html — a blank page (BUG-070).
            //
            // A newline-led element selector (`h1 { … }`, `body { … }`) is the
            // same case for bare HTML-element scopes: without this guard a
            // bodyless `@import "x"` followed by `\nh1 { … }` swallows `h1` as an
            // arg and `{ … }` as its body, vanishing the first scope (FUP-078).
            if inline_crossed_newline
                && (self.at_any(&[SyntaxKind::DOT, SyntaxKind::HASH, SyntaxKind::LT])
                    || (self.at(SyntaxKind::IDENT) && self.is_element_scope_block())
                    // A newline-led `&` begins a NEW element-ref / entity-scope
                    // statement (`&name;`, `&name .sel { … }`, `&name { @c }`), not a
                    // continuation of this directive's inline args. Without this a
                    // bodyless `@import "x"` followed by `\n&evernet { @peak }` swallows
                    // `&evernet` + its `{ … }` body, vanishing the entity scope
                    // (FEAT-142 WAVE A; the `&`-led sibling of the FUP-078 fix).
                    || self.at(SyntaxKind::AMPERSAND)
                    // A newline-led `ident:` / `--prop:` is a CSS DECLARATION of the
                    // enclosing style body, not a continuation of this directive's
                    // inline args. Without this a bodyless macro invocation
                    // (`@local-pointer`) followed by `--x: $sig;` swallows the whole
                    // declaration as args — the property vanishes from the CSS and a
                    // signal binding it carried never emits (the "cursor moves on Y
                    // only" class: the FIRST declaration after the directive silently
                    // dropped, the rest parsed fine). The IDENT+COLON twin of the
                    // FUP-078 element-scope guard, seeded with the name→first-arg
                    // newline so the FIRST arg is guarded too (FEAT-162 e2e).
                    || (self.at(SyntaxKind::IDENT)
                        && self.peek_non_trivia(1) == SyntaxKind::COLON)
                    // `:root { … }` / `* { … }` are newline-led selectors that
                    // begin a NEW rule, exactly like `.class`/`#id` above — they
                    // were simply missing from the list. Without this a bodyless
                    // directive (`@edit-toggle`) swallowed the selector as an arg
                    // and the rule's `{ … }` as its body, reporting E0946 against
                    // the DIRECTIVE up to ~94 lines above the rule that caused it
                    // (BUG-356, `demos/editable-blog`).
                    //
                    // Tested by reaching a `{`, not by the token: a `*` meaning
                    // multiplication and a `:` introducing a value both stay legal
                    // mid-run.
                    || (self.at_any(&[SyntaxKind::COLON, SyntaxKind::STAR])
                        && self.is_selector_scope_block()))
            {
                break;
            }
            let is_colon = self.at(SyntaxKind::COLON);
            in_value_position = in_value_position || is_colon;
            self.parse_inline_arg();
            last_was_value_colon = is_colon;
            // FUP-172. In VALUE position, a `(` whose balanced group is followed
            // by `.member` begins an EXPRESSION, not an argument list.
            //
            // `last_was_value_colon` alone only holds when `(` comes IMMEDIATELY
            // after the colon, so `: ($a).join(x)` parsed while
            // `: Array.from($a).join(x)` did not — by the time the loop broke at
            // `(`, the last consumed token was the ident `from`. The paren fell
            // through to `parse_arg_list`, which consumes ONLY `($a)` and stops,
            // stranding `.join` to be re-read at directive level as a `.class`
            // scope: "expected '{' after selector", pointing tokens past the
            // real problem.
            //
            // Making the flag merely STICKY over-reaches: `@preset easing ~x:
            // cubic-bezier(.4,0,.2,1)` is also a value-colon call, but its paren
            // IS an arg-list and must stay one. The trailing `.member` is what
            // distinguishes an expression from a call — so that, not the colon's
            // distance, is the test.
            if !last_was_value_colon
                && in_value_position
                && self.at(SyntaxKind::L_PAREN)
                && self.paren_group_is_followed_by_member()
            {
                last_was_value_colon = true;
            }
            inline_crossed_newline = self.skip_trivia_has_newline();
        }

        // A value-introducing COLON followed by `(` is a paren-led VALUE expression
        // (`@data derive $x : ($a || []).length;`), not a directive arg-list. Consume
        // the whole balanced expression up to `;`/`{` so the trailing `.member` and
        // operators survive intact (BUG-077). This precedes the arg-list branch so
        // the `(` is never mistaken for an argument list. Scoped to the paren-led
        // case only — broader value-colon rerouting regresses preset + type-
        // annotation directives whose `:` the form-matcher captures itself.
        if last_was_value_colon && self.at(SyntaxKind::L_PAREN) {
            self.parse_inline_expression();
        }

        // A value-position `@name` is one opaque ARG, generically. The helper
        // checks the immediately preceding COLON, so a sibling directive after a
        // completed value is never rerouted. A second call after an arg-list below
        // covers inline-union typerefs before the value colon.
        self.try_parse_directive_value_atom();

        // Handle inline assignment: @let name = expr;
        // After parsing inline args, if we see =, parse the assignment expression
        if self.at(SyntaxKind::EQUALS) {
            self.bump(); // consume =
            self.skip_trivia();
            // Parse the expression value - consume until ; or { or }
            self.parse_inline_expression();
        }

        // Optional argument list
        if self.at(SyntaxKind::L_PAREN) {
            self.parse_arg_list();
        }

        // Seed with the inline-arg loop's own newline state: when that loop broke
        // on a newline-led next statement (e.g. `@import "x"` then `\nh1 { … }`),
        // no trivia remains for `skip_trivia_has_newline` to observe, yet a newline
        // WAS crossed. Folding it in lets the `crossed_newline && !STRING` rule
        // below stop before swallowing the next statement (FUP-078). For directives
        // that broke on `(`/`{` the flag reflects newline-before-delimiter (usually
        // false) and the fresh scan after any arg-list still governs.
        let mut crossed_newline = inline_crossed_newline || self.skip_trivia_has_newline();

        // Optional post-arglist tokens (return type annotations, string messages).
        // After a newline, only consume STRING tokens (e.g., @assert (cond)\n  "message").
        // Non-string tokens on new lines belong to the next statement, not this directive.
        while !self.at_eof()
            && !self.at(SyntaxKind::L_BRACE)
            && !self.at(SyntaxKind::SEMICOLON)
            && !self.at(SyntaxKind::R_BRACE)
            && !self.at(SyntaxKind::KW_AS)
            && !self.at(SyntaxKind::AT_SIGN)  // Next directive
            && !self.at(SyntaxKind::DOLLAR)    // Variable declaration
            && !self.at(SyntaxKind::AMPERSAND) // Element reference
            && !self.at(SyntaxKind::TILDE)     // Preset reference
            && !self.at(SyntaxKind::PERCENT)   // Meta definition
            && !self.at(SyntaxKind::DOT)       // Nested scope
            && !self.at(SyntaxKind::HASH)      // ID selector
            && !self.at(SyntaxKind::LT)
        // Top-level HTML element (BUG-070)
        {
            // After a newline, only allow STRING (for continuation messages)
            if crossed_newline && !self.at(SyntaxKind::STRING) {
                break;
            }
            // BUG-216: a SECOND parenthesized group after the arg-list is one
            // balanced expression, not a token run. `@assert (cond) ("m " + a.b)`
            // is the canonical case — an assertion whose message reports the
            // failing value, which is the message you most want to write.
            //
            // Bumping token-by-token consumed `("m " + a` and then hit the `DOT`
            // guard above (a `.` begins a nested scope), so the loop broke mid-
            // expression and left `b)` to be parsed as a new statement — "expected
            // '{' after selector". That parse error was then SWALLOWED by the
            // opaque-block recompile, which returned an empty string for an
            // unparseable body, so the whole `@test` body vanished and the test
            // PASSED with nothing in it.
            //
            // Consuming the group as a balanced expression keeps the `.member`
            // access inside the expression where it belongs. The first `(` was
            // already taken by parse_arg_list, so reaching a `(` here always means
            // a following expression.
            if self.at(SyntaxKind::L_PAREN) {
                self.parse_balanced_paren_group();
                if self.skip_trivia_has_newline() {
                    crossed_newline = true;
                }
                continue;
            }
            // Wrap post-arglist tokens in ARG nodes so they're found by inline_args()
            self.start_node(SyntaxKind::ARG);
            self.bump();
            self.finish_node();
            // Track newlines within the loop too
            if self.skip_trivia_has_newline() {
                crossed_newline = true;
            }
        }

        // Optional as clause
        if self.at(SyntaxKind::KW_AS) {
            self.parse_as_clause();
        }

        // Optional `@use` import-list tail AFTER the alias: `as b only (…)` /
        // `as b hiding (…)` (FEAT-118 FUP-057). The `only`/`hiding` keyword is a
        // bare IDENT followed by an arg-list; absorb both into the directive node
        // so `inline_tail_text` can read them. Order-free: any mix follows `as`.
        loop {
            self.skip_trivia();
            if !self.at_keyword_text(&["only", "hiding"]) {
                break;
            }
            self.parse_inline_arg(); // the `only`/`hiding` keyword
            self.skip_trivia();
            if self.at(SyntaxKind::L_PAREN) {
                self.parse_arg_list();
            }
        }

        self.skip_trivia();

        // Second chance after an inline union / arg-list before the value colon.
        if self.try_parse_directive_value_atom() {
            self.skip_trivia();
        }

        // Optional body or semicolon.
        // A `{` that opens an OBJECT LITERAL value (`@data inline $x : { "k": v };`)
        // is the directive's VALUE, not its body — disambiguated by a quoted key
        // (`{` immediately followed by a STRING). Directive bodies hold
        // properties/directives whose names are IDENTs, never string literals, so the
        // lookahead is unambiguous (BUG-042 / FEAT-047 bare-object inline value).
        // A BARE key (`{ ink: #e8eef7; … }`) is the CSS-native typed seed
        // (FEAT-166). It has the same shape as a directive BODY of CSS
        // declarations, so it needs a discriminator the quoted form does not:
        // POSITION. This `{` follows the COLON that introduces a VALUE, and a
        // body never does — `:` introduces a value everywhere in the language,
        // which is what makes the test reliable rather than a heuristic.
        // NB: `last_was_value_colon` is NOT sufficient on its own. `@type Brand {
        // ink: color; }` sets it from the type's OWN field colons, so keying off
        // it alone swallowed every `@type` body as an object value — E0946 on
        // cms-testbed. The precise question is whether the token immediately
        // before this `{` is the value colon, which `prev_non_trivia` answers.
        let bare_object_value = self.prev_non_trivia() == SyntaxKind::COLON
            && self.peek_non_trivia(1) == SyntaxKind::IDENT
            && self.peek_non_trivia(2) == SyntaxKind::COLON;
        if self.at(SyntaxKind::L_BRACE)
            && (bare_object_value
                || (self.peek_non_trivia(1) == SyntaxKind::STRING
                    && self.peek_non_trivia(2) == SyntaxKind::COLON))
        {
            self.start_node(SyntaxKind::ARG);
            self.consume_balanced_brackets();
            self.finish_node();
            self.skip_trivia();
            if self.at(SyntaxKind::SEMICOLON) {
                self.bump();
            }
        } else if self.at(SyntaxKind::L_BRACE) {
            self.parse_body();
        } else if self.at(SyntaxKind::SEMICOLON) {
            self.bump();
        }

        self.finish_node();
    }

    /// Parse inline argument (identifier, reference, value, type annotation)
    fn parse_inline_arg(&mut self) {
        self.start_node(SyntaxKind::ARG);
        match self.current() {
            SyntaxKind::DOLLAR => self.parse_variable_ref(),
            SyntaxKind::AMPERSAND => self.parse_element_ref(),
            SyntaxKind::TILDE => self.preset_ref_retired(),
            SyntaxKind::COLON => {
                // Type annotation colon - consume as a single-token ARG
                self.bump();
            }
            SyntaxKind::DOT => {
                // CSS class selector: .class-name — combine DOT + IDENT into single ARG
                self.bump(); // consume DOT
                if self.at(SyntaxKind::IDENT) || self.current().is_keyword() {
                    self.bump(); // consume class name
                }
            }
            SyntaxKind::HASH => {
                // CSS ID selector: #id-name — combine HASH + IDENT into single ARG
                self.bump(); // consume HASH
                if self.at(SyntaxKind::IDENT) || self.current().is_keyword() {
                    self.bump(); // consume id name
                }
            }
            SyntaxKind::L_BRACKET
                if matches!(
                    self.peek_non_trivia(1),
                    SyntaxKind::L_BRACE
                        | SyntaxKind::STRING
                        | SyntaxKind::NUMBER
                        | SyntaxKind::NUMBER_WITH_UNIT
                        | SyntaxKind::L_BRACKET
                        | SyntaxKind::R_BRACKET
                ) =>
            {
                // Inline array literal value: consume the whole BALANCED bracket region
                // as a single ARG so object-literal arrays
                // (`@data inline $x : [{"k": v}, {"k": w}];`) are not truncated at the
                // inner `{` and mis-read as the directive's body block (BUG-042).
                // Depth-aware over [] {} (); STRING is one token so quotes stay inert.
                // Guarded by the opener lookahead so attribute selectors (`[data-x]`,
                // peek = IDENT) keep their per-token ARG shape.
                self.consume_balanced_brackets();
            }
            SyntaxKind::IDENT
            | SyntaxKind::STRING
            | SyntaxKind::NUMBER
            | SyntaxKind::NUMBER_WITH_UNIT
            | SyntaxKind::COLOR
            | SyntaxKind::KW_FROM
            | SyntaxKind::KW_TO => {
                // Consume the primary token
                self.bump();
                // Also consume trailing type modifiers: [] and ?
                // so "Product[]" and "string?" become single ARG nodes
                if self.at(SyntaxKind::L_BRACKET) && self.peek(1) == SyntaxKind::R_BRACKET {
                    self.bump(); // [
                    self.bump(); // ]
                }
                if self.at(SyntaxKind::QUESTION) {
                    self.bump(); // ?
                }
            }
            _ => self.bump(),
        }
        self.finish_node();
    }

    /// Consume a balanced `[ … ]` region as part of the current ARG node, tracking
    /// nesting across `[] {} ()` so inline object-literal arrays survive intact
    /// (`[{"k": v}, {"k": w}]`). STRING is a single token, so braces/brackets inside
    /// quotes never affect depth. Stops at EOF or when the matching `]` closes.
    fn consume_balanced_brackets(&mut self) {
        let mut depth = 0i32;
        while !self.at_eof() {
            match self.current() {
                SyntaxKind::L_BRACKET | SyntaxKind::L_BRACE | SyntaxKind::L_PAREN => {
                    depth += 1;
                    self.bump();
                }
                SyntaxKind::R_BRACKET | SyntaxKind::R_BRACE | SyntaxKind::R_PAREN => {
                    depth -= 1;
                    self.bump();
                    if depth <= 0 {
                        break;
                    }
                }
                _ => self.bump(),
            }
        }
    }

    /// Parse an inline expression for directive assignments like @let name = expr;
    /// Consumes tokens until reaching ; or { or } as statement boundaries
    /// Consume exactly ONE balanced `( … )` group as an EXPR node, stopping on the
    /// token after its matching `)`.
    ///
    /// `parse_inline_expression` is deliberately different: it consumes an
    /// expression up to a STATEMENT boundary (`;` `{` `}`), so after a group's
    /// depth returns to zero it keeps going. Using it for a directive's trailing
    /// message group therefore swallowed whatever followed — `@assert (c) ("m " +
    /// a.b)` on one line and `.after { … }` on the next consumed the scope, which
    /// is a WORSE failure than the BUG-216 drop it was fixing (a valid sibling
    /// statement silently disappears).
    ///
    /// An UNCLOSED group stops at EOF and leaves an error, rather than being
    /// treated as if it had closed — the caller's loop then terminates normally.
    fn parse_balanced_paren_group(&mut self) {
        debug_assert!(self.at(SyntaxKind::L_PAREN));
        self.start_node(SyntaxKind::EXPR);
        let mut depth = 0usize;
        while !self.at_eof() {
            // Trivia (whitespace AND comments) must be consumed as trivia, never
            // counted. A `(` inside a comment is not a paren — reading tokens
            // blindly made a prose comment containing "(border + elevation)" open
            // a group, which then ran to EOF and broke a stylesheet that had
            // parsed for as long as it existed.
            if self.current().is_trivia() {
                self.bump();
                continue;
            }
            if self.at(SyntaxKind::L_PAREN) {
                depth += 1;
            } else if self.at(SyntaxKind::R_PAREN) {
                depth -= 1;
                if depth == 0 {
                    self.bump(); // the matching `)` belongs to this group
                    break;
                }
            }
            self.bump();
        }
        if depth != 0 {
            self.errors.push(ParseError {
                message: "unclosed '(' in directive argument".to_string(),
                offset: self.current_token().map(|t| t.offset).unwrap_or(0),
                len: 1,
            });
        }
        self.finish_node();
    }

    /// Does the balanced paren group at the cursor have a `.member` after it?
    ///
    /// The question that separates a CALL from an EXPRESSION in directive value
    /// position: `cubic-bezier(.4,0,.2,1)` ends at its `)` and is an arg-list,
    /// while `Array.from($a).join(…)` continues into `.join` and is therefore
    /// one expression that must be consumed whole (FUP-172).
    ///
    /// Pure lookahead — the cursor does not move. Depth-counted so a nested
    /// group (`from(new Set($a))`) is skipped as a unit rather than ending the
    /// scan at its first inner `)`.
    fn paren_group_is_followed_by_member(&self) -> bool {
        debug_assert!(self.at(SyntaxKind::L_PAREN));
        let mut depth = 0usize;
        let mut index = self.pos;
        while let Some(token) = self.tokens.get(index) {
            match token.kind {
                SyntaxKind::L_PAREN => depth += 1,
                SyntaxKind::R_PAREN => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        // Walk past trivia to the first real token after `)`.
                        let mut after = index + 1;
                        while self.tokens.get(after).is_some_and(|t| t.kind.is_trivia()) {
                            after += 1;
                        }
                        return self
                            .tokens
                            .get(after)
                            .is_some_and(|t| t.kind == SyntaxKind::DOT);
                    }
                }
                // An unterminated group must not read as "followed by a member":
                // the recovery path owns that, and guessing here would reroute a
                // malformed directive into expression parsing.
                SyntaxKind::EOF | SyntaxKind::L_BRACE | SyntaxKind::SEMICOLON => return false,
                _ => {}
            }
            index += 1;
        }
        false
    }

    fn parse_inline_expression(&mut self) {
        self.start_node(SyntaxKind::EXPR);

        let mut paren_depth = 0;
        let mut bracket_depth = 0;

        while !self.at_eof() {
            // Track parentheses depth
            if self.at(SyntaxKind::L_PAREN) {
                paren_depth += 1;
            } else if self.at(SyntaxKind::R_PAREN) {
                if paren_depth > 0 {
                    paren_depth -= 1;
                } else {
                    break; // Unmatched ) - stop
                }
            }

            // Track bracket depth for array expressions
            if self.at(SyntaxKind::L_BRACKET) {
                bracket_depth += 1;
            } else if self.at(SyntaxKind::R_BRACKET) {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                } else {
                    break; // Unmatched ] - stop
                }
            }

            // Stop at statement boundaries (but only if not inside parens/brackets)
            if paren_depth == 0
                && bracket_depth == 0
                && (self.at(SyntaxKind::SEMICOLON)
                    || self.at(SyntaxKind::L_BRACE)
                    || self.at(SyntaxKind::R_BRACE))
            {
                break;
            }

            // Parse the current token appropriately
            match self.current() {
                SyntaxKind::DOLLAR => self.parse_variable_ref(),
                SyntaxKind::AMPERSAND => self.parse_element_ref(),
                SyntaxKind::TILDE => self.preset_ref_retired(),
                SyntaxKind::L_PAREN => {
                    self.bump(); // ( was already counted above
                }
                _ => self.bump(),
            }

            // Skip trivia between expression parts
            self.skip_trivia();
        }

        self.finish_node();
    }

    /// Parse an argument list: (arg1, arg2, name: value)
    /// Also supports space-separated args for CSS-like functions: grid(8 4)
    fn parse_arg_list(&mut self) {
        self.start_node(SyntaxKind::ARG_LIST);

        self.expect(SyntaxKind::L_PAREN);
        self.skip_trivia();

        while !self.at_eof() && !self.at(SyntaxKind::R_PAREN) {
            let pos_before = self.pos;
            self.parse_arg();
            self.skip_trivia();

            if self.at(SyntaxKind::COMMA) {
                self.bump();
                self.skip_trivia();
            } else if self.at(SyntaxKind::R_PAREN) || self.at_eof() {
                break;
            } else if self.pos == pos_before {
                // No progress made — avoid infinite loop
                break;
            }
            // Otherwise continue: space-separated args (e.g., grid(8 4))
        }

        self.expect(SyntaxKind::R_PAREN);

        self.finish_node();
    }

    /// Parse a meta definition parameter list with types and defaults
    /// e.g., (&el, axis: ("x" | "y") = "y", throttle: number = 0)
    fn parse_meta_param_list(&mut self) {
        self.start_node(SyntaxKind::ARG_LIST);

        self.expect(SyntaxKind::L_PAREN);
        self.skip_trivia();

        while !self.at_eof() && !self.at(SyntaxKind::R_PAREN) {
            self.parse_meta_param();
            self.skip_trivia();

            if self.at(SyntaxKind::COMMA) {
                self.bump();
                self.skip_trivia();
            } else {
                break;
            }
        }

        self.expect(SyntaxKind::R_PAREN);

        self.finish_node();
    }

    /// Parse a single meta parameter
    /// Formats:
    /// - &el (element ref)
    /// - $gl (data ref)
    /// - $t: number (typed data ref)
    /// - name: type (typed param without default)
    /// - name: type = default (typed param with default)
    fn parse_meta_param(&mut self) {
        self.start_node(SyntaxKind::ARG);

        match self.current() {
            // &el - element reference parameter
            SyntaxKind::AMPERSAND => {
                self.parse_element_ref();
            }
            // $name or $name: type - data/context reference
            // NOTE: We don't use parse_variable_ref() here because it greedily
            // consumes `:type` (designed for %form patterns like $name:expr),
            // which conflicts with the meta param syntax `$name: type` where
            // the colon is part of the parameter declaration, not a capture type.
            SyntaxKind::DOLLAR => {
                self.start_node(SyntaxKind::VARIABLE_REF);
                self.bump(); // $
                if self.at(SyntaxKind::IDENT) {
                    self.bump(); // name
                }
                self.finish_node();
                self.skip_trivia();
                // Check for type annotation: $name: type
                if self.at(SyntaxKind::COLON) {
                    self.bump(); // :
                    self.skip_trivia();
                    self.parse_meta_type();
                }
                self.skip_trivia();
                // Optional default value
                if self.at(SyntaxKind::EQUALS) {
                    self.bump(); // =
                    self.skip_trivia();
                    self.parse_meta_default_value();
                }
            }
            // name: type = default OR name: $var:type = default - typed parameter (IDENT or keyword like 'from')
            SyntaxKind::IDENT => {
                self.bump(); // name
                self.skip_trivia();
                if self.at(SyntaxKind::COLON) {
                    self.bump(); // :
                    self.skip_trivia();
                    // Check if next is $ (variable binding: name: $var:type = default)
                    if self.at(SyntaxKind::DOLLAR) {
                        self.parse_variable_ref(); // $var
                        self.skip_trivia();
                        // Check for type annotation on the variable
                        if self.at(SyntaxKind::COLON) {
                            self.bump(); // :
                            self.skip_trivia();
                            self.parse_meta_type();
                        }
                    } else {
                        self.parse_meta_type();
                    }
                    self.skip_trivia();
                    // Optional default value
                    if self.at(SyntaxKind::EQUALS) {
                        self.bump(); // =
                        self.skip_trivia();
                        self.parse_meta_default_value();
                    }
                }
            }
            _ if self.current().is_keyword() => {
                // Handle keywords used as parameter names (like 'from', 'to')
                self.bump(); // keyword as name
                self.skip_trivia();
                if self.at(SyntaxKind::COLON) {
                    self.bump(); // :
                    self.skip_trivia();
                    // Check if next is $ (variable binding: name: $var:type = default)
                    if self.at(SyntaxKind::DOLLAR) {
                        self.parse_variable_ref(); // $var
                        self.skip_trivia();
                        // Check for type annotation on the variable
                        if self.at(SyntaxKind::COLON) {
                            self.bump(); // :
                            self.skip_trivia();
                            self.parse_meta_type();
                        }
                    } else {
                        self.parse_meta_type();
                    }
                    self.skip_trivia();
                    // Optional default value
                    if self.at(SyntaxKind::EQUALS) {
                        self.bump(); // =
                        self.skip_trivia();
                        self.parse_meta_default_value();
                    }
                }
            }
            _ => {
                // Unknown parameter format - error
                self.start_error();
                self.bump();
                self.finish_node();
            }
        }

        self.finish_node();
    }

    /// Parse a meta type annotation
    /// e.g., number, string, ("x" | "y"), string[], number | null
    fn parse_meta_type(&mut self) {
        // Handle union type prefix: ("x" | "y")
        if self.at(SyntaxKind::L_PAREN) {
            self.bump(); // (
            self.skip_trivia();
            // Parse type alternatives
            loop {
                // Parse a type value (string literal or identifier)
                match self.current() {
                    SyntaxKind::STRING | SyntaxKind::IDENT | SyntaxKind::NUMBER => {
                        self.bump();
                    }
                    _ => break,
                }
                self.skip_trivia();
                if self.at(SyntaxKind::PIPE) {
                    self.bump(); // |
                    self.skip_trivia();
                } else {
                    break;
                }
            }
            self.skip_trivia();
            self.expect(SyntaxKind::R_PAREN);
        } else if self.at(SyntaxKind::IDENT) {
            self.bump(); // type name like number, string
        } else if self.at(SyntaxKind::STRING) {
            // String literal type like "x" | "y"
            self.bump();
        } else {
            // Unknown type
            return;
        }

        // Handle array suffix: []
        self.skip_trivia();
        if self.at(SyntaxKind::L_BRACKET) {
            self.bump(); // [
            self.skip_trivia();
            self.expect(SyntaxKind::R_BRACKET);
        }

        // Handle union suffix: | null or | other
        self.skip_trivia();
        while self.at(SyntaxKind::PIPE) {
            self.bump(); // |
            self.skip_trivia();
            if self.at(SyntaxKind::IDENT) || self.at(SyntaxKind::STRING) {
                self.bump();
            }
            self.skip_trivia();
        }

        // Handle optional type suffix: ? (e.g., any?, string?)
        if self.at(SyntaxKind::QUESTION) {
            self.bump();
        }
    }

    /// Parse a meta parameter default value
    /// e.g., "y", 0, [], {}, true, false
    fn parse_meta_default_value(&mut self) {
        match self.current() {
            SyntaxKind::STRING
            | SyntaxKind::NUMBER
            | SyntaxKind::NUMBER_WITH_UNIT
            | SyntaxKind::KW_TRUE
            | SyntaxKind::KW_FALSE
            | SyntaxKind::IDENT => {
                // `duration: time = 300ms` — at Kernel tier the unit is its own
                // token, so a plain `bump` would leave `ms` to be reported as an
                // unexpected token where the parser wanted `)`.
                self.bump_value_atom();
            }
            SyntaxKind::L_BRACKET => {
                self.parse_array_expr();
            }
            SyntaxKind::L_BRACE => {
                self.parse_object_or_body();
            }
            SyntaxKind::DOLLAR => {
                self.parse_variable_ref();
            }
            _ => {
                // Unknown default - skip
            }
        }
    }

    /// Parse a single argument (positional or named)
    fn parse_arg(&mut self) {
        // Check if this is a named argument (name: value or $name: value for pattern params)
        // Names can be identifiers OR keywords (e.g., "from:", "to:")
        let is_named = if (self.at(SyntaxKind::IDENT) || self.current().is_keyword())
            && self.peek(1) == SyntaxKind::COLON
        {
            true
        } else if self.at(SyntaxKind::DOLLAR) {
            // Look for $name: pattern (pattern parameter with default)
            // Skip $ then check for IDENT then COLON
            self.peek(1) == SyntaxKind::IDENT && self.peek(2) == SyntaxKind::COLON
        } else {
            false
        };

        if is_named {
            self.start_node(SyntaxKind::NAMED_ARG);
            if self.at(SyntaxKind::DOLLAR) {
                self.bump(); // $
            }
            self.bump(); // name (IDENT or keyword)
            self.bump(); // :
            self.skip_trivia();
            self.parse_arg_value();
            self.finish_node();
        } else {
            self.start_node(SyntaxKind::ARG);
            self.parse_arg_value();
            self.finish_node();
        }
    }

    /// Parse an argument value (handles complex expressions)
    fn parse_arg_value(&mut self) {
        // Parse the primary value
        match self.current() {
            SyntaxKind::DOLLAR => self.parse_variable_ref(),
            SyntaxKind::AMPERSAND => self.parse_element_ref(),
            SyntaxKind::TILDE => self.preset_ref_retired(),
            SyntaxKind::L_BRACKET => self.parse_array_expr(),
            SyntaxKind::L_BRACE => self.parse_object_or_body(),
            // An arrow function `(params) => body` is opaque JS (the @eval/@drive/@when
            // payload): consume it WHOLE with balanced bracket tracking so nested `{}`,
            // `[]`, and `( )` in the JS body never terminate it early. Without this the
            // body's first inner `}` ends the arg and the rest of the JS spills to the
            // top level, mis-read as CSS selectors/rule-blocks (BUG-095). A non-arrow
            // `(` stays a normal parenthesized expression.
            SyntaxKind::L_PAREN if self.is_arrow_fn() => self.parse_arrow_fn_balanced(),
            SyntaxKind::L_PAREN => self.parse_paren_expr(),
            SyntaxKind::STRING
            | SyntaxKind::NUMBER
            | SyntaxKind::NUMBER_WITH_UNIT
            | SyntaxKind::COLOR
            | SyntaxKind::KW_TRUE
            | SyntaxKind::KW_FALSE
            | SyntaxKind::IDENT => {
                // `@animate(fade-in, 500ms)` is TWO args. At Kernel tier the
                // dimension is two tokens, so bumping only the number would make
                // the stranded unit look like a third argument.
                self.bump_value_atom();
                // Check for function call: ident(...)
                if self.at(SyntaxKind::L_PAREN) {
                    self.parse_arg_list();
                }
            }
            _ => {
                // Unknown - just consume as error
                if !self.at_any(&[
                    SyntaxKind::COMMA,
                    SyntaxKind::R_PAREN,
                    SyntaxKind::R_BRACKET,
                    SyntaxKind::R_BRACE,
                    SyntaxKind::SEMICOLON,
                ]) {
                    self.start_error();
                    self.bump();
                    self.finish_node();
                }
            }
        }

        // Handle binary operators and member access
        loop {
            self.skip_trivia();
            match self.current() {
                // Member access: .field or method call: .method(args)
                SyntaxKind::DOT => {
                    self.bump();
                    if self.at(SyntaxKind::IDENT) {
                        self.bump();
                        // Check for method call: .method(args)
                        if self.at(SyntaxKind::L_PAREN) {
                            self.parse_arg_list();
                        }
                    }
                }
                // Function call: func(args)
                SyntaxKind::L_PAREN => {
                    self.parse_arg_list();
                }
                // Index access `[index]` OR array-type marker `[]`. An EMPTY pair
                // `Todo[]` is the array-type suffix (a type payload like
                // `Many(Todo[])` or any array type ref in arg position), NOT an
                // index expression — so consume `[ ]` directly without parsing an
                // inner value (which would mis-read the `]` and cascade to a
                // "expected '{' after selector" error). A non-empty `[expr]` stays
                // a normal index access.
                SyntaxKind::L_BRACKET => {
                    self.bump();
                    self.skip_trivia();
                    if self.at(SyntaxKind::R_BRACKET) {
                        self.bump(); // `]` — empty array-type marker
                    } else {
                        self.parse_arg_value();
                        self.skip_trivia();
                        self.expect(SyntaxKind::R_BRACKET);
                    }
                }
                // Binary operators
                SyntaxKind::PLUS
                | SyntaxKind::MINUS
                | SyntaxKind::STAR
                | SyntaxKind::SLASH
                | SyntaxKind::PERCENT  // modulo operator
                | SyntaxKind::EQUALS   // assignment operator
                | SyntaxKind::EQ_EQ
                | SyntaxKind::EQ_EQ_EQ
                | SyntaxKind::NOT_EQ
                | SyntaxKind::NOT_EQ_EQ
                | SyntaxKind::LT
                | SyntaxKind::GT
                | SyntaxKind::LT_EQ
                | SyntaxKind::GT_EQ
                | SyntaxKind::AND_AND
                | SyntaxKind::OR_OR
                | SyntaxKind::PIPE
                | SyntaxKind::QUESTION_QUESTION => {
                    self.bump();
                    self.skip_trivia();
                    self.parse_arg_value();
                }
                // "as" clause: $source as $alias (for @each iteration binding)
                SyntaxKind::KW_AS => {
                    self.bump(); // as
                    self.skip_trivia();
                    // Consume the alias: $name or plain ident
                    if self.at(SyntaxKind::DOLLAR) {
                        self.parse_variable_ref();
                    } else if self.at(SyntaxKind::IDENT) {
                        self.bump();
                    }
                    // Don't continue the loop — "as" clause terminates this arg value
                    break;
                }
                // Ternary operator
                SyntaxKind::QUESTION => {
                    self.bump();
                    self.skip_trivia();
                    self.parse_arg_value();
                    self.skip_trivia();
                    self.expect(SyntaxKind::COLON);
                    self.skip_trivia();
                    self.parse_arg_value();
                }
                _ => break,
            }
        }
    }

    /// Parse an as clause: as $name or as name
    fn parse_as_clause(&mut self) {
        self.bump(); // as
        self.skip_trivia();

        // Optional $ for variable binding
        if self.at(SyntaxKind::DOLLAR) {
            self.parse_variable_ref();
        } else if self.at(SyntaxKind::IDENT) {
            self.bump();
        }

        self.skip_trivia();

        // Optional type annotation — but NOT the `@use` tail keywords `only` /
        // `hiding`, which are import-list clauses that follow `as <alias>` and
        // carry their own `(…)` arg-list (FEAT-118 FUP-057). Consuming them here
        // as a “type” would orphan the `(…)` and break the parse.
        if self.at(SyntaxKind::IDENT) && !self.at_keyword_text(&["only", "hiding"]) {
            self.bump();
        }
    }

    /// True when the current token is an IDENT whose text matches one of
    /// `words`. Lets order-sensitive grammar (e.g. the `@use` tail) treat bare
    /// identifiers as soft keywords without minting real keyword tokens.
    fn at_keyword_text(&self, words: &[&str]) -> bool {
        self.at(SyntaxKind::IDENT)
            && self
                .current_text()
                .map(|t| words.contains(&t))
                .unwrap_or(false)
    }

    /// Parse a meta definition or meta-level macro invocation.
    ///
    /// Meta definitions: %macro name { ... }, %primitive name(...) { ... },
    ///                   %capture_type name { pattern }, %runtime-registry name { ... }
    /// Meta-level invocations: %preset easing ~linear: linear, %some-macro arg1 arg2
    ///
    /// The distinction is based on the keyword after %:
    /// - `macro`, `primitive`, `capture_type`, `captureType`, `runtime-registry` -> parse as meta definition
    /// - anything else -> parse as meta-level macro invocation (like a directive)
    fn parse_meta_def(&mut self) {
        // Peek ahead to see if this is a definition (macro/primitive) or invocation
        // We need to check what comes after % without consuming it
        let is_definition = {
            // Skip any whitespace after %
            let mut lookahead = 1;
            while let Some(t) = self.tokens.get(self.pos + lookahead) {
                if t.kind.is_trivia() {
                    lookahead += 1;
                } else {
                    break;
                }
            }
            // Check if keyword after % is a meta definition keyword
            // (macro, primitive, capture_type, runtime-registry)
            self.tokens
                .get(self.pos + lookahead)
                .map(|t| {
                    matches!(
                        t.text.as_str(),
                        "macro"
                            | "primitive"
                            | "capture_type"
                            | "captureType"
                            | "runtime-registry"
                            | "vendor"
                            // PLAN-076: %migration syntax-migration defs
                            | "migration"
                            // PLAN-123: %comment_type comment-roster defs.
                            // Its body is CLAUSES (%label/%docs/%field/...),
                            // so it parses through parse_meta_body like
                            // %macro and %migration — not as a raw body.
                            | "comment_type"
                            | "commentType"
                            // FEAT-168 / PLAN-122 W4: %scalar_type is a
                            // kinds-as-data roster def like %comment_type —
                            // its body is CLAUSES (%capture/%schema/%format/
                            // %zero/%widget/%docs), so it parses through
                            // parse_meta_body, not as a raw body.
                            | "scalar_type"
                            | "scalarType"
                            // FEAT-118 M2: MODULE.st manifest clauses parse as
                            // META_DEF nodes (routed to module_manifest, not the
                            // registry, by cst_to_stfile_with_registry).
                            | "module"
                            | "public"
                            | "reexport"
                            // FEAT-118 M3: the %using active-extension hook (has
                            // a `{ body }`, handled by parse_meta_definition).
                            | "using"
                    )
                })
                .unwrap_or(false)
        };

        if is_definition {
            self.parse_meta_definition();
        } else {
            self.parse_meta_invocation();
        }
    }

    /// Parse a meta definition: %macro name { ... }, %primitive name(...) { ... },
    /// %capture_type name { pattern }, %runtime-registry name { ... }
    fn parse_meta_definition(&mut self) {
        self.start_node(SyntaxKind::META_DEF);

        // %
        self.expect(SyntaxKind::PERCENT);
        self.skip_trivia();

        // Meta keyword (macro, primitive, capture_type, etc.)
        let meta_keyword = if self.at(SyntaxKind::IDENT) {
            let text = self.current_token().map(|t| t.text.to_string());
            self.bump();
            text
        } else {
            None
        };

        self.skip_trivia();

        // Meta name (the actual name of the macro/primitive)
        if self.at(SyntaxKind::IDENT) {
            self.bump();
        }

        self.skip_trivia();

        // FEAT-118 M2: a MODULE.st manifest clause (`%public (a, b)`,
        // `%reexport "coll:ns" (a, b)`) has an arbitrary inline tail (string,
        // parens, idents) and NO body. Consume the tail to the statement
        // boundary so it parses as a complete META_DEF rather than leaving a
        // stray STRING for the next top-level parse to choke on.
        if matches!(
            meta_keyword.as_deref(),
            Some("module") | Some("public") | Some("reexport")
        ) {
            // Consume the inline tail up to the next top-level construct. A
            // manifest clause is single-line; the boundary is the next `%`/`@`
            // (next directive/def) or a `.`/`#`/element selector starting a
            // scope block, EOF, or an explicit `;`. Newlines are trivia, so we
            // detect the boundary by the next NON-TRIVIA token's kind.
            loop {
                self.skip_trivia();
                if self.at_eof()
                    || self.at(SyntaxKind::SEMICOLON)
                    || self.at(SyntaxKind::PERCENT)
                    || self.at(SyntaxKind::AT_SIGN)
                    || self.at(SyntaxKind::L_BRACE)
                {
                    break;
                }
                self.bump();
            }
            self.eat(SyntaxKind::SEMICOLON);
            self.finish_node();
            return;
        }

        // Optional parameter list for primitives/macros with params
        // These are different from regular args - they have type annotations and defaults
        // e.g., %primitive scroll(&el, axis: ("x" | "y") = "y", throttle: number = 0) { ... }
        if self.at(SyntaxKind::L_PAREN) {
            self.parse_meta_param_list();
        }

        self.skip_trivia();

        // PLAN-133: optional `%uses a, b` clause — the primitive declares that
        // another primitive's PRELUDE must be on the page (cross-primitive
        // prelude dependency). Pulled into the expansion closure at compile
        // time; unknown names and cycles are compile errors.
        if self.at(SyntaxKind::PERCENT) && self.peek_next_non_trivia_is_ident("uses") {
            self.start_node(SyntaxKind::META_USES);
            self.bump(); // %
            self.skip_trivia();
            self.bump(); // uses
            self.skip_trivia();
            // ident (',' ident)*
            while self.at(SyntaxKind::IDENT) {
                self.bump();
                self.skip_trivia();
                if self.at(SyntaxKind::COMMA) {
                    self.bump();
                    self.skip_trivia();
                } else {
                    break;
                }
            }
            self.finish_node();
        }

        self.skip_trivia();

        // Body (optional - some meta defs might just be declarations)
        if self.at(SyntaxKind::L_BRACE) {
            // %capture_type and %vendor bodies contain field/pattern DSL - treat as raw text
            if matches!(
                meta_keyword.as_deref(),
                Some("capture_type") | Some("captureType") | Some("vendor")
            ) {
                self.parse_raw_body();
            } else {
                self.parse_meta_body();
            }
        }

        self.finish_node();
    }

    /// Parse a meta-level macro invocation: %preset easing ~linear: linear
    /// This is similar to a directive but with % prefix instead of @
    /// It invokes a previously defined macro at the meta/compile level
    fn parse_meta_invocation(&mut self) {
        // Use DIRECTIVE node so it can be processed like other directives
        self.start_node(SyntaxKind::DIRECTIVE);

        // %
        self.expect(SyntaxKind::PERCENT);
        self.skip_trivia();

        // Macro name being invoked (e.g., "preset")
        if self.at(SyntaxKind::IDENT) || self.current().is_keyword() {
            self.bump();
        }

        self.skip_trivia();

        // Inline arguments - consume until we hit a statement boundary
        // For %preset easing ~linear: linear, we need to consume:
        // "easing", "~linear", ":", "linear"
        while !self.at_eof()
            && !self.at(SyntaxKind::L_BRACE)
            && !self.at(SyntaxKind::R_BRACE)
            && !self.at(SyntaxKind::PERCENT)  // Next meta statement
            && !self.at(SyntaxKind::AT_SIGN)  // Next directive
            && !self.at(SyntaxKind::DOT)      // Scope block
            && !self.at(SyntaxKind::HASH)
        // ID selector
        {
            match self.current() {
                SyntaxKind::DOLLAR => self.parse_variable_ref(),
                SyntaxKind::AMPERSAND => self.parse_element_ref(),
                SyntaxKind::TILDE => self.preset_ref_retired(),
                _ => self.bump(),
            }
            self.skip_trivia();
        }

        // Optional body block
        if self.at(SyntaxKind::L_BRACE) {
            self.parse_body();
        }

        self.finish_node();
    }

    /// Parse a meta definition body: { %creates @name, %binds { ... }, etc. }
    fn parse_meta_body(&mut self) {
        self.start_node(SyntaxKind::BODY);

        self.expect(SyntaxKind::L_BRACE);
        self.skip_trivia();

        while !self.at_eof() && !self.at(SyntaxKind::R_BRACE) {
            let pos_before = self.pos;

            match self.current() {
                // Sub-clauses: %creates, %binds, %form, etc.
                SyntaxKind::PERCENT => {
                    self.parse_meta_clause();
                }
                // Directives: @test, @name, etc.
                SyntaxKind::AT_SIGN => {
                    self.parse_directive();
                }
                // Variable: $name
                SyntaxKind::DOLLAR => {
                    self.parse_variable_decl();
                }
                // Other content - just consume it
                _ => {
                    self.bump();
                }
            }

            self.skip_trivia();

            // Safety: ensure we make progress
            if self.pos == pos_before && !self.at(SyntaxKind::R_BRACE) && !self.at_eof() {
                self.bump();
            }
        }

        self.expect(SyntaxKind::R_BRACE);

        self.finish_node();
    }

    /// Parse a meta clause: %creates @name, %emit js { ... }, etc.
    /// Creates a DIRECTIVE node so it can be iterated via body.directives()
    fn parse_meta_clause(&mut self) {
        self.start_node(SyntaxKind::DIRECTIVE);

        // %
        self.expect(SyntaxKind::PERCENT);
        self.skip_trivia();

        // Clause keyword (creates, binds, form, emit, if, etc.)
        // Capture the keyword for special handling
        let keyword = if self.at(SyntaxKind::IDENT) || self.current().is_keyword() {
            let text = self.current_token().map(|t| t.text.to_string());
            self.bump();
            text
        } else {
            None
        };

        self.skip_trivia();

        // Inline content - varies by keyword
        // Consume inline tokens until we hit { or another % or }
        while !self.at_eof()
            && !self.at(SyntaxKind::PERCENT)
            && !self.at(SyntaxKind::R_BRACE)
            && !self.at(SyntaxKind::L_BRACE)
        {
            if self.at(SyntaxKind::AT_SIGN) {
                // Inline directive reference like @test
                self.parse_directive();
            } else if self.at(SyntaxKind::DOLLAR) {
                // Variable reference
                self.parse_variable_ref();
            } else {
                self.bump();
            }
            self.skip_trivia();
        }

        // Body block (like %emit js { ... } or %binds { ... })
        if self.at(SyntaxKind::L_BRACE) {
            // %form bodies contain pattern DSL - treat as raw text
            if keyword.as_deref() == Some("form") {
                self.parse_raw_body();
            } else if keyword.as_deref() == Some("emit") {
                // %emit bodies contain JS/CSS with embedded %yield, %cleanup, etc.
                // Use emit-aware parser that tracks braces but only interprets % tokens
                self.parse_emit_body();
            } else if keyword.as_deref() == Some("derives") {
                // %derives bodies contain CSS-like properties: $name: expression
                self.parse_body();
            } else {
                self.parse_meta_clause_body();
            }
        }

        self.finish_node();
    }

    /// Parse a body as raw text (just tracks brace nesting)
    /// Used for %form patterns where the content is a pattern DSL, not Spacetime syntax
    fn parse_raw_body(&mut self) {
        self.start_node(SyntaxKind::BODY);
        self.expect(SyntaxKind::L_BRACE);
        let mut depth = 1;
        while !self.at_eof() && depth > 0 {
            if self.at(SyntaxKind::L_BRACE) {
                depth += 1;
            } else if self.at(SyntaxKind::R_BRACE) {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            self.bump();
        }
        self.expect(SyntaxKind::R_BRACE);
        self.finish_node();
    }

    /// Parse a %emit body: tracks brace nesting but only interprets % tokens as meta clauses.
    /// Unlike parse_meta_clause_body, this does NOT try to interpret $ as variable refs
    /// or other non-% tokens as Spacetime syntax. This prevents JS code inside %emit js { }
    /// from being misinterpreted (template literals, arrow functions, etc.).
    fn parse_emit_body(&mut self) {
        self.start_node(SyntaxKind::BODY);
        self.expect(SyntaxKind::L_BRACE);

        let mut depth = 1;
        while !self.at_eof() && depth > 0 {
            match self.current() {
                SyntaxKind::L_BRACE => {
                    depth += 1;
                    self.bump();
                }
                SyntaxKind::R_BRACE => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    self.bump();
                }
                // Only interpret % tokens as meta clauses (%yield, %cleanup, etc.)
                SyntaxKind::PERCENT => {
                    self.parse_meta_clause();
                }
                _ => {
                    self.bump();
                }
            }
        }

        self.expect(SyntaxKind::R_BRACE);
        self.finish_node();
    }

    /// Parse a meta clause body: { %emit js { ... }, %cleanup { ... }, ... }
    fn parse_meta_clause_body(&mut self) {
        self.start_node(SyntaxKind::BODY);

        self.expect(SyntaxKind::L_BRACE);
        self.skip_trivia();

        while !self.at_eof() && !self.at(SyntaxKind::R_BRACE) {
            let pos_before = self.pos;

            match self.current() {
                // Meta clauses: %emit, %cleanup, %exports, etc.
                SyntaxKind::PERCENT => {
                    self.parse_meta_clause();
                }
                // Directives: @name { }
                SyntaxKind::AT_SIGN => {
                    self.parse_directive();
                }
                // Variables: $name
                SyntaxKind::DOLLAR => {
                    self.parse_variable_ref();
                }
                // Nested braces - just consume the block
                SyntaxKind::L_BRACE => {
                    self.parse_nested_braces();
                }
                _ => {
                    self.bump();
                }
            }

            self.skip_trivia();

            // Safety: ensure we make progress
            if self.pos == pos_before && !self.at(SyntaxKind::R_BRACE) && !self.at_eof() {
                self.bump();
            }
        }

        self.expect(SyntaxKind::R_BRACE);

        self.finish_node();
    }

    /// Parse nested braces without semantic interpretation
    fn parse_nested_braces(&mut self) {
        self.expect(SyntaxKind::L_BRACE);
        let mut depth = 1;
        while !self.at_eof() && depth > 0 {
            if self.at(SyntaxKind::L_BRACE) {
                depth += 1;
            } else if self.at(SyntaxKind::R_BRACE) {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            self.bump();
        }
        self.expect(SyntaxKind::R_BRACE);
    }

    /// Parse a variable reference: $name or $name:type or $name:type? or $name:type*
    fn parse_variable_ref(&mut self) {
        self.start_node(SyntaxKind::VARIABLE_REF);
        self.expect(SyntaxKind::DOLLAR);
        if self.at(SyntaxKind::IDENT) {
            self.bump();
        }
        // Handle property path: $name.field or method calls: $name.method(args)
        while self.at(SyntaxKind::DOT) {
            self.bump();
            if self.at(SyntaxKind::IDENT) {
                self.bump();
                // Check for method call: .method(args)
                if self.at(SyntaxKind::L_PAREN) {
                    self.parse_arg_list();
                }
            }
        }
        // Handle type annotation: $name:type (used in %form patterns)
        if self.at(SyntaxKind::COLON) {
            self.bump();
            // Type can be: ident, typeref, expr, duration, number, etc.
            if self.at(SyntaxKind::IDENT) {
                self.bump();
            }
        }
        // Handle modifier: $name:type? or $name:type* or $name:type+ (used in %form patterns)
        if self.at(SyntaxKind::QUESTION) || self.at(SyntaxKind::STAR) || self.at(SyntaxKind::PLUS) {
            self.bump();
        }
        // Handle default value: $name:type = default
        self.skip_trivia();
        if self.at(SyntaxKind::EQUALS) {
            self.bump();
            self.skip_trivia();
            // Default value can be: ident, string, number, etc.
            if self.at(SyntaxKind::IDENT)
                || self.at(SyntaxKind::NUMBER)
                || self.at(SyntaxKind::STRING)
            {
                self.bump();
            }
        }
        self.finish_node();
    }

    /// Parse a variable declaration: $name type : value ;
    fn parse_variable_decl(&mut self) {
        self.start_node(SyntaxKind::VARIABLE_REF);

        self.expect(SyntaxKind::DOLLAR);
        self.skip_trivia();

        // Name
        if self.at(SyntaxKind::IDENT) {
            self.bump();
        }

        self.skip_trivia();

        // Optional type
        if self.at(SyntaxKind::IDENT) {
            self.bump();
        }

        self.skip_trivia();

        // : value
        if self.at(SyntaxKind::COLON) {
            self.bump();
            self.skip_trivia();
            self.parse_arg_value();
        }

        self.skip_trivia();

        // ;
        if self.at(SyntaxKind::SEMICOLON) {
            self.bump();
        }

        self.finish_node();
    }

    /// Parse an element reference: &name or &name.facet.property or &$var.path
    /// The element can be:
    /// - Direct: &name (IDENT)
    /// - Indirect via variable: &$var (DOLLAR IDENT)
    fn parse_element_ref(&mut self) {
        self.start_node(SyntaxKind::ELEMENT_REF);
        self.expect(SyntaxKind::AMPERSAND);

        // Element name can be direct (IDENT) or via variable ($name)
        if self.at(SyntaxKind::DOLLAR) {
            // Indirect reference: &$var.path
            self.bump(); // $
            if self.at(SyntaxKind::IDENT) {
                self.bump(); // variable name
            }
        } else if self.at(SyntaxKind::IDENT) {
            self.bump();
        }

        // Handle property path: &name.facet.property or &$var.facet.property
        while self.at(SyntaxKind::DOT) {
            self.bump();
            if self.at(SyntaxKind::IDENT) {
                self.bump();
            }
        }

        // Handle param-position modifier: &name? / &name* / &name+ (BUG-165).
        // Mirrors parse_variable_ref's identical modifier consumption for `$name?`
        // (~L1861) — an element param and a value param share ONE meaning for a
        // trailing `?`/`*`/`+` in param-list position: optional/repeatable marker,
        // never a dangling ternary condition. Without this, parse_arg_value's
        // postfix loop sees the bare QUESTION after this node returns and
        // mis-parses it as `cond ? a : b`, failing on the closing `)` with
        // "expected COLON". No author-facing surface writes a ternary directly on
        // a bare element ref (conditions on refs are `@if(&footer) { ... }`), so
        // this is unambiguous at every parse_element_ref call site.
        if self.at(SyntaxKind::QUESTION) || self.at(SyntaxKind::STAR) || self.at(SyntaxKind::PLUS) {
            self.bump();
        }

        self.finish_node();
    }

    /// Parse an element reference statement: &name selector; or &name selector { body }
    /// This wraps the element ref in a statement-level node that includes the optional selector and body
    fn parse_element_ref_stmt(&mut self) {
        self.start_node(SyntaxKind::ELEMENT_REF_STMT);

        // Parse the element reference itself (&name or &name.facet.property)
        self.parse_element_ref();
        self.skip_trivia();

        // Check for template invocation: &template-name($arg) or &template-name($arg1, $arg2)
        if self.at(SyntaxKind::L_PAREN) {
            self.parse_arg_list();
            self.skip_trivia();
        }

        // Parse optional selector (everything until ; or {)
        // Selectors can include: .class, #id, [attr=value], :pseudo, combinators (>, +, ~, space)
        if !self.at(SyntaxKind::SEMICOLON)
            && !self.at(SyntaxKind::L_BRACE)
            && !self.at(SyntaxKind::R_BRACE)
            && !self.at_eof()
        {
            self.start_node(SyntaxKind::SELECTOR);
            while !self.at_eof()
                && !self.at(SyntaxKind::SEMICOLON)
                && !self.at(SyntaxKind::L_BRACE)
                && !self.at(SyntaxKind::R_BRACE)
            {
                self.bump();
            }
            self.finish_node();
            self.skip_trivia();
        }

        // Parse optional body { ... }
        if self.at(SyntaxKind::L_BRACE) {
            self.parse_body();
        }

        // Consume trailing semicolon
        if self.at(SyntaxKind::SEMICOLON) {
            self.bump();
        }

        self.finish_node();
    }

    // === PLAN-023 W0: first-class HTML (< sigil) + backtick holes ===

    /// Emit a builder token with explicit text (not sourced from the token stream).
    /// Used to reconstruct HTML raw segments and hole delimiters in the green tree.
    fn token_text(&mut self, kind: SyntaxKind, text: &str) {
        self.builder.token(rowan::SyntaxKind(kind.into()), text);
    }

    /// Parse an HTML element literal at top level: `<tag ...>...</tag>` (PLAN-023 W0).
    ///
    /// Strategy B (source-span): the token stream is unreliable inside HTML (backtick
    /// holes are not lexer tokens, and holes inside attribute strings are swallowed into
    /// a single STRING token). So we byte-scan the raw source from the current `<` to the
    /// matching close, splitting it into verbatim raw segments and `` `expr` `` holes. The
    /// raw segments are emitted as HTML_RAW tokens (handed verbatim to html5ever in W1);
    /// each hole inner is parsed as a full Spacetime expression into an HTML_HOLE node.
    fn parse_html_element(&mut self) {
        let start = self.current_token().map(|t| t.offset).unwrap_or(0);
        let scan = self.scan_html_region(start);

        self.start_node(SyntaxKind::HTML_ELEMENT);
        for seg in scan.segs {
            match seg {
                HtmlSeg::Raw(text) => {
                    if !text.is_empty() {
                        self.token_text(SyntaxKind::HTML_RAW, &text);
                    }
                }
                HtmlSeg::Hole(inner) => {
                    self.start_node(SyntaxKind::HTML_HOLE);
                    self.token_text(SyntaxKind::HTML_RAW, "`");
                    self.parse_hole_inner(&inner);
                    self.token_text(SyntaxKind::HTML_RAW, "`");
                    self.finish_node();
                }
            }
        }
        self.finish_node();

        // Advance the token cursor past every token that began inside the HTML region.
        // USUALLY the region end aligns to a `>` token boundary. But a stray quote in
        // HTML *text* (e.g. `the compiler's OWN`) makes the lexer emit a STRING token
        // that runs from the `'` to the next `'`/EOF — straddling `scan.end` and
        // swallowing everything after the markup (the CSS/JS that follows). The
        // byte-scanner above got the region right; the token stream did not. When the
        // token at the cursor straddles `scan.end`, RE-LEX the tail from `scan.end` so
        // the post-markup source (style rules, more directives) tokenizes cleanly
        // (BUG-072). The common no-straddle case is unchanged.
        while self.pos < self.tokens.len() {
            match self.tokens.get(self.pos) {
                Some(tok) if tok.offset + tok.text.len() <= scan.end => self.pos += 1,
                Some(tok) if tok.offset < scan.end => {
                    // Straddling token: discard it and everything after, re-lex the tail.
                    let tail = self.source[scan.end..].to_string();
                    let mut tail_tokens = Lexer::new(&tail).tokenize();
                    for t in &mut tail_tokens {
                        t.offset += scan.end;
                    }
                    self.tokens.truncate(self.pos);
                    self.tokens.extend(tail_tokens);
                    break;
                }
                _ => break,
            }
        }
    }

    /// Re-parse a backtick-hole inner string as a full Spacetime expression, emitting real
    /// CST nodes (VARIABLE_REF, ELEMENT_REF, …) into the shared builder. The token stream is
    /// temporarily swapped so the existing expression machinery can be reused verbatim.
    fn parse_hole_inner(&mut self, inner: &str) {
        let saved_tokens = std::mem::take(&mut self.tokens);
        let saved_pos = self.pos;
        let lexer = Lexer::new(inner);
        self.tokens = lexer.tokenize();
        self.pos = 0;
        self.skip_trivia();
        if !self.at_eof() {
            self.parse_inline_expression();
        }
        self.tokens = saved_tokens;
        self.pos = saved_pos;
    }

    /// Byte-scan the HTML region starting at `start` (a `<`). Returns the end byte offset
    /// (one past the region) and the ordered raw/hole segments. Depth-aware over element
    /// nesting; respects void elements, self-closing tags, comments, attribute strings, and
    /// raw-text elements (script/style/textarea/title — no hole extraction inside those).
    fn scan_html_region(&self, start: usize) -> HtmlScan {
        let b = self.source.as_bytes();
        let len = b.len();
        let mut segs: Vec<HtmlSeg> = Vec::new();
        let mut raw_start = start;
        let mut i = start;
        let mut depth: i32 = 0;
        let mut in_tag = false; // inside an opening tag `<name ...`
        let mut cur_open = false; // the tag being scanned is an open (not close) tag
        let mut cur_void = false; // the open tag is a void element
        let mut cur_rawtext: Option<String> = None; // open tag introduces raw-text content
        let mut in_str: Option<u8> = None; // inside an attribute string
        let mut raw_text: Option<String> = None; // inside raw-text element body

        let flush = |segs: &mut Vec<HtmlSeg>, from: usize, to: usize| {
            if to > from {
                segs.push(HtmlSeg::Raw(self_source_slice(b, from, to)));
            }
        };

        while i < len {
            // Inside a raw-text element body (script/style/…): consume until the matching
            // close tag; no hole extraction, no nested-tag tracking.
            if let Some(ref tn) = raw_text {
                if html_matches_close_tag(b, i, tn) {
                    let gt = html_find(b, i, b'>').unwrap_or(len - 1);
                    i = gt + 1;
                    depth -= 1;
                    raw_text = None;
                    if depth <= 0 {
                        flush(&mut segs, raw_start, i);
                        return HtmlScan { end: i, segs };
                    }
                } else {
                    i += 1;
                }
                continue;
            }

            let c = b[i];

            // Backtick holes: recognised in text position and inside attribute strings,
            // but NOT in raw-text bodies (handled above). `\\\`` escapes a literal backtick.
            if !in_tag || in_str.is_some() {
                if c == b'\\' && i + 1 < len && b[i + 1] == b'`' {
                    i += 2;
                    continue;
                }
                if c == b'`' {
                    flush(&mut segs, raw_start, i);
                    let mut j = i + 1;
                    while j < len {
                        if b[j] == b'\\' && j + 1 < len && b[j + 1] == b'`' {
                            j += 2;
                        } else if b[j] == b'`' {
                            break;
                        } else {
                            j += 1;
                        }
                    }
                    segs.push(HtmlSeg::Hole(self_source_slice(b, i + 1, j)));
                    i = (j + 1).min(len);
                    raw_start = i;
                    continue;
                }
            }

            // Inside an attribute string: only the matching quote (or a hole, handled above)
            // is significant.
            if let Some(q) = in_str {
                if c == q {
                    in_str = None;
                }
                i += 1;
                continue;
            }

            if in_tag {
                match c {
                    b'"' | b'\'' => {
                        in_str = Some(c);
                        i += 1;
                    }
                    b'/' if i + 1 < len && b[i + 1] == b'>' => {
                        // self-closing tag: no depth change
                        in_tag = false;
                        cur_rawtext = None;
                        i += 2;
                        if depth <= 0 {
                            flush(&mut segs, raw_start, i);
                            return HtmlScan { end: i, segs };
                        }
                    }
                    b'>' => {
                        in_tag = false;
                        i += 1;
                        if cur_open && !cur_void {
                            depth += 1;
                            if let Some(tn) = cur_rawtext.take() {
                                raw_text = Some(tn);
                            }
                        }
                        if depth <= 0 {
                            flush(&mut segs, raw_start, i);
                            return HtmlScan { end: i, segs };
                        }
                    }
                    _ => i += 1,
                }
                continue;
            }

            // Text position.
            if c == b'<' {
                // Comment: <!-- ... -->
                if i + 3 < len && &b[i..i + 4] == b"<!--" {
                    if let Some(e) = html_find_seq(b, i + 4, b"-->") {
                        i = e + 3;
                    } else {
                        i = len;
                    }
                    continue;
                }
                // Close tag: </name>
                if i + 1 < len && b[i + 1] == b'/' {
                    let gt = html_find(b, i, b'>').unwrap_or(len - 1);
                    i = gt + 1;
                    depth -= 1;
                    if depth <= 0 {
                        flush(&mut segs, raw_start, i);
                        return HtmlScan { end: i, segs };
                    }
                    continue;
                }
                // Open tag: <name ...
                let (name, after) = html_read_tag_name(b, i + 1);
                in_tag = true;
                cur_open = true;
                cur_void = html_is_void(&name);
                cur_rawtext = if html_is_raw_text(&name) {
                    Some(name)
                } else {
                    None
                };
                i = after;
                continue;
            }

            i += 1;
        }

        // Unterminated region: consume to EOF (html5ever reports structure errors in W1).
        flush(&mut segs, raw_start, len);
        HtmlScan { end: len, segs }
    }

    /// Parse a form splice: `--name;` or `--name(args);` (SIP-001c, BUG-241).
    /// Recognize, keep the sigil, keep the args —
    /// with the optional ARG_LIST for parameterized call sites.
    fn parse_form_ref(&mut self) {
        self.start_node(SyntaxKind::FORM_REF);
        self.bump(); // the `--name` IDENT (dash-merged by the lexer)
        self.skip_trivia();
        if self.at(SyntaxKind::L_PAREN) {
            self.parse_arg_list();
        }
        self.skip_trivia();
        if self.at(SyntaxKind::SEMICOLON) {
            self.bump();
        }
        self.finish_node();
    }

    /// A statement-position FORM SPLICE: the current token is a dash-merged
    /// IDENT (`--card-surface`) NOT followed (past trivia) by `:` — a colon
    /// makes it a CSS custom property (`--brand: red;`), which stays a
    /// property. Single-dash vendor idents never reach here: their text starts
    /// with `-w`/`-m`, not `--`.
    fn at_dashed_form_ref(&self) -> bool {
        if !self.at(SyntaxKind::IDENT) {
            return false;
        }
        let is_dashed = self
            .tokens
            .get(self.pos)
            .map(|t| t.text.starts_with("--"))
            .unwrap_or(false);
        if !is_dashed {
            return false;
        }
        let mut lookahead = 1;
        // Comments are trivia too: `--brand /* note */ : red` is a custom
        // property, not a splice.
        while matches!(
            self.peek(lookahead),
            SyntaxKind::WHITESPACE | SyntaxKind::COMMENT
        ) {
            lookahead += 1;
        }
        self.peek(lookahead) != SyntaxKind::COLON
    }

    /// A `~name` preset reference — RETIRED in SIP-001c (BUG-263). Easings are
    /// now `@form easing --name` forms; the `~` preset token kind is deleted
    /// from the CST. Reject LOUDLY rather than silently degrading to the default
    /// easing (the arc's banned silent-acceptance class). The recovery point is
    /// the statement/value boundary so the whole `~name` is swallowed and the
    /// enclosing declaration continues.
    fn preset_ref_retired(&mut self) {
        // Only tilde + ADJACENT identifier is the retired preset reference.
        // `~>` (sequence arrow in type annotations) and `~=` (claim contains
        // operator) are live operators spelled with a tilde — bump through them.
        if self.peek(1) != SyntaxKind::IDENT {
            self.bump();
            return;
        }
        self.error_recover(
            "`~name` preset references are retired: use a `@form easing --name` form or the bare CSS keyword",
            &[
                SyntaxKind::SEMICOLON,
                SyntaxKind::R_BRACE,
                SyntaxKind::R_PAREN,
                SyntaxKind::R_BRACKET,
                SyntaxKind::COMMA,
            ],
        );
    }

    /// Parse a body: { ... }
    fn parse_body(&mut self) {
        self.start_node(SyntaxKind::BODY);

        self.expect(SyntaxKind::L_BRACE);
        self.skip_trivia();

        while !self.at_eof() && !self.at(SyntaxKind::R_BRACE) {
            let pos_before = self.pos;
            self.parse_body_item();
            self.skip_trivia();
            // Safety: ensure we make progress to avoid infinite loop
            if self.pos == pos_before && !self.at(SyntaxKind::R_BRACE) && !self.at_eof() {
                // Force progress by consuming a token
                self.bump();
            }
        }

        self.expect(SyntaxKind::R_BRACE);

        self.finish_node();
    }

    /// Parse a body item (property, directive, nested scope, etc.)
    fn parse_body_item(&mut self) {
        match self.current() {
            // Attribute selector - check if it's a property or nested scope
            SyntaxKind::L_BRACKET => {
                // An EMPTY `[]` pair is never a selector/attribute — it is the
                // array-type marker trailing a type ref (`Todo[]`) that the lenient
                // statement path left at body-item start (e.g. a sum variant payload
                // `Many(Todo[])`). Consume `[ ]` and continue rather than treating
                // `[` as a `[attr]` selector (which fails with "expected '{' after
                // selector"). A non-empty `[...]` keeps the attribute-selector path.
                if self.peek(1) == SyntaxKind::R_BRACKET
                    || (self.peek(1) == SyntaxKind::WHITESPACE
                        && self.peek(2) == SyntaxKind::R_BRACKET)
                {
                    self.bump(); // [
                    self.skip_trivia();
                    self.bump(); // ]
                } else if self.is_attribute_property() {
                    self.parse_attribute_property();
                } else {
                    self.parse_scope_block();
                }
            }
            // Class selector - check if it's actually a scope block (.class { })
            // or just a stray dot (e.g., in JavaScript code like .toFixed())
            SyntaxKind::DOT => {
                if self.is_scope_block() {
                    self.parse_scope_block();
                } else if self.is_class_property() {
                    // Reactive class toggle at selector scope: `.active: $v;` (no `{`).
                    // Parse as a CSS_PROPERTY keeping the leading `.` so the emitter routes
                    // it to classList.toggle (BUG-068), not setAttribute.
                    self.parse_class_property();
                } else {
                    // Just consume the dot and continue
                    self.bump();
                }
            }
            // Nested scope (id selector)
            SyntaxKind::HASH => {
                self.parse_scope_block();
            }
            // Universal selector: only if `*` is followed (after optional whitespace) by `{`
            SyntaxKind::STAR => {
                if self.is_universal_scope_block() {
                    self.parse_scope_block();
                } else {
                    // Multiplication or other operator — just consume
                    self.bump();
                }
            }
            // Directive
            SyntaxKind::AT_SIGN => {
                self.parse_directive();
            }
            // Variable mutation ($var <- expr), property ($name: value), or declaration ($name type: value)
            SyntaxKind::DOLLAR => {
                // Check in order of specificity:
                // 1. Mutation: DOLLAR (path)+ WHITESPACE? LEFT_ARROW
                // 2. Property: DOLLAR IDENT WHITESPACE? COLON
                // 3. Declaration: DOLLAR IDENT IDENT COLON (type between name and colon)
                if self.is_mutation_statement() {
                    self.parse_mutation_statement();
                } else if self.is_variable_property() {
                    self.parse_variable_property();
                } else {
                    self.parse_variable_decl();
                }
            }
            // Child combinator (>), adjacent sibling (+) or general sibling (~) starting a
            // nested scope: `> .child { }`. Only when followed by a selector-starting token
            // (.class, #id, [attr], :pseudo, *, &).
            //
            // BUG-243: TILDE was absent here and fell through to the preset-ref arm below,
            // so a nested `~ .b { }` silently compiled to `.a .b` — a DESCENDANT selector,
            // styling the wrong elements with no diagnostic. `>` and `+` nested correctly,
            // so the inconsistency was invisible.
            //
            // The two TILDE meanings disambiguate exactly, which is why this is safe: a
            // COMBINATOR is followed by a selector-starting token, while a PRESET REF
            // (`~ease-out`) is followed by an IDENT — and IDENT is not in
            // `is_combinator_scope_block`'s set. Ordering the combinator first therefore
            // takes nothing away from the preset arm.
            SyntaxKind::GT | SyntaxKind::PLUS | SyntaxKind::TILDE
                if self.is_combinator_scope_block() =>
            {
                self.parse_combinator_scope_block();
            }
            // Ampersand: &:hover {} is nested scope, &name is element reference
            SyntaxKind::AMPERSAND => {
                if self.is_ampersand_scope_block() {
                    self.parse_scope_block();
                } else {
                    self.parse_element_ref_stmt();
                }
            }
            // Preset reference
            SyntaxKind::TILDE => {
                self.preset_ref_retired();
                self.skip_trivia();
                if self.at(SyntaxKind::SEMICOLON) {
                    self.bump();
                }
            }
            // Pseudo-class/element block: :hover { ... }
            SyntaxKind::COLON => {
                self.parse_pseudo_block();
            }
            // Could be element selector (body { }), CSS property, or other identifier-starting construct
            SyntaxKind::IDENT => {
                // Check if this is an element selector scope block (body { }, div { }, main { }, etc.)
                if self.is_element_scope_block() {
                    self.parse_scope_block();
                } else if self.at_dashed_form_ref() {
                    // BUG-241: a statement-position `--name;` was falling into
                    // `parse_css_property_or_statement`, whose no-colon arm just
                    // BUMPS the ident — the splice vanished with no diagnostic.
                    // Recognized here as a FORM_REF; validated against the form
                    // registry after rematch (unknown name = E0947). Arm order:
                    // AFTER the scope-block check (an element selector wins) and
                    // before the CSS-property fallthrough — and the `:` guard in
                    // `at_dashed_form_ref` keeps custom properties on the
                    // property path, so the two never compete.
                    self.parse_form_ref();
                } else {
                    // Could be a CSS property or other identifier-starting construct
                    self.parse_css_property_or_statement();
                }
            }
            // HTML element literal inside a body: `<li>…</li>` (PLAN-023 W4).
            // At statement-start in a scope/directive body, `<` immediately followed
            // by a tag name (IDENT/keyword) or `/` (close tag) is unambiguously HTML
            // — a comparison never starts a body statement. A lone `<` (e.g. stray
            // operator) falls through to the default consume.
            SyntaxKind::LT if self.lt_starts_html_element() => {
                self.parse_html_element();
            }
            // Keyword-starting constructs
            SyntaxKind::KW_IF => {
                self.parse_if_statement();
            }
            // Keywords that can appear as CSS property names (e.g., `from: products;`)
            kind if kind.is_keyword()
                && kind != SyntaxKind::KW_IF
                && kind != SyntaxKind::KW_ELSE =>
            {
                self.parse_css_property_or_statement();
            }
            // String literal (e.g., in JavaScript code inside @fn bodies)
            SyntaxKind::STRING => {
                // Just consume the string
                self.bump();
            }
            // Nested brace block (e.g., "viewing { }" inside @state_machine body)
            // Must consume the entire { ... } block to avoid confusing the closing }
            // with the parent body's closing brace
            SyntaxKind::L_BRACE => {
                self.parse_nested_braces();
            }
            _ => {
                // Unknown - skip to next statement boundary
                // Just consume the token and continue (more lenient than error_recover)
                self.bump();
            }
        }
    }

    /// Check if a `<` at the current position begins an HTML element literal (PLAN-023 W4).
    /// True when the `<` is immediately followed by a tag-name token (IDENT or keyword) or
    /// a `/` close-tag marker, with NO intervening whitespace — mirroring how HTML is
    /// written (`<li>`, `</ul>`). A `<` followed by whitespace or an operand is treated as
    /// a comparison and left to the default path. The no-whitespace rule keeps `a < b` (a
    /// genuine comparison, which the lexer surrounds with WHITESPACE) from being misread.
    fn lt_starts_html_element(&self) -> bool {
        // Next token must be a tag name (IDENT/keyword) or `/` close-tag, no whitespace.
        let next_ok = match self.tokens.get(self.pos + 1) {
            Some(tok) => {
                matches!(tok.kind, SyntaxKind::IDENT | SyntaxKind::SLASH) || tok.kind.is_keyword()
            }
            None => false,
        };
        if !next_ok {
            return false;
        }
        // The `<` must NOT follow an OPERAND — otherwise it is a comparison written
        // without spaces (`$n<5`, `i<n`, `foo()<bar`, `arr[0]<x`). At a genuine
        // statement start the preceding non-trivia token is a delimiter (`{` `}` `;`
        // `(` `,`) or the end of a previous HTML element, where `<` IS a tag start.
        // (PLAN-023 W4 — fixes the unspaced-comparison parse regression.)
        let mut p = self.pos;
        while p > 0 {
            p -= 1;
            match self.tokens.get(p) {
                Some(t) if matches!(t.kind, SyntaxKind::WHITESPACE | SyntaxKind::COMMENT) => {
                    continue;
                }
                Some(t) => {
                    return !matches!(
                        t.kind,
                        SyntaxKind::IDENT
                            | SyntaxKind::NUMBER
                            | SyntaxKind::NUMBER_WITH_UNIT
                            | SyntaxKind::STRING
                            | SyntaxKind::R_PAREN
                            | SyntaxKind::R_BRACKET
                            | SyntaxKind::GT
                    ) && !t.kind.is_keyword();
                }
                None => break,
            }
        }
        true
    }

    /// Check if `*` at current position starts a universal selector scope block: `* { }`
    /// Only returns true if `*` is directly followed (after optional whitespace) by `{`.
    fn is_universal_scope_block(&self) -> bool {
        let mut i = self.pos + 1; // Skip the *
        while i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::WHITESPACE {
            i += 1;
        }
        i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::L_BRACE
    }

    /// Check if `>` or `+` at current position starts a combinator scope block.
    /// Returns true only when the combinator is followed (after whitespace) by an
    /// unambiguous selector-starting token: `.`, `#`, `[`, `:`, `*`, or `&`.
    /// Does NOT match bare IDENT (e.g., `> div {}`) to avoid false positives with
    /// comparison operators in JS/expression contexts (e.g., `x > y`).
    fn is_combinator_scope_block(&self) -> bool {
        let mut i = self.pos + 1; // Skip the combinator (> or +)
        while i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::WHITESPACE {
            i += 1;
        }
        if i >= self.tokens.len() {
            return false;
        }
        matches!(
            self.tokens[i].kind,
            SyntaxKind::DOT
                | SyntaxKind::HASH
                | SyntaxKind::L_BRACKET
                | SyntaxKind::COLON
                | SyntaxKind::STAR
                | SyntaxKind::AMPERSAND
        )
    }

    /// Check if `&` at current position starts a nested scope block rather than element ref.
    /// Returns true when `&` is followed by selector-continuation tokens (`:`, `.`, `#`, `[`, `{`, `>`, `+`, `*`).
    /// Returns false when `&` is followed by IDENT or `$` (element reference: `&name`, `&$var`).
    fn is_ampersand_scope_block(&self) -> bool {
        let mut i = self.pos + 1; // Skip the &
        // Skip whitespace
        while i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::WHITESPACE {
            i += 1;
        }
        if i >= self.tokens.len() {
            return false;
        }
        matches!(
            self.tokens[i].kind,
            SyntaxKind::COLON
                | SyntaxKind::DOT
                | SyntaxKind::HASH
                | SyntaxKind::L_BRACKET
                | SyntaxKind::L_BRACE
                | SyntaxKind::GT
                | SyntaxKind::PLUS
                | SyntaxKind::TILDE
                | SyntaxKind::STAR
        )
    }

    /// Parse a combinator-leading nested scope block: `> .child { }`, `+ .sibling { }`
    /// The combinator token is included in the SELECTOR node so compose_selector can see it.
    fn parse_combinator_scope_block(&mut self) {
        self.start_node(SyntaxKind::SCOPE_BLOCK);
        self.start_node(SyntaxKind::SELECTOR);
        // Bump the combinator token (>, +) as part of the selector
        self.bump();
        self.skip_trivia();
        // Parse the rest of the selector (the part after the combinator)
        self.parse_selector_part();
        // Continue parsing selector if compound (e.g., > .child.active {})
        loop {
            self.skip_trivia();
            match self.current() {
                SyntaxKind::DOT
                | SyntaxKind::HASH
                | SyntaxKind::L_BRACKET
                | SyntaxKind::COLON
                | SyntaxKind::IDENT
                | SyntaxKind::STAR
                | SyntaxKind::AMPERSAND => self.parse_selector_part(),
                SyntaxKind::GT | SyntaxKind::PLUS | SyntaxKind::TILDE => {
                    self.bump();
                    self.skip_trivia();
                    self.parse_selector_part();
                }
                _ => break,
            }
        }
        self.finish_node(); // SELECTOR
        self.skip_trivia();
        if self.at(SyntaxKind::L_BRACE) {
            self.parse_body();
        }
        self.finish_node(); // SCOPE_BLOCK
    }

    /// Check if current position starts a scope block: .class { } or #id { }
    /// Returns true if the pattern is DOT/HASH + IDENT + (whitespace?) + L_BRACE
    fn is_scope_block(&self) -> bool {
        // We're at DOT (or HASH), check if it's followed by IDENT and eventually L_BRACE
        let mut i = self.pos + 1; // Skip the dot/hash

        // Skip any whitespace
        while i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::WHITESPACE {
            i += 1;
        }

        // Should have an identifier (or keyword used as class/id name)
        if i >= self.tokens.len()
            || (self.tokens[i].kind != SyntaxKind::IDENT && !self.tokens[i].kind.is_keyword())
        {
            return false;
        }
        i += 1;

        // Scan through selector continuation parts to find L_BRACE
        self.scan_selector_to_brace(i)
    }

    /// Check if current position starts an element selector scope block: body { } or div { }
    /// Returns true if we can scan through valid selector syntax and find a `{`.
    /// This distinguishes `body { }` from `price.toFixed(2)` (JS method call).
    fn is_element_scope_block(&self) -> bool {
        // We're at IDENT, check if it's followed by L_BRACE (directly or after selector parts)
        let i = self.pos + 1; // Skip the initial ident
        self.scan_selector_to_brace(i)
    }

    /// Whether a newline-led `:` or `*` begins a selector that reaches a `{`.
    ///
    /// `:root { … }` and `* { … }` start a RULE; a `:` introducing a value and a
    /// `*` meaning multiplication do not. Reuses `scan_selector_to_brace` so the
    /// vocabulary of "what may appear in a selector" stays in one place — only
    /// the count of leading tokens to skip differs (BUG-356).
    fn is_selector_scope_block(&self) -> bool {
        // `::before` — skip the second colon so the scan starts at the name.
        let skip = if self.at(SyntaxKind::COLON) && self.peek_non_trivia(1) == SyntaxKind::COLON {
            2
        } else {
            1
        };
        self.scan_selector_to_brace(self.pos + skip)
    }

    /// Scan from token index `i` through valid CSS selector continuation tokens,
    /// returning true only if the chain eventually reaches L_BRACE.
    /// Used by both `is_scope_block` (.class/# id selectors) and
    /// `is_element_scope_block` (bare element selectors).
    fn scan_selector_to_brace(&self, start: usize) -> bool {
        let mut i = start;

        loop {
            // Skip any whitespace
            while i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::WHITESPACE {
                i += 1;
            }

            if i >= self.tokens.len() {
                return false;
            }

            match self.tokens[i].kind {
                // Found the opening brace - this is a scope block!
                SyntaxKind::L_BRACE => return true,

                // Class selector continuation: body.class { }
                SyntaxKind::DOT => {
                    i += 1;
                    // Skip whitespace after dot
                    while i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::WHITESPACE {
                        i += 1;
                    }
                    // Must be followed by IDENT for class name
                    if i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::IDENT {
                        i += 1;
                        continue;
                    }
                    return false;
                }

                // ID selector continuation: body#id { }
                SyntaxKind::HASH => {
                    i += 1;
                    // Skip whitespace after hash
                    while i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::WHITESPACE {
                        i += 1;
                    }
                    // Must be followed by IDENT for id name
                    if i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::IDENT {
                        i += 1;
                        continue;
                    }
                    return false;
                }

                // Attribute selector: body[attr] { }
                SyntaxKind::L_BRACKET => {
                    // Skip to matching ]
                    let mut depth = 1;
                    i += 1;
                    while i < self.tokens.len() && depth > 0 {
                        match self.tokens[i].kind {
                            SyntaxKind::L_BRACKET => depth += 1,
                            SyntaxKind::R_BRACKET => depth -= 1,
                            _ => {}
                        }
                        i += 1;
                    }
                    continue;
                }

                // Descendant combinator: nav ul { }, body a { }
                SyntaxKind::IDENT => {
                    i += 1;
                    continue;
                }

                // Pseudo-class/element: a:hover, div::before, li:nth-child(2n+1)
                SyntaxKind::COLON => {
                    i += 1;
                    // Double colon for pseudo-element: ::before, ::after
                    if i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::COLON {
                        i += 1;
                    }
                    // Must be followed by IDENT or keyword (pseudo name like hover, first-child)
                    if i < self.tokens.len()
                        && (self.tokens[i].kind == SyntaxKind::IDENT
                            || self.tokens[i].kind.is_keyword())
                    {
                        i += 1;
                    } else {
                        // COLON not followed by pseudo name — this is a CSS property colon
                        return false;
                    }
                    // Optional functional args: :nth-child(2n+1)
                    if i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::L_PAREN {
                        let mut depth = 1;
                        i += 1;
                        while i < self.tokens.len() && depth > 0 {
                            match self.tokens[i].kind {
                                SyntaxKind::L_PAREN => depth += 1,
                                SyntaxKind::R_PAREN => depth -= 1,
                                _ => {}
                            }
                            i += 1;
                        }
                    }
                    continue;
                }

                // Combinators that continue selector: body > div { }, body + div { }
                SyntaxKind::GT | SyntaxKind::PLUS | SyntaxKind::TILDE => {
                    i += 1;
                    // Skip whitespace
                    while i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::WHITESPACE {
                        i += 1;
                    }
                    // Can be followed by IDENT, DOT, HASH, etc.
                    if i < self.tokens.len() {
                        match self.tokens[i].kind {
                            SyntaxKind::IDENT
                            | SyntaxKind::DOT
                            | SyntaxKind::HASH
                            | SyntaxKind::STAR => {
                                if self.tokens[i].kind == SyntaxKind::IDENT {
                                    i += 1;
                                }
                                continue;
                            }
                            _ => return false,
                        }
                    }
                    return false;
                }

                // Selector list: body, div { } - keep scanning
                SyntaxKind::COMMA => {
                    i += 1;
                    // Skip whitespace
                    while i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::WHITESPACE {
                        i += 1;
                    }
                    // Continue to next selector part
                    if i < self.tokens.len()
                        && matches!(
                            self.tokens[i].kind,
                            SyntaxKind::IDENT
                                | SyntaxKind::DOT
                                | SyntaxKind::HASH
                                | SyntaxKind::STAR
                                | SyntaxKind::L_BRACKET
                        )
                    {
                        if self.tokens[i].kind == SyntaxKind::IDENT {
                            i += 1;
                        }
                        continue;
                    }
                    return false;
                }

                // Anything else (L_PAREN for function calls, operators, etc.) - not a selector
                _ => return false,
            }
        }
    }

    /// Check if current position starts an attribute property: [attr]: value
    /// Returns true if the pattern is [...]<whitespace?>: (property)
    /// Returns false if the pattern is [...]<whitespace?>{ (scope block)
    fn is_attribute_property(&self) -> bool {
        // We're at L_BRACKET, need to find the matching R_BRACKET
        // then check what comes after
        let mut depth = 0;
        let mut i = self.pos;

        // Find the matching ]
        while i < self.tokens.len() {
            match self.tokens[i].kind {
                SyntaxKind::L_BRACKET => depth += 1,
                SyntaxKind::R_BRACKET => {
                    depth -= 1;
                    if depth == 0 {
                        // Found matching bracket, look at what's next
                        i += 1;
                        // Skip whitespace
                        while i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::WHITESPACE
                        {
                            i += 1;
                        }
                        if i < self.tokens.len() {
                            // `[attr]: value` (property) OR `[slot="x"] <- $sig` (content
                            // injection into an attribute/slot target, the component-body
                            // arrow form). Both are statements, not nested scope blocks.
                            return self.tokens[i].kind == SyntaxKind::COLON
                                || self.tokens[i].kind == SyntaxKind::LEFT_ARROW;
                        }
                        return false;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        false
    }

    /// Check if current position starts a variable property: $name: value
    /// Returns true if the pattern is DOLLAR + IDENT + (optional whitespace) + COLON
    /// Returns false if there's a type between name and colon (e.g., $name type: value)
    fn is_variable_property(&self) -> bool {
        // We're at DOLLAR
        if self.current() != SyntaxKind::DOLLAR {
            return false;
        }

        // Check for IDENT after DOLLAR
        if self.peek(1) != SyntaxKind::IDENT {
            return false;
        }

        // After IDENT, skip any whitespace to find COLON
        let mut i = 2;
        while self.peek(i) == SyntaxKind::WHITESPACE {
            i += 1;
        }

        // Check if we have a COLON directly (property) or another IDENT (declaration with type)
        self.peek(i) == SyntaxKind::COLON
    }

    /// Check if current position starts a mutation statement: $var <- expr;
    /// Returns true if the pattern is DOLLAR + (IDENT or DOT path)+ + (optional whitespace) + LEFT_ARROW
    fn is_mutation_statement(&self) -> bool {
        // We're at DOLLAR
        if self.current() != SyntaxKind::DOLLAR {
            return false;
        }

        // Skip past the variable reference ($name or $.path or $name.path)
        let mut i = 1;

        // First part after $ can be IDENT or DOT (for $.dataset style)
        if self.peek(i) == SyntaxKind::DOT {
            i += 1;
        }
        if self.peek(i) == SyntaxKind::IDENT {
            i += 1;
        }

        // Handle property path: .field.field...
        loop {
            if self.peek(i) == SyntaxKind::DOT && self.peek(i + 1) == SyntaxKind::IDENT {
                i += 2;
            } else {
                break;
            }
        }

        // Skip any whitespace to find LEFT_ARROW
        while self.peek(i) == SyntaxKind::WHITESPACE {
            i += 1;
        }

        // Check if we have a LEFT_ARROW (<-)
        self.peek(i) == SyntaxKind::LEFT_ARROW
    }

    /// Parse a mutation statement: $var <- expr;
    /// Used for state mutations like $currentId <- $.dataset.packId;
    fn parse_mutation_statement(&mut self) {
        self.start_node(SyntaxKind::CSS_PROPERTY);

        // Parse the target variable reference ($name or $.path or $name.path)
        self.parse_variable_ref();

        self.skip_trivia();

        // <-
        self.expect(SyntaxKind::LEFT_ARROW);
        self.skip_trivia();

        // Parse the expression value
        self.parse_css_value();

        self.skip_trivia();

        // ;
        if self.at(SyntaxKind::SEMICOLON) {
            self.bump();
        }

        self.finish_node();
    }

    /// Parse a variable property: $name: value
    /// Used in %derives blocks for computed values like $rawX: $axis != "y" ? $deltaX : 0
    fn parse_variable_property(&mut self) {
        self.start_node(SyntaxKind::CSS_PROPERTY);

        // Parse $name as the property "name" - we include $ in the node
        self.expect(SyntaxKind::DOLLAR);
        if self.at(SyntaxKind::IDENT) {
            self.bump();
        }

        self.skip_trivia();

        // :
        self.expect(SyntaxKind::COLON);
        self.skip_trivia();

        // Value(s)
        self.parse_css_value();

        self.skip_trivia();

        // ;
        if self.at(SyntaxKind::SEMICOLON) {
            self.bump();
        }

        self.finish_node();
    }

    /// Parse an attribute property: [attr="value"]: property_value;
    fn parse_attribute_property(&mut self) {
        self.start_node(SyntaxKind::CSS_PROPERTY);

        // Parse the attribute selector as the property name
        // Consume everything from [ to ]
        self.expect(SyntaxKind::L_BRACKET);
        while !self.at_eof() && !self.at(SyntaxKind::R_BRACKET) {
            self.bump();
        }
        if self.at(SyntaxKind::R_BRACKET) {
            self.bump();
        }

        self.skip_trivia();

        // Two forms:
        //   `[attr]: value;`      CSS-domain property
        //   `[slot="x"] <- $sig;` DOM-domain content injection (component body arrow)
        // The arrow form keeps the LEFT_ARROW in the CSS_PROPERTY node so the
        // component-body parser sees the verbatim `[..] <- ..` slice and routes it to
        // parse_injection. Either way we consume the value.
        if self.at(SyntaxKind::LEFT_ARROW) {
            self.bump();
        } else {
            self.expect(SyntaxKind::COLON);
        }
        self.skip_trivia();

        // Parse value
        self.parse_css_value();

        self.skip_trivia();

        // Optional ;
        if self.at(SyntaxKind::SEMICOLON) {
            self.bump();
        }

        self.finish_node();
    }

    /// Parse a pseudo block: :hover { ... }
    fn parse_pseudo_block(&mut self) {
        self.start_node(SyntaxKind::SCOPE_BLOCK);

        // Parse the pseudo selector wrapped in a SELECTOR node
        // so it matches the structure expected by ScopeBlock::selector()
        self.start_node(SyntaxKind::SELECTOR);
        self.parse_selector_part();
        self.finish_node(); // SELECTOR

        self.skip_trivia();

        // Parse body if present
        if self.at(SyntaxKind::L_BRACE) {
            self.parse_body();
        }

        self.finish_node();
    }

    /// Parse a CSS property or other identifier-starting statement.
    /// Works with both IDENT tokens and keyword tokens (e.g., `from`, `to`, `on`)
    /// that can serve as property names.
    fn parse_css_property_or_statement(&mut self) {
        // Look ahead to determine if this is a property (name: value;) or (name?: value;)
        // We need to actually see a COLON to confirm it's a property
        // Just having whitespace after IDENT is not enough (e.g., "return value" in JS)
        let mut is_property = false;
        let mut is_arrow = false;
        if self.at(SyntaxKind::IDENT) || self.current().is_keyword() {
            // Check if there's a colon (or `<-` arrow) after optional whitespace and
            // optional question mark. Comments are trivia: `--brand /* n */ : red`
            // is a property (was silently dropped before BUG-241 measured it).
            let mut lookahead = 1;
            while matches!(
                self.peek(lookahead),
                SyntaxKind::WHITESPACE | SyntaxKind::COMMENT
            ) {
                lookahead += 1;
            }
            // Allow for optional question mark (for optional fields like "email?: string")
            if self.peek(lookahead) == SyntaxKind::QUESTION {
                lookahead += 1;
                while self.peek(lookahead) == SyntaxKind::WHITESPACE {
                    lookahead += 1;
                }
            }
            is_property = self.peek(lookahead) == SyntaxKind::COLON;
            // BUG-071: `name <- $expr;` is a DOM content/attr injection at selector scope
            // (FEAT-072 spec: the `<-` surface works at file scope, not only @template
            // bodies). Recognize it here so it isn't dropped as a bare identifier.
            is_arrow = self.peek(lookahead) == SyntaxKind::LEFT_ARROW;
        }

        if is_property {
            self.parse_css_property();
        } else if is_arrow {
            self.parse_arrow_property();
        } else {
            // Just consume the identifier and any following tokens until statement end
            self.bump();
        }
    }

    /// Parse a selector-scope DOM injection `name <- value;` (BUG-071). Mirrors
    /// `parse_css_property` but expects a `LEFT_ARROW` instead of `COLON`, and keeps the
    /// arrow token INSIDE the CSS_PROPERTY node so the lowering (emit_reactive_binding_js,
    /// gated by `is_arrow_injection`) routes it to textContent / setAttribute.
    fn parse_arrow_property(&mut self) {
        self.start_node(SyntaxKind::CSS_PROPERTY);
        if self.at(SyntaxKind::IDENT) || self.current().is_keyword() {
            self.bump();
        } else {
            self.expect(SyntaxKind::IDENT);
        }
        self.skip_trivia();
        self.expect(SyntaxKind::LEFT_ARROW);
        self.skip_trivia();
        self.parse_css_value();
        self.skip_trivia();
        if self.at(SyntaxKind::SEMICOLON) {
            self.bump();
        }
        self.finish_node();
    }

    /// Parse a CSS property: name: value; or name?: value; (optional field)
    /// The property name can be an IDENT or a keyword token (e.g., `from`, `to`, `on`).
    /// Check if `.<ident> :` starts a reactive class-toggle property (no `{` block):
    /// `.active: $v;`. Distinguishes from a scope block (`.x { }`, checked first) and a
    /// stray dot (`.toFixed(`). Looks past whitespace: DOT (ws) IDENT|kw (ws) COLON.
    fn is_class_property(&self) -> bool {
        let mut i = self.pos + 1; // skip DOT
        while i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::WHITESPACE {
            i += 1;
        }
        if i >= self.tokens.len()
            || (self.tokens[i].kind != SyntaxKind::IDENT && !self.tokens[i].kind.is_keyword())
        {
            return false;
        }
        i += 1;
        while i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::WHITESPACE {
            i += 1;
        }
        i < self.tokens.len() && self.tokens[i].kind == SyntaxKind::COLON
    }

    /// Parse a reactive class-toggle property `.active: $v;` (BUG-068). Mirrors
    /// `parse_css_property` but consumes the leading `.` INTO the CSS_PROPERTY node so
    /// `CssProperty::name_text` reconstructs `.active` — the marker the emitter routes on.
    fn parse_class_property(&mut self) {
        self.start_node(SyntaxKind::CSS_PROPERTY);
        self.expect(SyntaxKind::DOT);
        self.skip_trivia();
        if self.at(SyntaxKind::IDENT) || self.current().is_keyword() {
            self.bump();
        } else {
            self.expect(SyntaxKind::IDENT);
        }
        self.skip_trivia();
        self.expect(SyntaxKind::COLON);
        self.skip_trivia();
        self.parse_css_value();
        self.skip_trivia();
        if self.at(SyntaxKind::SEMICOLON) {
            self.bump();
        }
        self.finish_node();
    }

    fn parse_css_property(&mut self) {
        self.start_node(SyntaxKind::CSS_PROPERTY);

        // Property name — accept IDENT or keyword tokens
        if self.at(SyntaxKind::IDENT) || self.current().is_keyword() {
            self.bump();
        } else {
            self.expect(SyntaxKind::IDENT);
        }
        // Optional question mark for optional fields (e.g., "email?: string")
        if self.at(SyntaxKind::QUESTION) {
            self.bump();
        }
        self.skip_trivia();

        // :
        self.expect(SyntaxKind::COLON);
        self.skip_trivia();

        // Value(s)
        self.parse_css_value();

        self.skip_trivia();

        // ;
        if self.at(SyntaxKind::SEMICOLON) {
            self.bump();
        }

        self.finish_node();
    }

    /// Parse CSS value(s) - can include transitions (->), keyframes, etc.
    fn parse_css_value(&mut self) {
        self.start_node(SyntaxKind::CSS_VALUE);

        // Parse values until ; or }
        while !self.at_eof() && !self.at(SyntaxKind::SEMICOLON) && !self.at(SyntaxKind::R_BRACE) {
            let pos_before = self.pos;

            // Check for keyframe block
            if self.at(SyntaxKind::L_BRACE) {
                self.parse_keyframe_block();
                break;
            }

            // Check for transition arrow
            if self.at(SyntaxKind::ARROW) {
                self.bump();
                self.skip_trivia();
                continue;
            }

            // Parse individual value
            self.parse_css_value_part();

            // Check for newline followed by property start BEFORE consuming trivia.
            // This allows properties without semicolons when separated by newlines.
            if self.at_newline_property_start() {
                break;
            }

            self.skip_trivia();

            // Safety: ensure we make progress to avoid infinite loop
            if self.pos == pos_before {
                // Force progress by consuming a token
                self.bump();
            }
        }

        self.finish_node();
    }

    /// Peek: is the next non-trivia token after the current one an IDENT
    /// with the given text? (Used for the `%uses` clause lookahead.)
    fn peek_next_non_trivia_is_ident(&self, text: &str) -> bool {
        let mut i = self.pos + 1;
        while i < self.tokens.len() && self.tokens[i].kind.is_trivia() {
            i += 1;
        }
        self.tokens
            .get(i)
            .is_some_and(|t| t.kind == SyntaxKind::IDENT && t.text == text)
    }

    /// Check if the L_PAREN at the current position starts an arrow function: `(params) => expr`
    /// Scans ahead to find the matching `)` and checks if followed by `=>` (FAT_ARROW).
    fn is_arrow_fn(&self) -> bool {
        if !self.at(SyntaxKind::L_PAREN) {
            return false;
        }
        let mut i = self.pos + 1;
        let mut depth = 1;
        // Find matching R_PAREN
        while i < self.tokens.len() && depth > 0 {
            match self.tokens[i].kind {
                SyntaxKind::L_PAREN => depth += 1,
                SyntaxKind::R_PAREN => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        // Skip trivia after )
        while i < self.tokens.len() && self.tokens[i].kind.is_trivia() {
            i += 1;
        }
        // FAT_ARROW is the current tokenization. Keep the legacy pair branch
        // while parsing older token streams / fixtures.
        self.tokens
            .get(i)
            .is_some_and(|token| token.kind == SyntaxKind::FAT_ARROW)
            || (i + 1 < self.tokens.len()
                && self.tokens[i].kind == SyntaxKind::EQUALS
                && self.tokens[i + 1].kind == SyntaxKind::GT)
    }

    /// Consume an arrow function `(params) => body` WHOLE, with balanced bracket
    /// tracking, as a single opaque expression node. The body is JS (an @eval /
    /// @drive / @when payload): a `{ … }` block body is consumed by matching braces
    /// (so nested object/array/call literals — `{}`, `[]`, `()` — inside it never
    /// terminate early; BUG-095); an expression body `() => expr` is consumed up to
    /// the enclosing boundary (`,` / `)` / `;` / `}` at depth 0). Precondition:
    /// `is_arrow_fn()` is true at the current `(`.
    ///
    /// LIMITATION (FUP-046): brace balance counts raw L_BRACE/R_BRACE
    /// tokens. The lexer emits single STRING tokens for `'`/`"` quotes, so a brace
    /// inside a quoted string is safe — but it has NO token for TEMPLATE LITERALS
    /// (backtick) or REGEX literals, whose contents lex as ordinary tokens. A literal
    /// `}` inside `` `…}…` `` or `/}/ ` would mis-count and terminate the body early.
    /// No current `.st` arrow payload uses those; until the lexer tracks template/
    /// regex spans, avoid an unbraced literal `}` inside a template/regex in an @eval
    /// body (or wrap it in a quoted string).
    fn parse_arrow_fn_balanced(&mut self) {
        self.start_node(SyntaxKind::EXPR);
        // 1) Consume the parameter list `( … )` with matched parens.
        let mut depth = 0i32;
        while !self.at_eof() {
            match self.current() {
                SyntaxKind::L_PAREN => {
                    depth += 1;
                    self.bump();
                }
                SyntaxKind::R_PAREN => {
                    depth -= 1;
                    self.bump();
                    if depth <= 0 {
                        break;
                    }
                }
                _ => self.bump(),
            }
        }
        self.skip_trivia();
        // 2) Consume `=>` (FAT_ARROW; retain legacy pair compatibility).
        if self.at(SyntaxKind::FAT_ARROW) {
            self.bump();
        } else {
            if self.at(SyntaxKind::EQUALS) {
                self.bump();
            }
            if self.at(SyntaxKind::GT) {
                self.bump();
            }
        }
        self.skip_trivia();
        // 3) Consume the body.
        if self.at(SyntaxKind::L_BRACE) {
            // Block body: consume balanced braces, tracking ALL bracket kinds so
            // nested arrays/objects/calls inside the JS never close the body early.
            let mut bdepth = 0i32;
            while !self.at_eof() {
                match self.current() {
                    SyntaxKind::L_BRACE | SyntaxKind::L_BRACKET | SyntaxKind::L_PAREN => {
                        if self.at(SyntaxKind::L_BRACE) {
                            bdepth += 1;
                        }
                        self.bump();
                    }
                    SyntaxKind::R_BRACE => {
                        bdepth -= 1;
                        self.bump();
                        if bdepth <= 0 {
                            break;
                        }
                    }
                    SyntaxKind::R_BRACKET | SyntaxKind::R_PAREN => self.bump(),
                    _ => self.bump(),
                }
            }
        } else {
            // Expression body `() => expr`: consume to the enclosing boundary at
            // depth 0 (`,`/`)`/`;`/`}`), tracking nested brackets so a `{}`/`[]`/`()`
            // in the expression doesn't stop it early.
            let mut edepth = 0i32;
            while !self.at_eof() {
                match self.current() {
                    SyntaxKind::L_PAREN | SyntaxKind::L_BRACKET | SyntaxKind::L_BRACE => {
                        edepth += 1;
                        self.bump();
                    }
                    SyntaxKind::R_PAREN | SyntaxKind::R_BRACKET | SyntaxKind::R_BRACE => {
                        if edepth <= 0 {
                            break;
                        }
                        edepth -= 1;
                        self.bump();
                    }
                    SyntaxKind::SEMICOLON | SyntaxKind::COMMA if edepth == 0 => break,
                    _ => self.bump(),
                }
            }
        }
        self.finish_node();
    }

    /// Parse a single CSS value part
    fn parse_css_value_part(&mut self) {
        match self.current() {
            SyntaxKind::DOLLAR => self.parse_variable_ref(),
            SyntaxKind::AMPERSAND => self.parse_element_ref(),
            SyntaxKind::TILDE => self.preset_ref_retired(),
            SyntaxKind::L_PAREN if self.is_arrow_fn() => {
                // Arrow function: (params) => expr — consume all tokens until ; or }
                while !self.at_eof()
                    && !self.at(SyntaxKind::SEMICOLON)
                    && !self.at(SyntaxKind::R_BRACE)
                {
                    self.bump();
                }
            }
            SyntaxKind::L_PAREN => self.parse_paren_expr(),
            SyntaxKind::STRING
            | SyntaxKind::NUMBER
            | SyntaxKind::NUMBER_WITH_UNIT
            | SyntaxKind::COLOR
            | SyntaxKind::IDENT => {
                // A dimension's unit rides with its number (Kernel tier). The
                // function-call check that follows still sees the token after
                // the whole value, so `translate(40px)` is unaffected.
                self.bump_value_atom();
                // Check for function call
                if self.at(SyntaxKind::L_PAREN) {
                    self.parse_arg_list();
                }
            }
            // Keywords used in CSS values
            SyntaxKind::KW_IN | SyntaxKind::KW_WITH => {
                self.bump();
            }
            // Operators and other tokens that can appear in expressions
            SyntaxKind::PLUS
            | SyntaxKind::MINUS
            | SyntaxKind::STAR
            | SyntaxKind::SLASH
            | SyntaxKind::DOT
            | SyntaxKind::COMMA
            | SyntaxKind::PERCENT
            | SyntaxKind::EQUALS
            | SyntaxKind::LT
            | SyntaxKind::GT
            | SyntaxKind::EXCLAIM
            | SyntaxKind::PIPE
            | SyntaxKind::QUESTION
            | SyntaxKind::QUESTION_QUESTION
            | SyntaxKind::CARET
            | SyntaxKind::AND_AND
            | SyntaxKind::OR_OR
            | SyntaxKind::EQ_EQ
            | SyntaxKind::EQ_EQ_EQ
            | SyntaxKind::NOT_EQ
            | SyntaxKind::NOT_EQ_EQ
            | SyntaxKind::LT_EQ
            | SyntaxKind::GT_EQ => {
                self.bump();
            }
            // Stop at statement/value boundaries
            SyntaxKind::SEMICOLON
            | SyntaxKind::R_BRACE
            | SyntaxKind::R_PAREN
            | SyntaxKind::R_BRACKET => {
                // Don't consume - let caller handle
            }
            _ => {
                // Unknown token in value context - consume to ensure progress
                // This prevents infinite loops when encountering unexpected tokens
                self.bump();
            }
        }
    }

    /// Parse a keyframe block: { 0%: v; 50%: v; 100%: v; }
    fn parse_keyframe_block(&mut self) {
        self.start_node(SyntaxKind::KEYFRAME_BLOCK);

        self.expect(SyntaxKind::L_BRACE);
        self.skip_trivia();

        while !self.at_eof() && !self.at(SyntaxKind::R_BRACE) {
            let pos_before = self.pos;
            self.parse_keyframe();
            self.skip_trivia();
            // Safety: ensure we make progress
            if self.pos == pos_before && !self.at(SyntaxKind::R_BRACE) && !self.at_eof() {
                self.bump();
            }
        }

        self.expect(SyntaxKind::R_BRACE);

        self.finish_node();
    }

    /// Parse a single keyframe: 50%: value; or 50%: value
    fn parse_keyframe(&mut self) {
        self.start_node(SyntaxKind::KEYFRAME);

        let start_pos = self.pos;

        // Percentage (could be NUMBER_WITH_UNIT like 50%)
        if self.at(SyntaxKind::NUMBER_WITH_UNIT) || self.at(SyntaxKind::NUMBER) {
            self.bump();
        }

        self.skip_trivia();

        // :
        if self.at(SyntaxKind::COLON) {
            self.bump();
            self.skip_trivia();
            // Value
            self.parse_css_value_part();
        }

        self.skip_trivia();

        // Optional ;
        if self.at(SyntaxKind::SEMICOLON) {
            self.bump();
        }

        // Safety: ensure we make progress to avoid infinite loop
        // If we didn't consume anything, consume one token
        if self.pos == start_pos && !self.at(SyntaxKind::R_BRACE) && !self.at_eof() {
            self.bump();
        }

        self.finish_node();
    }

    /// Parse an if statement: if condition { ... } else { ... }
    fn parse_if_statement(&mut self) {
        self.start_node(SyntaxKind::EXPR);

        self.expect(SyntaxKind::KW_IF);
        self.skip_trivia();

        // Condition
        self.parse_arg_value();
        self.skip_trivia();

        // Body
        if self.at(SyntaxKind::L_BRACE) {
            self.parse_body();
        }

        self.skip_trivia();

        // Optional else
        if self.at(SyntaxKind::KW_ELSE) {
            self.bump();
            self.skip_trivia();

            if self.at(SyntaxKind::KW_IF) {
                // else if
                self.parse_if_statement();
            } else if self.at(SyntaxKind::L_BRACE) {
                self.parse_body();
            }
        }

        self.finish_node();
    }

    /// Parse a parenthesized expression
    fn parse_paren_expr(&mut self) {
        self.start_node(SyntaxKind::PAREN_EXPR);
        self.expect(SyntaxKind::L_PAREN);
        self.skip_trivia();
        self.parse_arg_value();
        self.skip_trivia();
        self.expect(SyntaxKind::R_PAREN);
        self.finish_node();
    }

    /// Parse an array expression: [a, b, c]
    fn parse_array_expr(&mut self) {
        self.start_node(SyntaxKind::ARRAY_EXPR);
        self.expect(SyntaxKind::L_BRACKET);
        self.skip_trivia();

        while !self.at_eof() && !self.at(SyntaxKind::R_BRACKET) {
            self.parse_arg_value();
            self.skip_trivia();

            if self.at(SyntaxKind::COMMA) {
                self.bump();
                self.skip_trivia();
            } else {
                break;
            }
        }

        self.expect(SyntaxKind::R_BRACKET);
        self.finish_node();
    }

    /// Parse an object expression or body: { key: value, ... }
    fn parse_object_or_body(&mut self) {
        // Look ahead to determine if this is an object or a body
        // Objects have key: value pairs, bodies have statements

        // For now, treat { } as a body in most contexts
        // A more sophisticated approach would track context
        self.parse_body();
    }

    /// Build and return the parse result
    fn finish(self) -> ParseResult {
        let green = self.builder.finish();
        ParseResult {
            root: SyntaxNode::new_root(green),
            errors: self.errors,
        }
    }
}

/// Parse Spacetime source code into a CST
// === PLAN-023 W0: HTML region scanning support ===

/// One ordered piece of a scanned HTML region: either verbatim raw markup or a
/// `` `expr` `` hole whose inner is a Spacetime expression string.
enum HtmlSeg {
    Raw(String),
    Hole(String),
}

/// Result of scanning an HTML region: end byte offset (one past) + ordered segments.
struct HtmlScan {
    end: usize,
    segs: Vec<HtmlSeg>,
}

/// Slice `[from..to)` of a UTF-8 byte buffer into an owned String (lossless; the buffer
/// originates from valid UTF-8 source so slices land on char boundaries by construction —
/// every boundary we cut on is an ASCII delimiter `< > / \" ' \` `).
fn self_source_slice(b: &[u8], from: usize, to: usize) -> String {
    String::from_utf8_lossy(&b[from..to]).into_owned()
}

/// Read an HTML tag name starting at `i` (just past `<`). Returns (lowercased name, index
/// one past the name).
fn html_read_tag_name(b: &[u8], mut i: usize) -> (String, usize) {
    let start = i;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_alphanumeric() || c == b'-' || c == b':' {
            i += 1;
        } else {
            break;
        }
    }
    (
        String::from_utf8_lossy(&b[start..i]).to_ascii_lowercase(),
        i,
    )
}

/// Find the next byte `needle` at or after `from`.
fn html_find(b: &[u8], from: usize, needle: u8) -> Option<usize> {
    (from..b.len()).find(|&k| b[k] == needle)
}

/// Find the byte index of `seq` at or after `from` (returns the index of its first byte).
fn html_find_seq(b: &[u8], from: usize, seq: &[u8]) -> Option<usize> {
    if seq.is_empty() || from >= b.len() {
        return None;
    }
    (from..=b.len().saturating_sub(seq.len())).find(|&k| &b[k..k + seq.len()] == seq)
}

/// True if position `i` begins a close tag `</name` for `name` (case-insensitive).
fn html_matches_close_tag(b: &[u8], i: usize, name: &str) -> bool {
    if i + 1 >= b.len() || b[i] != b'<' || b[i + 1] != b'/' {
        return false;
    }
    let (got, _) = html_read_tag_name(b, i + 2);
    got == name
}

/// HTML void elements (no content, no close tag). Mirrors emit/html.rs::is_void_element.
/// Standalone byte-scan returning the END offset (one past) of the balanced HTML region
/// beginning at `start` (a `<`). Depth-aware over element nesting; respects void/
/// self-closing tags, comments, attribute strings, backtick holes, and raw-text elements.
/// Used by the EVENTS parser (which has no HTML grammar) to SKIP an HTML region wholesale
/// so `$`-refs / holes inside markup never produce spurious FormMatches (BUG-043). Mirrors
/// the segment-producing `scan_html_region`; kept as a free fn so both passes share it.
pub fn scan_html_end(source: &str, start: usize) -> usize {
    let b = source.as_bytes();
    let len = b.len();
    let mut i = start;
    let mut depth: i32 = 0;
    let mut in_tag = false;
    let mut cur_open = false;
    let mut cur_void = false;
    let mut cur_rawtext: Option<String> = None;
    let mut in_str: Option<u8> = None;
    let mut raw_text: Option<String> = None;

    while i < len {
        if let Some(ref tn) = raw_text {
            if html_matches_close_tag(b, i, tn) {
                let gt = html_find(b, i, b'>').unwrap_or(len - 1);
                i = gt + 1;
                depth -= 1;
                raw_text = None;
                if depth <= 0 {
                    return i;
                }
            } else {
                i += 1;
            }
            continue;
        }
        let c = b[i];
        // Backtick holes (text or attr-string position): skip with `\` escape.
        if !in_tag || in_str.is_some() {
            if c == b'\\' && i + 1 < len && b[i + 1] == b'`' {
                i += 2;
                continue;
            }
            if c == b'`' {
                let mut j = i + 1;
                while j < len {
                    if b[j] == b'\\' && j + 1 < len && b[j + 1] == b'`' {
                        j += 2;
                    } else if b[j] == b'`' {
                        break;
                    } else {
                        j += 1;
                    }
                }
                i = (j + 1).min(len);
                continue;
            }
        }
        if let Some(q) = in_str {
            if c == q {
                in_str = None;
            }
            i += 1;
            continue;
        }
        // HTML comment `<!-- ... -->`
        if !in_tag && c == b'<' && i + 3 < len && &b[i + 1..i + 4] == b"!--" {
            if let Some(end) = html_find_seq(b, i + 4, b"-->") {
                i = end + 3;
            } else {
                i = len;
            }
            continue;
        }
        if !in_tag && c == b'<' {
            in_tag = true;
            if i + 1 < len && b[i + 1] == b'/' {
                // close tag
                cur_open = false;
                let (_n, _) = html_read_tag_name(b, i + 2);
            } else {
                cur_open = true;
                let (name, _) = html_read_tag_name(b, i + 1);
                cur_void = html_is_void(&name);
                cur_rawtext = if html_is_raw_text(&name) {
                    Some(name)
                } else {
                    None
                };
            }
            i += 1;
            continue;
        }
        if in_tag {
            if c == b'"' || c == b'\'' {
                in_str = Some(c);
                i += 1;
                continue;
            }
            if c == b'>' {
                let self_close = i > 0 && b[i - 1] == b'/';
                in_tag = false;
                if cur_open {
                    if self_close || cur_void {
                        // No depth change. A void/self-closing element AT depth 0 is the
                        // whole region — mirror scan_html_region and return immediately,
                        // else (no close tag follows) the scan runs past the region end
                        // and the events parser over-skips later directives (BUG-043).
                        if depth <= 0 {
                            return i + 1;
                        }
                    } else if let Some(tn) = cur_rawtext.take() {
                        depth += 1;
                        raw_text = Some(tn);
                    } else {
                        depth += 1;
                    }
                } else {
                    depth -= 1;
                    if depth <= 0 {
                        return i + 1;
                    }
                }
                cur_open = false;
                cur_void = false;
                i += 1;
                continue;
            }
            i += 1;
            continue;
        }
        i += 1;
    }
    len
}

fn html_is_void(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// HTML raw-text / escapable-raw-text elements: their body is NOT markup and MUST NOT have
/// holes extracted (per html5ever CDATA rules; PLAN-023).
fn html_is_raw_text(tag: &str) -> bool {
    matches!(tag, "script" | "style" | "textarea" | "title")
}

pub fn parse(input: &str) -> ParseResult {
    let lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens, input.to_string());
    parser.parse_root();
    parser.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(input: &str) -> SyntaxNode {
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "Parse errors: {:?}",
            result.errors
        );
        result.root
    }

    fn parse_with_errors(input: &str) -> ParseResult {
        parse(input)
    }

    #[test]
    fn test_empty_file() {
        let root = parse_ok("");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_whitespace_only() {
        let root = parse_ok("   \n\t  ");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_simple_scope() {
        let root = parse_ok(".hero { }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);

        // Find the scope block
        let scope = root
            .children()
            .find(|n| n.kind() == SyntaxKind::SCOPE_BLOCK);
        assert!(scope.is_some());
    }

    #[test]
    fn test_scope_with_class() {
        let root = parse_ok(".my-class { }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_scope_with_id() {
        let root = parse_ok("#main { }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_directive() {
        let root = parse_ok("@import \"styles.st\";");
        let directive = root.children().find(|n| n.kind() == SyntaxKind::DIRECTIVE);
        assert!(directive.is_some());
    }

    #[test]
    fn test_qualified_directive_name() {
        // FEAT-118: `@scene/camera` parses as ONE directive whose name_text is
        // the qualified `scene/camera`.
        use crate::syntax::cst::ast::{AstNode, Directive};
        let root = parse_ok("@scene/camera(fov: 75) { }");
        let dir = root
            .descendants()
            .find_map(Directive::cast)
            .expect("directive");
        assert_eq!(dir.name_text().as_deref(), Some("scene/camera"));
    }

    #[test]
    fn test_multi_segment_qualified_name() {
        use crate::syntax::cst::ast::{AstNode, Directive};
        let root = parse_ok("@a/b/c { }");
        let dir = root
            .descendants()
            .find_map(Directive::cast)
            .expect("directive");
        assert_eq!(dir.name_text().as_deref(), Some("a/b/c"));
    }

    #[test]
    fn test_unqualified_name_unchanged() {
        // Regression guard: a plain directive name is returned verbatim.
        use crate::syntax::cst::ast::{AstNode, Directive};
        let root = parse_ok("@camera(fov: 75) { }");
        let dir = root
            .descendants()
            .find_map(Directive::cast)
            .expect("directive");
        assert_eq!(dir.name_text().as_deref(), Some("camera"));
    }

    #[test]
    fn test_spaced_slash_not_a_qualifier() {
        // CSS shorthand in VALUE position must NOT be eaten as a name qualifier:
        // `@grid cols: 1 / 3` — the spaced `/` ends the name run at `grid`.
        use crate::syntax::cst::ast::{AstNode, Directive};
        let root = parse_ok("@grid cols: 1 / 3 { }");
        let dir = root
            .descendants()
            .find_map(Directive::cast)
            .expect("directive");
        assert_eq!(
            dir.name_text().as_deref(),
            Some("grid"),
            "a spaced / in value position is not a name qualifier"
        );
    }

    #[test]
    fn test_directive_with_args() {
        let root = parse_ok("@animate(fade-in, 500ms) { }");
        let directive = root.children().find(|n| n.kind() == SyntaxKind::DIRECTIVE);
        assert!(directive.is_some());
    }

    #[test]
    fn test_directive_with_named_args() {
        let root = parse_ok("@state_machine(initial: \"idle\") { }");
        let directive = root.children().find(|n| n.kind() == SyntaxKind::DIRECTIVE);
        assert!(directive.is_some());
    }

    #[test]
    fn test_variable_decl() {
        let root = parse_ok("$count number : 0;");
        let var_ref = root
            .children()
            .find(|n| n.kind() == SyntaxKind::VARIABLE_REF);
        assert!(var_ref.is_some());
    }

    #[test]
    fn test_meta_def() {
        let root = parse_ok("%macro { }");
        let meta_def = root.children().find(|n| n.kind() == SyntaxKind::META_DEF);
        assert!(meta_def.is_some());
    }

    #[test]
    fn test_capture_type_def() {
        // %capture_type should be parsed as META_DEF, not DIRECTIVE
        let root = parse_ok("%capture_type param_list { ($name:binding)* }");
        let meta_def = root.children().find(|n| n.kind() == SyntaxKind::META_DEF);
        assert!(
            meta_def.is_some(),
            "capture_type should produce META_DEF node"
        );

        // Verify no DIRECTIVE node was created
        let directive = root.children().find(|n| n.kind() == SyntaxKind::DIRECTIVE);
        assert!(
            directive.is_none(),
            "capture_type should NOT produce DIRECTIVE node"
        );
    }

    #[test]
    fn test_runtime_registry_def() {
        // %runtime-registry should also be parsed as META_DEF
        let root = parse_ok("%runtime-registry functions { }");
        let meta_def = root.children().find(|n| n.kind() == SyntaxKind::META_DEF);
        assert!(
            meta_def.is_some(),
            "runtime-registry should produce META_DEF node"
        );
    }

    #[test]
    fn test_css_property() {
        let root = parse_ok(".test { opacity: 1; }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_css_transition() {
        let root = parse_ok(".test { opacity: 0 -> 1; }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_nested_scope() {
        let root = parse_ok(".parent { .child { } }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_error_recovery_missing_brace() {
        let result = parse_with_errors(".hero {");
        // Should still produce a tree
        assert_eq!(result.root.kind(), SyntaxKind::ROOT);
        // But with errors
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_error_recovery_invalid_token() {
        let result = parse_with_errors(".hero { @@@ }");
        // Should still produce a tree with error nodes
        assert_eq!(result.root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_lossless_roundtrip() {
        let input = ".hero {\n    opacity: 0 -> 1;\n}";
        let root = parse_ok(input);
        // The tree should contain all the original text
        assert_eq!(root.text().to_string(), input);
    }

    /// FUP-172: an ident-led call in directive VALUE position, with a trailing
    /// `.member`, is ONE expression.
    ///
    /// It used to parse as an arg-list that stopped at `)`, stranding `.join` to
    /// be re-read at directive level as a `.class` scope — "expected '{' after
    /// selector", pointing several tokens past the real problem. It hid in the
    /// comments pill for 22 commits because every test there asserted on SOURCE
    /// TEXT, which cannot fail on a file the compiler rejects.
    #[test]
    fn a_value_position_call_with_a_trailing_member_is_one_expression() {
        for value in [
            "Array.from($a).join(\", \")",
            "Array.from(new Set($a)).join(\", \")",
            "new Set($a).size",
            "Array.from(new Set($a.map(($x) => $x.f))).join(\", \")",
            // The `(`-immediately-after-`:` forms that always worked; they must
            // keep working (BUG-077's original case).
            "($a).join(\", \")",
            "($a || []).length",
        ] {
            let source =
                format!("@data derive $a array : [];\n@data derive $b string : {value};\n");
            let parsed = parse(&source);
            assert!(
                parsed.errors.is_empty(),
                "`{value}` must parse as one value expression, got: {:?}",
                parsed.errors
            );
        }
    }

    /// BUG-342 — a TYPED declaration whose value is an object literal.
    ///
    /// This CST-level gate PASSES: the concrete syntax tree handles
    /// `$u object: { name: "Ada" };` fine. It is kept as the boundary marker
    /// showing the defect is NOT here — `spacetime check` on the same source
    /// still fails E0946 "Expected capture `$value:Expr` but no tokens
    /// remaining", so the value is lost somewhere between this tree and the
    /// form matcher.
    ///
    /// Narrowed (see BUG-342): only TYPE + BRACE fails. `$u: { … }` without a
    /// type, `$u object: [1,2]`, `$u object: 3` and `$l string[]: ["a"]` all
    /// compile. Ruled out by reading the code: `ExprExtractor` (tracks brace
    /// depth correctly), `TyperefExtractor` (bounded, stops at the colon), and
    /// the `%form` in stdlib/syntax/local-state.st (the `balanced(';')` fix was
    /// tested with a full rebuild — no effect).
    #[test]
    fn a_typed_declaration_accepts_an_object_literal_value() {
        for source in [
            "$u object: { name: \"Ada\" };\n",
            "$u Foo: { a: 1, b: 2 };\n",
            "$u object: { nested: { deep: 1 } };\n",
        ] {
            let parsed = parse(source);
            assert!(
                parsed.errors.is_empty(),
                "`{source}` must parse, got: {:?}",
                parsed.errors
            );
        }
    }

    /// The counter-case the narrow fix exists to protect. A value-colon call
    /// WITHOUT a trailing member is a genuine arg-list and must stay one —
    /// making the value-position flag merely sticky broke exactly this.
    #[test]
    fn a_value_position_call_without_a_trailing_member_stays_an_arg_list() {
        // Presets are spelled `&name` — `~name` was retired and now parses to a
        // deliberate error, so the pre-BUG-347 fixtures could never satisfy
        // `errors.is_empty()`. The invariant under test is about the ARG LIST,
        // not the sigil, so it survives the rename intact.
        for source in [
            "@preset easing &soft-exit: cubic-bezier(0.4, 0, 0.2, 1);\n",
            "@preset easing &my-bounce: spring(350, 15, 1);\n",
        ] {
            let parsed = parse(source);
            assert!(
                parsed.errors.is_empty(),
                "`{source}` must still parse, got: {:?}",
                parsed.errors
            );
        }
    }

    #[test]
    fn test_preserves_comments() {
        let input = "// comment\n.hero { }";
        let root = parse_ok(input);
        assert_eq!(root.text().to_string(), input);
    }

    #[test]
    fn test_preserves_whitespace() {
        let input = "   .hero   {   }   ";
        let root = parse_ok(input);
        assert_eq!(root.text().to_string(), input);
    }

    #[test]
    fn test_complex_selector() {
        let root = parse_ok(".parent > .child + .sibling { }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_attribute_selector() {
        let root = parse_ok("[data-state=\"active\"] { }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_pseudo_class() {
        let root = parse_ok(".button:hover { }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_pseudo_element() {
        let root = parse_ok(".item::before { }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_universal_selector() {
        let root = parse_ok("* { }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_element_reference() {
        let root = parse_ok("&button.opacity");
        // This is parsed at top level, which isn't typical but tests the parsing
        let elem_ref = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::ELEMENT_REF);
        assert!(elem_ref.is_some());
    }

    #[test]
    fn test_form_ref_statement() {
        // BUG-241: a statement-position `--name;` is a FORM_REF, not a drop.
        let root = parse_ok(".a { --card-surface; }");
        let form_ref = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::FORM_REF);
        assert!(form_ref.is_some(), "--card-surface; must parse as FORM_REF");
    }

    #[test]
    fn test_form_ref_with_args() {
        let root = parse_ok(".a { --rise(12px); }");
        let form_ref = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::FORM_REF)
            .expect("--rise(12px); must parse as FORM_REF");
        let has_args = form_ref
            .children()
            .any(|n| n.kind() == SyntaxKind::ARG_LIST);
        assert!(has_args, "the call-site args must be kept");
    }

    #[test]
    fn test_custom_property_is_not_a_form_ref() {
        // The `:` guard: `--brand: red;` stays a CSS_PROPERTY.
        let root = parse_ok(".a { --brand: red; }");
        let form_ref = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::FORM_REF);
        assert!(form_ref.is_none(), "a custom property is not a splice");
        let prop = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::CSS_PROPERTY);
        assert!(prop.is_some(), "it must remain a CSS property");
    }

    #[test]
    fn test_keyframe_block() {
        let root = parse_ok(".test { opacity: { 0%: 0; 100%: 1; }; }");
        let keyframe_block = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::KEYFRAME_BLOCK);
        assert!(keyframe_block.is_some());
    }

    #[test]
    fn test_as_clause() {
        let root = parse_ok("@data(\"api/items\") as $items;");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_if_statement() {
        let root = parse_ok(".test { if $visible { opacity: 1; } else { opacity: 0; } }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_array_expression() {
        let root = parse_ok("@test([1, 2, 3]);");
        let array_expr = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::ARRAY_EXPR);
        assert!(array_expr.is_some());
    }

    #[test]
    fn test_functional_pseudo_class() {
        let root = parse_ok(".item:nth-child(2n+1) { }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_directive_inline_args() {
        let root = parse_ok("@animate fade-in 500ms { }");
        assert_eq!(root.kind(), SyntaxKind::ROOT);
    }

    #[test]
    fn test_macro_with_binds() {
        // This should parse without errors - tests the %binds clause parsing inside %macro
        let input = r#"%macro edit {
  %form {
    @edit(trigger: $trigger:ident)
  }

  %binds {
    contenteditable-enable(&self, state: "editing")
  }

  %binds {
    contenteditable-disable(&self, state: "editing")
  }
}"#;
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "Parse errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_macro_with_nested_scope_blocks() {
        // Tests that nested brace blocks (like "viewing { }") inside directive bodies
        // don't confuse the parser about which } closes which block
        let input = r#"%macro edit {
  %includes {
    @state_machine {
      viewing { cursor: pointer }
      editing { outline: 2px solid blue }
    }
  }
}"#;
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "Parse errors: {:?}",
            result.errors
        );

        // Verify the structure is correct - META_DEF should contain the whole thing
        let meta_def = result
            .root
            .children()
            .find(|n| n.kind() == SyntaxKind::META_DEF);
        assert!(meta_def.is_some(), "Should have META_DEF node");
    }

    #[test]
    fn test_variable_property() {
        // Variable properties like $rawX: value used in %derives blocks
        let root = parse_ok(".test { $rawX: 42; }");

        // Find the CSS_PROPERTY node
        let css_prop = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::CSS_PROPERTY);
        assert!(css_prop.is_some(), "Should parse $rawX: 42 as CSS_PROPERTY");

        // Check the structure includes DOLLAR and IDENT
        let prop = css_prop.unwrap();
        let has_dollar = prop
            .children_with_tokens()
            .any(|t| t.kind() == SyntaxKind::DOLLAR);
        let has_ident = prop
            .children_with_tokens()
            .any(|t| t.kind() == SyntaxKind::IDENT);
        assert!(has_dollar, "CSS_PROPERTY should contain DOLLAR");
        assert!(has_ident, "CSS_PROPERTY should contain IDENT");
    }

    #[test]
    fn test_variable_property_with_expression() {
        // More complex expression
        let root = parse_ok(".test { $rawX: $axis != \"y\" ? $deltaX : 0 }");

        let css_prop = root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::CSS_PROPERTY);
        assert!(
            css_prop.is_some(),
            "Should parse variable property with expression"
        );
    }

    #[test]
    fn test_derives_body_with_variable_properties() {
        // This matches what %derives blocks contain
        let input = r#"%macro test {
  %derives {
    $rawX: $axis != "y" ? $deltaX : 0
    $rawY: $axis != "x" ? $deltaY : 0
  }
}"#;
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "Parse errors: {:?}",
            result.errors
        );

        // Find the derives directive
        let derives_dir = result
            .root
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::DIRECTIVE)
            .find(|n| n.text().to_string().contains("derives"));
        assert!(derives_dir.is_some(), "Should find %derives directive");

        // Find CSS_PROPERTY nodes within derives body
        let derives = derives_dir.unwrap();
        let props: Vec<_> = derives
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::CSS_PROPERTY)
            .collect();

        // We should have 2 properties: $rawX and $rawY
        assert!(
            props.len() >= 2,
            "Should have at least 2 CSS_PROPERTY nodes in derives, found {}",
            props.len()
        );
    }

    #[test]
    fn test_method_call_in_ternary() {
        // Test case for the type-data.st issue
        let input = r#".test {
  $step3: $sortBy ? $step2.sort($sortBy, $sortDir) : $step2
}"#;
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "Parse errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_simple_method_call() {
        // Simpler test case
        let input = r#".test {
  result: $arr.sort($a, $b)
}"#;
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "Parse errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_mutation_statement() {
        // Test $var <- expr; mutation statement parsing
        let input = r#".test {
    $currentId <- $.dataset.packId;
}"#;
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "Parse errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_mutation_in_on_block() {
        // Test mutation inside @on click block
        let input = r#".pack-card {
    @on &.click {
        $currentId <- $.dataset.packId;
    }
}"#;
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "Parse errors: {:?}",
            result.errors
        );
    }

    // === PLAN-023 W0: first-class HTML (< sigil) + backtick holes ===

    /// Helper: collect the verbatim text under an HTML_ELEMENT node, with
    /// HTML_HOLE subtrees rendered as `<inner>` for assertion convenience.
    fn html_element_node(root: &SyntaxNode) -> Option<SyntaxNode> {
        root.children()
            .find(|n| n.kind() == SyntaxKind::HTML_ELEMENT)
    }

    #[test]
    fn test_html_top_level_simple() {
        // valid HTML is valid Spacetime: a top-level element parses cleanly.
        let result = parse("<main><h1>Hello</h1></main>");
        assert!(
            result.errors.is_empty(),
            "top-level HTML should parse without errors: {:?}",
            result.errors
        );
        let html = html_element_node(&result.root);
        assert!(html.is_some(), "expected an HTML_ELEMENT node at top level");
        // The node text must round-trip the exact source.
        assert_eq!(
            html.unwrap().text().to_string(),
            "<main><h1>Hello</h1></main>"
        );
    }

    #[test]
    fn test_html_void_and_attrs() {
        let result = parse("<img src=\"a.png\" alt=\"x\">");
        assert!(
            result.errors.is_empty(),
            "void element with attrs should parse: {:?}",
            result.errors
        );
        assert!(html_element_node(&result.root).is_some());
    }

    #[test]
    fn test_html_hole_in_text() {
        // `<li>`$x`</li>` — a backtick hole in text position becomes an HTML_HOLE node.
        let result = parse("<li>`$x`</li>");
        assert!(
            result.errors.is_empty(),
            "hole in text position should parse: {:?}",
            result.errors
        );
        let html = html_element_node(&result.root).expect("HTML_ELEMENT");
        let hole = html
            .descendants()
            .find(|n| n.kind() == SyntaxKind::HTML_HOLE);
        assert!(
            hole.is_some(),
            "expected an HTML_HOLE node inside the element"
        );
        // The hole must contain a VARIABLE_REF for $x (inner parsed as a full Spacetime expr).
        let var = hole
            .unwrap()
            .descendants()
            .find(|n| n.kind() == SyntaxKind::VARIABLE_REF);
        assert!(var.is_some(), "hole inner should parse $x as VARIABLE_REF");
    }

    #[test]
    fn test_html_hole_in_attr_value() {
        // href="`$url`/x" — hole inside an attribute value.
        let result = parse("<a href=\"`$url`/x\">go</a>");
        assert!(
            result.errors.is_empty(),
            "hole in attr value should parse: {:?}",
            result.errors
        );
        let html = html_element_node(&result.root).expect("HTML_ELEMENT");
        let hole = html
            .descendants()
            .find(|n| n.kind() == SyntaxKind::HTML_HOLE);
        assert!(hole.is_some(), "expected an HTML_HOLE node in attr value");
    }

    #[test]
    fn test_html_escaped_backtick_is_literal() {
        // \` is a literal backtick, NOT a hole opener.
        let result = parse("<code>a \\` b</code>");
        assert!(
            result.errors.is_empty(),
            "escaped backtick should parse as literal: {:?}",
            result.errors
        );
        let html = html_element_node(&result.root).expect("HTML_ELEMENT");
        // No hole — the backtick was escaped.
        assert!(
            html.descendants()
                .all(|n| n.kind() != SyntaxKind::HTML_HOLE),
            "escaped backtick must not produce an HTML_HOLE"
        );
    }

    #[test]
    fn test_html_does_not_break_comparison() {
        // `<` inside an expression (paren) must still be a comparison, not HTML.
        let result = parse("@test \"t\" {\n  @assert (a < b)\n}");
        assert!(
            result.errors.is_empty(),
            "comparison `<` must not be treated as HTML: {:?}",
            result.errors
        );
        assert!(
            html_element_node(&result.root).is_none(),
            "a comparison must not produce an HTML_ELEMENT"
        );
    }

    #[test]
    fn test_import_then_html_does_not_swallow_markup() {
        // BUG-070: a terminator-less `@import` immediately followed by a
        // newline-led top-level `<tag>` must NOT consume the markup as a `<`
        // comparison arg. The HTML_ELEMENT must survive so it reaches body_html
        // (otherwise: a blank page).
        let result = parse("@import \"stdlib/text\"\n<main><p>hi</p></main>\n.x { color: red; }");
        assert!(
            result.errors.is_empty(),
            "import + newline-led HTML should parse cleanly: {:?}",
            result.errors
        );
        let html = html_element_node(&result.root);
        assert!(
            html.is_some(),
            "the <main> after a bare @import must be an HTML_ELEMENT, not swallowed"
        );
        assert_eq!(html.unwrap().text().to_string(), "<main><p>hi</p></main>");
    }

    /// Count SCOPE_BLOCK nodes and collect their leading selector text.
    #[cfg(test)]
    fn scope_selectors(root: &SyntaxNode) -> Vec<String> {
        root.descendants()
            .filter(|n| n.kind() == SyntaxKind::SCOPE_BLOCK)
            .map(|n| {
                n.children()
                    .find(|c| c.kind() == SyntaxKind::SELECTOR)
                    .map(|s| s.text().to_string().trim().to_string())
                    .unwrap_or_default()
            })
            .collect()
    }

    #[test]
    fn test_import_then_element_scope_not_swallowed() {
        // FUP-078: a terminator-less `@import` immediately followed by a
        // newline-led BARE ELEMENT selector scope (`h1 { … }`, `body { … }`)
        // must NOT consume `h1` as an inline arg and `{ … }` as the import's
        // body. This is the IDENT-led sibling of the `.`/`#`/`<` cases already
        // covered by BUG-047/BUG-070; without the guard the first scope (and the
        // directive inside it) silently vanished, collapsing emit to 0 bytes.
        let result = parse("@import \"stdlib/text\"\nh1 { color: red }");
        assert!(
            result.errors.is_empty(),
            "import + newline-led element scope should parse cleanly: {:?}",
            result.errors
        );
        assert_eq!(
            scope_selectors(&result.root),
            vec!["h1".to_string()],
            "the element scope after a bare @import must survive as a SCOPE_BLOCK"
        );
    }

    #[test]
    fn test_import_then_element_scope_selector_shapes() {
        // Property-style table: every common element-led selector shape that may
        // follow a bare `@import` must round-trip to exactly one SCOPE_BLOCK with
        // the expected selector text. Covers compound, descendant/child/sibling
        // combinators, pseudo-class/element, attribute, and multi-scope runs.
        let cases: &[(&str, &[&str])] = &[
            ("h1 { color: red }", &["h1"]),
            ("h1.x { color: red }", &["h1.x"]),
            ("body { margin: 0 }", &["body"]),
            ("ul > li { color: red }", &["ul > li"]),
            ("nav a { color: red }", &["nav a"]),
            ("a + b { color: red }", &["a + b"]),
            ("a ~ b { color: red }", &["a ~ b"]),
            ("a:hover { color: red }", &["a:hover"]),
            ("li::before { content: \"x\" }", &["li::before"]),
            ("input[type=text] { color: red }", &["input[type=text]"]),
            // Two scopes: the FIRST (previously swallowed) and the second.
            ("body { margin: 0 }\nh1 { color: red }", &["body", "h1"]),
        ];
        for (tail, expected) in cases {
            let src = format!("@import \"stdlib/text\"\n{tail}");
            let result = parse(&src);
            assert!(
                result.errors.is_empty(),
                "[{tail}] should parse cleanly: {:?}",
                result.errors
            );
            let got = scope_selectors(&result.root);
            let want: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
            assert_eq!(got, want, "[{tail}] scope selectors mismatch");
        }
    }

    #[test]
    fn test_import_then_directive_bearing_scope_keeps_inner_directive() {
        // FUP-078 end-to-end shape at the CST level: the swallowed scope carried
        // a directive (`@balance`). After the fix the SCOPE_BLOCK exists AND
        // contains the inner DIRECTIVE — the precondition for it being
        // FormMatch'd and emitted (the 0-byte-runtime symptom).
        let result = parse("@import \"stdlib/text\"\nh1.x { @balance(lineHeight:1.2) }");
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        let scope = result
            .root
            .descendants()
            .find(|n| n.kind() == SyntaxKind::SCOPE_BLOCK)
            .expect("the h1.x scope must survive");
        let inner_directive = scope
            .descendants()
            .any(|n| n.kind() == SyntaxKind::DIRECTIVE);
        assert!(
            inner_directive,
            "the @balance directive inside the recovered scope must be present"
        );
    }

    #[test]
    fn test_import_guard_does_not_break_genuine_inline_args() {
        // Negatives: directives whose inline args legitimately include identifiers
        // / return types / value expressions must be UNAFFECTED by the FUP-078
        // element-scope guard. Each must yield NO spurious leading scope.
        let negatives: &[&str] = &[
            // @fn with a `: string {` return type + body — must stay ONE construct
            // (the regression that the structural-guard approach caused).
            "@fn formatPrice(price: number): string {\n  return 1;\n}\n.g { color: red }",
            // @assert with a newline-led continuation STRING message.
            "@assert (x == 1)\n  \"message\"",
            // @data fetch with a typed value + refresh body.
            "@data fetch $p P[] : \"/api\" { refresh: 5m }",
        ];
        for src in negatives {
            let result = parse(src);
            assert!(
                result.errors.is_empty(),
                "[{src}] should parse cleanly: {:?}",
                result.errors
            );
        }
        // @fn return-type must NOT spawn a stray `string` scope.
        let fn_src = "@fn formatPrice(price: number): string {\n  return 1;\n}\n.g { color: red }";
        let sels = scope_selectors(&parse(fn_src).root);
        assert_eq!(
            sels,
            vec![".g".to_string()],
            "@fn return-type `string` must not be parsed as an element scope"
        );
    }

    #[test]
    fn test_apostrophe_in_html_text_does_not_swallow_following_css() {
        // BUG-072: a `'` in HTML *text* made the lexer emit a STRING token that
        // ran past the markup, swallowing the CSS that follows. The byte-scanner
        // gets the region right; the straddling token must be re-lexed so the
        // trailing scope (`.x { … }`) survives.
        let result = parse(
            "<main><p>the compiler's OWN registry</p></main>\n.x { color: red; }\nbody { color: white; }",
        );
        // Both style scopes must parse (the bug dropped them to 0).
        let scope_count = result
            .root
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::SCOPE_BLOCK)
            .count();
        assert!(
            scope_count >= 2,
            "apostrophe in text must not swallow the following CSS scopes (got {scope_count})"
        );
    }

    #[test]
    fn test_left_arrow_token() {
        // Test that <- is lexed as LEFT_ARROW token
        let input = "$x <- 1";
        let lexer = super::super::lexer::Lexer::new(input);
        let tokens = lexer.tokenize();

        // Should have: DOLLAR, IDENT, WHITESPACE, LEFT_ARROW, WHITESPACE, NUMBER, EOF
        let left_arrow_token = tokens.iter().find(|t| t.kind == SyntaxKind::LEFT_ARROW);
        assert!(left_arrow_token.is_some(), "Should have LEFT_ARROW token");
        assert_eq!(left_arrow_token.unwrap().text, "<-");
    }

    #[test]
    fn test_strict_equality_in_paren_expr() {
        // @assert (x === 'yes') should parse without errors
        let input = "@test \"t\" {\n  @assert (x === 'yes')\n}";
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "Strict equality (===) in expression should parse without errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_strict_inequality_in_paren_expr() {
        // @assert (x !== false) should parse without errors
        let input = "@test \"t\" {\n  @assert (x !== 0)\n}";
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "Strict inequality (!==) in expression should parse without errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_strict_equality_with_keywords() {
        // @assert (val === true) and @assert (val === false) should parse
        let input = "@test \"t\" {\n  @assert (val === true)\n  @assert (val2 !== false)\n}";
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "Strict equality with true/false should parse without errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_strict_equality_with_string() {
        // @assert (window.__conditionMet === 'yes') should parse
        let input = "@test \"t\" {\n  @assert (window.__conditionMet === 'yes')\n}";
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "Strict equality with string RHS should parse without errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_wait_until_with_strict_equality_then_let() {
        // @wait_until with === using simple variable (no dot access) followed by @let
        let input = "@test \"t\" {\n  @let (start = Date.now())\n  @wait_until __cond === 'yes' timeout: 1000ms\n  @let (elapsed = Date.now() - start)\n  @assert (elapsed < 100)\n}";
        let result = parse(input);
        assert!(
            result.errors.is_empty(),
            "wait_until with === followed by @let should parse without errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_data_directive_inline_type_annotation_array() {
        // @data projects: Project[] { ... } should produce 3 inline ARGs: "projects", ":", "Project[]"
        // NOT 5 ARGs: "projects", ":", "Project", "[", "]"
        use crate::syntax::cst::ast::{AstNode, Directive};

        let root = parse_ok("@data projects: Project[] { src: \"/api\"; }");
        let directive = root
            .descendants()
            .find_map(Directive::cast)
            .expect("should find directive");

        let inline_args: Vec<String> = directive
            .inline_args()
            .filter_map(|a| a.value_text())
            .collect();

        assert_eq!(inline_args, vec!["projects", ":", "Project[]"]);
    }

    #[test]
    fn test_eval_arrow_fn_with_structured_js_body() {
        // BUG-095: an @eval (() => { … }) body containing JS array + object literals
        // must parse as ONE opaque arrow-fn arg, NOT have its `[`/`{` re-read as CSS
        // selectors/rule-blocks. The arrow body is balanced JS; the parser must
        // consume it whole (matching parens) and not stop at the first inner `}`.
        parse_ok(
            "@test \"t\" {\n  @mount { <main></main> }\n  @eval (() => { const a = []; const o = { k: 1 }; window.x = o.k; })\n  @then { @assert (window.x === 1) \"ok\"; }\n}",
        );
        // Array-of-object literal (the original repro).
        parse_ok(
            "@test \"t\" {\n  @eval (() => { const d = { c:[ { t:1 } ] }; window.x = d.c[0].t; })\n  @then { @assert (window.x === 1) \"ok\"; }\n}",
        );
        // Nested object with a string value containing a brace-like char.
        parse_ok(
            "@test \"t\" {\n  @eval (() => { const o = { type:'doc', content:[ { type:'p' } ] }; window.t = o.type; })\n  @then { @assert (window.t === 'doc') \"ok\"; }\n}",
        );
        // A brace inside a QUOTED string in the body is opaque (lexer emits one STRING
        // token) — must not break balance.
        parse_ok(
            "@test \"t\" {\n  @eval (() => { const s = 'a}b{c'; window.s = s; })\n  @then { @assert (window.s === 'a}b{c') \"ok\"; }\n}",
        );
        // Expression-body arrow (no block) still parses.
        parse_ok(
            "@test \"t\" {\n  @eval (() => window.y = 1)\n  @then { @assert (window.y === 1) \"ok\"; }\n}",
        );
    }

    #[test]
    fn test_directive_value_atom_is_generic() {
        // @match keeps its body as the outer derive value, not a sibling directive.
        parse_ok("@data derive $x State : @match { _ => Ready; };");
        // A non-match directive value is equally opaque and leaves its sibling intact.
        parse_ok("@data inline $x : @custom;\n@on click {}");
    }

    #[test]
    fn test_pseudo_led_scope_block() {
        // M1 / PLAN-046: a scope-block selector may START with a pseudo-class.
        // `:where(...)` / `:is(...)` carry a NESTED selector list (with descendant
        // combinators + commas) inside balanced parens; `:hover {}` is the simple
        // case. The top-level dispatch previously only opened a scope block on
        // `.`/`#`/`[`/`*`/ident, so a leading `:` errored. All must parse clean.
        parse_ok(":where(.stage h1, .stage p) { color: red; }");
        parse_ok(":is(.a, .b) .c { color: blue; }");
        parse_ok(":where(.s *) { font-family: sans-serif; }");
        parse_ok(":hover { opacity: 1; }");
        // Regression: a normal class scope block still parses.
        parse_ok(".card { color: green; }");
    }

    #[test]
    fn test_data_directive_paren_led_value_expression() {
        // BUG-077: a @data derive value expression that STARTS with `(` must parse
        // as a balanced inline expression — not have its `(` mistaken for a
        // directive arg-list (which left the trailing `.member` to be mis-read as a
        // `.class` selector → "expected '{' after selector"). All of these must parse
        // with ZERO errors.
        parse_ok("@data derive $x : ($a).length;");
        parse_ok("@data derive $z : ($a || []).length;");
        // The whole-ternary-in-parens form (the supported way to write a ternary
        // whose then-branch is a member access) must also parse.
        parse_ok("@data derive $y : ($a ? $a.length : 0);");
        // Regression guard: a normal (non-paren-led) value still parses.
        parse_ok("@data derive $n : $items.length;");
        parse_ok("@data fetch $api : \"/url\";");
    }

    #[test]
    fn test_data_directive_inline_type_annotation_optional() {
        // @data item: string? { } should produce 3 inline ARGs: "item", ":", "string?"
        use crate::syntax::cst::ast::{AstNode, Directive};

        let root = parse_ok("@data item: string? { }");
        let directive = root
            .descendants()
            .find_map(Directive::cast)
            .expect("should find directive");

        let inline_args: Vec<String> = directive
            .inline_args()
            .filter_map(|a| a.value_text())
            .collect();

        assert_eq!(inline_args, vec!["item", ":", "string?"]);
    }

    #[test]
    fn test_each_with_as_in_arg_list() {
        // @each($items as $item) should parse without errors and
        // produce a single ARG containing "$items as $item"
        use crate::syntax::cst::ast::{Arg, AstNode, Directive};

        let root = parse_ok(".list { @each($items as $item) { } }");
        let each_dir = root
            .descendants()
            .find_map(|n| {
                let dir = Directive::cast(n)?;
                if dir.name_text()? == "each" {
                    Some(dir)
                } else {
                    None
                }
            })
            .expect("should find @each directive");

        let arg_list = each_dir.arg_list().expect("should have arg list");
        // Should have exactly 1 positional arg containing the "as" clause
        let positional: Vec<Arg> = arg_list.positional_args().collect();
        assert_eq!(
            positional.len(),
            1,
            "should have 1 positional arg, got {:?}",
            positional
                .iter()
                .map(|a| a.value_text())
                .collect::<Vec<_>>()
        );

        // The single ARG should contain both the source and the alias
        let arg_text = positional[0].value_text().unwrap();
        assert!(
            arg_text.contains("items"),
            "arg should contain source: {}",
            arg_text
        );
        assert!(
            arg_text.contains("item"),
            "arg should contain alias: {}",
            arg_text
        );
    }
}
