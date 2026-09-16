# Data & Rendering — `@data`, `@each`, first-class HTML, SSG

Spacetime is the host language for its own websites: HTML is written inline, data
is one construct (`@data`), and lists render with `@each`. This reference covers
the unified data surface, the `<` HTML sigil and `` `…` `` holes, and how static
lists unroll into served markup (SSG) while dynamic ones hydrate.

> Status: PLAN-023. The legacy block-form `@data { … }` and `@computed { … }`
> are **deprecated** but still work; see [Migration](#migration).

---

## 1. The operator algebra

Four operators carry distinct meaning. Knowing them makes every construct below
read unambiguously.

| Operator | Meaning | Layer | Example |
|----------|---------|-------|---------|
| `:` | type / static value | author | `@data inline $n number : 3;` |
| `<-` | reactive assignment | author | `$cart <- $cart.concat(item)` |
| `->` | transition / yield / reducer step | author | `opacity: 0 -> 1` |
| `=` | **static literal default** — binds a fixed fallback to a named slot | author + library | `$alt string = ""`, `class="x"`, `refresh: $r:duration = 0` |

`=` binds a **static literal default** to a named slot. It has ONE meaning across
both author and meta layers: the value on the right is a dead literal (a quoted
string, a number, a bool), bound to the name on the left when nothing else fills
it. Two author-level uses share this meaning: an HTML **attribute** (`class="x"`,
`src="/logo.svg"`) and a **parameter default** in a `@template`/`@editable-*`
declaration (`$alt string = ""`, `$count number = 0`). The meta layer uses the same
`=` for `%macro`/`%primitive` parameter defaults (`refresh: $r:duration = 0`).

`=` and `:` are **different registers, not interchangeable**: `=` is a static
literal default (dead, evaluated once at definition); `:` introduces a
**reactive-or-typed value** that lives in the data system (`@data inline $n : 3`,
an in-template state init `$open bool: false`, a `@fn` return type `): number`).
A parameter therefore declares its TYPE with a space (`$price number`) and its
DEFAULT with `=` (`$price number = 0`) — a `:` in a parameter list is an ERROR
(E0929), not a silent drop.

The two RHS introducers:

- **`:` introduces a VALUE** — a literal, URL, or expression (`@data inline $x : 3;`,
  `@data fetch $u : "/api";`, `@data derive $t : $a * $b;`).
- **`from $src` introduces a SIGNAL SOURCE** — another reactive value the construct
  reads (`@data query $a from $products { … }`, `@data fold $sum from $cart : …`).

---

## 2. Sigils

| Sigil | One meaning | Phase |
|-------|-------------|-------|
| `$` | reactive data / signal / param NAME | runtime |
| `&` | element reference / template / selection hole | runtime |
| `@` | directives | runtime |
| `%` | macros / primitives / capture-types | compile-time (library) |
| `<` | **HTML element literal** | compile-time → served markup |
| `` ` `` | **the hole** — a Spacetime expression embedded in HTML | compile-time |
| `:` | **introduces a VALUE** (author) / a capture type (`%form`/`%capture_type` meta) | both |
| space | **introduces a TYPE** in any param declaration (`$href url`, `$price number`) | compile-time |

`<` and `` ` `` make *valid HTML valid Spacetime*: you write markup directly, and
drop reactive values into it with backticks.

Each sigil carries **exactly one** author-level meaning — no overloads:

- **`` ` `` is the one hole form.** Page markup, `@each` bodies, and `@template` /
  `@editable-mark` / `@editable-block` component bodies all interpolate with
  backticks. A bare `$x` / `&name` in HTML text is literal text (so `$5`, `AT&T`,
  `&amp;` need no escaping). Element substitution rides the same delimiter:
  `` `&sel` `` inserts a node or flattens a node sequence.
- **`:` always introduces a value; space always introduces a type.** A parameter
  declares its type with a space (`$price number`, `$href url`), matching signal
  declarations — never `name: type`. The `:` after a `)` in `@fn name(...): T` is the
  *return* type (still a value-position: the fn yields a value of type `T`). Inside
  `%form` / `%capture_type` (the grammar-defining meta-layer), `$name:ident` keeps
  `:` for capture types — a distinct layer from author code.
- **`=` gives a parameter its static default.** A default value uses `=`, not `:`
  (`$title = "Launch faster"`, `$count number = 0`) — the same `=` an HTML attribute
  uses (`class="x"`). This keeps `:` for reactive/typed values only; a `:` in a
  parameter list is rejected (E0929) rather than silently truncating the list.
- **`( )` is a grammar combinator** in `%form` / `%capture_type`: a parenthesized
  group `( … )?`/`( … )*` with ordered-choice `|`, matched intra-form (it never
  expands into sibling forms, so resolution stays a single prefix-bucket + specificity
  scan).
- **`&name` is an element/region hole; `&name[]` marks a *block* region.** In an
  `@editable-block` projection, a non-`&sel` element param is a named editable
  *region* (a sub-space the editor edits independently). A trailing `[]`
  (`&items[]`) marks the region as **block-kind** — it holds child blocks (a list)
  — versus a bare `&caption`, which is **inline-kind** (text + marks). This reuses
  the existing `[ ]` collection sigil (the `&name[]` selector-ref shape) rather than
  inventing region syntax: `@editable-block &columns(&left, &right)` declares two
  inline regions, `@editable-block &list(&items[])` one block region.
```st
@editable-block &columns(&left, &right) {
  <div class="cols"><div class="col">`&left`</div><div class="col">`&right`</div></div>
}
@editable-block &list(&items[]) { <ul>`&items`</ul> }   <!-- [] ⇒ block region -->
```

```st
<ul class="list">
  <li>`$title`</li>
