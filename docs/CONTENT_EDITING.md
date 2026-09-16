# Spacetime Content Editing — Unified Content Addressing Scheme

This document is the formal contract between three layers of the PROJ-107 click-to-edit system: provenance injection (server and compiler), browser-side editing (`dev-editable`, `dev-save`), and server-side handling (`handle_edit_ast`). Every selector format, every dispatch rule, and every hash input is specified here.

## Overview

In dev mode, text-bearing elements receive two attributes that together encode their source location:

- `data-st-id` — a stable content hash identifying the element within its source file
- `data-st-origin` — the full provenance string: `{file}::{locator}`

The browser reads these attributes to construct an `EditAst` message. The server dispatches that message based on file extension and selector type. The `__content` key in the patch object is the sentinel that routes the message to innerHTML replacement rather than property patching.

---

## Section 1: `data-st-origin` Format

### Format

```
data-st-origin="{file}::{locator}"
```

The `::` separator is the split point. Everything before it is the file path (relative to site root). Everything after it is the locator.

### Locator by Source Type

**HTML files** (`.html` / `.htm`): the locator is the bare `data-st-id` value. No selector syntax, no quotes, no brackets.

```html
data-st-origin="index.html::st-a3f2b1c0"
```

**`.st` files**: the locator is the AST path in `<css-selector> @<directive> <element-tag>` format.

```html
data-st-origin="components.st::.card @template h3"
```

### Why No Quotes in the Locator

The browser uses `querySelectorAll('[data-st-origin="..."]')` to find sibling instances after a successful edit. If the locator itself contained quotes, the attribute selector would break. The locator must be quote-free by design.

### Browser Selector Reconstruction

When `dev-editable` reads `data-st-origin`, it splits on `::` and reconstructs the full CSS selector based on file extension:

| File extension | Reconstruction rule | Example selector |
|---|---|---|
| `.html` / `.htm` | `[data-st-id="${locator}"]` | `[data-st-id="st-a3f2b1c0"]` |
| `.st` | locator used as-is | `.card @template h3` |

This reconstructed selector becomes the `selector` field in the `EditAst` message.

---

## Section 2: Content Hash Format

### ID Format

All auto-generated IDs follow the pattern `st-{first_8_hex_chars}`, e.g., `st-a3f2b1c0`.

Elements that already have a user-provided `data-st-id` keep theirs. Auto-generation skips them.

### Hash Inputs by Source Type

The hash inputs differ because the two injection sites have different information available.

**HTML files** (serve-time injection via lol_html in `src/server.rs`):

```
hash(file_path + ":" + tag_name + ":" + occurrence_index)
```

- `file_path` — relative path from site root, e.g., `index.html`
- `tag_name` — lowercase tag name, e.g., `h1`
- `occurrence_index` — 1-based count of how many times this tag has appeared so far in the document

lol_html is a streaming rewriter. It has no byte offsets. Structural position (occurrence index) is the stable identifier available at stream time.

Example: the first `h1` in `index.html` hashes `"index.html:h1:1"` → `st-a3f2b1c0`.

**`.st` template bodies** (compile-time injection in `form_compiler.rs`):

```
hash(file_path + ":" + ast_byte_offset + ":" + tag_name)
```

- `file_path` — relative path from site root, e.g., `components.st`
- `ast_byte_offset` — byte offset of the element within the `.st` source file
- `tag_name` — lowercase tag name, e.g., `h3`

The compiler has full AST position information, so byte offsets are available and stable.

Example: an `h3` at byte 245 in `components.st` hashes `"components.st:245:h3"` → `st-b7d4e2f1`.

### Hash Stability Caveats

HTML file IDs shift if new elements of the same tag type are inserted before existing ones (occurrence index changes). This is acceptable for dev-mode tooling. The page reloads frequently during development, and stale IDs are cleared on reload.

`.st` template IDs shift if the source file is reformatted (byte offsets change). Again, acceptable for dev mode.

---

## Section 3: `__content` Sentinel Semantics

### The Sentinel

