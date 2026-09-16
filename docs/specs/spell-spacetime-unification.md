# Spell ⟷ Spacetime: the unification blueprint

> Status: **design SETTLED, ready for autonomous implementation**. 2026-06-14.
> Settled decisions: L0 = full LSP→MetaRegistry cutover (§10.1); `.st` =
> bespoke thin `SpacetimeNameLexer` (§3.6). Start order: L0 + L1 in parallel.
> Audience: a future agent session running **inside Spacetime (verse)** with the
> Spell monorepo also checked out, tasked with building the abstractions below.
> This file is **self-contained**: it carries every path, struct, decision, and
> acceptance test needed. Read it fully before touching code.

---

## 0. The one-paragraph thesis

Spell (the coding harness) statically links ~17 tree-sitter languages and
hand-writes a `LanguageProfile` for each. Spacetime (verse) is a self-describing
meta-compiler whose grammar *is data* (`%form`/`%capture_type`/`%macro` in
`stdlib/`). Both independently **started and abandoned the same data-driven
dynamic seam** — Spell's `ProfileYaml`/`load_profile_json` (dead) and verse's
`treesitter_gen` structured emitter (dead, `#![allow(dead_code)]`). The
integration is therefore *mechanical, not speculative*: make Spell able to load
a language profile **at runtime from data**, and make Spacetime **emit that data
from the registry it already parses with**. One source of truth (the verse
`MetaRegistry`), two consumers (the verse runtime parser; Spell's `find`/`edit`).
Zero drift. The same move generalizes Spell to *any* externally-described
language and generalizes Spacetime to *any* editor/harness.

---

## 1. Ground truth (verified facts, do not re-derive)

### 1.1 Spell side (`~/code/ora/spell/crates/`)

**Three** partially-overlapping "language" concepts exist; unifying them is
in-scope:

| # | Concept | Crate / file | Owns | Coverage |
|---|---|---|---|---|
| 1 | `LanguageProfile` | `pi-code-engine/src/language/profile.rs:10` | tree-sitter `Language` + declaration/import/reference **patterns** + procedures + build-time production rules → drives **outline / `::§kind` / structural edit / graph extraction** | ~17 langs |
| 2 | `LanguageDialect` + `NameLexer` | `pi-code-path/src/dialect.rs:99` / `:12` | CodePath symbol-name **parse/render/match**, anchors (`¶`), qualifiers (`#body`/`#sig`), `edge_kinds`, `kind_aliases` (the `§kind`→raw-kinds map) | 8 langs wired, rest `None` |
| 3 | `SemanticBackend` + `semantic{}` KDL | `pi-code-graph/src/semantic/mod.rs:59` + `defaults.kdl:29` | LSP wiring → `#hover #type_definition #signature #inlay #diagnostics` | 17 langs |

**Dependency DAG** (no cycles today; keep it that way):
```
pi-code-path  (0 deps on pi-code-*, owns NameLexer/Dialect + CodePath grammar)
   ↑
pi-code-engine (owns LanguageProfile; imports pi-code-path for the dialect field)
   ↑
pi-code-graph  (owns SemanticBackend/LSP; imports pi-code-engine)
   ↑
pi-kernel      (host-agnostic read lane; bridges profile+dialect+graph;
                dialect_registry.rs = a SECOND ext→dialect map, hardcoded)
   ↑
pi-natives     (NAPI boundary to the TS agent; aggregates all pi-code-*)
```

**Registration today** (`pi-code-engine/src/language/mod.rs:48`):
`LanguageRegistry::with_builtins()` calls ~12 hardcoded `*_profile()` factories;
`pi-natives/src/lib.rs:32` wraps it in a `OnceLock`. `register()` is **public**
but never called with a runtime profile.

**The N-edit problem** — adding one language today touches:
1. `Cargo.toml` (tree-sitter crate dep) — compile-time
2. `pi-code-engine/build.rs` `GRAMMARS` const (+ `node-types.json` path)
3. `pi-code-engine/src/language/mod.rs` (a ~100–300-line `*_profile()` factory)
4. `pi-code-path/src/dialects/<lang>.rs` (`NameLexer` impl)
5. `pi-kernel/src/dialect_registry.rs` (hardcoded `match ext` arm + `OnceLock`)
6. `pi-code-graph/src/semantic/defaults.kdl` (`language`/`server` stanza)

**The dead seam**: `pi-code-engine/src/language/profile.rs:483` `ProfileYaml`
+ `:506` `load_profile_json()` — deserializes everything in `LanguageProfile`
**except** the three non-serde holes:
- `ts_language: tree_sitter::Language` (native fn ptr)
- `procedures: HashMap<String, Procedure>` (Rust closures)
- `production_rules`/`inverse_rules`/`all_types`/`supertypes` (build.rs codegen)

**Verified — the three holes are each fillable at runtime:**
- `production_rules` etc. are a **pure data transform** of `node-types.json`
  (`build.rs:410 generate_grammar_module` = deserialize `Vec<NodeType>` → build
  `HashMap`s, wrapped in `writeln!`). Porting to a runtime
  `grammar_from_node_types_json(&str) -> GeneratedGrammar` is ~80 trivial lines.
  **A WASM grammar ships its `node-types.json` next to the `.wasm`** → no codegen
  dependency. Consumed in exactly one place: `procedure/rules.rs:43`
  `matches_rule_expr`.
- `ts_language` can come from a **runtime-loaded WASM grammar** (spike below).
- `procedures` (e.g. Typst `promote_heading`) are only needed for *custom
  structural ops*; outline/`§`/edit/graph do **not** require them. A data-driven
  profile simply has `procedures = {}` until a generic procedure-DSL exists.

**`kind_aliases`** (`dialect.rs:108`, BUG-413) is already the canonical
`§function|§method|§class|§decl|§call|§import|§binding|§identifier` → raw-kinds
map. This is the cross-language `::§kind` seam; a new language fills this map.

### 1.2 Spacetime side (`~/code/ora/verse/`)

- Runtime parser is **NOT** tree-sitter. It's an event-based form-matcher
  (`src/syntax/events/`) over a `SyntaxRegistry` of `%form` patterns loaded from
  `stdlib/*.st`, producing a **rowan CST** (`SpacetimeLang`, `src/syntax/cst/`)
  and `FormMatch[]`. The grammar lives in `stdlib/`, not Rust.
- `src/treesitter_gen/` emits `tree-sitter-spacetime/grammar.js` but the
  structured per-directive emitter (`emit.rs::emit_directive_rule`, params,
  body) is **dead code**. Live behavior = inject a flat list of `@name`
  literals into a static generic CSS-ish `base_grammar.js`. The generated
  grammar has **35 named node kinds** (`directive`, `scope_block`,
  `meta_directive`, …) — generic, NOT per-`%form`. `node-types.json` +
  `grammar.json` already generated; ABI 14 (tree-sitter-compatible).
- The LSP (`src/lsp/`) is a **4th** source of truth: `FormRegistry` rebuilt from
  a hand-listed `include_str!` table (`form_registry.rs:807`, ~80 files) — a
  landmine (forget to add → LSP goes blind). Position scan is its own, not the
  runtime parser.
- **Export already exists**: `spacetime inspect registry --format json` →
  `RegistryLayerOutput { primitives[], macros[] }` (`src/cli/inspect.rs:135`);
  `DirectiveSignature`/`FormClause`/`CaptureType` are all `Serialize`.
- MCP (`src/mcp/`) = a **live-coding loop** over stdio JSON-RPC: `st_health`,
  `st_env_*`, `st_tab_open` (compile), `st_fn_put`/`st_mount`/`st_await` (the
  unified function-environment: compile a function, mount it into a region of the
  ONE persistent page, await its events), `st_inspect`, `st_workbench`, and
  `st_elicit` (native client prompt). One human→host signal path:
  `/__mcp/signal/{instance}`. (The legacy `st_propose`/`st_await_choice` browser
  picker was removed in PLAN-041 / FEAT-131 — picker/form/confirm now compose as
  region-mounted kit functions over that one sink.) No `resources`/`prompts`
  advertised; tools only.

### 1.3 The spike result (ran it; trust these numbers)

`tree-sitter = "0.25"` (locked **0.25.10**) has a `wasm` feature
(wasmtime-c-api v29). **Not currently enabled** in Spell.

Measured in a throwaway crate (`/tmp/wasmspike`):
- Enabling `features=["wasm"]` pulls **wasmtime + cranelift JIT** (gimli,
  cranelift-codegen/-frontend/-native, regalloc2, wasmtime-environ, object …).
- Incremental compile cost: **~35s debug / ~54s release**.
- Linked binary size delta: **363 KB → 6.59 MB** (≈ **+6.2 MB**).
- Round-trip **proven**: `WasmStore::new(&Engine::default())` →
  `store.load_language("json", bytes)` → `parser.set_wasm_store(store)` →
  `set_language` → parse. Loaded a real `tree-sitter-json.wasm` (25 kinds) in
  **2.45 ms**, parsed `{"a":[1,true,null]}` correctly. API:
  `tree_sitter::{wasmtime::Engine, Parser, WasmStore}`.
- Caveat: grammar `.wasm` must be ABI-compatible (13–15); verse grammar is ABI
  14 ✓. wasmtime `WasmStore` is `Send + Sync` (0.25).
- **NOT yet verified** (do before committing Layer 3): wasmtime building cleanly
  inside the `pi-natives` NAPI `.node` artifact on all target platforms; runtime
  cold-start of the cranelift engine under the agent; whether to gate behind a
  cargo feature so the +6 MB is opt-in.

**Spike verdict: feasible, worth it — but as its own milestone, feature-gated.**
The +6 MB / +35 s and the cranelift JIT are real. It unlocks two things at once:
(a) `.st` (and any) grammar with no Spell recompile, and (b) — your instinct —
a **general WASM extension substrate** for Spell independent of how grammars are
built. Recommendation: **FUP/spike now, implement after Layers 1–2 prove the
data path.** Don't block the integration on it.

---

## 2. Design decisions (locked, per your answers)

- **D1 — Target both.** Spell-grade `.st` intelligence *and* broad editor
  support. ∴ the generated tree-sitter grammar must become *faithful* (it feeds
  editors), and Spacetime must *also* emit a Spell `LanguageProfile`.
- **D2 — Coupling OK as first scope.** Spell may assume a `spacetime` binary is
  present and shell out to it. We may freely **upgrade Spacetime's binary** to
  add commands/outputs Spell needs. This unlocks the live path (hover/compile
  from a running Spacetime) that a static bundle can't reach.
- **D3 — Auto-generate the profile from Spacetime.** No hand-written `.st`
  profile in Spell's Rust (violates *both* repos' AGENTS laws). Spacetime emits
  the bundle from the live `MetaRegistry`.
