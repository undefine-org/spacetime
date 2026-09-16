# Declared compile-time effects — the substrate (R1)

> Status: **design, ready to build** (the "build now"). 2026-06-14.
> The bottom rung of `docs/specs/reflective-spacetime.md`. The mechanism that
> makes `docs/specs/module-system.md` (M1+) genuinely Spacetime-defined.
> Self-contained; carries every code anchor and the parity gate.

---

## 0. What this is

A small, **data-extensible vocabulary of compile-time effect clauses** that let a
`%form` DECLARE what it does to the registry — siblings to the existing
`%registers` and `%resolves`. A fixed enactor phase reads these declarations and
performs the irreducible operations (read a file, merge an AST, prefix a key,
install an alias) via a closed set of Rust primitives.

This replaces imperative registry mutation (forms running arbitrary code) — the
foot-gun — with **declared effects**: analyzable without running, orderable,
cacheable, agent-reasonable (soundness property 3, `reflective-spacetime.md` §3).

**M1 ships exactly one clause — `%imports` — and migrates `@import` onto it.**
`%installs` (R3) and `%reflects` (R2) are the same shape, deferred.

---

## 1. The fact that reframes the work

`@import` is ALREADY half-self-describing. `stdlib/macros/presets.st:176`:

```st
%macro import {
  %form { @import $path:string }
  %registers import { path: $path }
}
```

Its SYNTAX is a `%form`. But its EFFECT (read the file, merge defs flat-global)
is hardcoded Rust: the parser special-cases `@import` into `ImportAst { path }`
(`src/parser/mod.rs:693`), and `resolve_imports` (`src/parser/mod.rs:5913`) walks
those, resolves via `ImportResolver`, and `merge_ast` (`src/parser/mod.rs:6065`)
flattens every imported def into one global AST. The two halves are bridged by
the literal string `"import"` (`resolve_imports` strips `matches` whose
`macro_name == "import"`).

∴ the migration is not greenfield — it is **completing a migration already
underway**: move the effect from the `ImportAst` Rust special-case onto a declared
`%imports` clause, enacted by one phase. The `%form` already exists; we add the
effect declaration and delete the special-case.

---

## 2. The effect-clause class

An effect clause is a `%`-clause inside a `%macro`/`%primitive` body whose
presence DECLARES a compile-time registry effect. Existing members (precedent):

| Clause | Effect | Status |
|---|---|---|
| `%registers <cat> { ... }` | register the matched construct into a named category | HAVE |
| `%resolves { $sym -> ST.reg }` | map a symbol to a runtime registry location | HAVE |
| **`%imports { ... }`** | **load a module + merge/namespace its defs** | **M1** |
| `%installs { ... }` | register NEW forms (metaform) | R3 (deferred) |
| `%reflects { ... }` | expose registry state as readable data to the page | R2 (deferred) |

The class invariant (all members obey):
- the clause is **DATA** (parsed into an AST struct, serde-able), never executable
  code;
- it names a **declared effect** the enactor phase knows how to perform;
- adding a member is adding a data variant + one enactor arm — NOT a parser
  special-case (elegance bar: variants-as-data).

### 2.1 `%imports` shape (M1)

```st
%macro use {
  %form {
    @use $path:string
  }
  %imports {
    module: $path        // the module ref: "coll:ns" | "./rel" | "stdlib/x"
    as:     $alias?       // optional qualified alias  (PureScript)
    only:   ($names,*)?   // optional explicit import list
    hiding: ($names,*)?   // optional open-minus list
    global: false         // @import sets true (flat-global merge); @use false
  }
}
```

