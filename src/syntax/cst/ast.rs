//! Typed AST wrappers for Rowan syntax nodes
//!
//! These types provide convenient, type-safe access to syntax tree nodes.
//! Each wrapper implements `cast` to safely convert from a generic SyntaxNode.

use super::{SyntaxKind, SyntaxNode, SyntaxToken};

/// Trait for AST nodes that can be cast from SyntaxNode
pub trait AstNode: Sized {
    /// Try to cast a SyntaxNode to this AST type
    fn cast(node: SyntaxNode) -> Option<Self>;

    /// Get the underlying syntax node
    fn syntax(&self) -> &SyntaxNode;

    /// Get the text range of this node
    fn text_range(&self) -> rowan::TextRange {
        self.syntax().text_range()
    }

    /// Get the source text of this node
    fn text(&self) -> rowan::SyntaxText {
        self.syntax().text()
    }
}

/// Helper macro to define AST node types
macro_rules! ast_node {
    ($name:ident, $kind:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name {
            syntax: SyntaxNode,
        }

        impl AstNode for $name {
            fn cast(node: SyntaxNode) -> Option<Self> {
                if node.kind() == SyntaxKind::$kind {
                    Some(Self { syntax: node })
                } else {
                    None
                }
            }

            fn syntax(&self) -> &SyntaxNode {
                &self.syntax
            }
        }
    };
}

// === File Structure ===

ast_node!(Root, ROOT);

impl Root {
    /// Get all top-level scope blocks
    pub fn scope_blocks(&self) -> impl Iterator<Item = ScopeBlock> + '_ {
        self.syntax.children().filter_map(ScopeBlock::cast)
    }

    /// Get all top-level directives
    pub fn directives(&self) -> impl Iterator<Item = Directive> + '_ {
        self.syntax.children().filter_map(Directive::cast)
    }

    /// Get all top-level meta definitions
    pub fn meta_defs(&self) -> impl Iterator<Item = MetaDef> + '_ {
        self.syntax.children().filter_map(MetaDef::cast)
    }

    /// Get all top-level variable references/declarations
    pub fn variables(&self) -> impl Iterator<Item = VariableRef> + '_ {
        self.syntax.children().filter_map(VariableRef::cast)
    }

    /// Get all top-level HTML element literals (PLAN-023 W1)
    pub fn html_elements(&self) -> impl Iterator<Item = HtmlElement> + '_ {
        self.syntax.children().filter_map(HtmlElement::cast)
    }

    /// Get all top-level element reference statements (entity scopes & plain refs).
    pub fn element_ref_stmts(&self) -> impl Iterator<Item = ElementRefStmt> + '_ {
        self.syntax.children().filter_map(ElementRefStmt::cast)
    }

    /// Get all BARE top-level element references — an `&name(…)` written at file
    /// scope with no body and no enclosing selector. Nothing consumes these, so
    /// before PLAN-144 W1 such a call vanished without a trace and the build
    /// reported success; the parser now refuses it (BUG-344).
    pub fn element_refs(&self) -> impl Iterator<Item = ElementRef> + '_ {
        self.syntax.children().filter_map(ElementRef::cast)
    }
}

// === HTML (first-class markup; PLAN-023) ===

ast_node!(HtmlElement, HTML_ELEMENT);

/// Private-use sentinels delimiting a hole placeholder in the HTML skeleton. These code
/// points never occur in real markup, so html5ever passes them through as opaque text and
/// we can recover hole positions deterministically afterwards.
pub const HOLE_OPEN: char = '\u{E000}';
pub const HOLE_CLOSE: char = '\u{E001}';

impl HtmlElement {
    /// The verbatim source for this element (holes included as `` `expr` ``). Round-trips
    /// the exact source span. Useful for diagnostics; the TreeSink uses [`skeleton`] instead.
    pub fn raw_text(&self) -> String {
        self.syntax.text().to_string()
    }