- **D4 — Unify the three Spell language concepts** behind one source of truth,
  with seams that scale to more languages (Spell) and more harnesses (Spacetime).

---

## 3. The unified abstraction (Spell side)

### 3.1 Target shape — `LanguageProfile` becomes the single source of truth

Today `profile.dialect: Option<LanguageDialect>` and `pi-kernel`'s
`dialect_registry.rs` keeps a *parallel* hardcoded ext→dialect map. Collapse to:

```
LanguageProfile  (pi-code-engine)  ──the one registry entry per language──
├─ id, extensions, capabilities
├─ ts_language        : LanguageSource   ← NEW enum (see 3.2)
├─ grammar            : GeneratedGrammar  (prod/inverse/all_types/supertypes)
│                        ← build.rs OR runtime node-types.json (3.3)
├─ declarations/class_like/imports/exports/references/embedded  (serde today)
├─ dialect            : LanguageDialect   ← make NON-optional; default-derivable
│   ├─ name_lexer     : Arc<dyn NameLexer> (or a data-driven DefaultNameLexer)
│   ├─ anchors / qualifiers / edge_kinds
│   └─ kind_aliases   : §kind → [raw ts kinds]   ← the ::§ map
└─ procedures         : {} unless custom ops
```

**Kill the duplication:** `pi-kernel::dialect_registry::select_dialect(path)`
must stop being a hardcoded `match` and instead consult
`language_registry().match_path(path).dialect`. One ext→language map
(`LanguageRegistry.by_extension`), one place. (Risk: pi-kernel → pi-code-engine
dep direction — already holds, fine.)

