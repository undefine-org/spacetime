# Spacetime CMS — Local Content Admin (V1) → Squarespace Killer → Framer Killer

**Status:** Design spec (pre-plan). No code written yet.
**Author:** Synthesised from research + repo archaeology, 2026-06-01.
**Audience:** A fresh agent who will turn this into an org PLAN and start drafting.
Read this top to bottom; it is self-contained. Cross-references to existing
systems are given as concrete file paths so you can verify every claim.

---

## 0. One-paragraph thesis

Spacetime already contains the organs of a Git-based, schema-as-code,
in-context CMS — scattered across `stdlib/__dev__/` and `src/sync/`, gated
behind `--debug`. A Spacetime CMS is **not** a general headless CMS that caters
to many frontends. It caters to exactly one system — Spacetime — and that is
its entire advantage. The admin is **generated from the project's own
`@type`/`@data` definitions**: zero config, zero schema registration. Adding a
field to a type *is* adding it to the admin. The admin is itself a Spacetime
site (primitives + metasystem, no custom JS), served on a separate full-window
route, sharing the existing WebSocket edit protocol with the debug tools. From
that V1 foundation, the product climbs one rung at a time — entries → composable
relational + rich content (Squarespace killer) → pages, layout, theme, and
visual motion editing (Framer killer) — by having the admin **understand
progressively more of the `.st` program and write progressively more of it
back.** Never a separate database. Never a foreign content format.

---

## 1. Why this is different from every other CMS

The 2026 headless landscape (Contentful, Sanity, Storyblok, Payload, Strapi,
Directus, Hygraph; Git-based Tina/Decap/Keystatic) collapses to a few tension
axes, not a feature list. The repeated industry lesson: *the platform matters
less than what you build on top; pain is always at the edges — when content
models grow, when editors hit interface walls, when you price a renewal at
scale.* Every incumbent must keep its **content layer** and its **presentation
layer** as different substances that drift apart.

Spacetime collapses three artifacts that every other CMS keeps separate:

```
Traditional headless CMS              Spacetime-native CMS
──────────────────────────            ──────────────────────────
schema (CMS config)         ┐
presentation (frontend)     ┤── ALL THREE are the one .st program
behavior (animation/JS)     ┘     @type             = schema
                                  @each / @template = presentation
                                  @on/@scroll/@state = behavior
content (CMS database)      →     data/*.json + locales/*.json (git-tracked)
```

In Contentful a "Hero" content-type and the `<Hero>` that renders it are two
artifacts that drift. In Spacetime `@type Hero` + its `@template` + its
`@scroll reveal` are **one source span**. The CMS schema *cannot* drift from the
presentation because they are the same declaration. No API CMS can have this
property; even Tina/Payload only approximate it (schema-as-code, but
presentation still separate). This is the moat: **generation from the project's
own definitions**, possible only because the CMS serves exactly one system.

---

## 2. What already exists (verify before building)

This is the substrate. None of it is hypothetical — paths are real.

| Capability | Where | State |
|---|---|---|
| Content model `@type`/`@data`/`@computed` | `stdlib/macros/type-data.st` | ✓ schema-as-code |
| `@type` → JSON-schema endpoint | `src/server.rs::dev_types_handler` → `/__spacetime/dev/types.json` | ✓ |
| Site graph (pages, data sources, locales) | `src/server.rs::dev_site_handler` → `/__spacetime/dev/site.json` | ✓ |
| Type registry | `src/type_system/` (`TypeRegistry::from_form_matches`, `generate_json_schema`) | ✓ |
| Content store | `projects/<name>/data/*.json`, git-tracked | ✓ "the database" |
| Inline edit (click-to-edit) | `stdlib/__dev__/primitives/dev-editable.st` | ✓ |
| Edit persistence (value) | `src/sync/handlers.rs::handle_edit_json` ← `EditJson` | ✓ |
| Edit persistence (array CRUD) | `handle_edit_json_array` ← `EditJsonArray` (Insert/Delete/Reorder) | ✓ |
| Content edit (text in HTML/.st) | `handle_edit_ast` ← `EditAst` + `__content` sentinel | ✓ |
| Provenance addressing | `docs/CONTENT_EDITING.md` (`data-st-id`, `data-st-origin`) | ✓ |
| Collection CRUD overlays | `stdlib/__dev__/primitives/dev-collection-crud.st` | ✓ |
| Media upload + swap | `dev-media-picker.st` + `src/server.rs::upload_handler` → `/__spacetime/dev/upload` | ✓ |
| Save orchestration (ack/reject/optimistic) | `dev-save.st` (`window.__stDevSave`) | ✓ |
| Reactive re-render after save | `ST.setData(source, data)` on `DataUpdate` | ✓ |
| Content tree UI | `dev-content-tree.st`, `data-tree.st` (Shadow DOM) | ✓ (debug-flavoured) |
| WS protocol types | `src/sync/protocol.rs` (`ClientMessage`/`ServerMessage`) | ✓ |

