# FUP — Implicit Type Inference: Making Spacetime a Type-Inferred Language

> Status: RESEARCH / PROPOSAL
> Filed: 2026-06-06
> Provenance: surfaced while planning PLAN-036 ③/④ (token widgets, derived
> tokens). The maintainer correctly rejected the framing of "type inference as a
> widget convenience" — it is a question about Spacetime's **core semantics**:
> is the value `#FF0020` *known to be a color* by the language, or only by
> whatever ad-hoc code happens to look at it? This FUP documents what the
> repository actually does today, the fragmentation that absence-of-inference has
> caused, and a staged design to make literal type inference a first-class,
> single-source-of-truth property of the compiler.

---

## 0. TL;DR

Spacetime **already lexes** type evidence (`#FF0020` → `RawToken::Color`;
`600ms`/`8px` → `NUMBER_WITH_UNIT`) and **already has** a rich type vocabulary
(`CaptureType` has `Color`, `Length`, `Duration`, `Easing`, …). But that evidence
is **discarded** the moment a value is captured without a *declared* type: the
events grammar reifies it to a generic `CapturedValue::Expr`/`String`. Type is
known **only by declaration**, never **by inference**.

The cost is **four independent, drifting re-implementations** of "what kind of
value is this string?" — in the compiler, the sync protocol, the LSP, and the
visual linter — none of which is the source of truth, all of which can disagree.

This FUP proposes promoting the lexer's latent type evidence into a real
**inferred type on the value node**, consumed by a single classifier, so that
*every* downstream consumer (admin widgets, derived-token type-checking, LSP
color swatches, visual lint, emit) reads one answer. This is the difference
between "Spacetime has color types where you declare them" and "Spacetime *is* a
type-inferred language."

---

## 1. What the repository actually does today (grounded)

### 1.1 The lexer already extracts type evidence — then it's dropped

`src/syntax/cst/lexer.rs` tokenizes literals with their type already
distinguished:

- `SyntaxKind::Color` (lexer.rs:75) — `#hex` is its own token kind.
- `SyntaxKind::NUMBER_WITH_UNIT` (lexer.rs:315,327) — the lexer **merges**
  `NUMBER + unit-ident` (`px`/`em`/`rem`/`vh`/…) and `NUMBER + %` into one typed
  token. The unit table lives at lexer.rs:220.
- `Percent`, `Hash`, `Number`, `LeadingDecimalNumber` — all distinct.

So at the token layer, **`#FF0020` is already a Color and `600ms` is already a
dimensioned quantity.** This is the keystone: the evidence is *produced* and then
*thrown away*.

### 1.2 Capture is declaration-driven, with ZERO inference

`convert_property_value(value_str, capture_type)`
(`src/syntax/events/form_compiler.rs:1581`) is the whole story:

```rust
match capture_type {
    CaptureType::Color  => CapturedValue::Color(clean),   // ONLY if declared :color
    CaptureType::Number => clean.parse().map(Number).unwrap_or(String),
    CaptureType::Time | CaptureType::Duration => Expr(clean),
    CaptureType::Ident  => Ident(clean),
    _ => Expr(clean),                                       // everything else → opaque
}
```

The classification is a pure function **of the declared `CaptureType`**, not of
the value's shape. A `#FF0020` arriving where the macro form declared
`$x:color` becomes `Color`; the *same literal* arriving in any untyped position
becomes `Expr`. There is no `infer_*`, no `looks_like_color`, no value-shape
branch anywhere in the capture path (verified: `grep infer_type|is_color|
classify` in `src/syntax/**` returns only `classify_construct_start`, which is
about grammar productions, not value types).

### 1.3 The type vocabulary is already rich

`CaptureType` (`src/parser/meta_ast.rs:434`) and `CapturedValue`
(`src/syntax/form_match.rs:21`) both enumerate the domain scalars we'd want to
infer:

| CaptureType | CapturedValue | @type primitive (type_system.rs) |
|---|---|---|
| `Color` | `Color(String)` | `color` → `{type:string, format:color}` |
| `Length` | `Length(LengthValue{value,unit})` | `length` → `format:length` |
| `Duration` / `Time` | `Time(u32)` | `duration` → `format:duration` |
| `Easing` | — (falls to `Ident`/`Expr`) | — |
| `Number` | `Number(f64)` | `number` |
| `Bool` | `Bool(bool)` | `boolean` |