</ul>
```

`` `$title` `` is a hole: the inner is a full Spacetime expression. Escape a literal
backtick in HTML text with `` \` ``.

Backtick is the **one** hole form, everywhere — page markup, `@each` bodies, and
`@template` / `@editable-mark` / `@editable-block` component bodies alike. There is
no bare-hole form: a bare `$x` or `&name` in HTML text is literal text (so `$5`,
`AT&T`, and `&amp;` need no escaping). Element substitution uses the same delimiter:
`` `&sel` `` inserts a node or flattens a node sequence at that position.

---

## 3. `@data` — one construct, five kinds

`@data` declares a named reactive **source**. The *kind word* selects behavior; the
kind IS the macro (no hidden switch). Every kind shares the shape:

```
@data <kind> $name <Type>? <: value | from $src> <{ options }>? ;
```

| Kind | Form | Backed by | `static` |
|------|------|-----------|----------|
| `inline` | `@data inline $n T? : <value> ;` | local state | **true** |
| `fetch` | `@data fetch $n T? : <url> { refresh?; cache?; initial? }` | data-source | false |
| `derive` | `@data derive $n T? : <scalar-expr> ;` | derived-signal | false |
| `fold` | `@data fold $n T? from $src : <step> ;` | derived-signal (reduce) | false |
| `query` | `@data query $n T? from $src { where?; sort?; dir?; limit?; map?; reduce?; initial? }` | computed-source | false |

The `static` fact drives [SSG unrolling](#6-ssg-static-unroll-vs-hydrate). Today only
`inline` array sources are SSG-unrolled; every other kind hydrates at runtime. (A future
refinement may propagate staticity transitively — e.g. a `derive`/`query` over only
`inline` sources — so it too unrolls; see the staticity note in §6.)

### inline — static local state

```st
@data inline $seats number : 3;
@data inline $tags : ["new", "sale"];
```

A compile-time-constant value. Writable at runtime (`$seats <- 4`). Static, so a
list reading it can be SSG-unrolled.

### fetch — remote / async

```st
@data fetch $products Product[] : "/api/products";

@data fetch $news Article[] : "/api/news" {
  refresh: 30s;     // re-fetch interval
  initial: [];      // value before first load
}
```

Yields `$products`, plus `$products_loading`, `$products_error`, `$products_refetch`.

### derive — stateless computed (scalar)

```st
@data inline $price number : 10;
@data inline $qty   number : 3;
@data derive $total number : $price * $qty;
```

Dependencies are discovered from the expression (`$price`, `$qty`); `$total`
recomputes whenever either changes. (It is treated as dynamic for SSG today — it
hydrates at runtime — even when its deps are all static; see §6.)

### derive — union-producing (`@match`, cond-mode)

A derive whose RIGHT-HAND side is a cond-mode `@match` block produces a
first-class union signal instead of a scalar (PLAN-077):

```st
@type BadgeState {
  Blue | Orange | Turquoise | Green | NoProject | None
}

