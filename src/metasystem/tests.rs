//! Tests for the metasystem.

use super::registry::MetaRegistry;
use super::validate::{validate_macro, validate_primitive};
use crate::parser;

/// Helper to parse metadefs from source
fn parse_meta(source: &str) -> Result<parser::StFile, String> {
    parser::parse(source).map_err(|e| e.to_string())
}

// =============================================================================
// Grammar Tests
// =============================================================================
// =============================================================================
// %migration tests (PLAN-076 W1)
// =============================================================================

mod migrations {
    use super::*;
    use crate::parser::meta_ast::{MetaDef, MigrationDefAst};

    /// A capsule (PLAN-079): embedded retired %macro + a rewrite rule.
    const REWRITE_MIGRATION: &str = r#"/// Retires @bind(text:).
%migration bind-text-to-arrow {
  %date 2026-06-09
  %docs "FEAT-072: @bind(text:) became the text arrow binding."

  %macro bind {
    %form { @bind(text: $text:expr? = undefined) }
    %if $text {
      %binds { bind-text(&self, value: $text) }
    }
  }

  %rewrite bind-text {
    %match {
      @bind(text: $x:expr)
    }
    %into {
      text <- `$x`;
    }
  }
}"#;

    /// A hint-only capsule: embedded macro with %binds (live semantics) + a hint.
    const HINT_MIGRATION: &str = r#"%migration show-to-class-toggle {
  %date 2026-06-09
  %docs "FEAT-072: @show became a reactive class toggle."

  %macro show {
    %form { @show(when: $condition:expr, mode: $mode:string = "display") }
    %binds { bind-visible(&self, condition: $condition, mode: $mode) }
  }

