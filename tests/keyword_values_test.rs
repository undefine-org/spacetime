//! Closed sets are keywords (PLAN-156 L3).
//!
//! A param whose type is a set of literal words — an inline union
//! `$direction:("horizontal" | "vertical" | "both")` or a named keyword set
//! (`%capture_type light_kind { "ambient" | … }`) — takes the word bare or
//! quoted, and both are one value. A word outside the set is an error: it
//! used to compile silently, its param dropped (`@resize(direction:
//! sideways)` built `null != "vertical"`).

use spacetime::{CompileOptions, compile, parse};

fn compiled(src: &str) -> (Vec<String>, String) {
    let ast = match parse(src) {
        Ok(ast) => ast,
        Err(e) => return (vec![format!("PARSE: {e:?}")], String::new()),
    };
    let out = compile(&ast, CompileOptions::default());
    let errs = out.pipeline_errors.iter().map(|e| format!("{}: {}", e.code, e.message)).collect();
    (errs, out.js.clone())
}

fn page(value: &str) -> String {
    format!(".box {{ @resize(direction: {value}, min: 10px) }}\n<div class=\"box\"><div class=\"resize-handle\"></div></div>\n")
}

#[test]
fn a_bare_and_a_quoted_keyword_are_one_value() {
    let (e1, js1) = compiled(&page("horizontal"));
    let (e2, js2) = compiled(&page("\"horizontal\""));
    assert!(e1.iter().chain(&e2).all(|e| !e.starts_with('E')), "{e1:?} {e2:?}");
    assert!(js1.contains("\"horizontal\" != \"vertical\""), "the keyword reaches the derive");
    assert_eq!(js1, js2, "bare and quoted compile to the same page");
}

#[test]
fn a_word_outside_the_set_is_an_error() {
    let (errs, _) = compiled(&page("sideways"));
    assert!(
        errs.iter().any(|e| (e.starts_with("E0946") || e.starts_with("E0967")) && e.contains("sideways")),
        "`direction: sideways` is not one of horizontal, vertical, both — got {errs:?}"
    );
}

// ---- typed numbers (PLAN-156 L4) ---------------------------------------------

fn glow(value: &str) -> String {
    format!(
        "%macro glow {{\n  %form {{ @glow(soft: $soft:unit = 0.3) }}\n  %binds {{ noop() }}\n}}\n.a {{ @glow(soft: {value}) }}\n<div class=\"a\"></div>\n"
    )
}

/// A param typed with a scalar row's name matches as the row's `%capture`
/// (`number`) does: a number passes, a word is an error, not a silent `Expr`.
#[test]
fn a_scalar_row_name_types_a_param() {
    let (ok, _) = compiled(&glow("0.6"));
    assert!(ok.iter().all(|e| !e.starts_with('E')), "0.6 is a unit: {ok:?}");
    let (bad, _) = compiled(&glow("soft"));
    assert!(bad.iter().any(|e| e.starts_with("E0946")), "a word is not a unit: {bad:?}");
}

/// The rows carry what an editor needs.
#[test]
fn scalar_rows_carry_range_step_and_unit() {
    let (reg, _) = spacetime::compiler::cached_stdlib_registry();
    let unit = reg.scalar_types().find(|s| s.id == "unit").expect("unit row");
    assert_eq!(unit.range, (Some(0.0), Some(1.0)));
    assert_eq!(unit.step, Some(0.01));
    let m = reg.scalar_types().find(|s| s.id == "meters").expect("meters row");
    assert_eq!(m.unit, "m");
    assert_eq!(m.range, (None, None));
    let i = reg.scalar_types().find(|s| s.id == "intensity").expect("intensity row");
    assert_eq!(i.range, (Some(0.0), None));
}

// ---- named keyword sets -------------------------------------------------------

fn glow_kind(value: &str) -> String {
    format!(
        r#"%capture_type glow_kind {{ "soft" | "hard" | "neon" }}
%primitive glow-prim(&el, kind: string = "") {{
  %emit js {{ %&el.dataset.glow = %kind; }}
}}
%macro glow {{
  %form {{ @glow(kind: $kind:glow_kind = soft) }}
  %binds {{ glow-prim(&self, kind: $kind) }}
}}
.a {{ @glow(kind: {value}) }}
<div class="a"></div>
"#
    )
}

/// A named keyword set is the inline union's twin: bare or quoted, one value;
/// a word outside it is an error.
#[test]
fn a_named_keyword_set_takes_bare_and_quoted_alike() {
    let (e1, js1) = compiled(&glow_kind("neon"));
    let (e2, js2) = compiled(&glow_kind("\"neon\""));
    assert!(e1.iter().chain(&e2).all(|e| !e.starts_with('E')), "{e1:?} {e2:?}");
    assert!(js1.contains("dataset.glow = \"neon\""), "the word reaches the primitive: {js1}");
    assert_eq!(js1, js2);
    let (bad, _) = compiled(&glow_kind("warm"));
    assert!(bad.iter().any(|e| e.starts_with("E0946") && e.contains("warm")), "{bad:?}");
}

// ---- shorthands and longhands (PLAN-156 L5) ----------------------------------

fn spot(args: &str) -> String {
    format!(
        r#"%primitive spot-prim(&el, posX: number = 0, posY: number = 0, posZ: number = 0) {{
  %emit js {{ %&el.dataset.at = [%posX, %posY, %posZ].join(","); }}
}}
%macro spot {{
  %form {{ @spot(posX: $posX:number = 0, posY: $posY:number = 0, posZ: $posZ:number = 0) }}
  %shorthand pos {{ posX posY posZ }}
  %binds {{ spot-prim(&self, ...) }}
}}
.a {{ @spot({args}) }}
<div class="a"></div>
"#
    )
}

fn at(args: &str) -> String {
    let (errs, js) = compiled(&spot(args));
    assert!(errs.iter().all(|e| !e.starts_with('E')), "{args}: {errs:?}");
    let i = js.find("dataset.at = [").unwrap_or_else(|| panic!("{args}: no call in {js}"));
    js[i + 14..].split(']').next().unwrap().replace(' ', "")
}

/// `pos: 1 2 3` and `posX: 1, posY: 2, posZ: 3` are one thing in the IR; as in
/// CSS, the later of a shorthand and a longhand wins; one value sets them all.
#[test]
fn a_shorthand_and_its_longhands_are_one_thing() {
    assert_eq!(at("pos: 1 2 -3"), at("posX: 1, posY: 2, posZ: -3"));
    assert_eq!(at("pos: 0 0 0, posX: 5"), "5,0,0");
    assert_eq!(at("posX: 5, pos: 0 0 0"), "0,0,0");
    assert_eq!(at("pos: 0.42"), "0.42,0.42,0.42");
}