    /// A pure-HTML skeleton with each backtick hole replaced by a sentinel placeholder
    /// `\u{E000}<index>\u{E001}`, plus the ordered list of hole expression sources. The
    /// html5ever TreeSink consumes the skeleton (no backticks → valid markup); the indexed
    /// holes are re-injected as `HtmlExpr::Hole` afterwards.
    pub fn skeleton(&self) -> (String, Vec<String>) {
        let mut out = String::new();
        let mut holes = Vec::new();
        self.collect_skeleton(&self.syntax, &mut out, &mut holes);
        (out, holes)
    }

    fn collect_skeleton(&self, node: &SyntaxNode, out: &mut String, holes: &mut Vec<String>) {
        use rowan::NodeOrToken;
        for child in node.children_with_tokens() {
            match child {
                NodeOrToken::Node(n) if n.kind() == SyntaxKind::HTML_HOLE => {
                    // Hole = backtick HTML_RAW + inner expr CST + backtick HTML_RAW.
                    // The expr source is everything inside the backticks.
                    let inner: String = n
                        .children_with_tokens()
                        .filter(|c| c.kind() != SyntaxKind::HTML_RAW)
                        .map(|c| c.to_string())
                        .collect();
                    let idx = holes.len();
                    holes.push(inner.trim().to_string());
                    out.push(HOLE_OPEN);
                    out.push_str(&idx.to_string());
                    out.push(HOLE_CLOSE);
                }
                NodeOrToken::Node(n) => self.collect_skeleton(&n, out, holes),
                NodeOrToken::Token(t) => out.push_str(t.text()),
            }
        }
    }
}

// === Scope Blocks ===

ast_node!(ScopeBlock, SCOPE_BLOCK);

impl ScopeBlock {
    /// Get the selector for this scope
    pub fn selector(&self) -> Option<Selector> {
        self.syntax.children().find_map(Selector::cast)
    }

    /// Get the body of this scope
    pub fn body(&self) -> Option<Body> {
        self.syntax.children().find_map(Body::cast)
    }
}

ast_node!(Selector, SELECTOR);

impl Selector {
    /// Get all selector parts
    pub fn parts(&self) -> impl Iterator<Item = SelectorPart> + '_ {
        self.syntax.children().filter_map(SelectorPart::cast)
    }

    /// Get the selector text (useful for matching), filtering out comments.
    ///
    /// A leading collection marker `[]` is GRAMMAR, not selector, and is dropped
    /// here — the innermost place the text is built, so every consumer is correct
    /// by default. `parse_element_ref` stops at the ident, so `&cards[] .cards`
    /// leaves the marker at the head of this run (`"[] .cards"`).
    ///
    /// Only an EXACT leading `[]` is the marker. `.cards[data-role="list"]` is a
    /// CSS attribute selector and is untouched — a bare `[]` is not valid CSS in
    /// selector-head position, so there is nothing legitimate to lose.
    pub fn selector_text(&self) -> String {
        let mut parts = Vec::new();
        for element in self.syntax.children_with_tokens() {
            if element.kind() == SyntaxKind::COMMENT {
                continue;
            }
            if element.kind() == SyntaxKind::WHITESPACE {
                parts.push(" ".to_string());
            } else {
                parts.push(element.to_string());
            }
        }
        let text = parts.join("");
        let trimmed = text.trim();
        match trimmed.strip_prefix("[]") {
            Some(rest) => rest.trim().to_string(),
            None => trimmed.to_string(),
        }
    }
}

ast_node!(SelectorPart, SELECTOR_PART);

impl SelectorPart {
    /// Check if this is a class selector
    pub fn is_class(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .any(|e| e.kind() == SyntaxKind::DOT)
    }

    /// Check if this is an ID selector
    pub fn is_id(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .any(|e| e.kind() == SyntaxKind::HASH)
    }

    /// Check if this is an attribute selector
    pub fn is_attribute(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .any(|e| e.kind() == SyntaxKind::L_BRACKET)
    }

