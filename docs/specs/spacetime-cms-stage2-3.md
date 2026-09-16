# Spacetime CMS — Stage 2 & Stage 3 Deep Design

**Status:** Design exploration (pre-plan). Companion to `docs/specs/spacetime-cms.md`.
**Author:** 2026-06-01, with user direction folded in inline.
**Audience:** A fresh agent designing/planning Stage 2 (Squarespace killer) and
Stage 3 (Framer killer). Read `spacetime-cms.md` first for V1, the polarities,
and the thesis. This document goes deep on the three richest veins, resolves
several forks the user has now decided, and records the new opportunities,
questions, and challenges those decisions surface.

Mockups (in-repo, `docs/specs/cms-mockups/`):
- Vein 1 — Page builder: `cms-mockups/stage3-page-builder.png`
- Vein 2 — Block rich text: `cms-mockups/stage2-richtext-blocks.jpg` (alt: `…-alt.jpg`)
- Vein 3 — Relations + graph: `cms-mockups/stage2-relations-graph.png`
- V1 admin (prior): `cms-mockups/v1-talent-admin.png`

---

## 0. User decisions folded into this document

These were open in the V1 spec; the user has now decided them. They are
**binding** for Stage 2/3 design.

| # | Decision | Consequence |
|---|---|---|
| 2A | Relations use an **`id(Agent)` type**, not a bare `Agent` type. `Agent` alone would mean *nesting* the data; `id(Agent)` is a typed **reference**. | Needs a parameterized type `id(T)`. Codebase is settling on **parentheses for type arguments** (cf. capture maps, `$v:balanced(';')`), so `id(Agent)` is idiomatic. |
| 2B | Rich text: **serialize an html5ever document tree + Spacetime traits fully over the wire.** Uncompromising — the body *is* a real parsed DOM carrying Spacetime semantics, sent/received whole. | Rejects the "tag-field workaround." Commits to a real document model. `html5ever 0.39` is already a dependency. |
| 3A | The admin should expose the **whole Spacetime surface** — a true page builder, not a toy. | The builder is typed composition over the project's `@template` registry, which already stores `params: param_list`. |
| 3B | Brand/theme handled in a **Spacetime-native way as nice and easy as `@type`** — editable from the UX. | Implies a first-class `@theme`/token construct the admin reads & writes, not ad-hoc `_theme.st` string-patching. |
| 3C | Source round-trip = **EditAst, already handled.** | Stage 3 `.st` writes ride the existing `handle_edit_ast` surgical-patch path (`src/sync/handlers.rs`). Not a new problem. |
| 3D | Wants **mockups + a discussion of building the admin in Spacetime** for compile-time hydration. | Covered in §4 and §5.4. |

---

## 1. The spine (refresher)

```
V1    admin reads TYPES        → generates FORMS         writes data/*.json
SQS   admin reads TYPE GRAPH   → generates EDITORS       writes data/*.json (richer)
FRMR  admin reads TEMPLATES    → generates COMPOSER      writes @type + .st (source)
```

One property deepening across stages: **the admin understands progressively more
of the `.st` program and writes progressively more of it back.** V1 = content.
Stage 2 = the content graph + structured documents. Stage 3 = the source itself
(sections, pages, theme, motion). Always local, generated, git-diffable, no
foreign format, no custom JS (the admin is itself a Spacetime site).

Substrate facts verified in-repo (trust but verify at these paths):
- `html5ever 0.39`, `lol_html 2` — both already in `Cargo.toml`.
- Parameterized types via parens already parse (`src/parser/mod.rs:3583`,
  capture maps in `stdlib/capture-types/param_list.st`).
- `TypeExpr` today = `Primitive | Array | Object | Union(Vec<String>) | Reference`
  (`src/parser/ast.rs:348`). **String-only unions.** No `id(T)` yet, no object
  unions yet — both are additive.
