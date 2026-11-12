use super::*;

const LAMP: &str = r#"/// Lamp — a light on the stage.
///
/// @example a warm lamp
/// .stage { @lamp &l (power: 2) }
///
/// @param power  - how strong (default 1)
/// @param tint   - light color (default white)
/// @param posX/Y/Z - where it hangs
%macro lamp {
  %scope selector
  %form {
    @lamp $named:element? (power: $power:number = 1, tint: $tint:color = white, posX: $posX:number = 0, posY: $posY:number = 0, posZ: $posZ:number = 0)
  }
  %binds {
    the-lamp(&self, name: $named, power: $power, tint: $tint, posX: $posX, posY: $posY, posZ: $posZ)
  }
}

/// The lamp's three.js side.
///
/// @param &el    - the canvas
/// @param power  - how strong, in candela
/// @param decay  - how fast it fades
/// @yields $lit - true once the light is in the scene
%primitive the-lamp(
  &el,
  name: string = "",
  power: number = 1,
  tint: color = white,
  posX: number = 0,
  posY: number = 0,
  posZ: number = 0,
  decay: number = 2
) {
  %exports { $lit: bool }
  %emit js {
    %yield true -> $lit;
  }
}
"#;

fn files() -> BTreeMap<String, String> {
    [("lamp.st".to_string(), LAMP.to_string())].into()
}

fn after(transform: &str, files: &BTreeMap<String, String>) -> String {
    let edits = run(transform, files).expect("runs");
    let tx = crate::rewrite::Transaction { edits, base: BTreeMap::new() };
    let out = crate::rewrite::apply(files, &tx).expect("applies and still parses");
    out.get("lamp.st").cloned().unwrap_or_else(|| files["lamp.st"].clone())
}

#[test]
fn param_docs_move_onto_the_params() {
    let out = after("docs-onto-declarations", &files());
    // the macro's form: one param per line, each doc above its param
    assert!(out.contains("      /// how strong\n      power: $power:number = 1,"), "{out}");
    assert!(out.contains("      /// where it hangs\n      posX: $posX:number = 0,"), "{out}");
    assert!(out.contains("      posY: $posY:number = 0,\n      posZ: $posZ:number = 0\n    )"), "one doc for the group, on posX: {out}");
    // a doc that only said « light color (default white) » keeps « light color »
    assert!(out.contains("      /// light color\n      tint:"), "{out}");
    // the tags are gone from the doc blocks, the prose stays
    assert!(!out.contains("@param"), "{out}");
    assert!(out.starts_with("/// Lamp — a light on the stage.\n///\n/// @example a warm lamp\n"), "{out}");
    // the primitive: `power` is the macro's to document (it forwards it);
    // `decay` isn't forwarded, so it's documented on the primitive
    assert!(out.contains("  /// how fast it fades\n  decay: number = 2\n)"), "{out}");
    assert!(!out.contains("in candela"), "power is documented once, on the form: {out}");
    // the yield is documented on its export
    assert!(out.contains("  %exports {\n") || out.contains("/// true once the light is in the scene\n"), "{out}");
}

#[test]
fn the_result_parses_and_the_docs_reach_the_ast() {
    let out = after("docs-onto-declarations", &files());
    let ast = crate::parser::parse(&out).expect("parses");
    let m = ast
        .meta_defs
        .iter()
        .find_map(|d| match d {
            MetaDef::Macro(m) if m.name == "lamp" => Some(m),
            _ => None,
        })
        .expect("lamp");
    let form = m.form.as_ref().unwrap();
    let power = form.params.iter().find(|p| p.name == "power").unwrap();
    assert_eq!(power.doc.as_deref(), Some("how strong"));
    assert_eq!(form.params.iter().filter(|p| p.doc.as_deref() == Some("where it hangs")).count(), 1);
    let p = ast
        .meta_defs
        .iter()
        .find_map(|d| match d {
            MetaDef::Primitive(p) => Some(p),
            _ => None,
        })
        .unwrap();
    assert_eq!(p.params.len(), 8, "the primitive's params still read the same");
}

#[test]
fn examples_become_clauses() {
    let docs = after("docs-onto-declarations", &files());
    let f: BTreeMap<String, String> = [("lamp.st".to_string(), docs)].into();
    let out = after("examples-to-clauses", &f);
    assert!(!out.contains("@example"), "{out}");
    assert!(out.contains("  %example \"a warm lamp\" {\n    .stage { @lamp &l (power: 2) }\n  }\n}"), "{out}");
    let ast = crate::parser::parse(&out).expect("parses");
    let m = ast
        .meta_defs
        .iter()
        .find_map(|d| match d {
            MetaDef::Macro(m) if m.name == "lamp" => Some(m),
            _ => None,
        })
        .unwrap();
    assert_eq!(m.examples.len(), 1);
    assert_eq!(m.examples[0].source, ".stage { @lamp &l (power: 2) }");
}

#[test]
fn pure_forwards_become_a_spread() {
    let out = after("forward-binds", &files());
    assert!(out.contains("the-lamp(&self, name: $named, ...)"), "{out}");
    // and it means the same thing
    let ast = crate::parser::parse(&out).unwrap();
    let m = ast
        .meta_defs
        .iter()
        .find_map(|d| match d {
            MetaDef::Macro(m) if m.name == "lamp" => Some(m),
            _ => None,
        })
        .unwrap();
    let names: Vec<String> = m.binds[0]
        .args
        .iter()
        .filter_map(|a| match a {
            BindArg::Named { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(names, ["name", "power", "tint", "posX", "posY", "posZ"]);
}

#[test]
fn transforms_are_idempotent() {
    let once = after("docs-onto-declarations", &files());
    let f: BTreeMap<String, String> = [("lamp.st".to_string(), once.clone())].into();
    assert!(run("docs-onto-declarations", &f).unwrap().is_empty());
    let fwd = after("forward-binds", &f);
    let f2: BTreeMap<String, String> = [("lamp.st".to_string(), fwd)].into();
    assert!(run("forward-binds", &f2).unwrap().is_empty());
}

#[test]
fn names_expand_and_defaults_are_dropped() {
    assert_eq!(expand_names("posX/Y/Z"), ["posX", "posY", "posZ"]);
    assert_eq!(expand_names("distance/decay"), ["distance", "decay"]);
    assert_eq!(expand_names("minDistance/maxDistance"), ["minDistance", "maxDistance"]);
    assert_eq!(expand_names("&el"), ["el"]);
    assert_eq!(split_name("targetX/Y/Z- aim point"), ("targetX/Y/Z", "aim point"));
    assert_eq!(clean_doc("intensity", "intensity (default 1)"), None);
    assert_eq!(clean_doc("size", "target size (0 = no rescale); default 2").as_deref(), Some("target size (0 = no rescale)"));
    assert_eq!(clean_doc("tone", "\"aces\" | \"linear\" (default aces)").as_deref(), Some("\"aces\" | \"linear\""));
}