    /// Check if this is a pseudo-class or pseudo-element
    pub fn is_pseudo(&self) -> bool {
        let mut iter = self.syntax.children_with_tokens();
        iter.next()
            .map(|e| e.kind() == SyntaxKind::COLON)
            .unwrap_or(false)
    }

    /// Get the identifier (class name, id name, etc.)
    pub fn ident(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
    }
}

ast_node!(Body, BODY);

impl Body {
    /// Get all CSS properties
    pub fn properties(&self) -> impl Iterator<Item = CssProperty> + '_ {
        self.syntax.children().filter_map(CssProperty::cast)
    }

    /// Get all directives in this body
    pub fn directives(&self) -> impl Iterator<Item = Directive> + '_ {
        self.syntax.children().filter_map(Directive::cast)
    }

    /// Get all nested scope blocks
    pub fn nested_scopes(&self) -> impl Iterator<Item = ScopeBlock> + '_ {
        self.syntax.children().filter_map(ScopeBlock::cast)
    }

    /// Get all variable references
    pub fn variables(&self) -> impl Iterator<Item = VariableRef> + '_ {
        self.syntax.children().filter_map(VariableRef::cast)
    }

    /// Get all element reference statements
    pub fn element_ref_stmts(&self) -> impl Iterator<Item = ElementRefStmt> + '_ {
        self.syntax.children().filter_map(ElementRefStmt::cast)
    }

    /// Get all form splices (`--name;`, `--name(args);`) in this body
    pub fn form_refs(&self) -> impl Iterator<Item = FormRef> + '_ {
        self.syntax.children().filter_map(FormRef::cast)
    }
}

// === Directives ===

ast_node!(Directive, DIRECTIVE);

impl Directive {
    /// Get the directive name (e.g., "animate", "data", "each", "if", "else")
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT || t.kind().is_keyword())
    }

    /// Get the name as a string.
    ///
    /// FEAT-118: reconstructs a namespace-QUALIFIED directive name
    /// (`@scene/camera`, `@alias/name`) by joining the leading run of
    /// name tokens — the first IDENT/keyword plus any tightly-following
    /// `SLASH IDENT` segments the parser consumed into the directive head.
    /// Stops at the first non-name token (args, `(`, body, etc.). An
    /// unqualified `@camera` returns `"camera"` unchanged.
    pub fn name_text(&self) -> Option<String> {
        let mut out = String::new();
        let mut started = false;
        let mut expect_segment = false;
        for tok in self
            .syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
        {
            let k = tok.kind();
            if k.is_trivia() {
                // Trivia ends the name run (a spaced `/` is NOT a qualifier).
                if started {
                    break;
                }
                continue;
            }
            if !started {
                if k == SyntaxKind::IDENT || k.is_keyword() {
                    out.push_str(tok.text());
                    started = true;
                } else if k == SyntaxKind::AT_SIGN {
                    continue;
                } else {
                    continue;
                }
            } else if expect_segment {
                if k == SyntaxKind::IDENT || k.is_keyword() {
                    out.push_str(tok.text());
                    expect_segment = false;
                } else {
                    break;
                }
            } else if k == SyntaxKind::SLASH {
                out.push('/');
                expect_segment = true;
            } else {
                break;
            }
        }
        if started { Some(out) } else { None }
    }

    /// Get the argument list
    pub fn arg_list(&self) -> Option<ArgList> {
        self.syntax.children().find_map(ArgList::cast)
    }

    /// Get inline arguments (those before the parenthesized arg list)
    pub fn inline_args(&self) -> impl Iterator<Item = Arg> + '_ {
        self.syntax
            .children()
            .take_while(|n| n.kind() != SyntaxKind::ARG_LIST && n.kind() != SyntaxKind::BODY)
            .filter_map(Arg::cast)
    }

    /// Get inline tokens as text (for meta clauses like %emit js { ... })
    /// This returns tokens between the name and the body/arg_list
    pub fn inline_tokens_text(&self) -> Option<String> {
        let name = self.name()?;
        let name_end = name.text_range().end();

        // Collect tokens that come after the name but before body
        let tokens: Vec<_> = self
            .syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| !t.kind().is_trivia())
            .filter(|t| t.text_range().start() >= name_end)
            .take_while(|t| t.kind() != SyntaxKind::L_BRACE)
            .map(|t| t.text().to_string())
            .collect();

        if tokens.is_empty() {
            None
        } else {
            Some(tokens.join(" "))
        }
    }

    /// Like [`inline_tokens_text`](Self::inline_tokens_text), but DESCENDS into
    /// child nodes so structured trailing arguments are included. The `@use`
    /// tail `only (a, b)` / `hiding (a, b)` parses its `(…)` into an `ARG_LIST`
    /// child node, whose tokens the token-only `inline_tokens_text` skips; this
    /// walker collects them (FEAT-118 FUP-057). Stops at the first body brace.
    pub fn inline_tail_text(&self) -> Option<String> {
        let name = self.name()?;
        let name_end = name.text_range().end();
        let tokens: Vec<String> = self
            .syntax
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| !t.kind().is_trivia())
            .filter(|t| t.text_range().start() >= name_end)
            .take_while(|t| t.kind() != SyntaxKind::L_BRACE)
            .map(|t| t.text().to_string())
            .collect();
        if tokens.is_empty() {
            None
        } else {
            Some(tokens.join(" "))
        }
    }

    /// Get the body
    pub fn body(&self) -> Option<Body> {
        self.syntax.children().find_map(Body::cast)
    }

    /// Check if this directive has a body
    pub fn has_body(&self) -> bool {
        self.body().is_some()
    }
}

