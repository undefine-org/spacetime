# Cluster B — the Spell ⟷ Spacetime bridge (deferred until Cluster A done)

> Status: **READY** (Cluster A done — FUP-055/056/057 complete, the verse-side
> namespace backbone ships). This is the executable plan for the Spell-repo
> CONSUMER side. Anchors below were re-validated against `~/code/ora/spell` HEAD
> on 2026-06-15. Full design: `docs/specs/spell-spacetime-unification.md`.
>
> VERSE PRODUCER ARTIFACTS (all shipped, consumed by Cluster B):
> - `spacetime export-spell-profile [-o]` → `ProfileYaml`-shaped JSON (the field
>   names match `pi-code-engine` `ProfileYaml` 1:1: declarations, class_like,
>   imports, exports, references, separators, embedded_regions).
> - `tree-sitter-spacetime/` → grammar.js + `src/node-types.json` (38 named kinds
>   incl. macro_def/primitive_def/capture_type_def, each with a `name` field).
> - `spacetime lsp` (existing) for L1.
> - The namespace `/` separator (FUP-055/057) — `export-spell-profile` emits
>   `separators:["/"]`, so a Spacetime FQN `scene/camera` maps directly onto a
>   Spell CodePath `scene/camera.st::§macro`. THIS is the cross-repo payoff that
>   Cluster A unlocked: qualified `@scene/camera` in verse ≡ `::§` address in Spell.

---

## Why Cluster B exists (the original goal)

Make Spacetime `.st` files first-class in the **Spell** coding harness (outline,
`::§kind` navigation, structural edit, code-graph) — AND generalize so Spell can
onboard ANY language from data, and Spacetime can serve ANY editor/harness. Two
repos, ONE source of truth (the verse `MetaRegistry`).

The verse-side PRODUCER is essentially done this work-cycle:
- `export-spell-profile` emits a Spell `LanguageProfile` JSON from the live
  registry (commit 09e316a2).
- the faithful tree-sitter grammar emits real `macro_def`/`primitive_def`/
  `capture_type_def` nodes + `node-types.json` (commit dfc6bb80).

Cluster B is the **Spell-repo CONSUMER side** — not yet started. It lives in
`~/code/ora/spell/crates/pi-code-*`, NOT in verse.

---

## The seam strategy (how Spell consumes Spacetime)

Three language concepts in Spell are unconnected today and must unify behind ONE:
1. `LanguageProfile`  (pi-code-engine)  — tree-sitter outline/`::§`/edit/graph
2. `LanguageDialect` + `NameLexer` (pi-code-path) — CodePath symbol resolution
3. `SemanticBackend` + `semantic{}` KDL (pi-code-graph) — LSP wiring only

**Seam:** `LanguageProfile` becomes the single registry entry; `dialect`
non-optional; kill pi-kernel's parallel hardcoded ext→dialect map. A language is
then addable AT RUNTIME FROM DATA — Spacetime is just the first non-builtin.

### The layered rollout (each ships value; later layers optional)

```
L0  HYGIENE  (no behavior change)
  · verse: unify the 4 parsers onto one registry (the root cause of the
    "two parse passes" coordination tax — see below).
  · spell: LanguageProfile = single entry; dialect non-optional; delete
    pi-kernel dialect_registry parallel map; port build.rs grammar-gen → runtime
    fn (grammar_from_node_types).

L1  BRIDGE   (spell, config-only — ships value immediately)
  · spell.kdl: languages{ language "spacetime" { lsp command="spacetime"
    args="lsp" } } + a text-class .st profile so files are recognized.
  ⇒ hover/completion/diagnostics/go-to-def for .st NOW. Gap: no ::§kind yet.

L2  PROFILE  (the payoff)
  · spell: revive the DEAD ProfileYaml + load_profile_json
    (pi-code-engine/src/language/profile.rs:483) → register_profile_from_json;
    add DefaultNameLexer { separator } (for /-separated langs);
    add languages{} KDL profile/grammar keys.
  · verse: export-spell-profile (DONE) emits profile.json + node-types.json.
  ⇒ outline + ::§ + structural edit for .st, NO Spell recompile.

L3  WASM     (optional; spiked feasible: 2.45ms load, +6.2MB, feature-gated)
  · spell: LanguageSource::{Static,Wasm} + wasmtime WasmStore
    (feature "wasm-grammars"); grammar_from_node_types at registration.
  · verse: emit spacetime.wasm in export-spell-profile.
  ⇒ genuinely NEW grammars (incl .st) at runtime, zero Spell recompile.

L4  LIVE     first-party MCP integration + Canvas-on-Spacetime vision (§6 of the
             unification spec). Separate product bet.
```