Note the gaps already visible: `Easing` has a `CaptureType` but **no**
`CapturedValue` variant and **no** `@type` primitive — it degrades to `Ident`.
The vocabulary is *almost* aligned across the three layers but not quite, because
nothing forces them to agree.

### 1.4 The widget pipeline proves the value of declared types — and its ceiling

`primitive_to_schema` (`src/type_system.rs:613`) maps a **declared** `@type`
field to a `format`; `annotate_widgets` (`src/server.rs:~3210`) maps `format` →
admin widget:

```
@type Brand { scarlet: color }  →  format:color  →  &fld-color (swatch)   ✓
@scroll reveal(easing: ease-out) →  Ident          →  &fld-text  (generic) ✗
$brand : { scarlet: #FF0020 }    →  needs @type Brand to know it's a color
```

This is why **Brand works and Motion doesn't** (FEAT-106): Brand fields carry a
declared `@type`; motion params and bare literals don't, so they fall back to a
text box. The intelligence is real but **opt-in per declaration**.

### 1.5 The `@data inline` singleton discards its literal's types

`%macro data-inline` (`stdlib/macros/data-kind.st:22`):

```
@data inline $name:binding $type:typeref? : $value:expr ;
```

Two things to notice:
1. `$type:typeref?` is **optional** — and in practice absent. The brand singleton
   relies on a *separate* `@type Brand { … }` matched by name to recover types.
2. `$value:expr` captures the entire `{ scarlet: #FF0020, radius: 8px, … }` as
   **one opaque expression string**. Inside it, `#FF0020` (lexed as Color) and
   `8px` (lexed as NUMBER_WITH_UNIT) are flattened to text. The object literal's
   per-field type evidence never survives capture.

∴ Even though the literal *visibly* encodes its types, the singleton must lean on
a redundant `@type` declaration to recover what the value already showed.

---

## 2. The fragmentation: four classifiers, no source of truth

Because the compiler does not infer value types, **every consumer that needs to
know "is this a color?" rolled its own classifier.** These are real,
independent, drift-prone implementations found in the tree:

### 2.1 `sync/protocol.rs::classify_value` — the most complete shadow inferrer

`classify_value(raw) -> (ValueType, ControlHint)`
(`src/sync/protocol.rs:281`) is a **fully-formed value-shape inferrer** that the
rest of the compiler doesn't know exists:

```rust
if trimmed.starts_with('#') && (len==7||len==4) → ValueType::Color, ColorPicker
if starts_with("rgb")||starts_with("hsl")       → ValueType::Color, ColorPicker
if ends_with("ms")                              → ValueType::Duration{ms}
if ends_with('s')                               → ValueType::Duration{ms*1000}
for unit in [px,em,%,rem,vw,vh]                 → ValueType::Dimension{value,unit}
"ease-*" | "cubic-bezier(" | "linear"           → ValueType::Easing
```

`ValueType` (protocol.rs:215) duplicates the domain-scalar set:
`Color | Duration | Dimension | Easing | Number | Str | Identifier`. `ControlHint`
(protocol.rs:237) is a *third* parallel to `format` and admin widget names.

This is the inference we want — **built, tested (19 `test_classify_value_*`
cases), and walled off in the sync layer**, originally for the
`dev_theme_handler` CSS-custom-property scrape.

### 2.2 `lsp/colors.rs` — a second color detector

`provide_document_colors` (`src/lsp/colors.rs:13`, 322 lines) scans the document
byte-by-byte for `#` + hex digits to provide editor color swatches. Its own hex
parser, independent of `classify_value` and of `src/color/`.

### 2.3 `src/color/mod.rs` — the canonical color *value* parser (but not a detector)

`Color::from_hex`, `from_rgb_string`, `from_hsl_string`, `from_oklch_string`,
`parse_color_to_normalized_rgba` (`src/color/mod.rs`). This is the *real* color
domain model — used by emit/glsl, interpolation, etc. — but it parses a string
*known* to be a color; it is not the thing that *decides* a string is a color.
So the three detectors above each re-derive "is this a color?" before (sometimes)
delegating here.

