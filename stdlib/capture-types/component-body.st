// Component-body construct grammar (Phase B / FEAT-079 — component_body self-describing).
//
// A `@template` body is a sequence of HTML runs interleaved with CONSTRUCT segments. The
// generic `segment_component_body` scanner (Rust infra, construct-agnostic) splits the body
// at HTML-tag-depth + termination boundaries; each non-HTML segment is then classified by the
// `component_item` Choice below (first matching arm wins) and its coarse fields extracted by
// the engine. The Rust `reify_component_body` adapter maps each branch record into the typed
// `ComponentBodyDef` IR (scalar value-semantics only — typed initials, action-kind, arg split).
//
// This REPLACES the hand-written 11-branch Rust classifier + per-construct token parsers: the
// grammar of every template-body construct now lives HERE, in stdlib (self-describing
// metasystem discipline). Adding a construct = a leaf %capture_type + a `component_item` arm.
//
// Authoring rules (B-Wave 1): use BOUNDED leaf types before a literal (binding/ident/string,
// not greedy `expr`); `<-` lexes as one LEFT_ARROW token (literal "<-" matches it); `&self`
// and `@on` and `@exports` are contiguous multi-token literals; `balanced(c)` captures a
// depth-aware run stopping at `c` (the segment carries no spurious outer `c`).

// --- leaf: state declaration  `$count number: 0;` ---
// name = binding ($count); type = ident (number/bool/string); value = balanced run to `;`.
// The `:` value is REQUIRED (mirrors is_state_declaration): a bare `$name type` with no `:` is
// NOT a decl — it falls through to E0900 (uninitialised decls are unsupported).
%capture_type cb_state {
    $name:binding $stype:ident ":" $value:balanced(';') ";"?
}

// --- leaf: exports block  `@exports { $count: mut }` ---
// body = the brace-inner run; reifier splits on `;`, parses each `$name[: mut]`.
%capture_type cb_exports {
    "@exports" "{" $body:balanced('}')? "}"
}

// --- leaf: self property binding  `&self.content <- $expr;` ---
%capture_type cb_self_prop {
    "&self" "." $prop:ident "<-" $value:balanced(';') ";"?
}

// --- leaf: content injection  `text <- $x;` / `[slot="v"] <- $x;` / `[a="b"] <- $x;` ---
// Capture the VERBATIM target run up to the `<-` arrow (`balanced('<')` stops at the `<` of
// `<-`), then the value run to `;`. The reifier classifies the target string EXACTLY as
// parse_injection did (Text / Slot / Selector / Attr) — pure value-semantics on the split
// field, so a bracket selector keeps its verbatim internal whitespace (BUG-063 #4).
%capture_type cb_inject {
    $target:balanced('<') "<-" $value:balanced(';') ";"?
}

// --- leaf: reactive class toggle  `.active: $isActive;` / `.active: $count > 0;` ---
// class = ident (BEM `nav--open` lexes as one ident). The condition is captured VERBATIM as a
// balanced run after the literal `$` (the literal enforces the original `: $` requirement, so a
// non-`$` declaration like `.foo: bar;` is NOT a toggle and falls through). balanced(';')
// keeps a compound/dotted RHS whole (`$count > 0`, `$.featured`) — the reifier keeps it verbatim
// as var_name, matching parse_class_toggle's `trim_start_matches('$')` (BUG-063 #3).
%capture_type cb_toggle {
    "." $class:ident ":" "$" $var:balanced(';') ";"?
}

// --- leaf: root @on event block  `@on click { $open <- !$open; }` ---
// `balanced('{')` is unusable (the BalancedExtractor treats `{` as a depth-opener, never a
// stop), so capture the WHOLE post-`@on` tail in one balanced run: any `;`/`}` inside the
// action braces sits at depth>0 and is skipped, so `$tail` = `event { ...action... }` to the
// segment end. The reifier reuses the existing `@on event { body }` string splitter.
%capture_type cb_root_on {
    "@on" $tail:balanced(';')
}

// --- leaf: named selector ref  `&sidebar .sidebar { }` / `&cards[] .cards { @each } ` ---
// name = ident; optional `[]` collection marker; sel = selector (the SelectorExtractor balances
// `()` so functional pseudo-classes `:not(.x)`/`:nth-child(2)` are kept whole; BUG-065); body =
// block-inner run.
%capture_type cb_selector_ref {
    "&" $name:ident $collection:cb_collection? $sel:selector "{" $body:balanced('}') "}"
}

// collection marker `[]` (empty brackets) — distinguishes `&cards[]` from `&cards`.
%capture_type cb_collection {
    "[" "]"
}