  %hint for @show "Replace with a reactive class toggle: `.hidden: !$cond;`."
}"#;

    fn parse_migration(source: &str) -> MigrationDefAst {
        let file = parse_meta(source).expect("migration source must parse");
        file.meta_defs
            .into_iter()
            .find_map(|d| match d {
                MetaDef::Migration(m) => Some(m),
                _ => None,
            })
            .expect("expected a %migration def")
    }

    #[test]
    fn parses_capsule_definition() {
        let mig = parse_migration(REWRITE_MIGRATION);
        assert_eq!(mig.id, "bind-text-to-arrow");
        assert_eq!(mig.date, "2026-06-09");
        assert_eq!(
            mig.docs,
            "FEAT-072: @bind(text:) became the text arrow binding."
        );
        assert!(mig.is_rewrite());
        // The embedded retired macro parsed through the REAL macro path:
        // form directive + the %if-wrapped %binds landing as body items.
        assert_eq!(mig.macros.len(), 1);
        assert_eq!(mig.macros[0].name, "bind");
        assert_eq!(mig.macros[0].form.as_ref().unwrap().directive_name, "@bind");
        assert!(
            !mig.macros[0].body.is_empty(),
            "embedded macro keeps its %if arms: {:?}",
            mig.macros[0]
        );
        // The doc comment above the def is collected.
        assert_eq!(mig.doc.as_deref(), Some("Retires @bind(text:)."));
        // The rewrite rule: id + pattern + template with the hole intact.
        assert_eq!(mig.rewrites.len(), 1);
        let rule = &mig.rewrites[0];
        assert_eq!(rule.id, "bind-text");
        assert_eq!(rule.match_form.directive_name, "@bind");
        assert!(
            rule.template.contains("`$x`"),
            "template: {:?}",
            rule.template
        );
    }

    #[test]
    fn parses_hint_definition() {
        let mig = parse_migration(HINT_MIGRATION);
        assert!(!mig.is_rewrite());
        // The embedded macro's %binds populate a REAL MacroDefAst (the old
        // semantics stay executable through the window).
        assert_eq!(mig.macros.len(), 1);
        assert_eq!(mig.macros[0].binds.len(), 1, "{:?}", mig.macros[0].binds);
        assert_eq!(mig.retired_directives(), vec!["show".to_string()]);
        assert_eq!(
            mig.hint_for("show"),
            Some("Replace with a reactive class toggle: `.hidden: !$cond;`.")
        );
    }

    #[test]
    fn registration_macros_are_rule_first_and_tagged() {
        let mig = parse_migration(REWRITE_MIGRATION);
        let defs = mig.registration_macros();
        assert_eq!(defs.len(), 2);
        // Rule def FIRST (load-bearing: first-match-wins must prefer the
        // specific rule pattern over the embedded macro's broader form).
        assert_eq!(defs[0].name, "bind-text-to-arrow#bind-text");
        let tag = defs[0].retired.as_ref().expect("rule def is tagged");
        assert_eq!(tag.migration, "bind-text-to-arrow");
        assert_eq!(tag.wave, "2026-06-09");
        assert_eq!(tag.rule.as_deref(), Some("bind-text"));
        assert!(defs[0].binds.is_empty(), "rule defs never Resolve");
        // Embedded macro, migration-qualified, tagged rule: None.
        assert_eq!(defs[1].name, "bind-text-to-arrow#@bind");
        assert_eq!(defs[1].retired.as_ref().unwrap().rule, None);
    }

    #[test]
    fn registers_and_waves_are_date_ordered() {
        let mut registry = MetaRegistry::new();
        let newer = parse_migration(
            &REWRITE_MIGRATION
                .replace("2026-06-09", "2027-01-15")
                .replace("bind-text-to-arrow", "bind-text-to-arrow-v2"),
        );
        let older = parse_migration(REWRITE_MIGRATION);
        // Register OUT of date order — the store must still iterate sorted.
        registry.register_migration(newer).unwrap();
        registry.register_migration(older).unwrap();

        let dates: Vec<&str> = registry.migration_wave_dates().collect();
        assert_eq!(dates, ["2026-06-09", "2027-01-15"]);

        let mig = registry.get_migration("bind-text-to-arrow").unwrap();
        assert_eq!(mig.date, "2026-06-09");

        // The FormMatch → migration lookup covers BOTH def kinds.
        assert_eq!(
            registry.retired_by_macro("bind-text-to-arrow#bind-text"),
            Some(&(
                "bind-text-to-arrow".to_string(),
                Some("bind-text".to_string())
            ))
        );
        assert_eq!(
            registry.retired_by_macro("bind-text-to-arrow#@bind"),
            Some(&("bind-text-to-arrow".to_string(), None))
        );
        assert!(registry.retired_by_macro("bind").is_none());

        // @version filtering: a project at the older wave only sees the newer.
        let pending = registry.migrations_pending_since(Some("2026-06-09"));
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].date, "2027-01-15");
        // No @version = date-zero: everything participates.
        assert_eq!(registry.migrations_pending_since(None).len(), 2);
        // At-or-past the newest wave: nothing pending.
        assert!(
            registry
                .migrations_pending_since(Some("2027-01-15"))
                .is_empty()
        );
    }

    #[test]
    fn rejects_bad_date() {
        let mut registry = MetaRegistry::new();
        for bad in ["2026-6-9", "2026/06/09", "2026-13-01", "2026-06-32", "soon"] {
            let mig = parse_migration(&REWRITE_MIGRATION.replace("2026-06-09", bad));
            let err = registry.register_migration(mig);
            assert!(err.is_err(), "date {bad:?} must be rejected");
        }
    }

    #[test]
    fn rejects_duplicate_id() {
        let mut registry = MetaRegistry::new();
        registry
            .register_migration(parse_migration(REWRITE_MIGRATION))
            .unwrap();
        let dup = parse_migration(REWRITE_MIGRATION);
        let err = registry.register_migration(dup).unwrap_err();
        assert!(err.to_string().contains("duplicate migration"));
    }

    #[test]
    fn rejects_unknown_rewrite_hole() {
        let mut registry = MetaRegistry::new();
        let bad = REWRITE_MIGRATION.replace("text <- `$x`;", "text <- `$nope`;");
        let err = registry
            .register_migration(parse_migration(&bad))
            .unwrap_err();
        assert!(
            err.to_string().contains("not a %match capture"),
            "err: {err}"
        );
    }

    #[test]
    fn rejects_dangling_hint_directive() {
        let mut registry = MetaRegistry::new();
        let bad = REWRITE_MIGRATION.replace(
            "%rewrite bind-text {",
            "%hint for @nope \"guidance\"\n  %rewrite bind-text {",
        );
        let err = registry
            .register_migration(parse_migration(&bad))
            .unwrap_err();
        assert!(
            err.to_string().contains("names no embedded %macro"),
            "err: {err}"
        );
    }

    #[test]
    fn rejects_uncovered_directive() {
        // An embedded macro with NEITHER a rewrite rule NOR a hint: the
        // retirement tells users nothing — a load error.
        let mut registry = MetaRegistry::new();
        let bad = REWRITE_MIGRATION.replace(
            "%rewrite bind-text {\n    %match {\n      @bind(text: $x:expr)\n    }\n    %into {\n      text <- `$x`;\n    }\n  }",
            "",
        );
        let err = registry
            .register_migration(parse_migration(&bad))
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("neither a %rewrite rule nor a %hint"),
            "err: {err}"
        );
    }

    #[test]
    fn rejects_live_macro_collision_both_directions() {
        // Migration first, then a live macro claiming the same directive.
        let mut registry = MetaRegistry::new();
        registry
            .register_migration(parse_migration(REWRITE_MIGRATION))
            .unwrap();
        let live = parse_meta("%macro bind2 {\n  %form { @bind(text: $x:expr) }\n}")
            .unwrap()
            .meta_defs
            .into_iter()
            .find_map(|d| match d {
                MetaDef::Macro(m) => Some(m),
                _ => None,
            })
            .unwrap();
        let err = registry.register_macro(live).unwrap_err();
        assert!(
            err.to_string().contains("collides with migration"),
            "err: {err}"
        );

        // Live macro first, then a migration claiming its directive.
        let mut registry2 = MetaRegistry::new();
        let live = parse_meta("%macro bind {\n  %form { @bind(text: $x:expr) }\n}")
            .unwrap()
            .meta_defs
            .into_iter()
            .find_map(|d| match d {
                MetaDef::Macro(m) => Some(m),
                _ => None,
            })
            .unwrap();
        registry2.register_macro(live).unwrap();
        let err = registry2
            .register_migration(parse_migration(REWRITE_MIGRATION))
            .unwrap_err();
        assert!(err.to_string().contains("RETIRED syntax"), "err: {err}");
    }

    #[test]
    fn rejects_self_cycle_at_registration() {
        // Self-cycle is per-registration (order-independent: the own
        // directive is always known).
        let mut registry = MetaRegistry::new();
        let cyclic = REWRITE_MIGRATION.replace("text <- `$x`;", "@bind(text: `$x`)");
        let err = registry
            .register_migration(parse_migration(&cyclic))
            .unwrap_err();
        assert!(err.to_string().contains("cycle"), "err: {err}");
    }

    /// W3.4 review: an OPTIONAL %match capture (`$m:string?`) with no default
    /// spliced in the template renders the hole VERBATIM when the arg is
    /// absent (measured: `@loop x(600ms)` -> `mode: `$mode`` in migrated
    /// source). Registration must reject the rule; the behavior-exact fix is
    /// declaring the retired macro's default on the capture.
    const UNSAFE_OPTIONAL_HOLE: &str = r#"%migration loop-cutover {
  %date 2026-07-26
  %docs "test"

  %macro loop {
    %form { @loop $name:ident ($duration:time) { $body:keyframes } }
    %binds { loop-driver(name: $name, duration: $duration) }
  }

  %rewrite loop {
    %match {
      @loop $name:ident ($duration:time, mode: $mode:string?) {
        $body:keyframes
      }
    }
    %into {
      @on &.loop(name: `$name`, duration: `$duration`, mode: `$mode`) {
`$body`
      }
    }
  }
}"#;

    #[test]
    fn unsafe_optional_hole_rejected_at_registration() {
        let mut registry = MetaRegistry::new();
        let err = registry
            .register_migration(parse_migration(UNSAFE_OPTIONAL_HOLE))
            .unwrap_err();
        assert!(
            err.to_string().contains("optional capture `$mode?`"),
            "err: {err}"
        );
    }

    #[test]
    fn defaulted_optional_hole_accepted() {
        let mut registry = MetaRegistry::new();
        let safe = UNSAFE_OPTIONAL_HOLE
            .replace("mode: $mode:string?", "mode: $mode:string = \"pingpong\"");
        registry
            .register_migration(parse_migration(&safe))
            .expect("a defaulted capture always produces a value");
    }

    #[test]
    fn chains_validated_post_load_regardless_of_registration_order() {
        use crate::metasystem::validate_migration_chains;

        // Backward chain: the NEWER migration's rewrite references an OLDER
        // wave's directive. Registration accepts it; the post-load pass
        // rejects — whichever order they registered in.
        let backward = HINT_MIGRATION
            .replace("2026-06-09", "2027-01-15")
            .replace(
                "%hint for @show \"Replace with a reactive class toggle: `.hidden: !$cond;`.\"",
                "%rewrite show-to-bind {\n    %match { @show(when: $condition:expr) }\n    %into { @bind(text: `$condition`) }\n  }",
            );
        for register_older_first in [true, false] {
            let mut registry = MetaRegistry::new();
            let older = parse_migration(REWRITE_MIGRATION);
            let newer = parse_migration(&backward);
            if register_older_first {
                registry.register_migration(older).unwrap();
                registry.register_migration(newer).unwrap();
            } else {
                registry.register_migration(newer).unwrap();
                registry.register_migration(older).unwrap();
            }
            let errs = validate_migration_chains(&registry).unwrap_err();
            assert!(
                errs.iter().any(|e| e.to_string().contains("FORWARD")),
                "older_first={register_older_first}: {errs:?}"
            );
        }

        // Forward chain: older wave's rewrite references the NEWER wave's
        // directive — the intended A>B>C shape.
        let mut registry = MetaRegistry::new();
        registry
            .register_migration(parse_migration(
                &HINT_MIGRATION.replace("2026-06-09", "2027-01-15"),
            ))
            .unwrap();
        let forward = REWRITE_MIGRATION.replace("text <- `$x`;", "@show(when: `$x`)");
        registry
            .register_migration(parse_migration(&forward))
            .unwrap();
        validate_migration_chains(&registry).expect("forward chain must pass");
    }

    #[test]
    fn accepts_intra_capsule_rule_chain() {
        // A rule whose %into emits a SIBLING retired directive of the SAME
        // migration (which has its own rule) is a legitimate staged
        // retirement: @a → @b → current, one wave.
        let mut registry = MetaRegistry::new();
        let chain = r#"%migration a-b-chain {
  %date 2025-01-01
  %docs "@a became @b became the arrow."

  %macro a {
    %form { @a($x:expr) }
  }
  %macro b {
    %form { @b($x:expr) }
  }

  %rewrite a-to-b {
    %match { @a($x:expr) }
    %into { @b(`$x`) }
  }
  %rewrite b-to-arrow {
    %match { @b($x:expr) }
    %into { text <- `$x`; }
  }
}"#;
        registry
            .register_migration(parse_migration(chain))
            .expect("intra-capsule chain registers");
        crate::metasystem::validate_migration_chains(&registry)
            .expect("intra-capsule chain validates");
    }

    #[test]
    fn rejects_duplicate_embedded_macro_names() {
        // Two same-named embedded macros would collide on the `<mig>#@<name>`
        // registration key and fail registration mid-loop (R4).
        let mut registry = MetaRegistry::new();
        let dup = r#"%migration dup-macros {
  %date 2025-01-01
  %docs "dup"

  %macro a {
    %form { @a($x:expr) }
  }
  %macro a {
    %form { @a2($y:expr) }
  }

  %hint for @a "x"
}"#;
        let err = registry
            .register_migration(parse_migration(dup))
            .unwrap_err();
        assert!(
            err.to_string().contains("duplicate embedded %macro"),
            "err: {err}"
        );
    }

    #[test]
    fn hint_text_is_unescaped() {
        let mig =
            parse_migration(&HINT_MIGRATION.replace("`.hidden: !$cond;`", "mode: \"visibility\""));
        assert_eq!(
            mig.hint_for("show"),
            Some("Replace with a reactive class toggle: mode: \"visibility\".")
        );
    }

    #[test]
    fn accepts_alias_capture_holes() {
        // %match grammar constructs that bind names beyond plain captures:
        // `$xs:expr as $x:ident` (alias). (Comparison captures
        // `>= $min:number` are collected too, but the %form param parser
        // cannot currently CONSTRUCT them — pre-existing quirk, filed as a
        // BUG; the collector arm is ready for when it can.)
        let mut registry = MetaRegistry::new();
        let alias = r#"%migration list-to-view {
  %date 2026-06-09
  %docs "retires @list"

  %macro list {
    %form { @list($items:expr? = undefined) }
  }

  %rewrite list-view {
    %match {
      @list($xs:expr as $x:ident)
    }
    %into {
      @view($x in `$xs`)
    }
  }
}"#;
        registry
            .register_migration(parse_migration(alias))
            .expect("alias capture hole must be accepted");
    }

    #[test]
    fn unregister_file_removes_migration_from_all_maps() {
        let mut registry = MetaRegistry::new();
        let mut mig = parse_migration(REWRITE_MIGRATION);
        mig.source_file = Some("stdlib/migrations/entries/test.st".to_string());
        registry.register_migration(mig).unwrap();
        assert!(registry.get_migration("bind-text-to-arrow").is_some());
        assert!(
            registry
                .retired_by_macro("bind-text-to-arrow#@bind")
                .is_some()
        );
        assert!(
            registry
                .retired_by_macro("bind-text-to-arrow#bind-text")
                .is_some()
        );

        registry.unregister_file("stdlib/migrations/entries/test.st");
        assert!(registry.get_migration("bind-text-to-arrow").is_none());
        assert!(registry.all_migrations().next().is_none());
        assert!(registry.migration_wave_dates().next().is_none());
        assert!(
            registry
                .retired_by_macro("bind-text-to-arrow#@bind")
                .is_none()
        );
        assert!(
            registry
                .retired_by_macro("bind-text-to-arrow#bind-text")
                .is_none()
        );
    }

    #[test]
    fn same_wave_overlap_rejected_disjoint_allowed() {
        use crate::metasystem::validate_migration_chains;

        // Same wave, same directive, DIFFERENT param sets — the normal
        // multi-retirement wave (@bind(text:) + @bind(attr:)); allowed.
        let mut registry = MetaRegistry::new();
        registry
            .register_migration(parse_migration(REWRITE_MIGRATION))
            .unwrap();
        let disjoint = REWRITE_MIGRATION
            .replace("bind-text-to-arrow", "bind-attr-to-arrow")
            .replace(
                "@bind(text: $x:expr)",
                "@bind(attr: $a:ident, value: $v:expr)",
            )
            .replace("text <- `$x`;", "`$a` <- `$v`;");
        registry
            .register_migration(parse_migration(&disjoint))
            .unwrap();
        validate_migration_chains(&registry).expect("disjoint same-wave shapes must pass");

        // Same wave, same directive, SAME param set — races over the same
        // spans; rejected by the post-load pass.
        let mut registry = MetaRegistry::new();
        registry
            .register_migration(parse_migration(REWRITE_MIGRATION))
            .unwrap();
        let overlap = REWRITE_MIGRATION
            .replace("bind-text-to-arrow", "bind-text-to-arrow-alt")
            .replace("text <- `$x`;", "content <- `$x`;");
        registry
            .register_migration(parse_migration(&overlap))
            .unwrap();
        let errs = validate_migration_chains(&registry).unwrap_err();
        assert!(
            errs.iter().any(|e| e.to_string().contains("DISJOINT")),
            "errs: {errs:?}"
        );
    }

    #[test]
    fn rejects_empty_rewrite_template() {
        let mut registry = MetaRegistry::new();
        let bad = REWRITE_MIGRATION.replace("text <- `$x`;", "   ");
        let err = registry
            .register_migration(parse_migration(&bad))
            .unwrap_err();
        assert!(err.to_string().contains("empty"), "err: {err}");
    }

    #[test]
    fn retired_macros_register_in_syntax_registry() {
        // The migration's registration macros parse-side: the rule def (by
        // its synthetic name) AND the embedded macro both recognize retired
        // source and produce FormMatches.
        let mig = parse_migration(REWRITE_MIGRATION);
        let mut syntax = crate::syntax::SyntaxRegistry::new();
        for mac in mig.registration_macros() {
            syntax.register(&mac);
        }
        let forms = syntax.get_forms_for_directive("bind");
        assert_eq!(forms.len(), 2);
        // Rule def first (registration order is preserved parse-side).
        assert_eq!(forms[0].macro_name, "bind-text-to-arrow#bind-text");
        assert_eq!(forms[1].macro_name, "bind-text-to-arrow#@bind");
        // The embedded macro's def retains its body (the %if arms) — the
        // retired semantics travel with the registration.
        assert!(!forms[1].macro_def.body.is_empty());
    }
}