When an `EditAst` patch object contains the key `"__content"`, the server replaces the element's `innerHTML`. This is a hard branch guard.

```json
{
  "type": "EditAst",
  "file": "index.html",
  "selector": "[data-st-id=\"st-a3f2b1c0\"]",
  "patch": { "__content": "Welcome to Spacetime" },
  "op_id": "op_1709..."
}
```

### Why the Guard Exists

`apply_text_patch()` in `src/sync/handlers.rs` searches for `key:` patterns in `.st` source text and performs text-based property patching within a directive's source span. Routing `__content` through that function would either silently corrupt the file (if it found a spurious `__content:` pattern) or crash (if it found nothing and returned an error).

The `__content` branch is therefore checked **before** any existing processing:

```rust
fn handle_edit_ast_inner(site_dir, file, selector, patch) {
    let file_path = validate_file_path(site_dir, file)?;

    // BRANCH GUARD: __content edits use a completely separate code path
    if let Some(content) = patch.get("__content").and_then(|v| v.as_str()) {
        return handle_content_edit(&file_path, file, selector, content);
    }

    // EXISTING CODE: property patching (unchanged)
    let (css_selector, directive_name) = parse_ast_selector(selector)?;
    // ... apply_text_patch logic ...
}
```

### Non-`__content` Patches

If the patch does not contain `"__content"`, the existing property patching behavior is used unchanged. `parse_ast_selector()` is called, the directive is located, and `apply_text_patch()` runs. This path is unaffected by PROJ-107.

---

## Section 4: Selector Grammar for `.st` Content

### Existing Grammar (unchanged)

`parse_ast_selector()` in `src/sync/handlers.rs:324` handles the existing format:

```
<css-selector> @<directive>
```

It splits on the last ` @`. This function is not modified.

### New Grammar: `parse_content_selector()`

Content editing introduces a new function, `parse_content_selector()`, that handles three formats:

#### Format 1: HtmlElement

```
[data-st-id="st-a3f2b1c0"]
```

No `@` present. Used for HTML file origins. The selector is a lol_html-compatible attribute selector that `handle_content_edit_html` passes directly to lol_html.

#### Format 2: AstContent

```
.card @template h3
```

Has `@`, and there is text after the directive name. The text after the directive is the element path within the template's HTML string.

Parsed into three parts:
- `css_selector`: `.card`
- `directive`: `template`
- `element_path`: `h3`

#### Format 3: AstDirective

```
.hero @scroll
```

Has `@`, nothing after the directive name. This is the existing behavior, handled by `parse_ast_selector()`. `parse_content_selector()` recognizes it and returns an `AstDirective` variant for completeness, but the `__content` guard means this combination never reaches `handle_content_edit`.

### lol_html Selector Capabilities

The `element_path` in `AstContent` selectors must be a lol_html-compatible CSS selector. lol_html supports:

- Tag selectors: `h3`, `p`, `span`
- Class selectors: `.title`
- ID selectors: `#main`
- Attribute selectors: `[data-st-id="st-b7d4e2f1"]`
- Comma-separated lists: `h3, h4`

lol_html does **not** support:

- Descendant combinators: `.parent .child`
- Child combinators: `.parent > .child`
- Pseudo-selectors: `:first-child`, `:hover`

Template HTML elements are identified by tag name or attribute. Descendant paths are not needed because the element path targets a specific element type within the template body, and template bodies are typically shallow.

---

## Section 5: Dispatch Rules

### Exhaustive Dispatch Table

| File extension | Selector type | `__content` in patch | Action |
|---|---|---|---|
| `.html` / `.htm` | `HtmlElement` | yes | lol_html find+replace in HTML file |
| `.st` | `AstContent` | yes | parse AST, find FormMatch, lol_html on `ComponentBodyDef::html`, splice back |
| `.st` | `AstDirective` | no | existing `apply_text_patch` (property patching) |
| `.html` / `.htm` | any | no | Reject: property patching not supported for HTML files |
| other extension | any | any | Reject with clear message |
| `.st` | `HtmlElement` | yes | Reject: HtmlElement selector not valid for .st files |
| `.html` / `.htm` | `AstContent` | yes | Reject: AstContent selector not valid for HTML files |