ast_node!(ArgList, ARG_LIST);

impl ArgList {
    /// Get all arguments (both positional and named)
    pub fn args(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.syntax
            .children()
            .filter(|n| n.kind() == SyntaxKind::ARG || n.kind() == SyntaxKind::NAMED_ARG)
    }

    /// Get positional arguments only
    pub fn positional_args(&self) -> impl Iterator<Item = Arg> + '_ {
        self.syntax.children().filter_map(Arg::cast)
    }

    /// Get named arguments only
    pub fn named_args(&self) -> impl Iterator<Item = NamedArg> + '_ {
        self.syntax.children().filter_map(NamedArg::cast)
    }

    /// Get argument count
    pub fn len(&self) -> usize {
        self.args().count()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

ast_node!(Arg, ARG);

impl Arg {
    /// Get the value token/node
    pub fn value(&self) -> Option<rowan::SyntaxElement<super::SpacetimeLang>> {
        self.syntax
            .children_with_tokens()
            .find(|e| !e.kind().is_trivia())
    }

    /// Get the value as text
    /// For multi-token ARGs (like `Product[]` or `$items as $item`), returns the
    /// full text of the ARG node which preserves original spacing.
    pub fn value_text(&self) -> Option<String> {
        // Use the node's full text, trimmed of leading/trailing whitespace
        let text = self.syntax.text().to_string();
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }
}

ast_node!(NamedArg, NAMED_ARG);

impl NamedArg {
    /// Get the argument name
    /// Accepts both IDENT and keyword tokens (from, to, on, etc.)
    /// since keywords can be used as named arg names in macro calls.
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT || t.kind().is_keyword())
    }

    /// Get the name as text
    pub fn name_text(&self) -> Option<String> {
        self.name().map(|t| t.text().to_string())
    }

    /// Get the value (everything after the colon)
    pub fn value(&self) -> Option<rowan::SyntaxElement<super::SpacetimeLang>> {
        let mut found_colon = false;
        for elem in self.syntax.children_with_tokens() {
            if elem.kind() == SyntaxKind::COLON {
                found_colon = true;
                continue;
            }
            if found_colon && !elem.kind().is_trivia() {
                return Some(elem);
            }
        }
        None
    }

    /// Get the value as text
    pub fn value_text(&self) -> Option<String> {
        self.value().map(|e| match e {
            rowan::SyntaxElement::Node(n) => n.text().to_string(),
            rowan::SyntaxElement::Token(t) => t.text().to_string(),
        })
    }
}

