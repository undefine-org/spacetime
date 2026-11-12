//! `%example` clauses compile (PLAN-156).
//!
//! A macro's examples are pages: each is written to a scratch file and
//! compiled the way `spacetime check` compiles a page, against the workspace
//! (so `@import "stdlib/3d"` resolves). An example fails when it has an error,
//! or when it never uses the macro it documents: an example that silently
//! stopped matching its `%form` is the drift examples exist to catch.

use std::path::Path;

use crate::MetaRegistry;

/// One `%example` that doesn't hold.
#[derive(Debug, Clone)]
pub struct Failure {
    pub macro_name: String,
    pub title: String,
    pub problems: Vec<String>,
}

/// Compile every `%example` of every macro in `registry`.
pub fn check(registry: &MetaRegistry, workspace_root: &Path) -> Vec<Failure> {
    let scratch = std::env::temp_dir().join(format!("st-examples-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&scratch);
    let mut out = Vec::new();
    let mut macros: Vec<_> = registry.iter_macros().filter(|(_, m)| !m.examples.is_empty()).collect();
    macros.sort_by_key(|(k, _)| k.to_string());
    for (name, mac) in macros {
        for ex in &mac.examples {
            let directive = mac.form.as_ref().map(|f| f.directive_name.clone()).unwrap_or_default();
            let (mut problems, directives) = check_one(name, &ex.source, &scratch, workspace_root);
            // some directives never become matches (`@import` is resolved before
            // matching): the directive written in the source is a use, when no
            // match came from that directive at all (else another macro took it)
            let d = directive.trim_start_matches('@');
            if !d.is_empty() && written(&ex.source, d) && !directives.contains(d) {
                problems.retain(|p| !p.contains("never uses"));
            }
            if !problems.is_empty() {
                out.push(Failure { macro_name: name.to_string(), title: ex.title.clone(), problems });
            }
        }
    }
    let _ = std::fs::remove_dir_all(&scratch);
    out
}

/// Compile one example: its problems, and the directives its matches came from.
fn check_one(
    macro_name: &str,
    source: &str,
    scratch: &Path,
    workspace_root: &Path,
) -> (Vec<String>, std::collections::HashSet<String>) {
    let path = scratch.join("index.st");
    if let Err(e) = std::fs::write(&path, source) {
        return (vec![format!("write: {e}")], Default::default());
    }
    let compiler = match crate::compiler::Compiler::from_file(&path, workspace_root) {
        Ok(c) => c,
        Err(e) => return (vec![e], Default::default()),
    };
    let (used, directives) = {
        let ast = compiler.merged_ast();
        let mut all = Vec::new();
        crate::render::collect_scope_matches(&ast.scopes, &mut all);
        all.extend(ast.matches.iter().cloned());
        let directives = all.iter().map(|m| m.macro_name.trim_start_matches('@').to_string()).collect();
        (uses(&all, macro_name), directives)
    };
    let compiled = compiler.compile();
    let mut problems: Vec<String> = compiled
        .pipeline_errors
        .iter()
        .filter(|e| e.code.starts_with('E'))
        .map(|e| format!("{}: {}", e.code, e.message))
        .collect();
    if !used {
        problems.push(format!("the example never uses @{macro_name}: its source no longer matches the macro's %form"));
    }
    (problems, directives)
}

/// Is `@directive` written in `source` (followed by a non-name character)?
fn written(source: &str, directive: &str) -> bool {
    let d = format!("@{}", directive.trim_start_matches('@'));
    source.match_indices(&d).any(|(i, _)| {
        !source[i + d.len()..].chars().next().is_some_and(|c| c.is_alphanumeric() || c == '-' || c == '_')
    })
}

/// Does any match (nested ones included) come from `macro_name`?
fn uses(matches: &[crate::syntax::FormMatch], macro_name: &str) -> bool {
    fn nested(v: &crate::syntax::CapturedValue, name: &str) -> bool {
        use crate::syntax::CapturedValue as V;
        match v {
            V::Block(ms) => uses(ms, name),
            V::Array(items) => items.iter().any(|i| nested(i, name)),
            V::Named(f) => f.values().any(|i| nested(i, name)),
            // a body kept as text (`@test "…" { … }`'s steps) is read as source
            V::String(s) | V::Expr(s) if s.contains('@') => crate::parser::parse(s).is_ok_and(|ast| {
                let mut all = Vec::new();
                crate::render::collect_scope_matches(&ast.scopes, &mut all);
                all.extend(ast.matches.iter().cloned());
                uses(&all, name)
            }),
            _ => false,
        }
    }
    matches.iter().any(|m| {
        m.matched_macro.as_deref() == Some(macro_name)
            || m.macro_name == macro_name
            || m.captures.values().any(|v| nested(v, macro_name))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Examples that are right but whose MACRO is broken (PLAN-156 « Found on the
    /// way »). A ratchet both ways: no other example may fail, and an entry
    /// whose example starts to hold must leave the list.
    const KNOWN_BROKEN: &[(&str, &str)] = &[
        ("channel", "websocket.st names capture types it never declares"),
        ("cleanup", "a `{ $x:expr }` body capture drops its closing brace"),
        ("conditional", "no %binds/%emit (BUG-362)"),
        ("fade-in-stagger", "its `$_childSelector:selector` body capture never receives tokens"),
        ("fn", "a `{ $x:expr }` body capture drops its closing brace"),
        ("for-range", "no %binds/%emit (E0956)"),
        ("given-data", "`@given $x = …` loses to the selector form of @given"),
        ("localStorage", "the expression macro never applies"),
        ("realtime-custom", "websocket.st names capture types it never declares"),
        ("repeat", "no %binds/%emit (E0956)"),
        ("template-inline", "`&name(…) { … }` at file level is E0925"),
        ("ws-message-handler", "websocket.st names capture types it never declares"),
    ];

    #[test]
    fn every_stdlib_example_compiles_and_uses_its_macro() {
        let (registry, _) = crate::compiler::cached_stdlib_registry();
        let failures = check(&registry, Path::new("."));
        let unexpected: Vec<&Failure> =
            failures.iter().filter(|f| !KNOWN_BROKEN.iter().any(|(m, _)| *m == f.macro_name)).collect();
        assert!(unexpected.is_empty(), "stdlib examples that don't hold:\n{unexpected:#?}");
        for (m, why) in KNOWN_BROKEN {
            assert!(
                failures.iter().any(|f| f.macro_name == *m),
                "@{m}'s example holds now ({why} is fixed?): take it off KNOWN_BROKEN"
            );
        }
    }

    #[test]
    fn examples_are_parsed_in_order_with_their_titles() {
        let src = r#"
%macro glow {
  %form { @glow(gain: $gain:number = 1) }
  %example "two stops brighter" {
    .a { @glow(gain: 2) }
    <div class="a"></div>
  }
  %example "off" {
    .a { @glow(gain: 0) }
  }
}
"#;
        let ast = crate::parser::parse(src).expect("parses");
        let m = ast
            .meta_defs
            .iter()
            .find_map(|d| match d {
                crate::parser::meta_ast::MetaDef::Macro(m) => Some(m),
                _ => None,
            })
            .expect("glow");
        assert_eq!(m.examples.len(), 2);
        assert_eq!(m.examples[0].title, "two stops brighter");
        assert_eq!(m.examples[0].source, ".a { @glow(gain: 2) }\n<div class=\"a\"></div>");
        assert_eq!(m.examples[1].title, "off");
    }

    #[test]
    fn an_example_that_stopped_matching_is_caught() {
        let scratch = std::env::temp_dir().join(format!("st-example-test-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        let def = "%macro glow {\n  %form { @glow(gain: $gain:number = 1) }\n  %binds { noop() }\n}\n";
        let good = format!("{def}.a {{ @glow(gain: 2) }}\n<div class=\"a\"></div>\n");
        let stale = format!("{def}.a {{ @glowing(gain: 2) }}\n<div class=\"a\"></div>\n");
        let (ok, _) = check_one("glow", &good, &scratch, Path::new("."));
        assert!(ok.iter().all(|p| !p.contains("never uses")), "{ok:?}");
        let (bad, _) = check_one("glow", &stale, &scratch, Path::new("."));
        assert!(bad.iter().any(|p| p.contains("never uses @glow")), "{bad:?}");
        std::fs::remove_dir_all(&scratch).ok();
    }
}