// --- nested @each inside a collection selector-ref block ---
// `@each($source as $item[, key: $key]) { &t(args); ... }`. Reuses the SAME header grammar as
// the `each` macro (binding source/item, optional `key:` expr) + `template_invocation+` for the
// body invocations — so the @each construct is grammar, not a Rust string-parser (FUP-038).
// The reifier fills NestedEachDef { source, item, key, invocations } from these split fields.
%capture_type cb_nested_each {
    "@each" "(" $source:binding "as" $item:binding ( "," "key" ":" $key:expr )? ")"
    "{" ( $invocations:template_invocation )* "}"
}

// --- leaf: CSS / behavior block  `.sel { ... }` / `[attr] { ... }` / `#id { ... }` ---
// sel = selector (the SelectorExtractor balances `()` so `.foo:not(.bar)`/`.card:nth-child(2)`
// are kept whole instead of truncating at `(` and leaking the rule into html; BUG-065). body =
// block-inner run. Reifier re-classifies the body via cb_block_item: behavioral constructs ->
// directives; an all-CSS block -> css_rule. Tried AFTER cb_toggle/cb_selector_ref (more specific).
%capture_type cb_block {
    $sel:selector "{" $body:balanced('}') "}"
}

// --- leaf: template ref invocation  `&counter("Likes")` / `&likeCounter &counter(...)` ---
// Captured as a `&`-led balanced run; reifier applies the scalar template-call splitter
// (handles named/collection/dynamic-dispatch forms, arg split). Tried LAST among `&`-arms.
%capture_type cb_template_ref {
    "&" $ref:balanced(';') ";"?
}

// --- block-inner content binding  `text: $count;` / `text: currency($count);` / `text: 5 + $count;` ---
// A bare `prop: <rhs>` inside a `.sel { }` block (no leading sigil). The RHS is captured
// VERBATIM (balanced(';')) — NOT requiring a leading `$` — so a reactive value wrapped in a
// filter call or operator-prefixed (`currency($count)`, `5 + $count`) is kept whole. The reifier
// applies the original `rhs.contains('$')` gate: a `$` ANYWHERE -> ContentBinding (expr path,
// dep collection); no `$` (`color: red`) -> NOT a binding, the block stays a css_rule (BUG-065).
%capture_type cb_content_bind {
    $prop:ident ":" $val:balanced(';') ";"?
}

// === cb_block_item: classify ONE block-inner segment (the generic segmenter splits them) ===
// Mirrors the top-level component_item Choice but for the scoped constructs valid inside a
// `.sel { }` behavior block: scoped @on, class toggle, self-prop binding, content binding.
// Replaces reify_block's Rust string-prefix classification (FEAT-080).
%capture_type cb_block_item {
      ( $on:cb_root_on )
    | ( $toggle:cb_toggle )
    | ( $self_prop:cb_self_prop )
    | ( $content:cb_content_bind )
}

// --- leaf: root @media block  `@media (max-width: 700px) { .sel { ... } }` ---
// BUG-167: previously no `component_item` arm matched `@media`, so it fell through
// the unclassifiable branch and was SILENTLY DROPPED (worst failure mode — no
// error, no CSS). `balanced('{')` is unusable as the condition delimiter (the
// BalancedExtractor treats `{` as a depth-opener, never a stop — same reason
// cb_root_on captures its whole tail rather than splitting at the first `{`), so
// capture the WHOLE post-`@media` run in one balanced(';') scan: `$tail` = the
// `(condition) { ...css-only body... }` text to the segment end. The reifier
// (Rust side) splits `$tail` at its first top-level `{` into condition + body and
// hoists the body's CSS verbatim to the page's global stylesheet — template CSS is
// already unscoped (FUP-062 tracks proper scoping), so the hoist matches every
// other template-body CSS rule's current global-namespace behavior.
%capture_type cb_media {
    "@media" $tail:balanced(';')
}

// === component_item: the ordered Choice the classifier becomes ===
// First matching arm wins (mirrors the old if/else order). Specific arms precede general:
// state -> exports -> self_prop -> inject -> toggle -> root_on -> media -> match ->
// selector_ref -> block -> template_ref. `match_block` is reused from the enum
// module (stdlib/enum/capture-types/match-arms.st — PLAN-077 W1; formerly
// match-block.st)
// (closes the @match special-case — it is now just another arm). `media` (BUG-167) sits
// right after `root_on` (both are root `@name { }` directives) and before `match`/
// `selector_ref`/`block` so its `@media` literal never gets mis-consumed as a generic block.
%capture_type component_item {
      ( $state:cb_state )
    | ( $exports:cb_exports )
    | ( $self_prop:cb_self_prop )
    | ( $inject:cb_inject )
    | ( $toggle:cb_toggle )
    | ( $root_on:cb_root_on )
    | ( $media:cb_media )
    | ( $match:match_block )
    | ( $selector_ref:cb_selector_ref )
    | ( $block:cb_block )
    | ( $template_ref:cb_template_ref )
}