### `suppress_reload_path`

`suppress_reload_path` is set on **all** successful content edits. This prevents the file watcher from triggering a full page reload after the source file is written. The browser already has the new content in the DOM from the edit; a reload would be disruptive.

### Handler Functions

```
handle_edit_ast()
  └─ handle_edit_ast_inner()
       ├─ [__content present] → handle_content_edit()
       │    ├─ [.html + HtmlElement] → handle_content_edit_html()
       │    │    └─ update_element_by_selector() via lol_html
       │    ├─ [.st + AstContent]   → handle_content_edit_st()
       │    │    └─ parse AST → find FormMatch → lol_html on body.html → splice
       │    └─ [other]              → Reject
       └─ [__content absent] → parse_ast_selector() → apply_text_patch() (unchanged)
```

---

## Section 6: Worked Examples

### Example 1: HTML Heading (`<h1>` in `index.html`)

**Source file** (`index.html`):

```html
<body>
  <h1>Welcome to Our Product</h1>
  <p>The best product on the market.</p>
</body>
```

**After serve-time provenance injection** (debug mode, `src/server.rs`):

Hash input for the `h1`: `"index.html:h1:1"` → `st-a3f2b1c0`

```html
<body>
  <h1 data-st-id="st-a3f2b1c0" data-st-origin="index.html::st-a3f2b1c0">Welcome to Our Product</h1>
  <p data-st-id="st-d1e2f3a4" data-st-origin="index.html::st-d1e2f3a4">The best product on the market.</p>
</body>
```

**`readProvenance()` return value** (in `dev-editable`):

```javascript
{
  editType: 'content',
  stId: 'st-a3f2b1c0',
  file: 'index.html',
  selector: '[data-st-id="st-a3f2b1c0"]'
}
```

**`EditAst` message sent by `dev-save`**:

```json
{
  "type": "EditAst",
  "file": "index.html",
  "selector": "[data-st-id=\"st-a3f2b1c0\"]",
  "patch": { "__content": "Welcome to Spacetime" },
  "op_id": "op_1709123456789"
}
```

**Server dispatch path**:

1. `handle_edit_ast_inner` detects `__content` → calls `handle_content_edit`
2. File extension is `.html`, selector parses as `HtmlElement`
3. `handle_content_edit_html` reads `index.html`, runs lol_html with selector `[data-st-id="st-a3f2b1c0"]`
4. lol_html finds the `h1`, replaces innerHTML with `"Welcome to Spacetime"`
5. File written to disk, `Ack` returned with `suppress_reload_path`

---

### Example 2: HTML Paragraph (`<p>` in `index.html`)

**Source file** (`index.html`):

```html
<body>
  <h1>Welcome to Our Product</h1>
  <p>The best product on the market.</p>
</body>
```

**After serve-time provenance injection**:

Hash input for the `p`: `"index.html:p:1"` → `st-d1e2f3a4`

```html
<p data-st-id="st-d1e2f3a4" data-st-origin="index.html::st-d1e2f3a4">The best product on the market.</p>
```

**`readProvenance()` return value**:

```javascript
{
  editType: 'content',
  stId: 'st-d1e2f3a4',
  file: 'index.html',
  selector: '[data-st-id="st-d1e2f3a4"]'
}
```

**`EditAst` message**:

```json
{
  "type": "EditAst",
  "file": "index.html",
  "selector": "[data-st-id=\"st-d1e2f3a4\"]",
  "patch": { "__content": "The finest product available." },
  "op_id": "op_1709123456790"
}
```

**Server dispatch path**:

Same as Example 1. File extension `.html`, selector `HtmlElement`, lol_html replaces the `p` element's innerHTML.

---

### Example 3: `.st` Template Single Instance

**Source file** (`components.st`):

```css
.card {
  @template(name: card) {
    <div class="card">
      <h3>Card Title</h3>
      <p>Card description goes here.</p>
    </div>
  }
}
```

**After compile-time provenance injection** (debug build):

