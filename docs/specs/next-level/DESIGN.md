# PLAN-035 — Next-Level Admin: Design Tool, Not Content Form

Four features that take the admin from "edits your `.st` correctly" to "a tool
you'd choose over Framer." Each is a **recombination of primitives that already
exist** (verified in repo) — the Spacetime way: grammar + lowering in `.st`,
Rust only as a generic engine + dev-server delivery plumbing.

The cardinality→channel law (PLAN-034) still holds. These four extend the
*editing surface* and the *feedback loop*, not the storage model.

---

## ① Live Preview — the loop-closer  `[01-live-preview.png]`

**The gap:** edit scarlet → file writes → `location.reload()` → re-find your
place. Framer feels magical because the canvas updates *as you drag*.

**Why it's nearly free — the pieces already exist:**
- `$brand.scarlet` is a **reactive signal**; in CSS-property position it repaints
  via `style.setProperty` on publish (BUG-091, shipped).
- EditAst already patches the `.st` literal and sets `suppress_reload_path`
  (`src/sync/handlers.rs`) so the disk write does NOT trigger a reload.
- The project page already ships a `/ws` live-reload client
  (`LIVE_RELOAD_SCRIPT`, `src/server.rs:36`).

**The one missing primitive: a token-push bridge over the existing socket.**
The admin renders the project in a split-pane `<iframe>`. A token edit fans out
two ways:

```
scarlet slider drags
  ├─ optimistic: postMessage({token:'scarlet', value}) → iframe sets the signal
  │   → preview recolors in real-time (no reload, no disk yet)
  └─ on release (change event): EditAst → .st literal persists (suppress reload)
```

**Spacetime-native implementation:**
1. **New `DevServerMessage::TokenPreview { name, field, value }`**
   (`src/dev_server.rs`) — a broadcast variant alongside `Reload`. The admin
   sends it; the server fans it to all `/ws` clients (the iframe is one).
2. **Extend `LIVE_RELOAD_SCRIPT`** (the only Rust edit) so the project page's WS
   handler, on `TokenPreview`, calls into the runtime:
   `ST.setLocal('brand', {...prev, [field]: value})` — the SAME publish path a
   normal signal write uses, so every `$brand.scarlet` binding repaints. **No
   new render path; reuses the reactive runtime.**
3. **Admin side, pure `.st`:** the Brand pane's `@on &.input [data-path]` already
   fires per keystroke. Today it calls `ST.adBrandInput` (EditAst). Split it:
   `input` → `ST.adTokenPreview` (postMessage/WS, optimistic, no disk);
   `change` → `ST.adBrandInput` (EditAst, persist). The split is the existing
   input-vs-change distinction the drawer already uses.
4. **Layout:** admin shell becomes `grid: rail | editor | preview-iframe`. The
   iframe `src` is the project root (`/`), same origin → postMessage is trivial.

**Acceptance (GREEN = browser DOM):** drag scarlet → hero `background-color`
in the iframe changes within one frame, `index.st` unchanged; release → `.st`
literal updated, iframe NOT reloaded (scroll position preserved).

**Risk:** iframe is a separate document — the bridge is `postMessage` OR the
shared WS. Use the WS (already there, already same-origin, already wired) →
zero new transport. This is the "legitimately missing feature" — designed, not
faked with custom JS.

---

## ② Motion Editable — close the last read-only pane  `[02-motion-editable.png]`

**The gap:** Brand proved typed-value → form-renderer → EditAst. **Motion is
still a read-only display card** (`tools.st` `&ad-motion-card` just shows
`label` + `file`). It's the last place the admin *reflects* instead of *edits*.

**Why it's cheap — Wave C already delivers the data:**
- `dev_motion_handler` (`src/server.rs:3483`) already parses `@reveal(...)` /
  `@scroll name(...)` and emits **`params: [{key, value}]`** — the tunable
  numbers, already extracted.
- The shared form-renderer (`ST.adFormFields` → `&fld-<widget>`) already turns
  a schema × values into slider/number fields.
- EditAst's **AstDirective** branch (`__content` absent → `apply_text_patch`)
  already patches **directive props** by selector — exactly a motion param.

**Spacetime-native implementation:**
1. **Motion is a typed value too.** `dev_motion_handler` already has
   `params:[{key,value}]`; add a `schema` per motion (key → `widget: "length"`
   for `*px`/distance, `"duration"` for `*ms`, `"number"` for ratios, `"select"`
   + easing enum). Pure delivery shaping — same as `annotate_widgets` does for
   types. No new Rust *machinery*, just a schema field on the existing JSON.
2. **Pane uses the form-renderer**, not a display card:
   `@each($motions as $m) { <fieldset> @each($m.fields as $f){ &$f.tpl($f); } }`
   — the SAME `&fld-<widget>` dispatch Brand uses. `&ad-motion-card` DELETED
   (last AP2-shaped card gone).
3. **Write via EditAst AstDirective.** A motion param edit addresses
   `<file> §<directive> .<key>` — the existing directive-prop patch path. The
   `@reveal(distance: 40)` literal in `.st` updates; the file watcher OR a
   `TokenPreview`-style push repaints.
4. **Easing as `select`** reuses `&fld-select` (exists) with the CSS easing enum.

**Acceptance:** drag a `reveal`'s DURATION slider → the `@reveal(... duration:
600)` number in the source `.st` changes; siblings byte-exact; the animation on
the live page runs at the new duration.

**This fully dissolves "read-only panes"** — one editing model (typed value →
form-renderer → EditAst) across Brand, Posts, AND Motion.

---

## ③ Token-Aware Widgets — the craft layer  `[03-color-picker.png]`

**The gap:** a raw `#1166EE` hex text field is the tell that this is a dev tool.
A design tool gives you a **perceptual picker** and **drag-to-tune** scalars.