### 2.4 `analysis/visual_lint.rs` — yet another color-aware pass

36 `color`/`Color` references; reasons about colors for contrast/brand linting
with its own notion of where colors are.

### 2.5 The scorecard

| Layer | "is this a color?" | "is this a duration?" | "→ control/widget?" |
|---|---|---|---|
| compiler capture | only if declared `:color` | only if declared | — |
| `sync/classify_value` | shape-infer ✓ | shape-infer ✓ | `ControlHint` |
| `lsp/colors` | byte-scan ✓ | ✗ | LSP ColorInformation |
| `server/annotate_widgets` | from `format` | from `format` | admin widget name |
| `visual_lint` | its own ✓ | — | — |

**Five columns, no shared row.** This is precisely the "one need = one
implementation" violation the project's architecture rules call out. Any new
feature that needs value types (FEAT-107 picker, FEAT-108 derived-token checking)
faces a choice: pick one of the existing silos, or add a sixth.

---

## 3. Why this is foundational, not cosmetic

The maintainer's framing is exact. The widget richness is a *symptom*. The
foundational facts:

1. **Type evidence is produced then destroyed.** The lexer knows `#FF0020` is a
   Color and `8px` is a length-quantity. A type-inferred language would *carry*
   that to the value node. Spacetime drops it at capture. So Spacetime today is a
   *type-annotated* language (types where declared) wearing the costume of a
   type-inferred one (a lexer that secretly knows).

2. **Inference is the precondition for value-level type-checking.** Without it,
   `$hover : $brand.scarlet | darken(0.08)` (FEAT-108) cannot be checked —
   `darken` has to *trust* its argument is a color. With inference,
   `darken : (color, number) → color` becomes enforceable; `darken($brand.radius)`
   (a length) is a compile error. Type inference turns the pipe/`@fn` surface
   from "stringly-typed plumbing" into a checked algebra.

3. **It collapses the fragmentation.** One inferred type on the value node, one
   classifier, and `classify_value` / `lsp/colors` / `annotate_widgets` /
   `visual_lint` all become *readers* of the same fact instead of independent
   guessers. This is a net **deletion** of code and a removal of drift surface.

4. **It makes the metasystem honest.** The whole AGENTS.md thesis is a
   self-describing metasystem: grammar + lowering in `.st`, Rust a generic
   engine. A type system that only knows what you *redundantly declare* is not
   self-describing about *values*. Inference closes that gap: the value describes
   itself.

5. **It is universal, not opt-in.** Today, widget intelligence requires a
   declared `@type`. With inference, *every* color literal in the codebase — in
   motion params, raw CSS, `@data` blobs, component defaults, a one-off style —
   carries its type, and every tool gets richer for free, with zero annotation.

---

## 4. The design: a staged path to inferred value types

The guiding constraint (from AGENTS.md): **no Rust parser for an stdlib
construct; the metasystem is self-describing.** Inference must respect that — the
*vocabulary* of inferable types should ultimately be declarable, and the *engine*
generic. Staged so each tier ships value independently.

### Tier 0 — Unify the classifier (no new semantics, pure de-dup)

Before adding inference, collapse the four shadow classifiers into one. Promote
`classify_value` into a canonical `src/types/value_infer.rs` (or fold into
`src/color/` + a sibling), expressed in terms of the existing `CaptureType` /
`CapturedValue` vocabulary, and make `lsp/colors`, `annotate_widgets`,
`visual_lint`, and the sync layer all call it. **Net code deletion. Zero
behavior change.** This alone is worth doing and de-risks everything after.

- Risk: low. Each call-site swap is mechanical and independently testable.
- Proof: the existing 19 `test_classify_value_*` cases move with the function;
  add cross-consumer tests asserting LSP swatches and admin widgets agree.

### Tier 1 — Infer unambiguous literals at capture (the keystone)

Make `convert_property_value` (and the object-literal reify path) consult value
*shape* when no `CaptureType` is declared, for the **unambiguous** literals the
lexer already distinguishes:

```
#hex / rgb()/hsl()/oklch()  → CapturedValue::Color
NUMBER_WITH_UNIT(px,em,%,…)  → CapturedValue::Length
NUMBER_WITH_UNIT(ms,s)       → CapturedValue::Time
NUMBER                       → CapturedValue::Number
true/false                   → CapturedValue::Bool
```