`@import` (back-compat) is the SAME clause with `global: true` and no
alias/only/hiding — proving the spectrum (`module-system.md` §4: "@import floods,
@use scopes"). The existing `%macro import` gains `%imports { module: $path,
global: true }` and KEEPS `%registers` until the cutover completes.

AST (new, in `src/parser/meta_ast.rs`, sibling to `RegistersClause`):

```rust
pub struct ImportsClause {
    pub module: CaptureRef,          // $path
    pub alias: Option<CaptureRef>,   // as $alias
    pub only: Vec<CaptureRef>,       // only (...)
    pub hiding: Vec<CaptureRef>,     // hiding (...)
    pub global: bool,                // @import => true
    pub span: SourceSpan,
}
// MacroDefAst gains: pub imports: Option<ImportsClause>  (serde default None)
```

---

## 3. The enactor phase

One phase reads `%imports` declarations and performs the effect. It REPLACES the
`ImportAst` branch in `resolve_imports`, it does not run beside it at rest.

```
enact_imports(main_ast, registry, resolver):
  1. collect import effects = every FormMatch whose macro has an %imports clause
     (today: @import; M1: + @use). Each → ImportEffect { module, alias, only,
     hiding, global, site_file }.
  2. TOPOLOGICAL ORDER by dependency (uses-before-users) — resolve_imports
     ALREADY does this via its visited-set + queue + has_cycle. Reuse verbatim.
  3. for each effect in order:
     a. resolve `module` → file/stdlib bytes (ImportResolver, unchanged)
     b. parse → defs
     c. if global: merge_ast flat (today's behavior, byte-identical)
        else:      assign namespace to defs (Namespace from module ref, M0
                   Fqn keying), then merge; record alias/only/hiding into the
                   importing file's ImportScope
  4. build per-file ImportScope { open[], aliased{}, explicit{}, hidden{} } for
     the reference-side resolver (M1 dispatch, module-system.md §7).
```

**Rust-primitive boundary (soundness property 5).** The enactor is the
compile-time twin of `%emit js`: a CLOSED set of primitives the declared clauses
invoke. For `%imports`:
- `fs_read(path)` / stdlib lookup — `ImportResolver` (exists)
- `parse(bytes)` — `parser::parse` (exists)
- `merge_ast(target, src)` — exists; gains a namespace-aware variant
- `assign_namespace(defs, ns)` — new, trivial (sets `def.module`, M0 field)
- `install_scope_entry(scope, ...)` — new, hashmap insert

Nothing else. A clause CANNOT run arbitrary code; it can only name these effects.
That is the hygiene guarantee.

---

## 4. Stratification (soundness property 4)

M1's effects are single-stratum: `%imports` loads MODULES (L1 acting on L2), and
the loaded defs are ordinary forms. No form installs a form yet (that is R3
`%installs`). So M1 needs only the EXISTING ordering (`resolve_imports`'
topological uses-before-users), made explicit:

```
STRATUM 0  bootstrap stdlib (the @use/@import forms themselves) — loaded first, global
STRATUM 1  module loads (%imports effects) — topologically ordered, before any
           non-import directive matching in the importing file
STRATUM 2  ordinary directive matching/dispatch — consults the ImportScope built
           in stratum 1
```

The rule that keeps it well-founded: **an effect at stratum N may only depend on
declarations established at stratum < N.** M1 satisfies this trivially (imports
depend only on bootstrap forms). The doc records the rule now so R3 (`%installs`,
which adds a real stratum tower) inherits a stated discipline rather than
inventing one under pressure. NB: this is the METASYSTEM load stratum — distinct
from the runtime `Phase{Global,Selector}` (`pipeline/types.rs`), which is emission
phase. Do not conflate.

---

## 5. The migration (parity gate — non-negotiable)

The enactor touches the load-bearing, currently-green import path. Discipline:

```
STEP 1  add ImportsClause AST + parse it from the %imports body. No behavior
        change (clause parsed, not yet enacted). Tests stay green.
STEP 2  add the enactor ALONGSIDE the ImportAst path, gated to enact ONLY the
        global case (@import). Assert byte-identical output vs the ImportAst path
        on the import fixtures (compiler.rs import tests + the 2236 lib suite).
STEP 3  flip @import to route through %imports; DELETE the ImportAst special-case
        (parser branch + resolve_imports ImportAst walk + the "import" string
        strip). One live path at rest. Re-run full suite + headless.
STEP 4  add @use (the namespaced %imports: global:false + alias/only/hiding) and
        the reference-side ImportScope resolver. New behavior, new fixtures.
```

**Parity is proven before deletion. Never two live paths at rest.** If STEP 2
parity fails on any fixture, the enactor is wrong — fix before STEP 3.

Acceptance:
- STEP 1–3: `cargo test --lib` (2236) + `cargo test --test integration_tests`
  green; `@import` fixtures byte-identical; the `ImportAst` struct + its parser
  branch deleted (grep returns nothing).
- STEP 4: a 2-folder fixture where both export `camera`; bare `@camera` errors
  with a qualify hint (reuse `dispatch_ambiguity`, FEAT-088); `@s/camera`
  resolves; `@import` still floods global; dispatch candidate-count drops.

---

## 6. Why this earns its place (elegance bar, all four)

1. **Small extension of existing grammar/registry** — `%imports` is a clause
   sibling to `%registers`; no new subsystem.
2. **Variants are DATA** — effect clauses are AST structs + enactor arms, not a
   Rust match-arm per directive. Adding `%installs`/`%reflects` later is data.
3. **DELETES a special-case** — the `ImportAst` struct, its parser branch, and the
   `"import"` string-keyed strip in `resolve_imports` all go.
4. **Same-machinery test** — proven by the SAME compile/snapshot/integration
   suite that covers `@import` today (the parity gate), no bespoke harness.

---

## 7. Code anchors

- `src/parser/mod.rs:693` — `@import` → `ImportAst { path }` (the special-case to delete)
- `src/parser/ast.rs:37` — `ImportAst` struct (to delete after cutover)
- `src/parser/mod.rs:5913` — `resolve_imports` (the enactor's home; reuse its
  topological walk + `has_cycle`)
- `src/parser/mod.rs:6065` — `merge_ast` (gains namespace-aware variant)
- `src/parser/meta_ast.rs:616` — `RegistersClause` (the precedent shape for
  `ImportsClause`); `MacroDefAst` (add `imports` field)
- `stdlib/macros/presets.st:176` — `%macro import` (gains `%imports`, the
  self-describing migration target)
- `src/lsp/workspace/import_resolver.rs` — `ImportResolver` / `ResolvedImport`
  (the fs/stdlib primitive, unchanged)
- `src/metasystem/registry.rs` — `dispatch_ambiguity` (FEAT-088, reuse for the
  bare-name ambiguity error); M0 `Fqn`/`Namespace` keying
- `src/metasystem/module.rs` — M0 module identity (namespace assignment target)