**Why it fits the existing widget system:**
- Widgets are dispatched by `&fld-<widget>` templates (`drawer.st`). Adding a
  richer color widget = a richer `&fld-color` template — **no engine change**.
- `length`/`duration` already carry a `format` (`src/type_system.rs`) that the
  widget engine maps. A `range`/slider widget is one more `&fld-<widget>`.

**Spacetime-native implementation:**
1. **`&fld-color` becomes a popover picker** — a `@portal`-relocated panel
   (the SAME portal mechanism the drawer + media library use) holding an
   SV-square + hue/alpha sliders. The picker's output writes the same
   `data-path` input → the existing `@on &.input` → preview/persist. The picker
   *chrome* is declarative markup + reactive bindings; the H/S/V↔hex math is a
   handful of registered `@fn`s (`hslToHex`, `hexToHsv`) — pure functions, the
   same class as `currency`/`basename`. **No imperative widget object.**
2. **`length`/`duration` → `&fld-range`**: a native `<input type=range>` bound
   to the same `data-path`, with min/max/step inferred from the unit
   (`px`: 0–64, `ms`: 0–2000). Drag `8px`, drag `600ms`. The value readout is a
   reactive `text <- $f.value` hole.
3. **The SV-square fill** is itself a `$`-driven CSS gradient — reactive tokens
   styling the picker, dogfooding BUG-091.

**Acceptance:** open SCARLET → drag the SV handle → hex readout updates live →
(with ①) the preview hero recolors continuously → release persists to `.st`.

**Scope note:** this is the one with real surface area (picker geometry). Land
①+② first; this is polish that makes the loop *feel* like a design tool.

---

## ④ Derived Tokens — a SYSTEM, not a palette  `[04-derived-tokens.png]`

**The gap:** every token is a magic literal. Real design systems have
*relationships*: `ink = surface darkened 92%`, `hover = scarlet darkened 8%`,
`muted = ink at 60% alpha`. Change one literal → the whole system re-derives.

**Why it's already in the language — this is the deepest fit:**
- **`@data derive` / `computed-source`** (Wave A) already computes a `$`-value
  from other signals, with **dep-discovery + reactive re-run** (BUG-090). A
  derived token is `@data derive $ink : darken($brand.surface, 0.92)`.
- The **pipe + `@fn`** surface (Wave A) already gives `$brand.scarlet | darken(8%)`
  callable as fn OR pipe. `darken`/`lighten`/`alpha`/`mix` are just registered
  color `@fn`s — the same class as `includes`/`basename`.
- When `scarlet` changes (via EditAst), `computed-source`'s watch re-runs every
  dependent derive → `hover` repaints. **The reactivity is already built.**

**Spacetime-native implementation:**
1. **Derived tokens are `@data derive` over `$brand`:**
   ```spacetime
   @data derive $ink   : $brand.surface | darken(0.92);
   @data derive $hover : $brand.scarlet | darken(0.08);
   @data derive $muted : $ink | alpha(0.60);
   ```
   No new construct — the existing derive + pipe + color `@fn`s.
2. **Register color-algebra `@fn`s** (`darken`/`lighten`/`alpha`/`mix`) — pure
   functions in `%emit js`, callable fn-or-pipe. The ONLY new code, and it's
   stdlib `.st`, not Rust.
3. **Admin reflects literal vs derived automatically.** The brand delivery
   (`build_brand_json`) already separates `@data inline` (literal, editable) from
   `@data derive` (computed). The pane renders literals as editable pills,
   deriveds as **read-only formula chips** (`$f.formula` from the derive's source
   expr) with the resolved swatch — a third `&fld-derived` template. Self-
   describing: add a derive → a locked formula row appears, ZERO admin edits.
4. **The connector lines** (derived → source) come free: the derive's
   dep-discovery already knows `$hover depends on $brand.scarlet` — deliver that
   edge, draw it with the same declarative SVG the relation graph uses (FEAT-092).

**Acceptance:** edit `scarlet` → `hover`'s resolved swatch re-derives live
(no edit to `hover`); the formula chip `scarlet · darken 8%` is read-only; the
connector line links them.

**This is the "answers questions you never posed" primitive** — derived tokens
fall out of `@data derive` + color `@fn`s with nothing invented. It's the
feature that makes Spacetime a design-*system* tool.

---

## Sequencing — by leverage

```
① LIVE PREVIEW    ← do first; transforms the whole feel, everything compounds on it
② MOTION EDITABLE ← cheap (Wave C delivers data); closes the last read-only pane
③ TOKEN WIDGETS   ← craft polish; needs ① to shine (live perceptual picking)
④ DERIVED TOKENS  ← the system layer; deepest language fit, smallest new code
```

**Dependencies:** ① independent (highest leverage). ② independent (reuses
form-renderer + EditAst AstDirective). ③ depends on ① for the live feel. ④
independent of all but *best* with ① (live re-derive) — and is almost entirely
already-built primitives.

**New Rust, total across all four:** one `DevServerMessage::TokenPreview`
variant + one `LIVE_RELOAD_SCRIPT` extension (①), one `schema` field on
`motion.json` (②). Everything else — pickers, sliders, color algebra, derived
tokens, formula chips — is stdlib `.st` + registered `@fn`s. **The metasystem
expands the syntax; Rust stays a generic engine.**

## Tracked enablers (already filed)
- **FUP-044** bare-key typed object literals — would let `$brand : { scarlet:
  #FF0020 }` drop the quotes (cosmetic; not blocking).
- **BUG-084/085** nested-ternary-paren / nested-`@each`-in-template — ② and ④
  render nested fields; verify these don't bite (workarounds exist).