**The gap V1 fills:** these organs are debug-flavoured (an inspector overlay).
V1 assembles them into a **calm, content-first, full-window admin** with a
**generated** UI, distinct from the debug panel.

**The one thing `@data` does NOT do today:** it is purely client-side. The
`data-source` primitive (`stdlib/primitives/data/source.st`) emits a runtime
`fetch()`; `@each` renders into empty `data-st-source` containers in the
browser; the static export pipeline (`src/export/`, layers
`discover→bundle→html→emit`) strips dev attributes but **never pre-renders
content into static HTML**. "Compile-time hydration" is therefore a *missing
pipeline stage*, not an optimization. **V1 explicitly defers it** (see §6) — V1
uses the existing client-side re-render for preview feedback.

---

## 3. The polarities the CMS must manage

The CMS design *is* the disciplined management of these seams. They are
**correlated**, lining up into two coherent columns that are two products
sharing one substrate.

```
POLARITY              pole A  ───────────────  pole B
─────────────────────────────────────────────────────────────────
content kind     static (.st copy, hero)   dynamic (blog, profiles)
editor persona   designer (edits locally)  writer (no git, no local)
hydration timing compile-time (baked)      live (runtime fetch)
content locus    in-file (index.st body)   out-of-file (data/*.json)
edit surface     the deployed site itself  a separate admin app
write target     working tree (local)      server/branch (remote)
content shape    a page (route+layout)     an entry (row in a type)
unit of change   a commit (designer)       a publish (writer)
schema authorship dev writes @type         writer never sees it

   STATIC / DESIGNER / IN-FILE / PAGE / FRAMER-KILLER   ← "design surface" (user-front)
   DYNAMIC / WRITER / JSON / ENTRY / SQUARESPACE-LIKE   ← "content surface" (admin-front)
```

Decoupling admin-front from user-front is the cut along this seam. The admin is
the content-surface column; the user-front is where the design-surface column's
output lives. **One substrate (compiler + `.st` + `data/`), two emitters.** This
dissolves the "is it headless?" question: it is not headless (API-out,
decoupled) — it is *coupled* in-context editing, which is Storyblok/Tina/Framer's
strongest feature, and the thing Spacetime is uniquely built for.

---

## 4. V1 — Local Content Admin

### 4.1 Locked decisions (do not relitigate without the user)

1. **Surface:** a **separate full-window route** `/__spacetime/admin/`, distinct
   from the `__dev__` debug overlay. Clean content-first surface. **Shares the
   WS edit protocol** (`EditJson` / `EditJsonArray` / `/upload`).
2. **Display inference:** an **optional `@cms` hint macro** marks display fields,
   with **convention fallback** (works on `unkn` untouched).
3. **Hydration:** **keep current client-side `@each` re-render** (`DataUpdate` →
   `ST.setData`) for V1. **Defer compile-time hydration** to a later stage.
4. **Stdlib placement:** the admin lives in **its own folder
   `stdlib/__admin__/`** (sibling to `stdlib/__dev__/`), NOT under
   `stdlib/macros/` or `stdlib/primitives/`. Work is cleanly separated from both
   user-facing stdlib and dev tooling.

### 4.2 Shape

```
route     /__spacetime/admin/        full-window; NOT the debug overlay
identity  itself a Spacetime site    stdlib/__admin__/  (primitives + metasystem, no custom JS)
transport same WS protocol as __dev__ EditJson · EditJsonArray · /upload
gating    --debug serve              local only — no auth, no server (those are later layers)
feedback  save → DataUpdate → ST.setData → @each re-render   (client-side, V1)
```

Two surfaces over one substrate, born here: **debug panel = behavior; admin =
content.** Different routes, shared protocol.

### 4.3 Auto-derivation (the thesis, mechanised)

The admin reads `/__spacetime/dev/types.json` + `/__spacetime/dev/site.json` —
**zero config**.