- The state-machine subsystem already has **object unions with payloads**
  (`meta_ast.rs:136` `Union(Vec<UnionVariant>)`,
  `Connected { $send } | Error { $error }`) — proven code to borrow if needed.
- Template registry already stores `params: param_list` =
  `[{name, kind: binding|element, optional}]` (`stdlib/macros/template.st:110`).
  **This is the entire foundation for typed page composition** (§3).

---

# VEIN 3 — Relations as `id(T)` (the model that makes Stage 2 a graph)

(Presented first because the `id(T)` type decision ripples through 2B and all of
Stage 3.)

## 3.1 The decision: `id(Agent)`, not `Agent`

A relation field is a **reference**, not embedded data:

```
@type Talent {
  name: string;
  agent: id(Agent);          // single relation  → stored as an id string
  campaigns: id(WorkCard)[];  // multi relation   → stored as an array of id strings
}
```

- `id(Agent)` reads as "an id **of** an Agent." It is a **parameterized type**:
  the type constructor `id`, applied to the type argument `Agent`. This matches
  where the codebase is going (parens for type args).
- A bare `Agent` field would mean **nesting** a full Agent object inside Talent's
  JSON — denormalized, duplicated, un-editable-as-its-own-entry. `id(Agent)`
  keeps `Agent` a first-class collection and `Talent.agent` a pointer. This is
  the normalized, relational, correct model.

### Storage
`id(Agent)` serializes to the **bare id string** already used as the entry key:

```json
// data/talent.json
{ "name": "Carolina Mendes", "agent": "maria-fontes",
  "campaigns": ["vogue-ss25", "chanel-beauty"] }
```