**Make `NameLexer` data-driven for the common case.** Today every dialect is a
bespoke Rust `winnow` parser. Most languages need only "split on a separator,
match the leaf segment." Introduce a `DefaultNameLexer { separator: String,
quoting: Option<char> }` (covers `.`/`::`/`/`) so a *data* profile gets working
`::Symbol` resolution with zero Rust. Bespoke lexers stay for languages that
need them (Rust turbofish, Haskell). This is what lets a runtime-loaded profile
actually resolve symbols.

### 3.2 `LanguageSource` — how the `ts_language` hole is filled

```rust
// pi-code-engine
pub enum LanguageSource {
    /// Compile-time linked (today's path). Zero-cost, no wasm.
    Static(tree_sitter::Language),
    /// Runtime-loaded WASM grammar (Layer 3, feature = "wasm-grammars").
    Wasm { name: String, bytes: Arc<[u8]> },  // node-types.json carried in `grammar`
}
```

`LanguageRegistry` holds one `WasmStore` (shared `Engine`); parser construction
picks the path. Behind `#[cfg(feature = "wasm-grammars")]` so the +6 MB is
opt-in. Static path is the default and unchanged.

### 3.3 Runtime grammar derivation (de-risks Layer 3)

Port `build.rs:generate_grammar_module` to a runtime fn (same logic, no
`writeln!`):
```rust
// pi-code-engine
pub fn grammar_from_node_types(json: &str) -> Result<GeneratedGrammar>;
```
`build.rs` then becomes a thin caller of the *same* function (write its output
to `OUT_DIR`), eliminating two copies of the transform. Static langs keep using
the generated file; WASM/JSON langs call it at registration.

### 3.4 The runtime registration entry point