These have **no ambiguity** — `#FF0020` is *never* not a color; `600ms` is
*never* not a duration. This is the "pure win" tier: it can't surprise anyone.

Crucially, **prefer the lexer token kind over re-parsing the string** — the
lexer already emitted `SyntaxKind::Color` / `NUMBER_WITH_UNIT`. Tier 1 is largely
"stop discarding the token kind," not "add a new parser." That keeps it honest
re: the no-Rust-parser rule (we're reading the existing tokenization, not
re-lexing in an ad-hoc way).

- Consumers light up automatically: brand singleton no longer needs `@type` to
  render swatches; motion `duration`/`length`/`stagger` params get correct
  widgets; FEAT-107's picker applies to *any* color literal.
- Risk: medium. Some positions currently *rely* on getting an opaque `Expr`
  (e.g. an expression that merely starts with a number). Mitigate: infer only
  for **whole-token** matches (the entire value is one `Color`/`NUMBER_WITH_UNIT`
  token), never partial; everything else stays `Expr`. Gate behind emitted-JS +
  golden-snapshot diffs across the corpus.

### Tier 2 — Contextual inference from the bound param type (idents/numbers)

The ambiguous cases — `ease-out` (easing? ident?), `0` (number? unitless
duration?), `"first"` (enum? string?) — cannot be inferred from shape alone. But
the macro's `%form` **already declares the param type** (`$easing:easing`,
`$stagger:number`). That `CaptureType` is *present at capture time* and is simply
not propagated to the widget/inference layer.

Tier 2 = thread the declared param `CaptureType` through to the value node's
inferred type (and thus to `annotate_widgets`), so `easing: ease-out` surfaces an
**easing enum control** because the *macro* said the param is `:easing`, even
though the literal looks like a bare ident. This is "inference from context the
compiler already has," not new guessing.