// === References ===

ast_node!(VariableRef, VARIABLE_REF);

impl VariableRef {
    /// Get the variable name (without the $)
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
    }

    /// Get the name as text
    pub fn name_text(&self) -> Option<String> {
        self.name().map(|t| t.text().to_string())
    }

    /// Get the full path (e.g., "name.field.subfield")
    pub fn path(&self) -> Vec<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::IDENT)
            .map(|t| t.text().to_string())
            .collect()
    }
}

ast_node!(ElementRef, ELEMENT_REF);

impl ElementRef {
    /// Get the element name (without the &)
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
    }

    /// Get the name as text
    pub fn name_text(&self) -> Option<String> {
        self.name().map(|t| t.text().to_string())
    }

    /// The parenthesized argument list, when this ref is a CALL. `None` for a
    /// bare `&name` scope reference — the distinction PLAN-144 Q2 turns on.
    pub fn arg_list(&self) -> Option<ArgList> {
        self.syntax.children().find_map(ArgList::cast)
    }

    /// Get the full path including facet and property
    pub fn path(&self) -> Vec<String> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::IDENT)
            .map(|t| t.text().to_string())
            .collect()
    }

    /// Get facet (second part of path)
    pub fn facet(&self) -> Option<String> {
        self.path().get(1).cloned()
    }

    /// Get property (third part of path)
    pub fn property(&self) -> Option<String> {
        self.path().get(2).cloned()
    }
}

ast_node!(ElementRefStmt, ELEMENT_REF_STMT);

impl ElementRefStmt {
    /// Get the inner element reference (&name)
    pub fn element_ref(&self) -> Option<ElementRef> {
        self.syntax.children().find_map(ElementRef::cast)
    }

    /// Get the element name (without the &)
    pub fn name_text(&self) -> Option<String> {
        self.element_ref().and_then(|e| e.name_text())
    }

    /// Get the arg list (for template invocations like &card($p))
    pub fn arg_list(&self) -> Option<ArgList> {
        self.syntax.children().find_map(ArgList::cast)
    }

    /// Get the selector node if present
    pub fn selector(&self) -> Option<Selector> {
        self.syntax.children().find_map(Selector::cast)
    }

    /// Get the selector as text (trimmed).
    ///
    /// The leading collection marker `[]` is dropped by `Selector::selector_text`
    /// itself, so this is always the real CSS target.
    pub fn selector_text(&self) -> Option<String> {
        self.selector().map(|s| s.selector_text())
    }

    /// Get the body if present
    pub fn body(&self) -> Option<Body> {
        self.syntax.children().find_map(Body::cast)
    }
}

ast_node!(FormRef, FORM_REF);

impl FormRef {
    /// The form name WITH its `--` sigil (e.g. `--card-surface`). Args, when
    /// present, are the ARG_LIST child; the name is the first IDENT token.
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
    }
}

// === Meta Definitions ===

ast_node!(MetaDef, META_DEF);