```rust
// pi-code-engine — finish the dead ProfileYaml seam:
pub fn register_profile_from_json(
    reg: &mut LanguageRegistry,
    profile_json: &str,         // ProfileYaml (extended: + grammar source ref)
    node_types_json: &str,      // → grammar_from_node_types
    wasm_bytes: Option<Arc<[u8]>>,
) -> Result<()>;
```
Wire a discovery pass in `language_registry()` init (or a kernel call) that reads
profiles from a directory / KDL block (3.5) and calls `register()`.

### 3.5 The config seam (KDL) — extends the existing idiom

`pi-code-graph/src/semantic/defaults.kdl` already does
`language "x" { lsp "..." }` (LSP-only). Add a sibling that registers a *full
profile* so `.st` is recognized for outline/`§`/edit, not just LSP:

```kdl
languages {
    // points at a Spacetime-emitted bundle; ext + grammar + patterns all inside
    language "spacetime" {
        profile "~/.spell/languages/spacetime/profile.json"
        grammar wasm="~/.spell/languages/spacetime/spacetime.wasm" \
                node-types="~/.spell/languages/spacetime/node-types.json"
        // optional live LSP (Layer 1):
        lsp command="spacetime" args="lsp"
    }
}
```
Precedence: `<project>/.spell/config.kdl` > `~/.spell/config.kdl` > bundled
`defaults.kdl` (existing rule). A `languages{}` entry calls
`register_profile_from_json`.

### 3.6 The `.st` NameLexer — investigation result (RESOLVED)

**Question:** does `.st` use the generic `DefaultNameLexer` (§3.1) or a bespoke
one? **Answer: bespoke `SpacetimeNameLexer`, but thin — CSS-class tier, ~40 lines.**

Verified `.st` symbol-name model (from `stdlib/*.st` + `src/syntax/cst/lexer.rs`
+ `src/syntax/registry.rs::extract_prefix`):

| Axis | Shape | Example | Path sep? |
|---|---|---|---|
| Directive | `@name` | `@camera`, `@react-to`, `@fill` | none, hyphenated |
| Definition | `%macro name` / `%form` / `%primitive` / `%capture_type` | `%macro morph-to-color`, `%macro distort_chromatic` | none, hyphen/underscore |
| Pattern | `@pattern name` | `@pattern focusable` | none |
| Template invoke | `&name` | `&self`, `&card` | none |
| Variable | `$name` | `$color`, `$viewMatrix` | none |
| Element ref | `~name` | — | none |
| Selector | `.class` / `#id` / `&:pseudo` | CSS-derived | CSS combinators |

**Decisive findings:**
1. **No `::`/`.`-qualified symbol paths.** Grep for dotted/qualified directive
   names → zero hits. Names are flat tokens. The `.` in `$params.fov` is runtime
   field access inside `%derives` expressions, never an addressable symbol path.
2. **The discriminator is the prefix sigil** (`@%&$~.#`), which the registry
   already uses (`extract_prefix` = first char). That is a *kind* selector
   (→ `kind_aliases`), NOT a name-path separator.
3. **Names are hyphenated** (`morph-to-color`, `react-to`) — the one trap. CSS
   solved exactly this; `CssNameLexer` (`pi-code-path/src/dialects/css.rs`)
   already accepts `.`/`#`/hyphens/brackets and breaks on whitespace.

**∴ implications, locked:**
- `SpacetimeNameLexer`: accept optional sigil-prefix + `[A-Za-z0-9_-]+`, plus
  CSS selector forms (clone CSS bracket/`#` handling). Emit
  `NamePayload::Raw(rendered)` — no new payload variant needed (`NamePayload` is
  just `Raw|Quoted`). Model file: `dialects/css.rs`.
- **UPDATE — namespaces REVERSE the "no separator" verdict.** The bare-name
  finding held for a flat `.st`; with the module system (`docs/specs/module-
  system.md`) a qualified reference `@scene/camera` / `@s/camera` INTRODUCES the
  `/` path-split axis. ∴ `.st` re-unifies ONTO the composable primitives and the
  generic path-split lexer is now the RIGHT base, not the wrong one:
  ```
  SpacetimeNameLexer = sigil_prefix      // strip & record @ % & $ ~  → kind hint
                     · path_split("/")    // ← namespace axis (the DefaultNameLexer core)
                     · hyphenated_ident    // morph-to-color, react-to
                     · selector_forms      // .class / #id (global layer, P1)
  ```
  `DefaultNameLexer { separator, segment, sigil_prefix, selector_forms }` now
  onboards Go (`.`), Java (`.`), AND `.st` (`/`) FROM DATA; Rust/Haskell stay
  bespoke for turbofish/`impl for`. **Namespaces are the pressure that forces the
  generic lexer into existence** — `.st` is its first non-trivial consumer, not
  the exception that dodged it. (Supersedes §3.1's `{separator}`-only framing:
  the seam is a small library of composable primitives a data profile combines;
  the exporter emits a lexer *spec*, Spell writes no per-language Rust.) See §3.7.
