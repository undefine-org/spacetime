// @match/@view dispatch-mode arm grammar (PLAN-077 W1).
//
// MIGRATED from the old top-level match-block capture type (DELETED this wave). The
// migration preserves the runtime record contract EXACTLY —
// `stdlib/enum/dispatch.st`'s `dispatch-mount` (the W5 merge of
// the retired match-render + view primitives) reads `arm.pat.lit` /
// `arm.pat.wild` / `arm.inv[0]` in `%emit js`, so:
//   * `arm` is the canonical render-arm production (`$pat` + `$inv:template_invocation`)
//   * `match_pat` keeps `lit` (string) and `wild` (bare ident: `_` wildcard) branches
//     and ADDITIVELY gains a `variant` branch (destructure pattern,
//     variant-pattern.st). Branch ORDER is load-bearing and non-obvious:
//     ChoiceExtractor is first-match-wins with NO backtracking, and `lit`'s
//     StringExtractor accepts BARE IDENTS as enum-ish strings (FUP-079) — so
//     `lit` tried first would consume `Connected` from `Connected { $send }`
//     and the arm would fail at `{`. `variant` therefore comes FIRST (it fails
//     cleanly on any token stream without a `{` after the head ident: strings,
//     `_`, bare tags), then `lit` (quoted strings + bare payload-less tags like
//     `Blue` — the pre-PLAN-077 shape for every old pattern), then `wild`
//     (only `_` reaches it: StringExtractor refuses a lone `_` by design). Every
//     pre-existing pattern yields the IDENTICAL record as before.
//
// NEW this wave (additive, consumed by no pre-existing form):
//   * `match_value_arm` — the dispatch-mode arm for a VALUE-PRODUCING @match
//     (e.g. `@data derive $x T : @match $subject { Blue => …; }`, wired in W5).
//     Its consequence is an `expr` (a value), never a template invocation. The
//     `;` terminator is REQUIRED (not optional): `expr` is greedy and self-limits
//     only at `;` / `)` / `}` / `,` / `->` / `=>` boundaries, so an unterminated arm
//     would swallow the next arm's pattern — claims.st established the
//     "expr capture + required `;`" discipline for exactly this reason.

/// A single dispatch-mode match arm pattern: a variant destructure pattern
/// (`Connected { $send }`), a string literal (`"list"`, or a bare payload-less
/// variant tag like `Blue` — StringExtractor's FUP-079 bare-ident acceptance),
/// or the `_` wildcard. ORDER LOAD-BEARING — see header.
%capture_type match_pat {
    ( $variant:variant_pattern ) | ( $lit:string ) | ( $wild:ident )
}

/// A dispatch-mode RENDER arm: `pattern => &template(args);`. VERBATIM the
/// pre-PLAN-077 production — the `template_invocation` leaf absorbs its own
/// optional trailing `;` (BUG-066), and malformed-@match detection (E0922)
/// remains the component-body reifier's job, not the separator's.
%capture_type arm {
    $pat:match_pat "=>" $inv:template_invocation ";"?
}

/// A dispatch-mode VALUE arm (NEW, W1): `pattern => expr;` — for a @match in
/// value position (a derive's RHS), where the consequence is the VALUE the
/// signal takes when the arm matches, not a template to mount. `;` REQUIRED
/// (see header). No pre-existing form consumes this; W5's unified dispatch
/// forms reference it.
%capture_type match_value_arm {
    $pat:match_pat "=>" $value:expr ";"
}

/// A full dispatch-mode @match/@view render block: subject + ordered arms.
/// VERBATIM the pre-PLAN-077 production. (The cond-mode subjectless
/// `@match { (guard) => Variant; }` used by @data derive is a SEPARATE capture
/// type in cond-arms.st — its arms are boolean guards, not patterns, and it
/// never carries a subject. The subject token is the grammar-level
/// discriminator between the two modes.)
%capture_type match_block {
    "@match" $subject:binding "{" ( $arms:arm )* "}"
}