The `h3` is at byte offset 245 in `components.st`. Hash input: `"components.st:245:h3"` → `st-b7d4e2f1`.

The `ComponentBodyDef::html` string becomes:

```html
<div class="card">
  <h3 data-st-id="st-b7d4e2f1" data-st-origin="components.st::.card @template h3">Card Title</h3>
  <p data-st-id="st-c8e5f6a2" data-st-origin="components.st::.card @template p">Card description goes here.</p>
</div>
```

**`readProvenance()` return value** (user clicks the `h3`):

```javascript
{
  editType: 'content',
  stId: 'st-b7d4e2f1',
  file: 'components.st',
  selector: '.card @template h3'
}
```

**`EditAst` message**:

```json
{
  "type": "EditAst",
  "file": "components.st",
  "selector": ".card @template h3",
  "patch": { "__content": "New Card Title" },
  "op_id": "op_1709123456791"
}
```

**Server dispatch path**:

1. `handle_edit_ast_inner` detects `__content` → calls `handle_content_edit`
2. File extension is `.st`, selector parses as `AstContent` with `css_selector=".card"`, `directive="template"`, `element_path="h3"`
3. `handle_content_edit_st` parses `components.st` AST, finds the FormMatch for `.card @template`
4. Extracts `ComponentBodyDef::html` string
5. Runs lol_html on the HTML string with selector `h3`, replaces innerHTML with `"New Card Title"`
6. Splices modified HTML string back into `.st` source at the correct byte span
7. File written to disk, `Ack` returned with `suppress_reload_path`

---

### Example 4: `.st` Template Multi-Instance (`@each` producing 5 cards)

**Source file** (`components.st`):

```css
.card {
  @template(name: card) {
    <div class="card">
      <h3>Card Title</h3>
    </div>
  }
}

.product-list {
  @each(products) {
    template: "card";
  }
}
```

**After compile-time provenance injection**:

The `h3` in the template definition gets `data-st-id="st-b7d4e2f1"` and `data-st-origin="components.st::.card @template h3"`. This is injected **once** into `ComponentBodyDef::html`.

When `@each` instantiates the template 5 times, each instance renders with the same annotated HTML string. All 5 rendered `h3` elements share:

```html
<h3 data-st-id="st-b7d4e2f1" data-st-origin="components.st::.card @template h3">Card Title</h3>
```

**User clicks one instance's `h3`**:

`readProvenance()` returns the same value as Example 3. The `EditAst` message is identical.

**Server dispatch path**: identical to Example 3. The template source is updated once.

**After `Ack` — live propagation in `dev-save`**:

```javascript
// Reconstruct the origin attribute value
var originAttr = 'components.st::.card @template h3';

// Find all elements sharing this origin
var siblings = document.querySelectorAll(
  '[data-st-origin="' + CSS.escape(originAttr) + '"]'
);

// Update all siblings (except the one already edited)
for (var i = 0; i < siblings.length; i++) {
  if (siblings[i] !== editedElement) {
    siblings[i].innerHTML = 'New Card Title';
  }
}
```

All 5 instances update their innerHTML. No page reload occurs.

---

### Example 5: Element with Both `data-t` and `data-st-id` (Locale Wins)

**Source file** (`index.html`):

```html
<h1 data-t="hero.title">Welcome</h1>
```

**After serve-time provenance injection**:

The injection pass checks for `data-t` before computing a hash. Elements with `data-t` are skipped. The element is served as-is:

```html
<h1 data-t="hero.title">Welcome</h1>
```

No `data-st-id` or `data-st-origin` is added.

**`readProvenance()` return value** (user clicks the `h1`):

```javascript
// Priority chain: data-t checked first
var tKey = el.getAttribute('data-t');
if (tKey) {
  return { editType: 'locale', key: tKey, ... };
}
// data-st-id branch never reached
```

Returns `{ editType: 'locale', key: 'hero.title', ... }`.

**`EditAst` message**: none. `dev-save` constructs an `EditJson` message targeting the locale file instead.