Dependency: L0 ⟂ L1 (parallel). L2 needs L0. L3 needs L2 + the wasmtime spike
greenlight. L4 needs L1.

---

## Spell-side anchors (for when we return to the spell repo)

- `crates/pi-code-engine/src/language/profile.rs:483` — DEAD `ProfileYaml` +
  `:506 load_profile_json` (the seam to revive for L2).
- `crates/pi-code-engine/src/language/mod.rs:48 with_builtins` · `:66 register`
  (public, never called with a runtime profile) · `:87 match_path`.
- `crates/pi-code-engine/build.rs:410 generate_grammar_module` (port → runtime
  fn for L0/L2; it's a PURE data transform of node-types.json).
- `crates/pi-code-path/src/dialect.rs:12 NameLexer` · `:99 LanguageDialect`
  · `:108 kind_aliases` (the §kind map).
- `crates/pi-kernel/src/dialect_registry.rs:20 select_dialect` (the parallel map
  to DELETE — consult LanguageRegistry instead).
- `crates/pi-code-graph/src/semantic/defaults.kdl:29` (the `language … { lsp }`
  stanza idiom L1 extends).

## The matching verse-side artifacts Spell consumes
- `spacetime export-spell-profile [-o]` → profile.json (declarations,
  kind_aliases, ts_node_kinds, references, separators=["/"], ext=["st"]).
- `tree-sitter-spacetime/` → grammar.js + src/node-types.json (38 named kinds
  incl. macro_def/primitive_def/capture_type_def with `name` fields).
- `spacetime lsp` (existing) for L1.

---

## Note: the "two parse passes" tax (an L0 verse motivation)

`parser::parse()` runs BOTH (every call):
  (a) CST parser (src/syntax/cst/parser.rs) → lossless rowan tree (structure,
      spans, edits) — registry-AGNOSTIC.
  (b) event-matcher (src/syntax/events/parse_matches) → Vec<FormMatch>
      (which %form each directive matched + captures) — registry-AWARE.
NOT a special case for @ — both handle all directives uniformly, producing
different representations (tree vs. dispatch). FUP-053 had to teach the namespace
qualifier to BOTH (CST name_text + event-matcher skip_namespace_qualifier) or the
tree and dispatch disagree. Unifying these (+ the LSP's 3rd/4th registries) onto
one registry is verse-side L0 hygiene.

---

## Executable plan (validated against spell HEAD 2026-06-15)

### The exact seam (re-validated)

`pi-code-engine` already has BOTH halves of the L2 bridge — they are just not
connected:
- `language/profile.rs:483 ProfileYaml` — struct whose fields are BYTE-FOR-BYTE
  the verse exporter's JSON keys (declarations, class_like, imports, exports,
  references, separators, embedded_regions, capabilities). `:506 load_profile_json`
  parses it. ONLY test-caller today (`language/mod.rs:2039`) → production-dead.
- `language/mod.rs:67 LanguageRegistry::register(LanguageProfile)` — the live
  registration entry every builtin uses (`:48 with_builtins`). `:87 match_path`
  resolves a file by extension.

The gap between `ProfileYaml` (data) and `LanguageProfile` (`profile.rs:10`, what
`register` needs) is exactly FOUR fields, and each has a known source:
| LanguageProfile field | source |
|---|---|
| declarations/class_like/imports/exports/references/separators/embedded_regions/capabilities/extensions | DIRECT from ProfileYaml (verse export) |
| production_rules / inverse_rules / all_types / supertypes | `build.rs:410 generate_grammar_module(node_types)` — a PURE transform of node-types.json; port to a runtime `grammar_from_node_types(json)->GeneratedGrammar` |
| ts_language: tree_sitter::Language | static-linked (build dep) OR wasmtime-loaded (L3) |
| dialect: Option<LanguageDialect> | build a `DefaultNameLexer{separators}` + kind_aliases from the profile (CodePath side) |

### L0 — HYGIENE (no behavior change; spell ⟂ verse, parallel)
- **spell**: port `build.rs:410 generate_grammar_module` → a runtime
  `pub fn grammar_from_node_types(json:&str) -> GeneratedGrammar` (same code, input
  is the json string instead of a build-script file read). Keep build.rs calling
  it so nothing regresses. ACCEPTANCE: a unit test feeds TS's own node-types.json
  to the runtime fn and asserts identical ProductionRules to the generated module.
- **spell**: make `LanguageProfile.dialect` non-optional path-wise — every builtin
  already has one; just stop treating None as a branch. Then DELETE the parallel
  hardcoded `pi-kernel/src/dialect_registry.rs:20 select_dialect` ext→lexer match
  (TS/rs/py/go/hs/html/css/md) and route through `LanguageRegistry.match_path(...)
  .dialect` instead — ONE source of truth for ext→dialect.
- **verse**: (optional, deeper) unify the CST + event-matcher + 2 LSP registries
  onto one. NOT a blocker for the spell consumer; it removes the "two parse
  passes" coordination tax (the qualifier-to-both-passes work FUP-053 had to do).

### L1 — BRIDGE (spell, config-only; ships value immediately, no L0 needed)
- `spell.kdl` / `defaults.kdl`: a `languages { language "spacetime" { lsp
  command="spacetime" args="lsp"; extensions="st" } }` stanza (the idiom at
  `pi-code-graph/src/semantic/defaults.kdl:29`) + a TEXT-class `.st` profile so
  files are recognized.
- ACCEPTANCE: open a `.st` in Spell → hover/completion/diagnostics/go-to-def via
  the verse LSP. GAP (closed by L2): no `::§kind` outline/structural-edit yet.

### L2 — PROFILE (the payoff; needs L0's grammar_from_node_types)
- **spell**: a `pub fn register_profile_from_json(reg:&mut LanguageRegistry,
  profile_json:&str, node_types_json:&str, ts_language)` that: load_profile_json
  → ProfileYaml; grammar_from_node_types → the 4 grammar fields; build a
  `DefaultNameLexer{ separators }` (NEW, in pi-code-path — generic for any
  /-separated lang) + kind_aliases from `ProfileYaml`; assemble `LanguageProfile`;
  `reg.register(...)`. Add `languages{}` KDL keys (profile=path, grammar=path).
- **verse**: `export-spell-profile` (DONE) feeds profile.json; node-types.json is
  already in `tree-sitter-spacetime/src/`.
- ACCEPTANCE: with the Spacetime profile registered, `find "x.st"` returns an
  OUTLINE of `%macro`/`%primitive`/`@directive` symbols; `find
  "x.st::§macro_def"` navigates; `edit "x.st::Sym#body"` restructures — NO Spell
  recompile. The `/`-separator means `scene/camera.st::§macro` resolves a
  namespaced verse macro as a Spell CodePath.

### L3 — WASM (optional; spike was GREEN: 2.45ms load, +6.2MB, feature-gated)
- **spell**: `LanguageSource::{Static, Wasm}` + a wasmtime `WasmStore` behind a
  `wasm-grammars` feature; load `ts_language` from a `.wasm` at registration so a
  GENUINELY NEW grammar (incl. `.st`) needs zero static link.
- **verse**: emit `spacetime.wasm` in `export-spell-profile`.

### L4 — LIVE (separate product bet)
- First-party MCP integration + Canvas-on-Spacetime (§6 of the unification spec).

### Dependency DAG
```
L0(grammar_from_node_types) ─┐
L0(delete select_dialect)  ──┼→ L2(register_profile_from_json) → L3(wasm) ─→ L4
L1(spell.kdl lsp stanza) ────┘  (L1 ships independently, needs nothing)
```
L0 ⟂ L1 (parallel). L2 needs L0. L3 needs L2. L4 needs L1.

### First PR when returning to spell
L1 (pure config, ships `.st` LSP today) + L0 step `grammar_from_node_types`
(unblocks L2, zero behavior change). Both are small, independently mergeable, and
de-risk the larger L2 profile wiring.