@data derive $badge BadgeState : @match {
  ($isNoProject)                                   => NoProject;
  ($pushPreview_pending || $promote_pending)       => Orange;
  ($isUnauthorized)                                => Blue;
  ($statusData && $statusData.preview_hash != $statusData.production_hash) => Turquoise;
  ($statusData && $statusData.preview_hash == $statusData.production_hash) => Green;
  _ => None;
}
```

The grammar, in four rules:

1. **No subject.** Cond-mode `@match { … }` has no scrutinee — each arm's
   pattern is a parenthesized boolean GUARD, its consequence a variant
   CONSTRUCTOR (`Blue`, or `Loaded($data)` for a payload-carrying variant).
   The parens are load-bearing: the lexer has no `=>` token, so an
   unparenthesized guard would swallow `=> Variant` into the guard text.
2. **First match wins.** Guards evaluate top-to-bottom; the first truthy
   guard's constructor is the value. Arm order IS the precedence — a chain
   of `$showX = ($isX && !$isY && …)` boolean derives collapses into one
   ordered arm list.
3. **The last arm must be `_`.** Guards are arbitrary booleans, so the
   compiler cannot prove coverage; the checkable invariant is that the chain
   always terminates in a value. A missing catch-all is hard error **E0931**.
4. **The type is declared.** Either a named `@type` sum (as above — unknown
   consequence variants are error **E0932**) or an inline variant list
   (`$badge { Blue | Orange | None } : @match { … }`).

Dependencies are discovered from the guard text (here `$isNoProject`,
`$pushPreview_pending`, `$promote_pending`, `$isUnauthorized`, `$statusData`);
the union recomputes and republishes when any dependency changes.

**The produced signal is a plain object** — `{type: 'Orange'}` for a
payload-less variant, `{type: 'Loaded', data: …}` with payload fields spread
alongside `type`. That one shape is consumed uniformly by `@state(when: $badge
is Blue)` gates, dispatch-mode `@match $badge { … }`, and payload destructure
(see docs/language/reactive-output.md §6 "Tagged sums" and the unified
enum-signal story there).

### fold — stateful accumulation

```st
@data fold $cartTotal number from $cart : acc + (item.price * item.qty);
@data fold $cartCount number from $cart : acc + item.qty;
```

Reduces over the `from` source array. `acc` is the accumulator (starts at 0),
`item` the current element. The step may also read other `$signals`.

### query — array pipeline

```st
@data query $active Product[] from $products {
  where: $.inStock == true;   // filter
  sort:  $.price;             // sort key
  dir:   asc;                 // asc | desc
  limit: 6;
  map:   $;                   // optional transform
}
```

The array counterpart to `derive`. `$.field` is the current item's field. All
clauses are optional.

---

## 4. `@each` — rendering lists

`@each` renders a body once per item in a source. The body is **HTML** (the W4
surface) or template invocations.

```st
@data inline $todos : ["Alpha", "Beta", "Gamma"];

<ul class="list"></ul>