**Content edit path**: NOT taken. The priority chain ensures `data-t` always wins. An element cannot be both a locale edit and a content edit.

The same logic applies to `data-st-bind`: elements with that attribute are skipped by the provenance injection pass and handled by the data edit path.

---

## Rejection Messages

| Condition | Rejection reason |
|---|---|
| File extension not `.html`, `.htm`, or `.st` | `"Content editing not supported for .{ext} files"` |
| `.html` file, element not found | `"Element with data-st-id=\"...\" not found"` |
| `.st` file, scope not found | `"Scope '{selector}' not found"` |
| `.st` file, directive not found | `"Directive '@{name}' not found"` |
| `.st` file, element not found in template HTML | `"Element '{path}' not found in template body"` |
| `.html` file with non-`__content` patch | `"Property patching not supported for HTML files"` |
| Path traversal attempt | `"Path traversal not allowed"` |

---

## Relationship to Existing Protocol

### `EditHtml` (deprecated)

`EditHtml` was the original mechanism for HTML content editing. It is being deprecated in favor of `EditAst` with `__content`. The server handler for `EditHtml` will delegate to `handle_content_edit` internally for backwards compatibility, but new browser-side code must not construct `EditHtml` messages.

### `EditJson`

Unchanged. Used for locale edits (`data-t`) and data edits (`data-st-bind`). Content edits never produce `EditJson` messages.

### `EditAst` (extended)

The unified message type for all source-file edits. The `__content` key in `patch` is the new branch. All other `EditAst` messages (property patching for `.st` directives) are unaffected.

---

## Dev Mode Gating

Provenance injection runs only when the server is started with `--debug` (which sets `AppMode::Dev { debug: true }` in `src/server.rs`). In production mode, no `data-st-id` or `data-st-origin` attributes are emitted. Zero overhead.

The compiler-side injection similarly runs only in debug builds. The `ComponentBodyDef::html` string is annotated only when the debug config flag is set.

---

## Section 8: PLAN-034 — Cardinality-Keyed Content Architecture

PLAN-034 dissolved two anti-patterns in the CMS admin (FEAT-092): **AP1**
(JS arrow-functions inside `@data derive` + `ST.*` shaping helpers) and **AP2**
(helpers hardcoding JSON field names + emitting a parallel shadow-schema of
descriptors). The fix is a single architectural law, plus two grammar/runtime
surfaces that already existed and were re-surfaced.

### The law: cardinality picks storage + edit-channel, not the renderer

```
singleton  (one instance, structural) → typed literal in .st → @cms(T) → EditAst
collection (many instances, scale)     → @data + JSON          → @cms(T) → EditJson
```

Both are typed `$`-values; **one form-renderer** (`@each` over a schema ×
dynamic-dispatch `&$f.tpl($f)` to `&fld-<widget>` templates) renders both. The
only difference is *where the value persists* and *which edit op carries a
write* — a storage policy, never a programming-model split. The admin author
never reverse-engineers display from raw JSON; the server delivers
source-determined facts (type, widget, group, signature, label) and `.st` owns
UI bindings (which template renders a kind, ordering, copy).

### Brand singleton (Wave B): tokens live in code, edited via EditAst

Design tokens are a **singleton** — one typed instance — so they live in the
project source as a typed literal, not in a JSON file and not as CSS custom
properties scanned by JS:

```spacetime
@type Brand { scarlet: color; ink: color; radius: length; reveal: duration; }
$brand Brand : { "scarlet": "#FF0020", "ink": "#0a0a0a", "radius": "8px", "reveal": "600ms" };
@cms(Brand) { editable: inline; }     // exposes Brand to the admin; write-channel = EditAst
```

Reference tokens directly as first-class `$`-values (no `var()`, no JSON):

```spacetime
.hero { background: $brand.scarlet; border-radius: $brand.radius; }
```

- **Server delivery**: `/__spacetime/dev/brand.json` (`build_brand_json`) finds the
  `@data inline` marked `@cms(T){ editable: inline }`, returns
  `{ name, type, schema (widget-annotated), values, file }`. Domain types
  `color`/`length`/`duration` carry a `format` so the widget engine maps
  `color → color swatch`, `length`/`duration → text`.