This matches `unkn`'s existing `id`/`slug` convention exactly — no envelope, no
`{"$ref":…}`. **Rename safety** (renaming an entry's id breaks referrers) is a
known limitation deferred to a follow-up (see §6 Challenges); bare ids win for
V1-of-Stage-2 because they're already the convention and stay git-legible.

## 3.2 What the type system needs

`id(T)` is a new `TypeExpr` variant — a *type application*:

```
TypeExpr::Apply { ctor: "id", args: vec![TypeExpr::Reference("Agent")] }
   (or a dedicated TypeExpr::Ref(String) if we special-case `id` — but a general
    Apply node is the more extensible primitive; see "Architecture principle 1:
    the right primitive answers questions you never posed.")
```

- **JSON-schema** (`src/type_system/generate_json_schema`): `id(Agent)` →
  `{ "type": "string", "x-st-ref": "Agent" }`. The `x-st-ref` annotation is what
  the admin reads to render a picker instead of a text input.
- **Validation**: a value for `id(Agent)` must be a string that exists as an id
  in the `Agent` collection — a *referential* check the type system can offer
  (warn-level locally).
- **Resolution for display**: given `agent: "maria-fontes"`, the admin reads
  `@data agents` and looks up the entry to render its title/thumbnail in the chip.

> **Hidden opportunity — the graph view falls out for free.** Once every relation
> is a typed `id(T)` edge, the set of all `@data` collections + all `id(T)` fields
> *is* a directed graph. The Anytype-style "Graph view" (the attached reference
> image; mockup `34.png`) is then a pure read over the type+data registry —
> nodes = entries, edges = `id(T)` fields, edge labels = field names. No extra
> modeling. This is the single biggest "wow" per unit of work in Stage 2.

> **Hidden opportunity — `@computed` saved views.** `@computed … { where: $expr }`
> already exists (`stdlib/macros/type-data.st`). Surface it in the admin and you
> get **structured saved views** ("Talents in Fashion", "Work from 2025") as
> first-class navigation objects — without GROQ, without hand-written queries.
> Relations + computed = a content-ops layer Squarespace simply does not have.

## 3.3 The admin surface (mockup `34.png`)
- Single relation → a **chip + searchable picker** over the target collection.
- Multi relation `id(T)[]` → **multi-chip** field with add/remove/reorder
  (reusing `EditJsonArray` semantics on the id array).
- `List | Graph` segmented control; Graph view renders the `id(T)` edge set.
- The `id(Agent)` signature is shown in muted monospace beside the field label —
  the type is part of the UX, teaching the model as you use it.

## 3.4 Challenges / questions (Vein 3)
- **Referential integrity on delete.** Deleting `maria-fontes` while Talents
  reference her → dangling ids. Options: warn-on-delete (scan referrers),
  soft-block, or tolerate (dev-mode). Recommend **warn-on-delete** for Stage 2.
- **Rename propagation.** Changing an entry's id should update referrers.
  Deferred (bare-id limitation). A follow-up could add an opt-in
  `id`-stability/rename-cascade. *Carry as a Stage-2 FUP.*
- **Bidirectional relations.** Does `Talent.agent → Agent` imply
  `Agent.talents`? Anytype shows backlinks ("Related blocks" in the reference
  image). Recommend **computed backlinks** (derive, don't store) — the graph
  already knows the inverse edges.

---

# VEIN 2 — Rich text as a *constrained Spacetime AST* (allowlist/denylist)

> **Reframed by the user, 2026-06-03 — this is now the canonical model.**
> Original framing: "serialize an html5ever tree + bespoke Spacetime traits over
> the wire." The user asked the sharp question: *what is the difference between
> that and a Spacetime AST stripped of `%`-directives, with an allowlist/denylist?*
> **Answer: there is no difference — the AST framing is the correct primitive,
> and the html5ever-tree framing was over-engineered.** This section is rewritten
> accordingly. It removes a bespoke wire schema AND a bespoke sanitizer; both
> collapse into one allow/denylist over the AST that already exists.

## 2.1 The realization: rich text IS a `HtmlExpr` subtree

Spacetime **already** parses HTML (in `.st` component bodies and in served HTML)
via an html5ever custom `TreeSink` into its own IR — verified at
`src/html/treesink.rs:1` ("html5ever custom TreeSink → `ir::HtmlExpr`") and
`src/ir/code.rs:299`:

```rust
pub enum HtmlExpr {
    Element { tag: String, attrs: Vec<(String, Vec<AttrPart>)>, children: Vec<HtmlExpr> },
    Text(String),
    Hole(JsExpr),   // an embedded Spacetime expression hole (e.g. $.image)
    Raw(String),
}
```

html5ever is therefore **not a parallel tree** — it is already the front-end that
produces the HTML portion of the Spacetime AST/IR. The "Spacetime traits" the
previous draft invented (a `@scroll` on a quote, a `$binding` on an image `src`)
**are just AST nodes** — an `Element` with a directive child, an `AttrPart::Hole`.
Naming them "traits" and inventing a wire schema described the AST and gave it a
new name. ∴ **Rich text = a Spacetime AST subtree, constrained by an
allowlist/denylist of node kinds.**

```
@type Article {
  title: string;
  body: richtext;     // = a constrained HtmlExpr/AST subtree, NOT a new tree type
}
```

## 2.2 The allowlist / denylist (this single list replaces 3 subsystems)

The model is one policy applied to AST node kinds:

```
ALLOW  (structure)    h1 h2 h3 · p · ul/ol/li · blockquote · img · a · figure/figcaption
ALLOW  (inline marks) strong · em · code · br        ← the "hard 20%" is NATIVE AST, free
ALLOW  (Spacetime)    AttrPart::Hole ($.field bindings) · a curated set of motion
                      directives on blocks (@scroll/@reveal) — animatable prose
DENY   (out of scope) %primitive · %emit js · %macro · <script> · <style> · <iframe>
                      · on* event attrs · javascript: URLs · form controls
```

What this collapses:
- **No bespoke wire schema** (was Q6 / `FUP-richtext-document-schema`): the wire
  format is the **existing AST serialization** that `EditAst` already round-trips
  (`src/sync/protocol.rs`, `handle_edit_ast`; user confirms 3C "taken care of").
  `JsonValue` (`src/parser/ast.rs:362`) / the IR serde already exist. The schema
  work shrinks to *"define the allowlist,"* not *"design a document format."*
- **No bespoke sanitizer** (was Q8 / `FUP-spacetime-sanitizer`): sanitization
  **is** the denylist — a node-kind filter over the AST, not a regex HTML
  scrubber. This also honours `AGENTS.md`: *don't write a Rust HTML
  parser/fallback; the metasystem is self-describing.* The list is the policy.
- **No object-union TypeExpr** (was Q9): blocks are AST nodes, not union
  variants. Already true; now firmly unnecessary for rich text.

> The previous draft's two FUPs (`richtext-document-schema`,
> `spacetime-sanitizer`) are **superseded** by a single, smaller artifact:
> `FUP-richtext-ast-allowlist` (FUP-029) (define the allow/deny policy + where it is
> enforced). See §7.

## 2.3 Why this is still uncompromising (the dividends survive)

Nothing good is lost by dropping the bespoke framing — the power was always in
the AST:

- **Inline marks are native** — `<strong>`/`<em>`/`<a>` are ordinary `Element`
  nodes. The hard 20% of rich text needs no inline-span model.
- **Animatable prose** — because a block is an AST `Element`, it can legally
  carry an allowlisted `@scroll`/`@reveal`. A Quote can reveal; a paragraph can
  stagger in. No CMS has animated structured content; their rich text is inert.
- **Bindings inside prose** — `AttrPart::Hole` means `<img src=$.image>` works
  inside a body. Content can reference data.
- **Provenance already addresses every node** — `data-st-id` / `data-st-origin`
  from `docs/CONTENT_EDITING.md` apply unchanged; editing a block round-trips via
  the existing content-edit path.
- **The editor is generated, and extensible by declaration** — the block-type
  menu is the *allowlist* rendered as choices. A developer who allowlists a new
  node kind (or registers a block `@template`) makes it appear in every writer's
  menu. Extensible by editing a list / declaring a type — never by editing the
  editor. The metasystem dividend, applied to prose.

## 2.4 The admin surface (mockup `cms-mockups/stage2-richtext-blocks.jpg`)
- Body renders as a vertical stack of **blocks** = the top-level `HtmlExpr`
  children of the `richtext` value; hover gutter (drag handle + node-kind tag).
- `+` insertion between blocks; the block-type menu = the **allowlist**.
- Floating **inline toolbar** (Bold/Italic/Link) wraps the selection in
  `strong`/`em`/`a` AST nodes — denylist rejects anything outside policy.
- Reading-width column, editorial, calm.

## 2.5 Challenges / questions (Vein 2, revised)
- **Where is the allow/denylist enforced?** Three candidate points, likely all:
  (a) the admin editor only *offers* allowlisted kinds; (b) the server validates
  an incoming `EditAst` body against the policy before persisting; (c) the
  compiler/`TreeSink` already rejects `%`-directives in this context. Server-side
  (b) is the security-bearing one. *Core of `FUP-richtext-ast-allowlist` (FUP-029).*
- **Storage locus.** Inline serialized AST subtree inside the entry JSON, vs a
  sidecar `.st`/`.html` fragment addressed by provenance? Inline = simple, one
  file; sidecar = cleaner git diffs + reuses provenance addressing. *Open — §6 Q7.*
- **Is `richtext` a distinct `TypeExpr`, or `Reference` to a built-in?** Adding a
  `richtext` primitive type is the smallest change; it signals "this string field
  is an allowlisted AST," driving the admin to render the block editor. *§6 Q6.*
- **Round-trip into `.st` templates.** A body rendered via a block `@template`
  must round-trip via EditAst (3C, handled).

---

# VEIN 1 — Page-builder-as-typed-composition (the Framer killer core)

The vein the user most wants to read. Mockup: `31.png`. This is where the admin
exposes the **whole Spacetime surface** (3A) as a visual design tool whose every
move emits clean `.st`.

## 1.1 The key realization: composition is already typed

A Spacetime page is **already** an ordered list of template invocations.
`unkn`'s `index.st` literally composes the homepage this way:

```
.site-header { &unkn-nav() }
…           { &unkn-hero() }
…           { &unkn-mission() }   // etc. — a page IS a section list
```

And the template registry **already stores each template's typed interface**
(`stdlib/macros/template.st:110`, `params: param_list`):

```
param_list = [ { name, kind: "binding" | "element", optional: bool }, … ]
   binding param ($headline, $service)  →  a DATA need
   element param (&content, &footer?)   →  a SLOT (composition point)
```

∴ **The compiler already knows, for every section/template, what data it
consumes and what slots it offers.** The page builder is not inventing a
component model — it is *visualizing the one that already exists.* This is the
unlock that makes a Framer-grade builder tractable.

## 1.2 Typed composition = structurally valid pages by construction

Framer lets you drop anything anywhere — soup. Spacetime can do better: because
templates have **typed params and typed slots**, the builder only permits
placements whose requirements are satisfiable.

```
PLACEMENT RULE
  to place &service-card($service) you must bind $service to:
     • an entry of type Service        (a single)
     • or an @each over Service[]      (a collection view)
  to fill &modal(&content) you may drop any template/markup into the &content slot.
  optional params (&footer?) may be left empty; required ones cannot.
```

The builder is a **type-checked editor**: a section is placeable iff its bindings
can be satisfied in the current scope, and a slot accepts a child iff the child
is a valid element. **A page is correct by construction** — you cannot build a
broken composition. This is a category Framer cannot reach, because Framer has no
type system under the canvas.

> **Hidden opportunity — the palette writes itself.** The "Components" palette
> (mockup `31.png` left rail) is just `registry.templates()` rendered with their
> signatures. Add a `@template` to the project → it appears in the palette, with
> its data needs and slots shown as chips. The design tool's component library is
> **the project's own templates**, auto-discovered, zero registration. Framer
> makes you build components in Framer; Spacetime's components are the code.

## 1.3 What "the whole Spacetime surface" means (3A) — the world-class UX

The user wants the admin to expose *all* of Spacetime, not a watered-down subset.
Mapping the design-tool affordances to the constructs that already exist:

```
DESIGNER ACTION            EMITS / EDITS                         BACKED BY
──────────────────────────────────────────────────────────────────────────────
add a section              insert &template() into page .st       template registry + EditAst (3C)
reorder sections           reorder invocations in page .st        EditAst
bind a section to data     set @each(source) or a single $binding  @data / @each (exists)
fill a slot                drop child into &slot                   element params (exists)
edit text in a section     in-context content edit                 dev-editable + EditAst (exists)
edit a section's motion    tune @scroll/@reveal params (slider)    EditAst + PROJ-097 visual-controls
restyle via tokens         pick a theme token                      @theme tokens (Vein 3B, §1.5)
create a new page          scaffold pages/<route>.st               new — page templating
make a new component       promote a selection to a @template      new — "extract component"
```

### World-class UX principles for the builder (synthesizing Framer + Squarespace + Anytype)
1. **Direct manipulation on the real rendered page** (mockup `31.png` center) —
   not a wireframe. The canvas is the **compile-time-hydrated** site (§Stage 3D),
   so what you arrange is what ships. (This is *why* 3D hydration becomes
   necessary at Stage 3.)
2. **Selection reveals structure, not chaos** — selecting a section shows its
   *typed* inspector (DATA / PARAMS / SLOTS / MOTION / STYLE), so the designer
   learns the model by touching it. The type is the teacher.
3. **Insertion is guided** — drop-zones appear only where a section is valid
   (typed composition). No invalid states to undo.
4. **Two-way with code** — a "View .st" toggle shows the emitted source live.
   Designer and developer see one artifact. This *is* the differentiator: the
   builder's output is hand-quality `.st`, diffable in git, optimizable by the
   compiler — never Framer's opaque React-with-inline-styles.
5. **Motion is first-class** (mockup `31.png` MOTION panel) — a scroll-reveal is a
   slider for `duration`/`stagger` + an easing curve, because the motion is a
   *declaration* (`@scroll reveal { duration: 800ms }`) the tool reads and writes.
   **Visually tuning real, declarative, exportable motion is the single most
   differentiated feature in the whole product.**

## 1.4 Emitting `.st` — rides EditAst (3C, already handled)

Every builder mutation is a surgical `.st` patch through the existing
`handle_edit_ast` path. Adding a section = inserting a `&template()` invocation
node; reordering = moving nodes; retuning motion = patching a directive's
property. The user confirms EditAst round-trip is solved (3C), so Stage 3's
"admin writes source" is **not a new persistence problem** — it is new *UI* over
a solved *protocol*.

## 1.5 Brand/theme as a Spacetime-native construct (3B) — "as nice as `@type`"

The user wants brand handled the Spacetime-native way — a first-class construct
as easy as `@type`, editable from the UX. Today `unkn` keeps tokens in
`_theme.st` as `var(--unkn-*)` via convention + an `AGENTS.md` rule. Proposal: a
first-class **`@theme`** declaration (sibling to `@type`):

```
@theme unkn {
  color  accent      #FF0020;     // typed tokens — color/length/duration/font…
  color  ink         #000000;
  color  paper       #FFFFFF;
  font   display     "Oranienbaum", serif;
  length gutter      24px;
  duration reveal    800ms;
}
```

- **Typed tokens** → the theme editor renders the *right control per token type*
  (color → swatch, length → slider, font → picker) by the same §4.3 widget logic
  as forms. Brand editing becomes **just another generated form** — "as nice and
  easy as `@type`."
- **Guardrails as a feature.** Because you edit *tokens*, not arbitrary values,
  brand integrity is structural: a designer picks `accent`, they cannot smear
  rust-orange where scarlet belongs (`unkn` `AGENTS.md` rule, enforced by
  construction). Framer gives infinite rope; Spacetime gives a *designed*
  palette. Constraint = feature.
- **One source of truth.** `@theme` compiles to the `:root { --unkn-* }` custom
  properties already in use; the admin reads/writes the `@theme` block via
  EditAst. No string-scraping `_theme.st`.

> This `@theme` construct is itself a **follow-up worth its own design** — it
> interacts with the existing `var(--*)` convention and the `_theme.st` pattern
> across projects. Carry as a Stage-3 FUP (see §6).

## 1.6 Challenges / questions (Vein 1)
- **"Extract component."** Promoting a canvas selection into a new `@template`
  (with inferred params) is the hardest builder feature — it's *generalization*
  (which sub-parts become `$params`?). High value (it's how designers grow the
  component library), genuinely hard. *Stage-3 deep-dive FUP.*
- **New-page scaffolding.** `pages/<route>.st` from a layout template + route
  registration. Mechanically straightforward; needs a page/route model the
  export pipeline understands.
- **Static vs dynamic section.** A placed section may be one-off copy or a
  collection view (`@each`). The builder needs an explicit "bind to data" step —
  the static/dynamic polarity resurfacing at the layout layer.
- **Canvas fidelity needs hydration.** The "edit on the real page" UX *requires*
  compile-time hydration (§Stage-3D) so the canvas matches the shipped output.

---

# §4. Building the admin IN Spacetime (the dogfood discussion, 3D)

The admin is itself a Spacetime site (`stdlib/__admin__/`, no custom JS). This is
not just ideological — it is the **ultimate proof** of the page builder: the
Framer killer is built with the thing it competes against. The admin's own UI is
`@type` (its view-models) + `@template` (its widgets) + `@each` (its lists) +
`@on`/`@scroll` (its interactions) + primitives wrapping unavoidable browser APIs
via `%emit js` (drag, file input — exactly as `__dev__` already does).

```
stdlib/__admin__/
  index.st                      entry; --debug gate; mounts the admin at /__spacetime/admin/
  primitives/
    admin-ws.st                 reuse/extend the dev WS (EditJson/EditJsonArray/EditAst/upload)
    admin-schema.st             fetch /__spacetime/dev/types.json → view-models
    admin-drag.st               drag/reorder (browser API wrapper)
    admin-graph.st              relation graph canvas (Vein 3)
    admin-richtext.st           html5ever document editor (Vein 2)
  macros/  (admin-internal)
    collection-list.st          @each over a collection → editable rows
    entry-form.st               schema → form (the §4.3 widget generator)
    relation-picker.st          id(T) → picker
    page-canvas.st              the typed-composition builder (Vein 1)
    theme-editor.st             @theme → token controls (3B)
```

> **Note on placement:** per the user's earlier directive, the admin lives in its
> **own folder `stdlib/__admin__/`** — NOT under `stdlib/macros/` or
> `stdlib/primitives/`. Work stays cleanly separated from user stdlib and from
> `__dev__`.

## 4.1 Compile-time hydration (Stage 3D) — why and how
The page-builder canvas (mockup `31.png`) must show the **real** rendered page
(real data, real layout). Today `@each` renders client-side only (the V1/Stage-2
preview path). For the builder, the canvas must equal the shipped artifact →
**compile-time hydration becomes necessary**: a new `src/export/` stage that
reads `data/*.json`, evaluates `@each`/templates at build, and bakes static HTML.

```
THE FORK (decide at Stage 3):
  prerender-only      bake HTML, no client re-attach.  Simple. Canvas = static truth.
  prerender + hydrate baked HTML the runtime re-adopts for live filtering/animation.
                      The SSR "difficulty cliff." Defer until a concrete feature needs it.
RECOMMENDATION: prerender-only for the builder canvas; the builder doesn't need
live client interactivity to show layout truth. Keeps us off the cliff.
```

## 4.2 Mockup the user asked for (3D)
`31.png` is the compile-time-hydrated builder canvas. A dedicated "hydration
preview" mockup (split: source `.st` ↔ baked page) is a recommended next artifact
when Stage 3 planning starts.

---

# §5. New veins that surfaced during this exploration

Things that emerged while designing the three the user named:

1. **Animatable prose** (from Vein 2). Because rich-text nodes carry Spacetime
   motion traits, a Quote can `@reveal`, a paragraph can stagger in. No CMS has
   *animated structured content* because their content is inert. Differentiator.

2. **The content graph as a navigation surface** (from Vein 3). `id(T)` edges +
   `@computed` views turn the admin's left rail from a flat list into a
   navigable knowledge graph (the Anytype reference image). This is a *product
   identity* opportunity: Spacetime CMS as "Anytype for websites."

3. **The type is the teacher** (cross-cutting UX). Showing `id(Agent)`,
   `richtext`, `&content` signatures *in the UI* (monospace chips) means using
   the admin teaches the Spacetime model. Onboarding and product are the same
   surface. This should be an explicit design principle.

4. **`@theme` as a general construct** (from 3B). Once tokens are typed and
   editable, `@theme` could subsume responsive/dark-mode token sets — the theme
   editor becomes a *design-system* editor, not just a color picker. Larger than
   brand; a real design-tokens feature.

5. **"Extract component" as the growth loop** (from Vein 1). The way a project's
   component library grows is designers promoting selections into `@template`s.
   This closes the loop: the builder doesn't just *use* the component model, it
   *grows* it. Hardest + highest-leverage Stage-3 feature.

6. **Spacetime-aware sanitization** (from Vein 2). A sanitizer that strips
   scripts but *preserves Spacetime traits* (`@scroll`, `$binding`) is a small but
   essential new primitive — generic HTML sanitizers would destroy the semantics.

---

# §6. Open questions & challenges to resolve in planning

```
ID  AREA            QUESTION / CHALLENGE                                    LEANING
──────────────────────────────────────────────────────────────────────────────────
Q1  id(T) type      new TypeExpr::Apply{ctor,args} vs special-case Ref      Apply (general primitive)
Q2  id storage      bare id string vs {"$ref":…} envelope                   bare id (matches unkn)
Q3  ref integrity   delete a referenced entry → ?                           warn-on-delete (Stage 2)
Q4  rename cascade  changing an id breaks referrers                         FUP (deferred)
Q5  backlinks       store inverse edges or derive?                          derive (graph knows)
Q6  richtext type    distinct `richtext` TypeExpr vs Reference to built-in   add `richtext` primitive (smallest)
Q7  richtext store  inline AST subtree in entry JSON vs sidecar by provenance open; sidecar cleaner diffs
Q8  allow/deny enforce where policy is checked (editor/server/TreeSink)     server-side is security-bearing
Q9  object unions   needed for blocks?                                      NO — blocks are AST nodes
Q10 @theme          new construct design + var(--*) migration               Stage-3 FUP
Q11 extract-comp    promote selection → @template w/ inferred params        Stage-3 deep-dive FUP
Q12 hydration       prerender-only vs hydrate-adopt                         prerender-only first
Q13 page/route model new-page scaffolding + route registration             needed for Stage 3
```

> **Note on Q9:** the V1 spec worried about needing *object unions* (`Block =
> Heading | Paragraph`). The user's html5ever decision (2B) likely **sidesteps**
> that: blocks are nodes in one document tree, not variants of a union type. The
> object-union TypeExpr extension may not be needed at all for rich text — a
> meaningful simplification. (It may still be wanted elsewhere; the
> state-machine subsystem already has it if so.)

---

# §7. Follow-ups to create (for the planning agent)

Existing (from V1 spec): `FUP-macro-body-scope` (FUP-026), `FUP-macro-namespaces` (FUP-027).

New, surfaced here:
| FUP | Purpose |
|---|---|
| `FUP-id-type-relations` (FUP-028) | Design `id(T)` as `TypeExpr::Apply`; JSON-schema `x-st-ref`; referential validation. |
| `FUP-richtext-ast-allowlist` (FUP-029) | **Supersedes** `FUP-richtext-document-schema` + `FUP-spacetime-sanitizer`. Define the rich-text allow/denylist over AST/`HtmlExpr` node kinds, and where it is enforced (editor offers / server validates / TreeSink rejects). Rich text = constrained Spacetime AST; no bespoke schema, no bespoke sanitizer (Q6/Q7/Q8). |
| `FUP-theme-construct` (FUP-031) | `@theme` typed-token construct + `var(--*)` migration (3B / Q10). |
| `FUP-extract-component` (FUP-032) | Promote a builder selection into a `@template` with inferred params (Q11). |
| `FUP-compile-time-hydration` (FUP-030) | The `src/export/` prerender stage for the builder canvas (Q12 / Stage 3D). |
| `FUP-id-rename-cascade` (FUP-033) | Optional id rename propagation to referrers (Q4). |

---

# §8. Stage DoDs (restated, sharpened)

**Stage 2 (Squarespace killer):** a writer manages **relational** (`id(T)` +
graph view + computed views), **richly structured** (html5ever Spacetime
documents with animatable blocks and native inline marks), optionally
**localized** content, with a **media library** and **live side-by-side
preview** — entirely local, entirely generated from the project's types.

**Stage 3 (Framer killer):** a designer composes whole **pages** via
**typed composition** over the project's own `@templates` (structurally valid by
construction), edits **theme tokens** (`@theme`, brand-safe by construction) and
**real declarative motion** (sliders over `@scroll`/`@reveal`), on a
**compile-time-hydrated canvas that is the real page**, and the admin emits
**clean `.st` indistinguishable from hand-authored source** (via EditAst). The
admin is itself a Spacetime site — the Framer killer is built with the thing it
beats.