```
DECLARATION                       →  ADMIN ELEMENT
─────────────────────────────────────────────────────────────────
@data X: T[]                      →  a collection "X", count = json.length
@data Y: T                        →  a "single" (one entry, e.g. site settings)

field type  →  form widget:
  string                          →  text input
  string  (desc/body/bio-named)   →  textarea           (heuristic on field name)
  number                          →  number input
  boolean                         →  toggle
  "A" | "B" | "C"  (union)        →  select
  url + image-ish name            →  media field  (reuse dev-media-picker + /upload)
  string[]                        →  repeatable chip/tag editor
  T  (nested object type)         →  nested fieldset
  T[] (nested array of type)      →  nested repeatable list  (recurse)
  field named `slug`              →  read-only, auto-derived from the title field
```

**Inference, not annotation.** V1 reads existing types untouched. `unkn` (6
types: `Talent`, `Service`, `WorkCard`, `Reason`, `Stat`, `StripRow` →
`projects/unkn/index.st:35-85`) gets a full admin with **zero `.st` edits**.

### 4.4 Display inference — convention with `@cms` override

Convention fills the gaps; `@cms` is the escape hatch for ambiguity.

```
FALLBACK (no annotation) — unkn works today:
  title     = first string named  name | title | label
  subtitle  = first string named  headline | subtitle | description
  thumbnail = field typed url / named  image | photo | avatar
  chips     = string[] named  categories | tags
```

```
OPTIONAL OVERRIDE — CANONICAL FORM (implemented & preferred): file-scope
`@cms(TypeName)` taking the target type as a paren argument:

@type Talent {
  id: string; slug: string; name: string;
  headline: string; image: string; categories: string[];
}
@cms(Talent) {
  title:     name;
  subtitle:  headline;
  thumbnail: image;
  chips:     categories;
}
```

