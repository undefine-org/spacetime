//! BUG-373 / BUG-374 — two ways a page survived `st → edn → st` looking fine
//! and doing nothing.
//!
//! Both were found while wiring `@view $status { "thinking" => &thinking(); }`
//! — the affordance that binds a scalar assign to a template. Neither produced
//! an error at any stage: the document round-tripped, `check` passed, and the
//! behaviour was simply absent.
//!
//! **BUG-373 — the arm's invocation became a string.**
//! `%capture_type arm { $pat:match_pat "=>" $inv:template_invocation ";"? }`.
//! `template_invocation` is a BUILTIN capture type with no `%capture_type`
//! grammar, so `render_by_capture_type` fell through to `value_text`, which
//! renders a `{name, args}` map as a bare word:
//!
//!     @view $status { "thinking" => &thinking(); }   (in)
//!     @view $status { "thinking" => "thinking"; }    (out)
//!
//! That re-parses — an arm whose consequence is a quoted word is well-formed —
//! so nothing complained and the template never mounted.
//!
//! **BUG-374 — a scoped form lost its selector.**
//! `read.rs` has always understood `(sel "…" form…)`. Egress never emitted one,
//! so every macro inside a selector block was written at file level:
//!
//!     .slot { @each($status as $s) { &t(); } }   (in)
//!     @each($status as $s) { &t(); }             (out)
//!
//! `:st/scopes` looks like it should carry this but holds only `:decls` (CSS),
//! so the scope survived as an empty shell while the form that gave it meaning
//! floated free. Both halves round-trip individually — only their RELATIONSHIP
//! was lost, which is why a corpus gate counting forms never saw it.

use spacetime::edn;
use spacetime::syntax::STDLIB_REGISTRY;

/// `st → edn → st`, the operation both bugs hid inside.
fn round_trip(src: &str) -> String {
    let file = spacetime::parser::parse(src).expect("input parses");
    let registry = &*STDLIB_REGISTRY;
    let doc = edn::doc::to_document(&file);
    let text = edn::doc::write_document(&doc, registry);
    let back = edn::to_st_file(&text, registry).expect("EDN reads back");
    edn::print::print_file(&back, registry).expect("prints")
}

const VIEW_PAGE: &str = r#"
@import "stdlib/macros/data-kind"
@import "stdlib/macros/host"
@import "stdlib/enum/dispatch"
@host $chat : live("A.B")
@data subscribe $status from $chat : status;
@template &thinking() { <div class='thinking'><span></span></div> }
@template &idle() { <div class='idle'></div> }
.status-slot { @view $status { "thinking" => &thinking(); _ => &idle(); } }
"#;

#[test]
fn dispatch_arm_keeps_its_template_invocation() {
    let out = round_trip(VIEW_PAGE);

    assert!(
        out.contains("&thinking"),
        "the arm must render a TEMPLATE INVOCATION, not a bare word.\n\
         A `\"thinking\" => \"thinking\";` arm re-parses happily and mounts \
         nothing.\n\nGot:\n{out}"
    );
    assert!(
        !out.contains(r#"=> "thinking""#),
        "the invocation was flattened to a string literal:\n{out}"
    );
    // The catch-all must stay a wildcard, not become the word `_` quoted.
    assert!(
        out.contains("_ =>"),
        "the wildcard arm must survive as `_`:\n{out}"
    );
}

#[test]
fn dispatch_arm_does_not_double_its_terminator() {
    let out = round_trip(VIEW_PAGE);
    assert!(
        !out.contains(";;"),
        "`template_invocation` absorbs its own `;` (BUG-066) and the `arm` \
         grammar declares another; exactly one must survive:\n{out}"
    );
}

#[test]
fn a_scoped_form_keeps_its_selector() {
    let src = r#"
@import "stdlib/macros/data-kind"
@import "stdlib/macros/host"
@host $chat : live("A.B")
@data subscribe $status from $chat : status;
@template &t() { <i class='t'></i> }
.slot { @each($status as $s, key: null) { &t(); } }
"#;
    let out = round_trip(src);

    assert!(
        out.contains(".slot"),
        "the selector must survive: a bind at file level is attached to \
         nothing, and the page compiles clean while doing nothing.\n\nGot:\n{out}"
    );
    // and the form must be INSIDE it, not merely mentioned somewhere
    let slot_at = out.find(".slot").expect("selector present");
    let each_at = out.find("@each").expect("each present");
    assert!(
        each_at > slot_at,
        "`@each` must sit inside `.slot`, not before it:\n{out}"
    );
}

/// The round-tripped page must still COMPILE — the strongest statement
/// available here, since both bugs produced pages that parsed.
#[test]
fn round_tripped_view_page_still_checks() {
    let out = round_trip(VIEW_PAGE);
    let file = spacetime::parser::parse(&out)
        .unwrap_or_else(|e| panic!("round-tripped page must re-parse: {e:?}\n\n{out}"));
    assert!(
        !file.matches.is_empty(),
        "a document with no forms is not a page:\n{out}"
    );
}