.list {
  @each($todos as $t) {
    <li class="todo">`$t`</li>
  }
}
```

- `$t` is the per-item binding; `` `$t` `` / `` `$t.field` `` / `` `$index` `` are
  holes resolved against the item.
- An **empty** `@each { }` body is valid (renders nothing).
- The body may instead be template invocations (`&card($t);`) — see templates.

`@each` re-renders whenever its source signal changes:

```st
$todos <- $todos.concat("Delta");   // list grows reactively
```

---

## 5. Item references inside `@each`

| Token | Resolves to |
|-------|-------------|
| `$t` (the item binding) | the whole current item |
| `$t.field` / `$.field` | a field of the current item |
| `$index` | the 0-based position |
| `$otherSignal` | a global signal (left as-is by `@each`, resolved by normal binding) |

---

## 6. SSG: static unroll vs hydrate

This is the payoff of the `static` fact. For a **static** source (e.g.
`@data inline`), the list rows are **unrolled into the served HTML at compile
time** — a crawler or a JS-disabled client sees real markup:

```html
<!-- served HTML for the @each above -->
<ul class="list"><li class="todo">Alpha</li><li class="todo">Beta</li><li class="todo">Gamma</li></ul>
```

At runtime the same `@each` hydrates and produces the **identical** DOM
(`unroll == hydrate`), so there is no flash or duplication. For a **dynamic**
source (`fetch` / `query` / a mutated signal) the container is served empty and
filled at runtime.

You do not opt into this — it follows from the source's kind. Author a list directly
over an `@data inline` array and it is automatically SEO-friendly. (Lists over
`derive`/`query`/`fetch` hydrate at runtime; transitive-static unrolling is a planned
refinement, not yet shipped.)

### `@template` invocations (FEAT-154)

The SAME `unroll == hydrate` invariant extends to `@template` invocations: an
invocation whose args are ALL compile-time literals unrolls into served HTML —
a crawler / no-JS client sees real markup instead of an empty container:

```st
@template &card($title) {
  <article class="card"><h3>`$title`</h3></article>
}
<main class="page"></main>
.page { &card("Hello"); }
```

```html
<!-- served HTML -->
<main class="page"><article class="card" data-st-ssg="1"><h3>Hello</h3></article></main>
```

At runtime the factory-built (reactive) instance **replaces** the
`data-st-ssg="1"` placeholder rather than appending alongside it — first paint
comes from the static render, interactivity from the runtime factory, and the
DOM converges to the same shape either way.

Staticity rule (deliberately conservative — v1 scope):

- every arg must be a literal (string/number/bool); a `$`-ref arg makes the
  WHOLE invocation dynamic (left to the runtime, unchanged behavior).
- the invoking selector must match exactly ONE empty container (same
  uniqueness proof the `@each` unroll above already enforces).
- a template with any ELEMENT param (`&content`), or a body-slot invocation
  (`&card("T") { <p>Body</p> }`), is **not** unrolled — no static HTML source
  for a slot exists in v1. Left dynamic.
- the template's own local state initials (`$count number: 0;`) seed their
  holes, matching the runtime factory's first-paint value.
- nested composition (a static `@each` row invoking a template, or a template
  invoking another) is **not** unrolled transitively in v1 — a named follow-up,
  same as the `derive`/`query` transitive-staticity note above. It still
  renders correctly at runtime; it simply isn't pre-unrolled.

---

## 6b. Template-body surface — what works inside `@template { … }`

A `@template` body is architecturally **the same construct-recognition
machinery as file scope** (the same `component_item` stdlib grammar, the same
instance-first lexical scope resolution) — not a restricted sublanguage. The
following table is the verified surface as of PLAN-070:

| Construct | Works inside `@template { }`? |
|---|---|
| HTML + backtick holes (incl. full expressions, ternaries) | ✓ |
| Local state (`$open bool: false;`) | ✓ (instance-scoped, isolated per invocation) |
| `@on <event> { … }` — mutation body (`$var <- expr`) | ✓ |
| `@on <event> { … }` — animation-only body (keyframes, `& { a -> b }`) | ✓ |
| Class toggles, content/attr bindings | ✓ |
| `@each` (nested, template-invoking or inline HTML) | ✓ |
| `@match` | ✓ |
| `@data inline` declared inside the body | ✓ (registers globally — prefer local state for instance data) |
| `@fn`, `@effect`, `@portal` | ✓ |
| Plain CSS, pseudo-classes/elements | ✓ |
| `@media` / `@supports` / `@keyframes` | ✓ (hoisted to the page stylesheet, globally-namespaced) |
| `@reveal`, `@scroll`, preset animation macros | ✓ |
| Optional element param (`&footer?`) | ✓ |
| Nested template invocation | ✓ |
| `@exports` + named instance refs | ✓ |
| Static (literal-arg) invocation SSG unroll | ✓ (see §6a) |
| Scoped CSS (no cross-template class collision) | ✗ — not yet; BEM-prefix your classes. Tracked: FUP-062 |
| Transitive-static unroll (nested static compositions) | ✗ — not yet; named follow-up in §6a |

An UNKNOWN directive in a body (a genuine typo, zero forms registered) surfaces
E0900 rather than silently vanishing.

---

## 7. Migration

The legacy block forms are deprecated. Map them as:

| Legacy | Unified |
|--------|---------|
| `@data x: T { src: "/u"; cache: 5m }` | `@data fetch $x T : "/u" { refresh: 5m }` |
| `@data x: T { initial: v }` | `@data inline $x T : v;` |
| `@computed $x: T { from: $s; where: …; sort: $.p asc }` | `@data query $x T from $s { where: …; sort: $.p; dir: asc }` |
| `@computed $x: T { from: $s; reduce: (a,$)=>…; initial: 0 }` | `@data fold $x T from $s : a + …` |

Notes:
- Type moves from `$name: Type` (colon) to `$name Type` (positional); `:` now
  introduces the value.
- The item alias `$` becomes `item` (fold) or `$.field` (query/each).
- `sort: $.price asc` splits into `sort: $.price; dir: asc;`.
- `fold` hardcodes `initial: 0`; for a non-zero accumulator use `@data query … { reduce; initial }`.

---

## 8. Meta-compiler note (library authors)

Everything above is **stdlib grammar**, not compiler builtins. The compiler ships
the *alphabet* (sigils, the `<`/`` ` `` HTML terminals, the capture-type engine,
PEG terminals like `balanced(';')`) and the matching engine. The `@data` kinds,
`@each`, the query pipeline — all are `%macro`/`%primitive`/`%capture_type`
definitions in `stdlib/`. A new data kind is a new macro; a new terminal is one
stanza. See `docs/language/macros.md` and `docs/language/capture-types.md`.