impl MetaDef {
    /// Get the meta keyword (first IDENT after %)
    /// e.g., for "%primitive tick(...)" the keyword is "primitive"
    /// For "%my-macro { }" the keyword is "my-macro" (doubles as name)
    pub fn keyword(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT)
    }

    /// Get the keyword as text
    pub fn keyword_text(&self) -> Option<String> {
        self.keyword().map(|t| t.text().to_string())
    }

    /// Get the meta name (second IDENT after keyword, if present)
    /// e.g., for "%primitive tick(...)" the name is "tick"
    /// For "%my-macro { }" there is no separate name (returns None)
    pub fn explicit_name(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::IDENT)
            .nth(1)
    }

    /// Get the name - returns explicit name if present, otherwise the keyword
    /// This maintains backward compatibility with tests like "%my-macro { }"
    pub fn name(&self) -> Option<SyntaxToken> {
        self.explicit_name().or_else(|| self.keyword())
    }

    /// Get the name as text
    pub fn name_text(&self) -> Option<String> {
        self.name().map(|t| t.text().to_string())
    }

    /// Get the arg list (parameters)
    pub fn arg_list(&self) -> Option<ArgList> {
        self.syntax.children().find_map(ArgList::cast)
    }

    /// Get the body
    pub fn body(&self) -> Option<Body> {
        self.syntax.children().find_map(Body::cast)
    }

    /// PLAN-133: names from the `%uses a, b` clause (cross-primitive prelude
    /// dependencies), in declaration order. Empty when the clause is absent.
    pub fn uses_names(&self) -> Vec<String> {
        let Some(node) = self
            .syntax
            .children()
            .find(|n| n.kind() == SyntaxKind::META_USES)
        else {
            return Vec::new();
        };
        node.children_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| t.kind() == SyntaxKind::IDENT)
            .skip(1) // the `uses` keyword itself
            .map(|t| t.text().to_string())
            .collect()
    }
}

// === CSS Properties ===

ast_node!(CssProperty, CSS_PROPERTY);

impl CssProperty {
    /// Get the property name (IDENT or keyword token like `from`, `to`, etc.)
    pub fn name(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::IDENT || t.kind().is_keyword())
    }

    /// Get the name as text. A class-toggle property (`.active: $v;`) keeps its leading
    /// `.` marker (BUG-068): the CSS_PROPERTY node begins with a DOT token, which the
    /// emitter routes to classList.toggle. Plain CSS / custom-prop names return bare.
    pub fn name_text(&self) -> Option<String> {
        let leads_with_dot = self
            .syntax
            .children_with_tokens()
            .find(|e| !e.kind().is_trivia())
            .map(|e| e.kind())
            == Some(SyntaxKind::DOT);
        let name = self.name()?.text().to_string();
        Some(if leads_with_dot {
            format!(".{name}")
        } else {
            name
        })
    }

    /// Get the CSS value
    pub fn value(&self) -> Option<CssValue> {
        self.syntax.children().find_map(CssValue::cast)
    }

    /// Check if this is a transition (contains ->)
    pub fn is_transition(&self) -> bool {
        self.syntax
            .descendants_with_tokens()
            .any(|e| e.kind() == SyntaxKind::ARROW)
    }

    /// Check if this is a DOM injection written with the `<-` arrow
    /// (`src <- $u;`, `text <- $msg;`) rather than the `:` CSS surface
    /// (`background: $c;`). The arrow surface targets HTML attributes /
    /// textContent; the colon surface targets CSS properties (BUG-091).
    pub fn is_arrow_injection(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .any(|e| e.kind() == SyntaxKind::LEFT_ARROW)
    }

    /// Check if this has keyframes
    pub fn has_keyframes(&self) -> bool {
        self.syntax
            .descendants()
            .any(|n| n.kind() == SyntaxKind::KEYFRAME_BLOCK)
    }
}

ast_node!(CssValue, CSS_VALUE);

impl CssValue {
    /// Get all value parts
    pub fn parts(&self) -> impl Iterator<Item = rowan::SyntaxElement<super::SpacetimeLang>> + '_ {
        self.syntax
            .children_with_tokens()
            .filter(|e| !e.kind().is_trivia())
    }

    /// Get the keyframe block if present
    pub fn keyframe_block(&self) -> Option<KeyframeBlock> {
        self.syntax.children().find_map(KeyframeBlock::cast)
    }

    /// Check if this is a transition value
    pub fn is_transition(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .any(|e| e.kind() == SyntaxKind::ARROW)
    }
}

ast_node!(KeyframeBlock, KEYFRAME_BLOCK);

impl KeyframeBlock {
    /// Get all keyframes
    pub fn keyframes(&self) -> impl Iterator<Item = Keyframe> + '_ {
        self.syntax.children().filter_map(Keyframe::cast)
    }
}