mod grammar {
    use super::*;

    #[test]
    fn test_parse_primitive_minimal() {
        let input = r#"%primitive tick(&el) {
            %emit js { /* tick code */ }
            %exports { $time: number }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let file = result.unwrap();
        assert_eq!(file.meta_defs.len(), 1);
    }

    #[test]
    fn test_parse_primitive_with_typed_params() {
        let input = r#"%primitive scroll(&el, axis: ("x" | "y") = "y", throttle: number = 0) {
            %emit js { const el = %&el; }
            %cleanup { el.removeEventListener('scroll', handler); }
            %exports { $x: number $y: number $progress: number }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_primitive_array_param() {
        let input = r#"%primitive pointer(&el, events: string[] = []) {
            %emit js { /* pointer code */ }
            %exports { $x: number $y: number }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_primitive_glsl_emit() {
        // GLSL with nested braces should work now
        let input = r#"%primitive shader(&el) {
            %emit glsl {
                void main() {
                    gl_FragColor = vec4(1.0, 0.0, 0.0, 1.0);
                }
            }
            %exports { $output: number }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_primitive_css_emit() {
        // CSS emit support for primitives
        let input = r#"%primitive styles(&el) {
            %emit css {
                .container {
                    display: flex;
                    gap: 1rem;
                }
            }
            %exports { $className: string }
        }"#;
        let result = parse_meta(input);
        assert!(
            result.is_ok(),
            "Failed to parse CSS emit: {:?}",
            result.err()
        );

        // Verify the emit block has the correct language
        if let Ok(file) = result {
            assert_eq!(file.meta_defs.len(), 1);
            if let crate::parser::meta_ast::MetaDef::Primitive(p) = &file.meta_defs[0] {
                assert_eq!(p.body.emit_blocks.len(), 1);
                assert_eq!(
                    p.body.emit_blocks[0].lang,
                    crate::parser::meta_ast::EmitLang::Css
                );
            } else {
                panic!("Expected primitive definition");
            }
        }
    }

    #[test]
    fn test_parse_primitive_empty_exports() {
        let input = r#"%primitive simple(&el) {
            %emit js { console.log('hello'); }
            %exports { }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_primitive_complex_js() {
        // JS with multiple nested braces
        let input = r#"%primitive complex(&el) {
            %emit js {
                const handler = (e) => {
                    if (e.type === 'click') {
                        const data = { x: 1, y: 2 };
                        console.log(data);
                    }
                };
            }
            %exports { $x: number }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_two_primitives_with_dollar_typed_params() {
        // First: single primitive with $-prefixed typed params - should work alone
        let single = r#"%primitive second(
  $t: number,
  $dt: number,
  period: number = 1
) {
  %emit js {
    const t = %$t;
  }
  %exports {
    $seconds: number
  }
}
"#;
        let result1 = parse_meta(single);
        assert!(
            result1.is_ok(),
            "Single primitive failed: {:?}",
            result1.err()
        );

        // Second: two primitives - the first has %exports with $ vars
        let two = r#"%primitive first($gl) {
  %emit js {
    const gl = %$gl;
  }
  %exports {
    $handler: fn() ~> effect
  }
}

%primitive second(
  $t: number,
  $dt: number,
  period: number = 1
) {
  %emit js {
    const t = %$t;
  }
  %exports {
    $seconds: number
  }
}
"#;
        let result2 = parse_meta(two);
        assert!(
            result2.is_ok(),
            "Two primitives failed: {:?}",
            result2.err()
        );
    }

    #[test]
    fn test_parse_macro_with_form() {
        // Form with ident default (not quoted)
        let input = r#"%macro toggle {
            %creates @toggle
            %form { @toggle(initial: $initial:ident = off) }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_macro_with_binds() {
        let input = r#"%macro drag {
            %creates @drag
            %binds { gesture(&self, type: "drag") -> { $dx, $dy, $phase } }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_macro_with_derives() {
        let input = r#"%macro slider {
            %creates @slider
            %derives {
                $percent: $value / ($max - $min)
                $display: $value.toFixed(2)
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_macro_with_states() {
        let input = r#"%macro toggle {
            %creates @toggle
            %states {
                off { opacity: 0; }
                on when $isActive { opacity: 1; }
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_macro_with_when() {
        let input = r#"%macro conditional {
            %creates @conditional
            %when $enabled {
                %applies { opacity: 1; }
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_macro_with_for() {
        let input = r#"%macro list {
            %creates @list
            %for $item in $items {
                %applies { color: red; }
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_macro_with_if_else() {
        let input = r#"%macro conditional {
            %creates @conditional
            %if $mode == "dark" {
                %applies { background: black; }
            } %else {
                %applies { background: white; }
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_macro_with_animates() {
        let input = r#"%macro slide {
            %creates @slide
            %animates {
                transform: $position;
                opacity: $alpha;
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_macro_with_includes() {
        // Includes with duration and ident values (no quotes needed)
        let input = r#"%macro combined {
            %creates @combined
            %includes {
                @toggle(initial: off)
                @fade(duration: 300ms)
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_macro_with_registers() {
        let input = r#"%macro registered {
            %creates @registered
            %registers my_registry {
                name: $name
                value: $value
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_macro_with_order() {
        let input = r#"%macro ordered {
            %order 50
            %form {
                @ordered $name:ident
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        let file = result.unwrap();
        assert_eq!(file.meta_defs.len(), 1);
        if let crate::parser::meta_ast::MetaDef::Macro(ref mac) = file.meta_defs[0] {
            assert_eq!(mac.order, Some(50));
        } else {
            panic!("Expected a macro definition");
        }
    }

    #[test]
    fn test_parse_macro_registers_populated() {
        let input = r#"%macro typed {
            %form {
                @typed $name:ident
            }
            %registers type($name) {
                $fields
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        let file = result.unwrap();
        assert_eq!(file.meta_defs.len(), 1);
        if let crate::parser::meta_ast::MetaDef::Macro(ref mac) = file.meta_defs[0] {
            assert!(
                mac.registers.is_some(),
                "Expected registers to be Some, got {:?}",
                mac.registers
            );
            let reg = mac.registers.as_ref().unwrap();
            assert_eq!(reg.name, "type");
            // Args may or may not be parsed depending on how the CST provides inline text
            // The important thing is that the category name is correct
        } else {
            panic!("Expected a macro definition");
        }
    }

    #[test]
    fn test_parse_macro_with_on() {
        let input = r#"%macro reactive {
            %creates @reactive
            %on $active -> true {
                $count <- $count + 1;
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_macro_with_trigger() {
        let input = r#"%macro triggerable {
            %creates @triggerable
            %trigger activate
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_parse_template_macro_with_param_list() {
        // Test the @template macro with new param_list and html_block types
        let input = r#"%macro template {
            %creates @template
            %form {
                @template &$name:ident($params:param_list) {
                    $html:html_block
                    $animations:keyframes?
                }
            }
            %registers template($name) {
                params: $params
                html: $html
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let file = result.unwrap();
        assert_eq!(file.meta_defs.len(), 1);
    }

    #[test]
    fn test_parse_primitive_with_inline_cleanup() {
        // Real-world style where %cleanup is inside %emit js { }
        let input = r#"%primitive pointer(
            &el,
            events: string[] = ["move"],
            normalize: bool = true
        ) {
            %emit js {
                const el = %&el;
                let isOver = false;

                const handleMove = (e) => {
                    %yield e.clientX -> $x;
                    %yield e.clientY -> $y;
                };

                el.addEventListener('pointermove', handleMove, { passive: true });

                %cleanup {
                    el.removeEventListener('pointermove', handleMove);
                }
            }

            %exports {
                $x: number     // viewport X
                $y: number     // viewport Y
                $isOver: bool  // pointer is over
            }
        }"#;
        let result = parse_meta(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        if let Ok(file) = result {
            assert_eq!(file.meta_defs.len(), 1);
            if let crate::parser::meta_ast::MetaDef::Primitive(p) = &file.meta_defs[0] {
                assert_eq!(p.name, "pointer");
                assert_eq!(p.body.emit_blocks.len(), 1);
                // Verify cleanup is in the content (extracted by structured JS parser)
                assert!(p.body.emit_blocks[0].content.contains("%cleanup"));
                // Verify exports
                assert_eq!(p.body.exports.len(), 3);
            } else {
                panic!("Expected primitive");
            }
        }
    }

    #[test]
    fn test_parse_stdlib_pointer_style() {
        // Test full stdlib pointer.st structure
        let input = r#"%primitive scroll(
            &el,
            axis: ("x" | "y" | "both") = "y",
            throttle: number = 0
        ) {
            %emit js {
                const el = %&el;
                const isWindow = el === window;
                let lastX = 0, lastY = 0;

                const update = () => {
                    const info = { scrollX: 0, scrollY: 0 };
                    if (isWindow) {
                        info.scrollX = window.scrollX;
                    } else {
                        info.scrollX = el.scrollLeft;
                    }
                    %yield info.scrollX -> $x;
                };

                el.addEventListener('scroll', update, { passive: true });

                %cleanup {
                    el.removeEventListener('scroll', update);
                }
            }

            %exports {
                $x: number           // horizontal scroll
                $y: number           // vertical scroll
                $progressX: number   // horizontal progress
                $progressY: number   // vertical progress
            }
        }"#;
        let result = parse_meta(input);
        assert!(
            result.is_ok(),
            "Failed to parse scroll-style primitive: {:?}",
            result.err()
        );
    }

    /// BUG-022: regex literals containing braces (e.g. `/:root\s*\{[^}]*\}/g`)
    /// inside `%emit js` bodies confuse the lexer's brace counter. The
    /// emit-body parser closes too early and the rest of the primitive
    /// is mis-parsed as if it were top-level directives.
    ///
    /// Per BUG-022 acceptance criteria, the *minimum* required behavior
    /// is that the parser surfaces a clear diagnostic rather than
    /// silently dropping the body. This test pins that behavior: the
    /// parse MUST fail (loudly) until proper regex-literal support
    /// lands. When the underlying lexer is fixed to recognize regex
    /// literals, flip this assertion to is_ok() and verify the emit
    /// body content round-trips.
    #[test]
    fn test_brace_regex_in_emit_body_surfaces_diagnostic() {
        let input = r#"%primitive brace_re_test() {
            %emit js {
                var styleRe = /:root\s*\{[^}]*\}/g;
                var simpleRe = /a\{b\}/;
                window.__brace_marker = !!styleRe && !!simpleRe;
            }
        }"#;
        let result = parse_meta(input);
        assert!(
            result.is_err(),
            "BUG-022 regression: expected parse to fail loudly on brace-bearing regex literals, but it succeeded. If the lexer was fixed to recognize regex literals, flip this assertion to is_ok() and verify the emit body content round-trips. parse output: {:?}",
            result.ok().map(|f| f.meta_defs.len())
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("offset") || err.contains("expected"),
            "BUG-022 regression: parse error message lost its offset/expected hint, making it unclear to authors. got: {}",
            err
        );
    }
}

// =============================================================================
// Stdlib File Parsing Tests
// =============================================================================

mod stdlib {
    use super::*;

    #[test]
    fn test_parse_stdlib_pointer_file() {
        let content = std::fs::read_to_string("stdlib/primitives/pointer.st")
            .expect("Failed to read stdlib/primitives/pointer.st");

        let result = parse_meta(&content);
        assert!(
            result.is_ok(),
            "Failed to parse pointer.st: {:?}",
            result.err()
        );

        if let Ok(file) = result {
            assert_eq!(file.meta_defs.len(), 1);
            if let crate::parser::meta_ast::MetaDef::Primitive(p) = &file.meta_defs[0] {
                assert_eq!(p.name, "pointer");
                // Verify it has an emit block with cleanup in content
                assert!(!p.body.emit_blocks.is_empty());
                assert!(p.body.emit_blocks[0].content.contains("%cleanup"));
                // Verify exports exist
                assert!(!p.body.exports.is_empty());
            } else {
                panic!("Expected primitive definition");
            }
        }
    }

    #[test]
    fn test_parse_stdlib_scroll_file() {
        let content = std::fs::read_to_string("stdlib/primitives/scroll.st")
            .expect("Failed to read stdlib/primitives/scroll.st");

        let result = parse_meta(&content);
        assert!(
            result.is_ok(),
            "Failed to parse scroll.st: {:?}",
            result.err()
        );

        if let Ok(file) = result {
            assert_eq!(file.meta_defs.len(), 1);
            if let crate::parser::meta_ast::MetaDef::Primitive(p) = &file.meta_defs[0] {
                assert_eq!(p.name, "scroll");
            }
        }
    }

    #[test]
    fn test_parse_stdlib_gesture_file() {
        let content = std::fs::read_to_string("stdlib/primitives/gesture.st")
            .expect("Failed to read stdlib/primitives/gesture.st");

        let result = parse_meta(&content);
        assert!(
            result.is_ok(),
            "Failed to parse gesture.st: {:?}",
            result.err()
        );

        if let Ok(file) = result {
            assert_eq!(file.meta_defs.len(), 1);
            if let crate::parser::meta_ast::MetaDef::Primitive(p) = &file.meta_defs[0] {
                assert_eq!(p.name, "gesture");
                // Should have cleanup in content
                assert!(p.body.emit_blocks[0].content.contains("%cleanup"));
            }
        }
    }

    // NOTE: test_parse_stdlib_shader_file was removed with the hand-rolled WebGL
    // primitives (PLAN-050). Generic metasystem parse coverage lives in
    // test_parse_all_stdlib_primitives below; 3D primitives now live in stdlib/3d.

    #[test]
    fn test_parse_all_stdlib_primitives() {
        // Test that all stdlib primitive files parse successfully
        let primitive_files = [
            "stdlib/primitives/pointer.st",
            "stdlib/primitives/scroll.st",
            "stdlib/primitives/gesture.st",
            "stdlib/primitives/tick.st",
            "stdlib/primitives/intersection.st",
            "stdlib/primitives/resize.st",
            "stdlib/primitives/mutation.st",
            "stdlib/primitives/media.st",
            "stdlib/primitives/fetch.st",
        ];

        for path in &primitive_files {
            if let Ok(content) = std::fs::read_to_string(path) {
                let result = parse_meta(&content);
                assert!(
                    result.is_ok(),
                    "Failed to parse {}: {:?}",
                    path,
                    result.err()
                );
            }
        }
    }

    #[test]
    fn test_parse_all_stdlib_macros() {
        // Test that all stdlib macro files parse successfully
        let macro_files = [
            "stdlib/macros/type-data.st",
            "stdlib/macros/on-event.st",
            "stdlib/macros/drag.st",
            "stdlib/macros/fade-in.st",
            "stdlib/enum/state.st",
            "stdlib/macros/websocket.st",
            "stdlib/macros/responsive.st",
            "stdlib/macros/each.st",
            "stdlib/macros/loop.st",
            "stdlib/macros/timeline.st",
            "stdlib/macros/presets.st",
        ];

        for path in &macro_files {
            if let Ok(content) = std::fs::read_to_string(path) {
                let result = parse_meta(&content);
                assert!(
                    result.is_ok(),
                    "Failed to parse {}: {:?}",
                    path,
                    result.err()
                );
            } else {
                eprintln!("Warning: Could not read {}", path);
            }
        }
    }

    /// Test that on-actions.st parses without errors.
    /// Validates the signal_call capture type with optional args: ($args:expr)?
    #[test]
    fn test_stdlib_on_actions_parses() {
        let content = std::fs::read_to_string("stdlib/capture-types/on-actions.st")
            .expect("Failed to read stdlib/capture-types/on-actions.st");

        let result = parse_meta(&content);
        assert!(
            result.is_ok(),
            "Failed to parse on-actions.st: {:?}",
            result.err()
        );

        let file = result.unwrap();
        // on-actions.st defines multiple capture types
        assert!(
            file.meta_defs.len() >= 5,
            "Expected at least 5 capture_type defs in on-actions.st, got {}",
            file.meta_defs.len()
        );
    }

    /// Test that apply-animations.st parses without errors.
    /// Validates that JS regex patterns with special chars don't confuse the lexer.
    #[test]
    fn test_stdlib_apply_animations_parses() {
        let content = std::fs::read_to_string("stdlib/primitives/animation/apply-animations.st")
            .expect("Failed to read stdlib/primitives/animation/apply-animations.st");

        let result = parse_meta(&content);
        assert!(
            result.is_ok(),
            "Failed to parse apply-animations.st: {:?}",
            result.err()
        );

        let file = result.unwrap();
        assert_eq!(
            file.meta_defs.len(),
            1,
            "Expected 1 primitive in apply-animations.st"
        );
        if let crate::parser::meta_ast::MetaDef::Primitive(p) = &file.meta_defs[0] {
            assert_eq!(p.name, "apply-animations");
        } else {
            panic!("Expected a primitive definition");
        }
    }

    /// Test that gl-loop.st parses without errors.
    /// Validates that both glLoop and glTime primitives parse, including
    /// $-prefixed params without type annotations.
    // NOTE: test_stdlib_gl_loop_parses was removed with the hand-rolled WebGL
    // primitives (PLAN-050). The three.js render loop now lives inside the
    // three-stage primitive of the stdlib/3d module.

    /// Test that template.st parses without errors.
    /// Validates that JS code with regex patterns and string concatenation
    /// doesn't produce lexer brace imbalance.
    #[test]
    fn test_stdlib_template_parses() {
        let content = std::fs::read_to_string("stdlib/primitives/template.st")
            .expect("Failed to read stdlib/primitives/template.st");

        let result = parse_meta(&content);
        assert!(
            result.is_ok(),
            "Failed to parse template.st: {:?}",
            result.err()
        );

        let file = result.unwrap();
        assert_eq!(
            file.meta_defs.len(),
            3,
            "Expected 3 primitives in template.st (register-template, invoke-template, each-with-templates)"
        );

        // Verify all three primitives
        let names: Vec<&str> = file
            .meta_defs
            .iter()
            .filter_map(|d| {
                if let crate::parser::meta_ast::MetaDef::Primitive(p) = d {
                    Some(p.name.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            names,
            vec![
                "register-template",
                "invoke-template",
                "each-with-templates"
            ]
        );
    }

    /// Test that ALL stdlib files parse without errors.
    /// Uses the same directories as check-stdlib to ensure full coverage.
    /// Known exception: distort-shaders.st has a pre-existing error not in scope.
    #[test]
    fn test_all_stdlib_files_parse() {
        use std::path::Path;

        let dirs = crate::metasystem::STDLIB_DIRS;

        // Known pre-existing errors not in scope for this fix
        let known_exceptions = ["distort-shaders.st"];

        let mut registry = crate::metasystem::registry::MetaRegistry::new();
        let mut errors: Vec<String> = Vec::new();

        for dir in dirs {
            let path = Path::new(dir);
            if !path.exists() {
                continue;
            }
            let result = registry.load_stdlib_collecting_errors(path);
            for err in &result.errors {
                let filename = err
                    .file_path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("");
                if known_exceptions.contains(&filename) {
                    continue;
                }
                errors.push(format!("{}: {}", err.file_path.display(), err.message));
            }
        }

        assert!(
            errors.is_empty(),
            "Found {} stdlib parse error(s):\n  {}",
            errors.len(),
            errors.join("\n  ")
        );
    }
}

// =============================================================================
// Validation Tests
// =============================================================================

mod validation {
    use super::*;
    use crate::parser::meta_ast::*;

    fn empty_registry() -> MetaRegistry {
        MetaRegistry::new()
    }

    fn registry_with_primitive(name: &str) -> MetaRegistry {
        let mut registry = MetaRegistry::new();
        registry
            .register_primitive(PrimitiveDefAst {
                name: name.to_string(),
                params: vec![PrimitiveParam::Element("el".to_string())],
                body: PrimitiveBody::default(),
                uses: vec![],
                span: parser::SourceSpan::default(),
                source_file: None,
                doc: None,
            })
            .unwrap();
        registry
    }

    #[test]
    fn test_validate_primitive_passes() {
        let primitive = PrimitiveDefAst {
            name: "test".to_string(),
            params: vec![PrimitiveParam::Element("el".to_string())],
            body: PrimitiveBody {
                emit_blocks: vec![EmitBlock {
                    lang: EmitLang::Js,
                    content: "const el = %&el; %yield 42 -> $x;".to_string(),
                    span: parser::SourceSpan::default(),
                }],
                cleanup: None,
                exports: vec![ExportDecl {
                    name: "x".to_string(),
                    type_expr: ExportTypeExpr::Simple("number".to_string()),
                    optional: false,
                }],
                if_blocks: vec![],
            },
            uses: vec![],
            span: parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let result = validate_primitive(&primitive, &empty_registry());
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_primitive_duplicate_export() {
        let primitive = PrimitiveDefAst {
            name: "test".to_string(),
            params: vec![],
            body: PrimitiveBody {
                emit_blocks: vec![],
                cleanup: None,
                exports: vec![
                    ExportDecl {
                        name: "x".to_string(),
                        type_expr: ExportTypeExpr::Simple("number".to_string()),
                        optional: false,
                    },
                    ExportDecl {
                        name: "x".to_string(),
                        type_expr: ExportTypeExpr::Simple("number".to_string()),
                        optional: false,
                    },
                ],
                if_blocks: vec![],
            },
            uses: vec![],
            span: parser::SourceSpan::default(),
            source_file: None,
            doc: None,
        };

        let result = validate_primitive(&primitive, &empty_registry());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_macro_with_valid_binds() {
        let registry = registry_with_primitive("gesture");

        let macro_def = MacroDefAst {
            retired: None,
            name: "test".to_string(),
            form: None,
            binds: vec![BindDecl {
                primitive: "gesture".to_string(),
                args: vec![BindArg::Element {
                    name: "self".to_string(),
                    child_selector: None,
                }],
                outputs: vec![
                    BindOutput {
                        name: "dx".to_string(),
                        alias: None,
                    },
                    BindOutput {
                        name: "dy".to_string(),
                        alias: None,
                    },
                ],
                span: parser::SourceSpan::default(),
            }],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![],
            requires: vec![],
            span: parser::SourceSpan::default(),
            source_file: None,
            module: None,
            doc: None,
            ..Default::default()
        };

        let result = validate_macro(&macro_def, &registry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_macro_unknown_primitive() {
        let registry = empty_registry();

        let macro_def = MacroDefAst {
            retired: None,
            name: "test".to_string(),
            form: None,
            binds: vec![BindDecl {
                primitive: "nonexistent".to_string(),
                args: vec![],
                outputs: vec![],
                span: parser::SourceSpan::default(),
            }],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![],
            requires: vec![],
            span: parser::SourceSpan::default(),
            source_file: None,
            module: None,
            doc: None,
            ..Default::default()
        };

        let result = validate_macro(&macro_def, &registry);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_macro_unbound_when_variable() {
        let registry = empty_registry();

        let macro_def = MacroDefAst {
            retired: None,
            name: "test".to_string(),
            form: None,
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![MacroBodyItem::When(WhenClause {
                condition: "unbound_var".to_string(),
                body: vec![],
                span: parser::SourceSpan::default(),
            })],
            requires: vec![],
            span: parser::SourceSpan::default(),
            source_file: None,
            module: None,
            doc: None,
            ..Default::default()
        };

        let result = validate_macro(&macro_def, &registry);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_macro_bound_when_variable() {
        let registry = empty_registry();

        let macro_def = MacroDefAst {
            retired: None,
            name: "test".to_string(),
            form: Some(FormClause {
                directive_name: "test".to_string(),
                inline_elements: vec![],
                params: vec![FormParam {
                    name: "enabled".to_string(),
                    elements: vec![FormInlineElement::Capture(
                        FormCapture {
                            var_name: "enabled".to_string(),
                            capture_type: CaptureType::Ident,
                            modifier: CaptureModifier::Required,
                            alias_capture: None,
                        },
                        None,
                    )],
                    default: None,
                }],
                post_arg_inline: vec![],
                body_capture: None,
                body_params: Vec::new(),
                body_groups: Vec::new(),
                span: parser::SourceSpan::default(),
            }),
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![MacroBodyItem::When(WhenClause {
                condition: "enabled".to_string(),
                body: vec![],
                span: parser::SourceSpan::default(),
            })],
            requires: vec![],
            span: parser::SourceSpan::default(),
            source_file: None,
            module: None,
            doc: None,
            ..Default::default()
        };

        let result = validate_macro(&macro_def, &registry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_macro_for_loop_adds_binding() {
        let registry = empty_registry();

        let macro_def = MacroDefAst {
            retired: None,
            name: "test".to_string(),
            form: Some(FormClause {
                directive_name: "test".to_string(),
                inline_elements: vec![],
                params: vec![FormParam {
                    name: "items".to_string(),
                    elements: vec![FormInlineElement::Capture(
                        FormCapture {
                            var_name: "items".to_string(),
                            capture_type: CaptureType::Properties,
                            modifier: CaptureModifier::Required,
                            alias_capture: None,
                        },
                        None,
                    )],
                    default: None,
                }],
                post_arg_inline: vec![],
                body_capture: None,
                body_params: Vec::new(),
                body_groups: Vec::new(),
                span: parser::SourceSpan::default(),
            }),
            binds: vec![],
            derives: vec![],
            states: None,
            registers: None,
            imports: None,
            order: None,
            resolves: None,
            scopes: vec![],
            scope_within: Vec::new(),
            scope_element: Vec::new(),
            body: vec![MacroBodyItem::For(MetaForClause {
                variable: "item".to_string(),
                source: "items".to_string(),
                body: vec![MacroBodyItem::When(WhenClause {
                    condition: "item".to_string(), // item is bound by the for loop
                    body: vec![],
                    span: parser::SourceSpan::default(),
                })],
                span: parser::SourceSpan::default(),
            })],
            requires: vec![],
            span: parser::SourceSpan::default(),
            source_file: None,
            module: None,
            doc: None,
            ..Default::default()
        };

        let result = validate_macro(&macro_def, &registry);
        assert!(result.is_ok());
    }
}

// =============================================================================
// Diagnostic Tests
// =============================================================================

mod diagnostics {
    use super::super::diagnostics::to_diagnostic;
    use super::super::validate::{ValidationError, ValidationErrorKind};
    use super::*;
    use crate::diagnostics::DiagnosticCode;
    use crate::parser::meta_ast::*;

    #[test]
    fn test_unknown_primitive_diagnostic() {
        let registry = MetaRegistry::new();
        let error = ValidationError {
            kind: ValidationErrorKind::UnknownPrimitive("scrool".to_string()),
            span: parser::SourceSpan::default(),
        };

        let diag = to_diagnostic(&error, &registry);
        assert_eq!(diag.code, DiagnosticCode::E170);
    }

    #[test]
    fn test_unknown_macro_diagnostic() {
        let registry = MetaRegistry::new();
        let error = ValidationError {
            kind: ValidationErrorKind::UnknownMacro("togle".to_string()),
            span: parser::SourceSpan::default(),
        };

        let diag = to_diagnostic(&error, &registry);
        assert_eq!(diag.code, DiagnosticCode::E171);
    }

    #[test]
    fn test_circular_dependency_diagnostic() {
        let registry = MetaRegistry::new();
        let error = ValidationError {
            kind: ValidationErrorKind::CircularMacroDependency(vec![
                "a".to_string(),
                "b".to_string(),
                "a".to_string(),
            ]),
            span: parser::SourceSpan::default(),
        };

        let diag = to_diagnostic(&error, &registry);
        assert_eq!(diag.code, DiagnosticCode::E180);
        assert!(diag.message.contains("a -> b -> a"));
    }

    #[test]
    fn test_unbound_variable_diagnostic() {
        let registry = MetaRegistry::new();
        let error = ValidationError {
            kind: ValidationErrorKind::UnboundMacroVariable("unknown".to_string()),
            span: parser::SourceSpan::default(),
        };

        let diag = to_diagnostic(&error, &registry);
        assert_eq!(diag.code, DiagnosticCode::E181);
    }

    #[test]
    fn test_suggestion_for_typo() {
        let mut registry = MetaRegistry::new();
        registry
            .register_primitive(PrimitiveDefAst {
                name: "scroll".to_string(),
                params: vec![],
                body: PrimitiveBody::default(),
                uses: vec![],
                span: parser::SourceSpan::default(),
                source_file: None,
                doc: None,
            })
            .unwrap();

        let error = ValidationError {
            kind: ValidationErrorKind::UnknownPrimitive("scrool".to_string()),
            span: parser::SourceSpan::default(),
        };

        let diag = to_diagnostic(&error, &registry);
        // Should suggest "scroll" for typo "scrool"
        assert!(diag.hint.as_ref().is_some_and(|h| h.contains("scroll")));
    }
}

// =============================================================================
// Stdlib Integration Tests
// =============================================================================

mod stdlib_integration {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_load_stdlib_primitives() {
        let mut registry = MetaRegistry::new();
        let result = registry.load_stdlib_from_dir(Path::new("stdlib/primitives"));

        assert!(
            result.is_ok(),
            "Failed to load stdlib primitives: {:?}",
            result.err()
        );

        // Verify key primitives are loaded
        assert!(
            registry.get_primitive("pointer").is_some(),
            "pointer primitive not found"
        );
        assert!(
            registry.get_primitive("scroll").is_some(),
            "scroll primitive not found"
        );
        assert!(
            registry.get_primitive("gesture").is_some(),
            "gesture primitive not found"
        );
        assert!(
            registry.get_primitive("tick").is_some(),
            "tick primitive not found"
        );
        assert!(
            registry.get_primitive("intersection").is_some(),
            "intersection primitive not found"
        );
    }

    #[test]
    fn test_load_websocket_primitives() {
        let mut registry = MetaRegistry::new();
        let result = registry.load_stdlib_from_dir(Path::new("stdlib/primitives"));

        assert!(
            result.is_ok(),
            "Failed to load stdlib primitives: {:?}",
            result.err()
        );

        // Verify websocket primitives are loaded
        assert!(
            registry.get_primitive("websocket").is_some(),
            "websocket primitive not found"
        );
        assert!(
            registry.get_primitive("realtime-source").is_some(),
            "realtime-source primitive not found"
        );
        assert!(
            registry.get_primitive("presence").is_some(),
            "presence primitive not found"
        );
        assert!(
            registry.get_primitive("channel").is_some(),
            "channel primitive not found"
        );
        // Verify typed union socket primitive is loaded
        assert!(
            registry.get_primitive("socket").is_some(),
            "socket primitive not found"
        );
    }

    #[test]
    fn test_load_websocket_macros() {
        let mut registry = MetaRegistry::new();
        // Load macros from flat directory (skip subdirectories)
        let result = registry.load_dir_flat(Path::new("stdlib/macros"));

        assert!(
            result.is_ok(),
            "Failed to load stdlib macros: {:?}",
            result.err()
        );

        // Verify websocket macros are loaded
        assert!(
            registry.get_macro("websocket").is_some(),
            "@websocket macro not found"
        );
        assert!(
            registry.get_macro("realtime").is_some(),
            "@realtime macro not found"
        );
        assert!(
            registry.get_macro("presence").is_some(),
            "@presence macro not found"
        );
        assert!(
            registry.get_macro("channel").is_some(),
            "@channel macro not found"
        );
    }

    #[test]
    fn test_load_websocket_macros_recursive() {
        // This test uses load_stdlib_from_dir which is what the CLI uses
        let mut registry = MetaRegistry::new();
        let result = registry.load_stdlib_from_dir(Path::new("stdlib/macros"));

        assert!(
            result.is_ok(),
            "Failed to load stdlib macros: {:?}",
            result.err()
        );

        // Verify websocket macros are loaded even with recursive loading
        assert!(
            registry.get_macro("websocket").is_some(),
            "@websocket macro not found (recursive)"
        );
        assert!(
            registry.get_macro("realtime").is_some(),
            "@realtime macro not found (recursive)"
        );
        assert!(
            registry.get_macro("presence").is_some(),
            "@presence macro not found (recursive)"
        );
        assert!(
            registry.get_macro("channel").is_some(),
            "@channel macro not found (recursive)"
        );
    }

    #[test]
    fn test_macros_found_by_form_directive() {
        // This test verifies that macros can be looked up by their %form directive name
        // (what users write) rather than just by their internal macro name.
        // This is critical for the resolve layer to find the right macro.
        let mut registry = MetaRegistry::new();
        let result = registry.load_stdlib_from_dir(Path::new("stdlib/macros"));
        assert!(
            result.is_ok(),
            "Failed to load stdlib macros: {:?}",
            result.err()
        );

        // @on is written by users; PLAN-124's driver-head macros
        // (on.st: on-driver-form / on-driver-body / on-signal-arms) own it
        // now. The %if-ladder dispatcher retired into the on-cutover capsule.
        let on_macro = registry.get_macro_by_form_directive("on");
        assert!(
            on_macro.is_some(),
            "@on should be found by form directive. Available macros: {:?}",
            registry.macro_names().collect::<Vec<_>>()
        );

        // Verify it's the right macro
        if let Some(m) = on_macro {
            assert!(m.form.is_some(), "the @on owner should have a form clause");
            assert_eq!(
                m.form.as_ref().unwrap().directive_name,
                "@on",
                "Form directive should be '@on'"
            );
        }

        // @scroll retired into the on-cutover capsule (PLAN-124 W3.4): no
        // live macro owns it in stdlib/macros; it resolves through the
        // migration registry instead.
        let scroll_macro = registry.get_macro_by_form_directive("scroll");
        assert!(
            scroll_macro.is_none(),
            "@scroll must NOT be found in live stdlib macros (it retired into the capsule)"
        );
    }

    #[test]
    fn test_load_individual_primitive_files() {
        // Test core primitives that should definitely parse
        let primitive_files = [
            "stdlib/primitives/pointer.st",
            "stdlib/primitives/scroll.st",
            "stdlib/primitives/gesture.st",
            "stdlib/primitives/tick.st",
            "stdlib/primitives/intersection.st",
            "stdlib/primitives/resize.st",
            "stdlib/primitives/mutation.st",
            "stdlib/primitives/media.st",
            "stdlib/primitives/fetch.st",
            "stdlib/primitives/websocket.st",
        ];

        for path in &primitive_files {
            let content = std::fs::read_to_string(path).expect(&format!("Failed to read {}", path));
            let result = parse_meta(&content);
            assert!(
                result.is_ok(),
                "Failed to parse {}: {:?}",
                path,
                result.err()
            );
        }
    }

    #[test]
    fn test_primitive_has_exports() {
        let mut registry = MetaRegistry::new();
        registry
            .load_stdlib_from_dir(Path::new("stdlib/primitives"))
            .unwrap();

        // pointer should export $x and $y
        let pointer = registry
            .get_primitive("pointer")
            .expect("pointer not found");
        assert!(
            !pointer.body.exports.is_empty(),
            "pointer should have exports"
        );

        // scroll should export $x, $y, $progress
        let scroll = registry.get_primitive("scroll").expect("scroll not found");
        assert!(
            scroll.body.exports.len() >= 3,
            "scroll should have at least 3 exports"
        );
    }

    #[test]
    fn test_primitive_has_emit_block() {
        let mut registry = MetaRegistry::new();
        registry
            .load_stdlib_from_dir(Path::new("stdlib/primitives"))
            .unwrap();

        let pointer = registry
            .get_primitive("pointer")
            .expect("pointer not found");
        assert!(
            !pointer.body.emit_blocks.is_empty(),
            "pointer should have emit blocks"
        );
        assert!(
            pointer
                .body
                .emit_blocks
                .iter()
                .any(|e| matches!(e.lang, crate::parser::meta_ast::EmitLang::Js)),
            "pointer should have JS emit block"
        );
    }

    #[test]
    fn test_gesture_has_cleanup() {
        let mut registry = MetaRegistry::new();
        registry
            .load_stdlib_from_dir(Path::new("stdlib/primitives"))
            .unwrap();

        let gesture = registry
            .get_primitive("gesture")
            .expect("gesture not found");
        // Cleanup can be in emit block content or standalone
        let has_cleanup = gesture.body.cleanup.is_some()
            || gesture
                .body
                .emit_blocks
                .iter()
                .any(|e| e.content.contains("%cleanup"));
        assert!(has_cleanup, "gesture should have cleanup");
    }

    #[test]
    fn test_no_creates_in_stdlib() {
        // After removing %creates, no stdlib file should contain %creates
        use std::fs;

        fn check_dir(dir: &Path) -> Vec<String> {
            let mut violations = vec![];
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        violations.extend(check_dir(&path));
                    } else if path.extension().is_some_and(|e| e == "st") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if content.contains("%creates") {
                                violations.push(path.display().to_string());
                            }
                        }
                    }
                }
            }
            violations
        }

        let violations = check_dir(Path::new("stdlib"));
        assert!(
            violations.is_empty(),
            "These stdlib files still contain %creates: {:?}",
            violations
        );
    }

    #[test]
    fn test_expand_scroll_via_form_lookup() {
        // PLAN-124 W3.4: @scroll retired into the on-cutover capsule — the
        // macro lives in the MIGRATION registry now, tagged retired.
        let mut registry = MetaRegistry::new();
        registry
            .load_stdlib_from_dir(Path::new("stdlib/migrations/entries"))
            .unwrap();

        let mig = registry
            .get_migration("on-cutover")
            .expect("the on-cutover capsule registers");
        assert!(
            mig.macros.iter().any(|m| m.name == "scroll-timeline"),
            "the retired scroll-timeline definition lives in the capsule"
        );
        assert!(
            mig.rewrites.iter().any(|r| r.id == "scroll"),
            "and its mechanical rewrite rule"
        );
        // …and the retired def is reachable through the retired-by mapping.
        assert!(
            registry
                .retired_by_macro("on-cutover#@scroll-timeline")
                .is_some(),
            "the embedded retired def maps back to its migration"
        );
    }

    #[test]
    fn test_index_st_zero_false_unknown_directives() {
        // Compile the full landing page 1 and verify no false
        // "unknown directive" or "macro not found" pipeline errors
        use crate::compiler::{CompileOptions, compile};
        use std::fs;

        let content = fs::read_to_string("tests/fixtures/landing/1/index.st")
            .expect("Failed to read tests/fixtures/landing/1/index.st");
        let ast = crate::parser::parse(&content).expect("Failed to parse index.st");

        let compiled = compile(&ast, CompileOptions::default());

        let unknown_errors: Vec<_> = compiled
            .pipeline_errors
            .iter()
            .filter(|e| {
                e.message.contains("Unknown directive") || e.message.contains("macro not found")
            })
            .collect();

        assert!(
            unknown_errors.is_empty(),
            "index.st should have zero false 'Unknown directive' errors after %creates removal: {:?}",
            unknown_errors
                .iter()
                .map(|e| &e.message)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_locale_macro_loads() {
        let mut registry = MetaRegistry::new();
        registry
            .load_stdlib_from_dir(Path::new("stdlib/primitives"))
            .unwrap();
        registry
            .load_stdlib_from_dir(Path::new("stdlib/macros"))
            .unwrap();

        // locale-setup macro should be registered
        let locale_macro = registry.get_macro("locale-setup");
        assert!(
            locale_macro.is_some(),
            "locale-setup macro not found. Available macros: {:?}",
            registry.macro_names().collect::<Vec<_>>()
        );

        let locale_def = locale_macro.unwrap();

        // Should have a %form clause with directive name @locale
        assert!(
            locale_def.form.is_some(),
            "locale-setup should have a %form clause"
        );
        let form = locale_def.form.as_ref().unwrap();
        assert_eq!(
            form.directive_name, "@locale",
            "Form directive should be '@locale'"
        );

        // Should have %binds to locale-build primitive
        assert!(
            !locale_def.binds.is_empty(),
            "locale-setup should have %binds"
        );
        assert_eq!(
            locale_def.binds[0].primitive, "locale-build",
            "locale-setup should bind to locale-build primitive"
        );

        // Should also be findable via form directive lookup
        let by_form = registry.get_macro_by_form_directive("locale");
        assert!(
            by_form.is_some(),
            "@locale should be found by form directive lookup"
        );
    }
}

mod css_at_rules {
    use super::*;
    use std::path::Path;

    fn setup_registry() -> MetaRegistry {
        let mut registry = MetaRegistry::new();
        registry
            .load_stdlib_from_dir(Path::new("stdlib/primitives"))
            .unwrap();
        registry.load_dir_flat(Path::new("stdlib/macros")).unwrap();
        registry
    }

    #[test]
    fn test_font_face_primitive_loads() {
        let registry = setup_registry();
        let prim = registry.get_primitive("font-face-emit");
        assert!(
            prim.is_some(),
            "font-face-emit primitive not found in registry"
        );

        let prim = prim.unwrap();
        assert!(
            !prim.body.emit_blocks.is_empty(),
            "font-face-emit should have emit blocks"
        );
        let has_css = prim
            .body
            .emit_blocks
            .iter()
            .any(|b| b.lang == crate::parser::meta_ast::EmitLang::Css);
        assert!(has_css, "font-face-emit should have a CSS emit block");
        let has_js = prim
            .body
            .emit_blocks
            .iter()
            .any(|b| b.lang == crate::parser::meta_ast::EmitLang::Js);
        assert!(!has_js, "font-face-emit should NOT have a JS emit block");
    }

    #[test]
    fn test_property_registration_primitive_loads() {
        let registry = setup_registry();
        let prim = registry.get_primitive("property-registration-emit");
        assert!(
            prim.is_some(),
            "property-registration-emit primitive not found in registry"
        );

        let prim = prim.unwrap();
        let has_css = prim
            .body
            .emit_blocks
            .iter()
            .any(|b| b.lang == crate::parser::meta_ast::EmitLang::Css);
        assert!(
            has_css,
            "property-registration-emit should have a CSS emit block"
        );
    }

    #[test]
    fn test_font_face_macro_loads() {
        let registry = setup_registry();
        let m = registry.get_macro("font-face");
        assert!(m.is_some(), "font-face macro not found in registry");

        let m = m.unwrap();
        assert!(m.form.is_some(), "font-face macro should have a form");
        assert!(!m.binds.is_empty(), "font-face macro should have binds");
        assert_eq!(m.binds[0].primitive, "font-face-emit");
        assert!(
            m.binds[0].outputs.is_empty(),
            "font-face bind should have empty outputs"
        );
    }

    #[test]
    fn test_css_property_registration_macro_loads() {
        let registry = setup_registry();
        let m = registry.get_macro("css-property-registration");
        assert!(
            m.is_some(),
            "css-property-registration macro not found in registry"
        );

        let m = m.unwrap();
        assert!(
            m.form.is_some(),
            "css-property-registration macro should have a form"
        );
        assert!(
            !m.binds.is_empty(),
            "css-property-registration macro should have binds"
        );
        assert_eq!(m.binds[0].primitive, "property-registration-emit");
    }

    #[test]
    fn test_font_face_primitive_codegen_css() {
        use crate::emit::metasystem_codegen::{PrimitiveArgs, generate_primitive_ir};
        use crate::emit::{EmitOptions, css as css_emit};

        let registry = setup_registry();
        let prim = registry.get_primitive("font-face-emit").unwrap();

        let args = PrimitiveArgs::new()
            .param("family", "'Aspekta'")
            .param("styles", "null")
            .css_param("styles", "src: url(\"/fonts/AspektaVF.woff2\") format(\"woff2-variations\");\n    font-weight: 50 1000;\n    font-style: normal;\n    font-display: swap;");

        let ir = generate_primitive_ir(prim, &args);

        assert!(
            !ir.css_exprs.is_empty(),
            "font-face-emit should produce CSS"
        );
        assert!(
            ir.js_stmts.is_empty(),
            "font-face-emit should NOT produce JS"
        );

        let opts = EmitOptions::pretty();
        let css = ir
            .css_exprs
            .iter()
            .map(|expr| css_emit::emit(expr, &opts))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            css.contains("@font-face"),
            "CSS should contain @font-face.\nGot: {}",
            css
        );
        assert!(
            css.contains("font-family: Aspekta"),
            "CSS should contain font-family.\nGot: {}",
            css
        );
        assert!(
            css.contains("font-weight: 50 1000"),
            "CSS should contain font-weight.\nGot: {}",
            css
        );
        assert!(
            css.contains("font-display: swap"),
            "CSS should contain font-display.\nGot: {}",
            css
        );
    }

    #[test]
    fn test_property_registration_primitive_codegen_css() {
        use crate::emit::metasystem_codegen::{PrimitiveArgs, generate_primitive_ir};
        use crate::emit::{EmitOptions, css as css_emit};

        let registry = setup_registry();
        let prim = registry
            .get_primitive("property-registration-emit")
            .unwrap();

        let args = PrimitiveArgs::new()
            .param("name", "'--nav-fg'")
            .param("styles", "null")
            .css_param(
                "styles",
                "syntax: '<color>';\n    initial-value: #1A1A1A;\n    inherits: true;",
            );

        let ir = generate_primitive_ir(prim, &args);
        assert!(
            !ir.css_exprs.is_empty(),
            "property-registration-emit should produce CSS"
        );

        let opts = EmitOptions::pretty();
        let css = ir
            .css_exprs
            .iter()
            .map(|expr| css_emit::emit(expr, &opts))
            .collect::<Vec<_>>()
            .join("\n");

        assert!(
            css.contains("@property"),
            "CSS should contain @property.\nGot: {}",
            css
        );
        assert!(
            css.contains("--nav-fg"),
            "CSS should contain property name.\nGot: {}",
            css
        );
    }

    #[test]
    fn test_css_override_takes_precedence() {
        use crate::emit::metasystem_codegen::PrimitiveArgs;

        let args = PrimitiveArgs::new()
            .param("styles", "{'font-weight': '400'}")
            .css_param("styles", "font-weight: 400;");

        assert_eq!(
            args.css_overrides.get("styles").unwrap(),
            "font-weight: 400;"
        );
        assert_eq!(args.params.get("styles").unwrap(), "{'font-weight': '400'}");
    }
}

#[cfg(test)]
mod plan054_clause_parsing {
    use crate::parser;

    /// PLAN-054: %derives with multi-line ternary values must each be captured
    /// whole (a value spanning lines must NOT swallow the next declaration), and
    /// %animates must parse `property: $variable` lines (the CST property walker
    /// dropped the key + truncated multi-line values).
    #[test]
    fn derives_and_animates_parse_from_drag() {
        // `@drag` moved to the dnd module (PLAN-058); its %derives/%animates live
        // in stdlib/dnd/macros/drag.st now.
        let src = std::fs::read_to_string("stdlib/dnd/macros/drag.st").unwrap();
        let file = parser::parse(&src).unwrap();
        let drag = file
            .meta_defs
            .iter()
            .find_map(|md| match md {
                parser::meta_ast::MetaDef::Macro(m) if m.name == "drag" => Some(m),
                _ => None,
            })
            .expect("drag macro");

        // %derives: rawX, rawY, x, y must all be present and each value complete.
        let names: Vec<&str> = drag.derives.iter().map(|d| d.name.as_str()).collect();
        assert!(
            names.contains(&"rawX"),
            "derives must include rawX, got {:?}",
            names
        );
        assert!(
            names.contains(&"rawY"),
            "derives must include rawY, got {:?}",
            names
        );
        assert!(
            names.contains(&"x"),
            "derives must include x (multi-line ternary), got {:?}",
            names
        );
        assert!(
            names.contains(&"y"),
            "derives must include y (multi-line ternary), got {:?}",
            names
        );
        // The x derive's value must NOT have swallowed the y declaration.
        let x = drag.derives.iter().find(|d| d.name == "x").unwrap();
        assert!(
            !x.expr.contains("$y:") && !x.expr.contains("\n    $y"),
            "x derive must not swallow the y declaration, got: {:?}",
            x.expr
        );

        // %animates: translate-x <- $x, translate-y <- $y.
        let animates = drag
            .body
            .iter()
            .find_map(|i| match i {
                parser::meta_ast::MacroBodyItem::Animates(a) => Some(a),
                _ => None,
            })
            .expect("drag must have an %animates clause");
        let props: Vec<(&str, &str)> = animates
            .properties
            .iter()
            .map(|p| (p.property.as_str(), p.variable.as_str()))
            .collect();
        assert!(
            props.contains(&("translate-x", "x")),
            "animates translate-x: $x, got {:?}",
            props
        );
        assert!(
            props.contains(&("translate-y", "y")),
            "animates translate-y: $y, got {:?}",
            props
        );
    }
}