NB (decided 2026-06-03, user): the file-scope `@cms(TypeName)` form is the
CANONICAL surface — preferred over a nested `@type { @cms { … } }`. Rationale:
(1) it parses cleanly today (the nested form collides with `@type`'s
`$fields:properties` body capture, zeroing the type's fields); (2) it is a real
metasystem macro with no Rust special-casing of placement; (3) it keeps the
display hints visually adjacent to the type without entangling two captures. The
nested/namespaced form (`@type.cms`) is NO LONGER a goal for V1; FUP-026/FUP-027
are reframed from "required to ship @cms" to "optional future ergonomics."

`@cms` is a macro that `%registers` display metadata into the type registry,
surfaced through `types.json`. Convention fills any display role the hint
omits. This honours the metasystem (self-describing; no Rust special-casing of
display rules).

> **PLACEMENT (decided 2026-06-03):** the canonical surface is file-scope
> `@cms(TypeName)` (`%scope file`), which parses cleanly today and needs no
> macro-system changes. The earlier requirement — that `@cms` be *nested inside*
> the `@type` body and rejected elsewhere with a rich error — is **no longer a V1
> goal**, because the nested form collides with `@type`'s `$fields:properties`
> body capture (it zeroes the type's fields). The two language capabilities that
> *would* enable a nested/namespaced form are now **OPTIONAL future ergonomics**,
> not blockers:
> - **FUP-026** (macro body-scope + rich placement errors) — would let a macro be
>   "valid only inside another macro's body."
> - **FUP-027** (macro namespaces) — the symbol for nesting (`@type.cms` etc.).
>
> If those ever land, `@cms(TypeName)` MAY gain a nested alias; until then the
> paren form is canonical and preferred. (Per `AGENTS.md`: `@cms` is a real
> metasystem macro — no Rust special-casing of placement.)

### 4.5 Writer capabilities (V1)

```
ACTION          UI                    →  PROTOCOL                        STATUS
─────────────────────────────────────────────────────────────────────────────
edit value      form field            →  EditJson                        ✓ exists
add entry       "+ New <type>"         →  schema default → EditJsonArray Insert
delete entry    row control           →  EditJsonArray Delete            ✓ exists
reorder entry   drag handle           →  EditJsonArray Reorder           ✓ exists
replace image   media field           →  /upload → EditJson              ✓ exists
see result      save → DataUpdate     →  ST.setData → @each re-render     ✓ exists
```

∴ V1 is **almost entirely new frontend** (the `stdlib/__admin__/` site) + the
`@cms` macro + schema→form generation. The **persistence half already runs.**

### 4.6 UX direction (validated reference points)

- **Object-types-as-navigation** (ref: attached Anytype `types_index.png`): the
  collections derived from `@type`/`@data` ARE the primary left-rail navigation,
  exactly like Anytype's "Object types". Plus a "Pages" section from
  `site.json`.
- **Squarespace-grade form polish**: calm, gallery-like, generous whitespace,
  one accent colour, schema-generated fields behind a refined slide-in drawer.
- **Three-pane**: left rail (collections + pages) · center (collection list,
  searchable, editable rows with thumbnail + title + subtitle + chips + drag
  handle) · right drawer (schema-generated entry editor with sticky footer).
- **Mockups first**: every admin screen is generated via `GenerateUIScreen` and
  **validated by the user before implementation** (design principle #1). The V1
  Talents mockup is approved as the reference
  (`artifact://…/generate_image/23.png`).

The admin UI is built **fully in Spacetime** (design principle #1): primitives
wrap any unavoidable browser API via `%emit js`; macros compose them; the
collection rail, list, and form are themselves `@each`/`@template`/`@type`
driven — the CMS admin is a Spacetime site that happens to edit Spacetime sites.

### 4.7 V1 Definition of Done

> Open `/__spacetime/admin/` while serving `unkn` with `--debug` →
> Talents/Services/Work/Reasons/Stats/Strip auto-listed in the rail with counts →
> click a talent → edit name/headline/image/categories in a **generated** form →
> add, delete, reorder talents → the live page reflects every change via
> client-side re-render. **No `.st` edits required** to get the admin; `@cms`
> available to refine display roles.

### 4.8 V1 build seams (for the PLAN; not yet implemented)

```
1. @cms macro             stdlib/__admin__/  — registers display hints into the type registry.
                          Body-scoped to @type (rich-error path tracked as FUP-macro-body-scope).
2. types.json+            extend dev_types_handler (src/server.rs) to emit @cms hints
                          + computed display-role inference alongside the JSON-schema.
3. schema → form          recursive widget generation from JSON-schema, in the admin site.
4. admin site             stdlib/__admin__/  — collections rail, collection list, entry drawer,
                          built from primitives + macros (no custom JS).
5. /__spacetime/admin/    new route in src/server.rs, --debug gated, serving the compiled admin
                          runtime + styles (mirror the dev runtime/styles handlers).
```

Reuses, unchanged: `EditJson*`, `/upload`, `ST.setData`, `dev-media-picker`,
the whole `src/sync/` layer.

---

## 5. Stage 2 — Squarespace killer (still 100% local)

The jump from *entry* to *composable, relational, rich content*. Three
additions, each a natural extension of the type graph. Still local, still no
server, still no auth.

### 5.1 Relations (the biggest gap between "JSON editor" and "CMS")

A field whose type is another `@type` (a typeref) renders as a **relation
picker**, not free text:

```
@type Talent {
  agent: Agent;          // single relation  → picker linking one Agent entry
  campaigns: WorkCard[]; // multi relation   → picker linking many WorkCard entries
}
```

The type graph already expresses references (`docs/DATA_SYSTEM.md` → "Type
References"). The admin renders `typeref` fields as pickers over the target
collection. Stored as id references in the JSON; resolved for display by reading
the target `@data`. This turns the flat collection list into a **content
graph** — and pairs naturally with an Anytype-style **graph view** of entries
and their relations (ref: attached `types_index.png` "Graph view").

**New work:** relation widget; id-reference resolution in the admin; optional
referential-integrity warnings (deleting an entry referenced elsewhere). No new
protocol — still `EditJson`.

### 5.2 Structured rich text — as Spacetime types, not a foreign format

A writer's `bio`/`body` outgrows a textarea — it needs headings, embedded media,
quotes, callouts. **Spacetime-native answer:** rich text *is* an `@each`-able
**block array**, expressed in the type system itself:

```
@type Block =                       // union of block variants (uses existing union types)
    Heading | Paragraph | Image | Quote | Embed;

@type Heading   { level: 1|2|3; text: string; }
@type Paragraph { text: string; }       // inline marks (bold/italic/link) TBD — see below
@type Image     { src: url; alt: string; caption?: string; }
@type Quote     { text: string; cite?: string; }
@type Embed     { kind: "youtube"|"vimeo"|"figma"; url: url; }

@type Article {
  title: string;
  body: Block[];                    // ← the rich text, as data
}
```

- The admin renders a **block editor** (add/reorder/delete blocks; each block's
  fields come from its `@type` via the same §4.3 widget rules — **recursion
  pays off**).
- The site renders the same blocks via `@each(body)` + a `@template` per block
  variant. **One representation, two consumers.**
- **No Portable Text, no MDX, no foreign AST.** It is Spacetime types all the
  way down — which means the block editor is *also generated*, and new block
  types added by a developer appear in the editor for free.

**Open question (carry into Stage 2 design):** inline marks within a paragraph
(bold/italic/link spans). Options: (a) a constrained sanitised-HTML string
(reuse `dev-save.st`'s `__stSanitizeContentHtml`); (b) a nested inline-span
type array. (a) ships fast and reuses the content-edit path; (b) is purer but
heavier. Recommend (a) for Stage 2, revisit for Stage 3. This is the single
genuinely new *content-model* primitive the product needs; everything else is
assembly.

> **Why this matters across stages:** block-structured content is *also* the
> Framer-killer's component model (a page is a `Section[]`, a section is a block).
> Solving rich-text-as-blocks (Stage 2) is a down-payment on Stage 3 — polarity
> "content shape" (entry vs page) turns out to be the same problem at two scales.

### 5.3 Media library + live preview

- Promote `dev-media-picker`'s one-shot upload into a **browsable asset grid**
  (read `projects/<name>/assets/`; upload via existing `/upload`; pick into any
  media field).
- **Split-view live preview**: the admin form beside a live-updating page
  preview. V1 already re-renders via `ST.setData`; Stage 2 tightens it into a
  visible "see it as you type" loop — Squarespace's core value — **all local**.

### 5.4 Stage 2 Definition of Done

> A writer manages relational, block-structured content with a media library and
> live side-by-side preview, entirely locally, entirely generated from the
> project's types. Killing Squarespace *for Spacetime sites*: structured,
> relational, rich content, git-diffable, zero CMS config.

---

## 6. Stage 3 — Framer killer

The jump from *content* to *layout*. The writer/designer composes **pages**, not
just entries. This is where the admin begins to **emit `.st`**, not just
`data/*.json`. Compile-time hydration (deferred from V1, §2) likely becomes
necessary here so composed pages render statically.

### 6.1 The Framer-killer primitives (each maps to an existing Spacetime construct)

```
CAPABILITY          MAPS TO
─────────────────────────────────────────────────────────────────
add a section       insert a @template / module invocation into a page's .st
arrange sections    reorder template calls in page composition.
                    (unkn ALREADY composes pages this way:
                     `.site-header { &unkn-nav() } …` — a section list. See projects/unkn/index.st)
style controls      edit theme tokens (_theme.st → var(--unkn-*)) via sliders/pickers.
                    PROJ-097 "visual controls" already gestures at this.
edit motion         tune @scroll / @fade-in params visually — UNIQUE to Spacetime.
new page            scaffold pages/<route>.st from a layout template + bind @data.
component palette   the project's @templates ARE the component library — place/drag them.
```

### 6.2 Why Spacetime out-classes Framer (not just matches it)

- **Reviewable, versionable output.** Framer exports opaque React with inline
  styles — un-diffable, un-reviewable, locked-in. Spacetime emits **`.st` a
  designer can read, git can diff, the compiler can optimise.** The admin's
  output is the *same source a developer writes by hand.* Designer and developer
  edit one artifact from two surfaces. No incumbent can copy this, because their
  content layer and code layer are different substances; in Spacetime they are
  the same substance.
- **Real, declarative, exportable motion.** In Spacetime an animation is
  `@scroll reveal { opacity: 0->1; duration: 800ms }` — a *declaration the admin
  can expose as visual controls.* Framer's motion is imperative config inside a
  runtime. A Framer killer that lets you **visually tune declarative, real,
  exportable motion** is a genuinely new category, not a clone. (`dev-review.st`
  already audits timelines — the inspection half exists.)

### 6.3 What Stage 3 newly requires

```
- admin WRITES .st         emit/patch template invocations & page composition (extends EditAst).
- compile-time hydration   the deferred pipeline stage (§2): read data/ → evaluate @each →
                           bake static HTML in src/export/. Needed so composed pages render.
- theme-token editing      structured read/write of _theme.st var(--*) tokens.
- motion control surface   map @scroll/@fade-in params ↔ visual controls (builds on PROJ-097).
- page scaffolding         generate pages/<route>.st; register routes.
```

### 6.4 Stage 3 Definition of Done

> A designer composes whole pages — sections, layout, theme, and motion —
> visually and locally, and the admin emits clean `.st` indistinguishable from
> hand-authored source. Killing Framer *for Spacetime sites*: visual page design
> whose output is real, reviewable, optimizable Spacetime source with real
> declarative motion.

---

## 7. The through-line (what makes all three stages one product)

```
V1     reads types  → generates forms                  (admin consumes @type, writes json)
SQS    reads types  → generates rich/relational editors (admin understands the type GRAPH)
FRMR   reads templates → generates layout composer;
                          WRITES @type & .st            (admin authors source)
```

One escalating capability: **the admin understands progressively more of the
`.st` program and writes progressively more of it back** — first its data shape,
then its content graph, then its structure & motion. Never a separate CMS
database. Never a foreign format. Never custom JS (the admin is itself a
Spacetime site). The "killer" property at every stage is **generation from the
project's own definitions** — the thing you cannot get from a CMS that must
cater to many systems. This one caters to exactly one: Spacetime.

---

## 8. Explicitly out of scope (named so they are not smuggled in)

These are *later layers*. The user's directive: make the **local experience
rock-solid** and make it a **killer locally** before any server. Do **not** let
"emulate all we need locally" drag in undecided server contracts.

```
DEFERRED                        why it can wait                            earliest stage
──────────────────────────────────────────────────────────────────────────────────────
auth / identity / roles         local single-user needs none              post-Stage-3 (servers)
multi-editor conflict / CRDT    local = 1 writer; LWW EditJson is fine     post-Stage-3
draft / review / publish        compile-time "publish = rebuild"; branch-based later   server era
hosting / deploy / CI-CD        explicitly excluded for now                server era
remote upload of sites          the foundation server (5-verb) comes after local is solid  server era
```

> **Forward-compat hygiene (cheap, do it from V1):** make the local admin write
> through the **same verbs** a future server would expose
> (`GET types` · `GET/PUT data/*` · `POST build`). The existing
> `EditJson`/`op_id`/`Ack`/`Reject` protocol already has this transport-agnostic
> shape (`src/sync/protocol.rs`) — it does not care whether the other end is
> localhost disk or a remote. Honour that and "local → remote" later becomes a
> transport swap, not a rewrite.

---

## 9. Follow-ups created alongside this spec

| FUP | File | Purpose |
|---|---|---|
| Macro body-scope + rich errors | `@tasks/follow-ups/FUP-macro-body-scope.org` | Give the macro system a "valid only inside macro X's body" scope, so `@cms` outside `@type` yields a rich, specific error. No Rust special-casing — fix at the metasystem scope layer. |
| Macro namespaces (research) | `@tasks/follow-ups/FUP-macro-namespaces.org` | Decide the symbol/grammar for nesting a macro under another (`@type.cms` / `@type::cms` / `@type/cms` / namespace block), so `@cms` can live in its appropriate namespace. |

---

## 10. Reading list for the picking-up agent (verify, don't trust)

```
docs/DATA_SYSTEM.md                    @type/@data/@each, type refs, filters
docs/CONTENT_EDITING.md                provenance addressing contract (data-st-id / data-st-origin)
@tasks/projects/PROJ-094-...           dev tools architecture (stdlib/__dev__ thesis + data flow)
@tasks/projects/PROJ-107-...           unified EditAst content editing
stdlib/__dev__/                        the organs to reuse (esp. dev-save, dev-collection-crud,
                                       dev-media-picker, dev-content-tree, data-tree)
stdlib/macros/type-data.st             @type/@data/@computed macros (where @cms is a sibling concept)
src/sync/protocol.rs                   ClientMessage/ServerMessage — the edit protocol
src/sync/handlers.rs                   handle_edit_json / _array / _ast — persistence
src/server.rs                          dev_types_handler, dev_site_handler, upload_handler, routes
src/type_system/                       TypeRegistry, generate_json_schema
src/export/                            static export pipeline (where compile-time hydration lands)
projects/unkn/index.st (l.35-135)      the real V1 target: 6 types, 6 @data, root-level @each
projects/unkn/data/*.json              the content the V1 admin manages
projects/unkn/AGENTS.md                brand constraints for the admin's own styling if themed
```

Design principles (user, binding):
1. Admin frontend **fully in Spacetime**; mockups via `GenerateUIScreen`,
   **validated by the user before implementation**.
2. Admin pages & links **fully generated** from the `.st` data definitions,
   organised with world-class UX (Framer/Squarespace/Anytype insights).
3. V1 = a local admin that manages the `unkn` talent JSONs in an interface
   **separate from the debug** panel, **auto-deriving every collection and field
   from the project's types.**