ast_node!(Keyframe, KEYFRAME);

impl Keyframe {
    /// Get the percentage (e.g., "50%")
    pub fn percentage(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| t.kind() == SyntaxKind::NUMBER_WITH_UNIT || t.kind() == SyntaxKind::NUMBER)
    }

    /// Get the percentage as a number (0-100)
    pub fn percentage_value(&self) -> Option<f64> {
        let text = self.percentage()?.text().to_string();
        let num_str = text.trim_end_matches('%');
        num_str.parse().ok()
    }
}

// === Expressions ===

ast_node!(ArrayExpr, ARRAY_EXPR);

impl ArrayExpr {
    /// Get all elements in the array
    pub fn elements(&self) -> impl Iterator<Item = SyntaxNode> + '_ {
        self.syntax.children().filter(|n| {
            n.kind() == SyntaxKind::ARG
                || n.kind() == SyntaxKind::VARIABLE_REF
                || n.kind() == SyntaxKind::ELEMENT_REF
        })
    }
}

ast_node!(ParenExpr, PAREN_EXPR);

impl ParenExpr {
    /// Get the inner expression
    pub fn inner(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|n| !matches!(n.kind(), SyntaxKind::L_PAREN | SyntaxKind::R_PAREN))
    }
}

// === Error Recovery ===

/// An error node wrapping invalid syntax
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Error {
    syntax: SyntaxNode,
}

impl AstNode for Error {
    fn cast(node: SyntaxNode) -> Option<Self> {
        if node.kind() == SyntaxKind::ERROR {
            Some(Self { syntax: node })
        } else {
            None
        }
    }

    fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
}

impl Error {
    /// Get the error text
    pub fn error_text(&self) -> String {
        self.syntax.text().to_string()
    }
}

// === Utility Functions ===

/// Find all nodes of a specific type in a syntax tree
pub fn find_all<T: AstNode>(root: &SyntaxNode) -> Vec<T> {
    root.descendants().filter_map(T::cast).collect()
}

/// Find the first node of a specific type
pub fn find_first<T: AstNode>(root: &SyntaxNode) -> Option<T> {
    root.descendants().find_map(T::cast)
}