- **Admin pane**: fetches `brand.json` → `ST.adFormFields(schema, values)` (the
  same shaper the entry drawer uses) → `@each` × `&$f.tpl` → `&fld-<widget>`.
- **Write**: `ST.adBrandInput` sends `EditAst` with the **file-scope binding
  selector** `$brand §data` (below).

### File-scope binding selector for EditAst

A singleton is a *top-level* `@data inline` directive — no CSS scope. EditAst's
selector grammar is extended so a selector beginning with `$name` (rather than a
`<css> §<directive>` pair) resolves a **file-scope** binding by name:

```
selector = "$brand §data"   →  find the file-scope @data inline whose binding is `brand`
```

`handle_edit_ast_inner` branches: a `$`-led selector searches `ast.matches`
(file scope) by `get_binding("name")`; otherwise the existing scope-lookup path
runs. The matched directive's source span feeds `apply_text_patch`.

### Object-literal patching (BUG-092 / BUG-093)

`apply_text_patch` is shared by two write shapes:

| surface | key form | terminator | value encoding |
|---|---|---|---|
| directive body (`duration: 800ms;`) | bare `key:` | first `;`/`}` | verbatim |
| object literal (`"ink": "#0a0a0a",`) | quoted `"key":` | first **depth-0** `,`/`;`/`}` (respecting nesting + quotes) | `serde_json::to_string` (escaped) |

The depth-aware terminator prevents editing one object key from swallowing its
siblings (BUG-092). String object-entry values are **JSON-escaped** via
`serde_json::to_string`, so a value containing `"`/`}`/newline can neither
corrupt the `.st` literal nor inject `.st` source from a dev-ws message
(BUG-093).

### Query surface (Wave A): declarative filter/map replace JS-in-derive

`@data query` (backed by the `computed-source` primitive) gained working
`where`/`map` lowering + reactivity (BUG-090). Expressions are raw strings
rewritten in `%emit js`: `$.field → item.field`, `$sig → SpacetimeLocal['sig']`
(quote-aware, dep-discovered + watched). A left-insertion **pipe** desugars
`a | f(b)` → `f(a, b)`; query helper fns (`includes`/`contains`/`basename`/
`titleize`/…) are usable bare or as pipe targets. A member-path source
`from $doc.images` resolves the root signal and walks the tail.

```spacetime
@data query $assets from $assetDoc.images {
  where: $.path | includes($mediaQuery);          // reactive: a keystroke re-filters
  map:   { path: $.path, name: $.path | basename };
  limit: 400;
}
```

This replaced the admin media library's arrow-fn derive — the last
JS-in-derive case — with zero JS surface.

### Reactive `$`-value in CSS (BUG-091)

A reactive signal in a CSS-property position (`background: $brand.scarlet;`) now
emits `node.style.setProperty`, not `setAttribute`. The `:` surface is CSS; the
`<-` arrow surface is attribute injection (`src <- $u;`). `CssDeclaration` carries
an `is_injection` flag (set when the property node has a `<-`) so
`emit_reactive_binding_js` routes the two correctly.

### Self-describing acceptance

The whole architecture is validated by the `@data registry` property, now for
content: **add a field to `@type Brand` (+ a value) → the admin Brand pane
renders AND edits it with zero admin-code changes.** A new `accent: color` field
appears as a 6th color-swatch field; editing it rewrites the `.st` literal via
EditAst with siblings byte-exact. Same for a new `@template` (Components pane) or
`@reveal`/`@scroll` (Motion pane) — they surface from their server contracts with
no admin edits.

### What remains as tracked follow-ups

- **FUP-044** — bare-key typed object literals (`{ scarlet: #FF0020 }`); today the
  singleton uses quoted-key JSON values. A unified `%capture_type` would own
  key-quoting + domain-scalar lowering in one place.
- **BUG-084/085** — nested-ternary-paren derive grammar; nested `@each` in a
  template body. Worked around, not blocking.