- `kind_aliases` for `.st` keys on the **sigil + definition keyword**, not name
  shape: `§function → [macro_def, primitive_def]`, `§class → [template_def/pattern]`,
  `§decl → [capture_type_def, value_decl]`, `§import → [meta_directive @use/@import]`.

### 3.7 Module-aware CodePath — `/` unifies four axes (depends on module system)

The Spacetime module system (`docs/specs/module-system.md`) makes ONE separator
`/` carry four meanings at once — the coincidence that makes Spell addressing of
`.st` fall out for free:

```
filesystem:     stdlib/scene/camera.st
module FQN:     std:scene/camera
reference:      @scene/camera   (or @s/camera via alias)
Spell CodePath: scene/camera.st::§macro      ← SAME / separator
```

Consequences for the Spell side:
- **`path_split("/")` is the shared primitive.** The same axis that resolves a
  `.st` namespace reference resolves a Spell CodePath segment. The
  `SpacetimeNameLexer` (§3.6) and the module resolver (`module-system.md` §7)
  agree by construction.
- **Namespaced `.st` defs → qualified Spell symbols.** A macro registered as
  `std:scene/camera` surfaces in Spell outline as a symbol whose CodePath is the
  namespace path. `find { target: "file.st::§macro" }` lists them; a future
  `::scene/camera` member-path addresses one. The exporter (§4) emits the
  namespace as the symbol's qualified name — no extra Spell work.
- **Re-export = profile boundary.** A module's `%public`/`%reexport`
  (`module-system.md` §5) is exactly the per-namespace unit the exporter (§4.1)
  bundles. Modularity on the verse side = addressability on the Spell side, one
  design, zero glue.
- **`@use`/`@import` → Spell `import` patterns.** The profile's `imports` /
  `references` (the serde `ImportPattern`/`ReferencePattern` in
  `pi-code-engine/src/language/profile.rs`) are emitted from the module grammar:
  `@use "std:scene"` is an import edge; `@scene/camera` is a reference. This
  lights up Spell `import→`/`ref→` graph edges for `.st` from the same source.
- **`%using` scoped capture-types do NOT cross into Spell statically.** The
  Elixir-soul active hook (`module-system.md` P5) installs scope-local grammar
  at compile time; a static Spell profile sees the union of declarable kinds, not
  the per-scope activation. This is the one place the static bundle is an
  approximation of the live compiler — the LIVE lane (MCP `st_outline`, §6)
  closes it when exact per-scope dialect matters. (Honesty gate: the static
  profile must not imply scope-accurate `%using` resolution it cannot do.)

This section depends on the module system landing (its M0–M1, M4); until then the
`.st` profile addresses flat names and `/` is inert in CodePath for `.st`.

---

## 4. The emit side (Spacetime) — auto-generate the bundle

**Replace the dead `treesitter_gen` structured emitter with a faithful one, and
add a Spell-profile emitter — both reading the *same* `MetaRegistry`.**

### 4.1 New command
```
spacetime export-spell-profile [--out DIR] [--include GLOB...]
  ⇒ DIR/profile.json        (Spell ProfileYaml: extensions, declarations,
  │                           imports, references, kind_aliases, separators)
  ⇒ DIR/grammar.js          (faithful tree-sitter grammar from %form shapes)
  ⇒ DIR/node-types.json     (from `tree-sitter generate`, or emitted directly)
  ⇒ DIR/spacetime.wasm      (optional; if tree-sitter + emcc/docker present)
  ⇒ DIR/highlights.scm      (from %form prefix semantics)
```
All derived from `MetaRegistry` (the **same** structure the runtime parser uses).
Edit a `%form` in `stdlib/` → re-run → Spell updates. **Zero drift.**

### 4.2 What "faithful" means for the grammar (D1)

The current generic grammar has node kinds like `directive`, `scope_block` —
too coarse for `.st` symbols (`%macro`, `%primitive`, `@template &card`, selector
scopes). Two acceptable strategies (pick per construct):

- **(a) Enrich the grammar** with named declaration nodes
  (`macro_def`, `primitive_def`, `capture_type_def`, `template_def`) so
  tree-sitter outline + Spell `DeclarationPattern` can target them directly.
  Resurrect `emit.rs::emit_directive_rule` driven by `extract_directives` →
  emit one rule per top-level `%`-construct. Editors benefit too.
- **(b) Keep coarse nodes + emit `NameExtractor` strategies** in `profile.json`
  that dig names out of generic nodes (e.g. `ChildText`/`ListFormArg`). Cheaper,
  but editor highlighting stays generic.

**Recommendation:** (a) for the handful of top-level definition forms (they're
finite and stable), (b) for the open-ended directive space. This is the
"faithful for symbols, generic for the long tail" split.

### 4.3 Mapping table — `CaptureType` → Spell `NameExtractor`/kinds