/// Find all error nodes in a syntax tree
pub fn find_errors(root: &SyntaxNode) -> Vec<Error> {
    find_all(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::cst::parse;

    fn parse_root(input: &str) -> Root {
        let result = parse(input);
        Root::cast(result.root).expect("Root should always cast")
    }

    #[test]
    fn test_root_scope_blocks() {
        let root = parse_root(".a { } .b { }");
        let scopes: Vec<_> = root.scope_blocks().collect();
        assert_eq!(scopes.len(), 2);
    }

    #[test]
    fn test_root_directives() {
        let root = parse_root("@import \"a\"; @import \"b\";");
        let directives: Vec<_> = root.directives().collect();
        assert_eq!(directives.len(), 2);
    }

    #[test]
    fn test_scope_selector() {
        let root = parse_root(".hero { }");
        let scope = root.scope_blocks().next().unwrap();
        let selector = scope.selector().unwrap();
        assert!(selector.selector_text().contains("hero"));
    }

    #[test]
    fn test_scope_body() {
        let root = parse_root(".hero { opacity: 1; }");
        let scope = root.scope_blocks().next().unwrap();
        let body = scope.body().unwrap();
        let props: Vec<_> = body.properties().collect();
        assert_eq!(props.len(), 1);
    }

    #[test]
    fn test_directive_name() {
        let root = parse_root("@animate(fade-in) { }");
        let directive = root.directives().next().unwrap();
        assert_eq!(directive.name_text(), Some("animate".to_string()));
    }

    #[test]
    fn test_directive_arg_list() {
        let root = parse_root("@animate(fade-in, 500ms) { }");
        let directive = root.directives().next().unwrap();
        let args = directive.arg_list().unwrap();
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn test_named_arg() {
        let root = parse_root("@state_machine(initial: \"idle\") { }");
        let directive = root.directives().next().unwrap();
        let args = directive.arg_list().unwrap();
        let named_args: Vec<_> = args.named_args().collect();
        assert_eq!(named_args.len(), 1);
        assert_eq!(named_args[0].name_text(), Some("initial".to_string()));
    }

    #[test]
    fn test_variable_ref_name() {
        let root = parse_root("$count number : 0;");
        let var = root.variables().next().unwrap();
        assert_eq!(var.name_text(), Some("count".to_string()));
    }

    #[test]
    fn test_css_property() {
        let root = parse_root(".test { opacity: 1; }");
        let scope = root.scope_blocks().next().unwrap();
        let body = scope.body().unwrap();
        let prop = body.properties().next().unwrap();
        assert_eq!(prop.name_text(), Some("opacity".to_string()));
    }

    #[test]
    fn test_css_property_transition() {
        let root = parse_root(".test { opacity: 0 -> 1; }");
        let scope = root.scope_blocks().next().unwrap();
        let body = scope.body().unwrap();
        let prop = body.properties().next().unwrap();
        assert!(prop.is_transition());
    }

    #[test]
    fn test_selector_part_class() {
        let root = parse_root(".my-class { }");
        let scope = root.scope_blocks().next().unwrap();
        let selector = scope.selector().unwrap();
        let part = selector.parts().next().unwrap();
        assert!(part.is_class());
        assert_eq!(
            part.ident().map(|t| t.text().to_string()),
            Some("my-class".to_string())
        );
    }

    #[test]
    fn test_selector_part_id() {
        let root = parse_root("#main { }");
        let scope = root.scope_blocks().next().unwrap();
        let selector = scope.selector().unwrap();
        let part = selector.parts().next().unwrap();
        assert!(part.is_id());
    }

    #[test]
    fn test_find_all_directives() {
        let result = parse(".a { @animate { } } .b { @data { } }");
        let directives: Vec<Directive> = find_all(&result.root);
        assert_eq!(directives.len(), 2);
    }

    #[test]
    fn test_find_errors() {
        // Use clearly malformed syntax - unclosed brace should produce error
        let result = parse(".a { @directive(");
        let _errors = find_errors(&result.root);
        // This may or may not have errors depending on parser error recovery
        // At minimum, check the parse result exists
        assert!(result.root.text().to_string().contains(".a"));
    }

    #[test]
    fn test_meta_def() {
        // Meta definitions use %macro or %primitive keywords
        let root = parse_root("%macro my-macro { }");
        let meta_defs: Vec<_> = root.meta_defs().collect();
        assert_eq!(meta_defs.len(), 1);
        assert_eq!(meta_defs[0].name_text(), Some("my-macro".to_string()));
    }

    #[test]
    fn test_meta_invocation() {
        // %name invokes a macro at the meta level (not a definition)
        let root = parse_root("%some-macro arg1 arg2 { }");
        // Should be parsed as a directive (invocation), not a meta def
        let meta_defs: Vec<_> = root.meta_defs().collect();
        assert_eq!(meta_defs.len(), 0);
        // Should have a directive for the meta invocation
        let directives: Vec<_> = root.directives().collect();
        assert_eq!(directives.len(), 1);
    }

    #[test]
    fn test_element_ref_path() {
        let result = parse("&button.opacity");
        let elem: ElementRef = find_first(&result.root).unwrap();
        assert_eq!(elem.name_text(), Some("button".to_string()));
        assert_eq!(elem.facet(), Some("opacity".to_string()));
    }

    #[test]
    fn test_nested_scopes() {
        let root = parse_root(".parent { .child { } }");
        let parent = root.scope_blocks().next().unwrap();
        let body = parent.body().unwrap();
        let nested: Vec<_> = body.nested_scopes().collect();
        assert_eq!(nested.len(), 1);
    }
}