- This is what makes FEAT-106 motion params *fully* typed (today they're all
  text because the param `CaptureType` isn't carried to the server contract).
- Requires `CaptureType::Easing` to gain a `CapturedValue::Easing` variant + a
  `@type easing` primitive (close the vocabulary gap from §1.3).

### Tier 3 — Inferred types in the type-checker (value-level checking)

With value nodes carrying inferred types, extend the `TypeRegistry` /
`generate_json_schema` machinery so `@fn` signatures over domain scalars are
**checked against inferred argument types**:

```
@fn darken($c color, $amt number) : color { … }
$hover : $brand.scarlet | darken(0.08)   ✓  scarlet: color
$bad   : $brand.radius  | darken(0.08)   ✗  radius: length, expected color
```

This is the tier that turns FEAT-108 derived tokens from "trust the pipe" into "a
checked color algebra," and gives the language real value-level type errors with
miette spans.

- Risk: higher (it's a type checker). But it's *additive* — inference produces
  the types; checking is a separate pass that can start as warnings.

### Tier 4 — Self-describing inference (the metasystem-honest endpoint)

Ultimately the *set* of inferable literal shapes should be declarable in stdlib,
not hardcoded in Rust — e.g. a `%infer` / `%literal` form that says "a token
matching `#[0-9a-f]{3,8}` infers as `color`." Then adding a new inferable domain
scalar (say `angle` for `45deg`) is an stdlib stanza, zero Rust. This mirrors the
`%capture_type` story (PLAN-023 W2 migrated properties/keyframes/params to
stdlib). Inference becomes the value-layer dual of the self-describing grammar.

- This is the north star, not a near-term ask. Tiers 0–2 deliver ~90% of the
  practical value; Tier 4 is what makes it *Spacetime-shaped* rather than a bolt-on.

---

## 5. Interaction with PLAN-036 ③/④

- **FEAT-107 (token widgets)** does **not require** inference to ship — the
  perceptual picker is a richer `&fld-color` template + pure `@fn` HSV math,
  driven by the *declared* `format:color` that brand fields already carry. But
  with Tier 1, the picker would apply to *every* color literal, not just
  `@type`-declared fields.
- **FEAT-108 (derived tokens)** does **not require** inference to ship its
  *runtime* (the `@data derive` reactivity is built). But its *safety* (rejecting
  `darken(aLength)`) is exactly Tier 3. Without inference, FEAT-108 is
  correct-when-authored-correctly; with it, FEAT-108 is *checked*.

∴ Recommendation: ship ③/④ on declared types now (they're self-contained and
high-impact), and pursue the inference tiers as a parallel foundational track
whose first deliverable (Tier 0 de-dup) is pure cleanup with no feature risk.

---

## 6. Concrete first PR (if/when greenlit)

**Tier 0 + a sliver of Tier 1**, scoped tight:

1. Create `src/types/value_infer.rs`: move `classify_value` here, rename to
   `infer_value_type(&str) -> InferredType` returning a unified enum that maps
   1:1 to both `format` (server) and `CaptureType` (compiler). Keep all 19
   tests, add the `oklch`/named-color cases `classify_value` currently misses.
2. Make `lsp/colors`, `annotate_widgets`, `visual_lint`, and the sync caller all
   delegate to it. Delete the duplicate hex scanners. Assert (new test) that the
   LSP swatch set and the admin color-widget set are identical for a corpus file.
3. In `convert_property_value`, for an **undeclared** capture whose source token
   kind is `SyntaxKind::Color` or `NUMBER_WITH_UNIT`, emit `Color`/`Length`/`Time`
   instead of `Expr` — behind a golden-snapshot gate across `examples/` +
   `tests/fixtures/` to prove zero emit regressions.

Acceptance (per methodology — GREEN = emitted/behavioral, not `cargo check`):
- `$brand : { scarlet: #FF0020 }` with the `@type Brand` line **removed** still
  renders a color swatch in the admin (inference, not declaration, drove it).
- A bare `.x { foo: #abcdef }` literal gets an LSP color swatch *and* the admin
  widget engine agrees it's a color — from the **same** call.
- Full corpus emit snapshots unchanged for all non-color/length/time values.

---

## 7. Risks, non-goals, open questions

- **Non-goal:** full Hindley-Milner / flow inference. This is *literal-shape +
  declared-context* inference for domain scalars, not general type reconstruction.
- **Risk — over-inference:** a string that merely *looks* like a color in a
  free-text field. Mitigation: Tier 1 infers only whole-token unambiguous
  literals; free text (`String` capture) is never re-inferred.
- **Risk — two parsers (CST vs events grammar):** per the standing rule, both
  parse paths must agree. Tier 1 should read the **lexer token kind** (shared
  substrate) rather than re-deciding in each grammar, which keeps them aligned by
  construction.
- **Open:** should inferred types be *visible* in `@type`-less code as real types
  (i.e. can you write `darken($x)` where `$x` is an inferred-color binding)? That's
  the Tier 3 boundary and needs its own design.
- **Open:** `oklch()`/`color-mix()` and modern CSS color — `src/color/` handles
  them; the shadow `classify_value` does not. Tier 0 must reconcile to the richer
  `src/color/` capability.

---

## 8. Appendix — file map (evidence)

| Concern | File:symbol |
|---|---|
| Lexer color/dimension tokens | `src/syntax/cst/lexer.rs:75` (Color), `:315/:327` (NUMBER_WITH_UNIT), `:220` (unit table) |
| Declaration-driven capture (no inference) | `src/syntax/events/form_compiler.rs:1581` `convert_property_value` |
| Type vocabulary | `src/parser/meta_ast.rs:434` `CaptureType`; `src/syntax/form_match.rs:21` `CapturedValue` |
| Shadow inferrer #1 (sync) | `src/sync/protocol.rs:281` `classify_value`; `:215` `ValueType`; `:237` `ControlHint` |
| Shadow inferrer #2 (LSP) | `src/lsp/colors.rs:13` `provide_document_colors` |
| Canonical color value model | `src/color/mod.rs` `Color`, `from_hex`, `parse_color_to_normalized_rgba` |
| Color-aware lint | `src/analysis/visual_lint.rs` |
| Declared-type → widget | `src/type_system.rs:613` `primitive_to_schema`; `src/server.rs:~3210` `annotate_widgets` |
| Singleton discards literal types | `stdlib/macros/data-kind.st:22` (`$value:expr`, `$type:typeref?`) |