`capture_map.rs` already maps `CaptureType` → tree-sitter rule refs. Extend with
the inverse intent: which `%capture_type`/`%form` field becomes a Spell
declaration `name`/`body`/`kind`. The `kind_aliases` Spell needs:
```
§function → [macro_def, primitive_def]      (a Spacetime "callable" form)
§class    → [template_def]                    (a component)
§decl     → [capture_type_def, value_decl]
§import   → [meta_directive(@import …)]
§identifier → [identifier, variable_ref, element_ref, preset_ref]
```
(Author this from the registry, don't hardcode in Spell.)

### 4.4 Fix the 4-parser rot (prerequisite hygiene, your challenge #3-to-me)

Before/with the emitter: fold the **LSP `FormRegistry`** onto the **runtime
`SyntaxRegistry`/`MetaRegistry`** so there is ONE registry the LSP, the runtime
parser, AND the exporter all read. Delete the hand-listed `include_str!` table
(`form_registry.rs:807`) in favor of the embedded-stdlib mechanism the compiler
already uses. This makes the exported bundle *trustworthy by construction* —
otherwise we'd be exporting from a frozen subset.

---

## 5. Layered rollout (each layer ships value; later layers optional)

```
L0  HYGIENE  Unify verse's 4 parsers onto one registry (4.4).
             Unify Spell's dialect_registry → LanguageRegistry (3.1).
             Port build.rs grammar gen → runtime fn (3.3). [no behavior change]

L1  BRIDGE   Spell: `languages{} … lsp command="spacetime" args="lsp"` +
             a text-class `.st` profile so files are recognized.
             ⇒ hover/completion/diagnostics/go-to-def for .st NOW.
             Honest gap: no ::§kind yet. Pure config + existing verse LSP.

L2  PROFILE  Spell: finish ProfileYaml → register_profile_from_json (3.4),
             DefaultNameLexer (3.1), languages{} profile/grammar keys (3.5).
             Verse: `export-spell-profile` (4.1) emitting profile.json +
             node-types.json against an ALREADY-LINKED-or-text grammar.
             ⇒ outline + ::§ + structural edit for .st, no Spell recompile,
               for any language that has a static grammar OR uses text fallback.

L3  WASM     Spell: LanguageSource::Wasm + WasmStore (feature-gated, 3.2),
             grammar_from_node_types at registration.
             Verse: emit spacetime.wasm in export-spell-profile.
             ⇒ genuinely NEW grammars (incl .st) with zero Spell recompile.
             Also: the general WASM-extension substrate (your runtime-independence
             instinct) — evaluate separately, see §7.

L4  LIVE     First-party MCP integration + Canvas convergence. See §6.
```

**Dependency:** L0 ⟂ L1 (parallel). L2 needs L0. L3 needs L2 + the spike
greenlight. L4 needs L1 (the bridge) and is otherwise independent.

---

## 6. Vision: MCP first-party + Canvas convergence (L4)

### 6.1 Two layers, two jobs — do NOT route find/edit through MCP

- **Static lane (Layers 1–3):** `.st` *code intelligence* (outline, `§`, edit,
  graph) flows through `find`/`edit` over a loaded `LanguageProfile`. This is
  the harness's structural surface; it must be local and synchronous.
- **Live lane (MCP):** Spacetime's MCP keeps owning the *live-coding loop*
  (compile, render, propose, elicit, env/tab). It is **complementary**, not a
  parser. Add introspection tools so the agent can cross the lanes:
  - `st_outline { entry|code }` → the registry-derived outline (same data the
    exporter uses) — lets the agent inspect without a file on disk.
  - `st_profile` → returns the live Spell profile bundle (so Spell can *fetch*
    it from a running Spacetime instead of a pre-built file — the D2 live path).
  - `st_compile { code }` → already exists as `st_tab_open`; surface diagnostics
    as `structuredContent` (already does).

### 6.2 First-party registration in Spell

Today MCP servers are user-config. Make Spacetime **first-party**: when a
`.st`/`spacetime.kdl`/`stdlib/` workspace is detected, Spell auto-registers the
`spacetime` MCP server (the `mcp { server "spacetime" … }` block in the verse
`mcp/mod.rs` doc-comment becomes a built-in default, not copy-paste). Gate on
`spacetime` binary presence (D2).

### 6.3 Canvas → Spacetime (`&function(...)` as native ad-hoc UI) — the big bet

**Current Canvas:** Spell renders structured data in QML windows
(`packages/coding-agent/src/modes/qml/canvas/*.qml`) driven by a `bridge.props`
JSON payload via the `canvas` tool (`write`/`launch`/`send_message`). Components
(`DataTable`, `TreeView`, `DiffView`, `LogStream`) are hand-written QML.

**The bet:** Spacetime's `@template &card($title) { … }` / `&fn(...)` invocation
model is *exactly* an ad-hoc-interface constructor. A Spacetime-backed Canvas
would let the agent **describe** an interface in `.st` (declarative, data-bound,
animated, no custom JS — verse's whole thesis) and have it rendered live, instead
of shipping bespoke QML per view. `&component(data: $x)` becomes the Spell-native
"emit a view" primitive. Verse already proves this works headlessly: the MCP
unified environment (`live.rs`, `stdlib/__mcp__/env.st`) mounts a compiled `.st`
function into a region of the host page with zero custom JS.

**Why it's compelling:**
- One renderer (the Spacetime compiler) instead of QML-vs-web-vs-? divergence.
- Data binding + state machines + animation come *for free* from stdlib.
- The agent authors UI in the same language it's getting intelligence for —
  dogfooding both products.
- Spacetime gains a first-class **harness** (Spell) as a render target →
  generalizes verse beyond "websites" to "agent-native interfaces."

**Why it's a bet, not a given (challenge to you):**
- Canvas today is QML/native (desktop, `pi-qml`); Spacetime targets the web
  (DOM/CSS/JS). Convergence means either (a) Spell Canvas renders Spacetime's
  HTML/JS output in a webview, or (b) Spacetime gains a QML backend. (a) is far
  cheaper and aligns with the existing `BrowserWindow.qml`. Pick (a).
- Latency: Canvas is interactive; a compile-per-update loop must be warm. Verse's
  MCP already keeps a warm env; measure before committing.
- Scope: this is a *product* decision, not a refactor. Record as a **vision FUP**
  with a spike (render one real Spell view — e.g. a task board — as a `.st` page
  in a webview Canvas) before any cutover.

**Recommendation:** L4a = MCP introspection tools + first-party registration
(low risk, high value, do it). L4b = Canvas-on-Spacetime = **spike + FUP**, gated
on an explicit product call after L1–L3 land.

---

## 7. Spell-on-WASM as a general extension substrate (your runtime-independence note)

The wasmtime dependency you'd pull for tree-sitter grammars (§1.3) is also a
general WASM host. This is a *separate* strategic question from grammars:
- **For grammars:** justified, scoped, feature-gated (Layer 3).
- **As a Pi-independent runtime / plugin substrate:** much larger; wasmtime
  becomes the sandbox for *arbitrary* Spell extensions (tools, language servers,
  renderers) decoupled from the Pi/NAPI host. This is a credible path to "Spell
  runs independent of Pi," but it's a multi-quarter architecture, not part of
  this integration.
- **Verdict:** record as a **strategic spike FUP** ("wasmtime as Spell's
  universal extension ABI"), explicitly downstream of the grammar use proving
  wasmtime-in-NAPI viable. Don't conflate with the language work.

---

## 8. Unified seams — the deliverable contract

**On Spell (scales to more languages):**
- `S1` `LanguageProfile` = single registry entry; `dialect` non-optional;
  `dialect_registry.rs` consults the registry (no parallel map).
- `S2` `register_profile_from_json` + `grammar_from_node_types` + `languages{}`
  KDL = a language can be **added at runtime from data**.
- `S3` `DefaultNameLexer` = data profiles get `::Symbol`/`§` without Rust.
- `S4` `LanguageSource::{Static,Wasm}` = grammar can be linked or loaded.

**On Spacetime (scales to more harnesses/editors):**
- `T1` ONE registry feeds runtime parser + LSP + exporter (kill the 4-parser
  rot).
- `T2` `export-spell-profile` (and a faithful `generate-grammar`) emit editor +
  harness artifacts from that registry.
- `T3` MCP gains `st_outline`/`st_profile` so a harness can consume intelligence
  live, not just from files.

**Generalization claim to hold true at the end:**
- Spell can onboard *any* language that ships {grammar (static|wasm) +
  node-types.json + profile.json} — Spacetime is just the first non-builtin.
- Spacetime can serve *any* editor (via faithful tree-sitter + queries) and *any*
  harness (via the profile bundle + MCP) — Spell is just the first harness.

---

## 9. Acceptance tests (definition of done per layer)

- **L0:** `cargo test -p pi-code-engine` green after `build.rs` calls
  `grammar_from_node_types`; `select_dialect` returns the registry's dialect for
  all builtin exts (add a test asserting parity with the old hardcoded map).
  Verse: one `MetaRegistry` instance backs `lsp` + `compile` + `inspect`
  (assert `inspect registry` count == LSP `FormRegistry` count).
- **L1:** open a `.st` file in a Spell session with `languages{} lsp` configured
  → hover on a `@directive` shows its signature (served by `spacetime lsp`).
- **L2:** `spacetime export-spell-profile --out /tmp/st` produces
  `profile.json` + `node-types.json`; Spell loads them via `languages{}`;
  `find { target: "x.st#outline" }` lists `%macro`/`@template` symbols;
  `find { target: "x.st::§function" }` resolves.
- **L3:** with `--features wasm-grammars`, the same flow works using
  `spacetime.wasm` and **no** matching static grammar linked; `find` parses an
  `.st` file end-to-end. Cold-load < 50 ms; binary-size delta documented.
- **L4a:** Spell auto-registers the `spacetime` MCP server on detecting an `.st`
  workspace; `st_outline { code }` returns the registry outline.
- **L4b (spike):** one real Spell view renders as a `.st` page in a webview
  Canvas; latency measured; go/no-go recorded in the FUP.

---

## 10. Open questions to resolve at implementation start

1. **DECIDED → full cutover.** L0 fully migrates the verse LSP `FormRegistry`
   onto the one `MetaRegistry` (no thin-bridge half-step). The exporter then
   reads that single registry — making the exported bundle trustworthy by
   construction (§4.4).
2. **DECIDED → `SpacetimeNameLexer` as a DATA-DRIVEN composition. See §3.6/§3.7.**
   Initial finding (flat `.st`): bare sigil-prefixed hyphenated idents, no path
   separator. **Then namespaces (the module system, `docs/specs/module-
   system.md`) reversed it:** `@scene/camera` introduces a `/` path-split axis,
   so `.st` re-unifies onto the composable-primitive lexer
   (`sigil_prefix · path_split("/") · hyphenated_ident · selector_forms`). The
   generic path lexer is now the RIGHT base; `.st` is its first non-trivial
   consumer. NEW dependency: the module system (its M0–M1) gates the `/` axis;
   until then `.st` addresses flat names.
3. WASM feature gating — one `wasm-grammars` feature on `pi-code-engine`, or a
   workspace-level switch (affects `pi-natives` `.node` size for everyone)?
4. Bundle distribution — does `spacetime` *ship* the Spell bundle (so Spell
   fetches it via the binary), or does Spell build it on first run via
   `export-spell-profile`? (D2 allows either; live-fetch is simplest.)
5. Canvas L4b — webview-of-Spacetime-HTML (cheap) confirmed over QML-backend-for-
   Spacetime (expensive)?

---

## Appendix A — exact code anchors

Spell:
- `crates/pi-code-engine/src/language/profile.rs:10` LanguageProfile · `:483`
  ProfileYaml · `:506` load_profile_json
- `crates/pi-code-engine/src/language/mod.rs:48` with_builtins · `:87` match_path
  · `:66` register
- `crates/pi-code-engine/build.rs:56` GRAMMARS · `:410` generate_grammar_module
- `crates/pi-code-engine/src/language/generated.rs:24` include_grammar!
- `crates/pi-code-engine/src/procedure/rules.rs:43` matches_rule_expr (the ONLY
  production_rules consumer)
- `crates/pi-code-path/src/dialect.rs:12` NameLexer · `:99` LanguageDialect ·
  `:108` kind_aliases
- `crates/pi-code-path/src/dialects/rust.rs` example dialect
- `crates/pi-kernel/src/dialect_registry.rs:20` select_dialect (the parallel map
  to delete)
- `crates/pi-code-graph/src/semantic/mod.rs:59` SemanticBackend · `defaults.kdl:29`
  language stanzas
- `crates/pi-natives/src/lib.rs:32` language_registry() OnceLock

Spacetime (verse):
- `src/treesitter_gen/{mod,walker,emit,capture_map}.rs` + `base_grammar.js`
  (resurrect emit.rs)
- `src/cli/generate_grammar.rs` · `src/cli/inspect.rs:135` RegistryLayerOutput
- `src/metasystem/registry.rs` MetaRegistry · `src/syntax/{registry,mod}.rs`
  SyntaxRegistry · `src/syntax/cst/` rowan CST
- `src/lsp/form_registry.rs:807` the hand-listed include_str! table (delete)
- `src/mcp/{tools,protocol,live,state}.rs`
- `tree-sitter-spacetime/src/{node-types.json,grammar.json}` (already generated,
  ABI 14)

## Appendix B — spike reproduction
```bash
# /tmp/wasmspike/Cargo.toml: tree-sitter = { version="0.25", features=["wasm"] }
# src/main.rs: Engine::default() → WasmStore::new → store.load_language(name,bytes)
#              → parser.set_wasm_store(store) → set_language → parse
curl -sL https://registry.npmjs.org/tree-sitter-json/-/tree-sitter-json-0.24.8.tgz \
  | tar xz && # package/tree-sitter-json.wasm
cargo run --release -- package/tree-sitter-json.wasm json '{"a":[1,true,null]}'
# → loaded 'json' kinds=25 load=2.45ms parse=0.019ms root=(document (object …))
# binary: 363KB → 6.59MB ; compile +35s debug
```
