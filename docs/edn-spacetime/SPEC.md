# EDN ⟷ Spacetime — Specification

**Status:** normative. Implements PLAN-148.
**Audience:** an implementor who has never seen this design. You should need zero
follow-up questions.

Every claim marked *(measured)* was produced by running a probe against the live
`STDLIB_REGISTRY` and against both readers (`clojure-reader` 0.3 and
`BeamLisp.Reader`) on 2026-08-13. Numbers are facts, not estimates.

---

## 1. Thesis

Spacetime's grammar is **already data**.

`src/syntax/ARCHITECTURE.md` states it plainly: *"All user-facing syntax in
Spacetime is defined via %form patterns. There are no hardcoded parsers for
specific syntax constructs."* — 415 registered forms across 128 stdlib `.st`
files *(measured)*.

Therefore an EDN surface is **not** a hand-written translator with one branch per
construct. It is a **projection of the form registry**: implement it once, and it
covers all 415 forms — and every form anyone adds later, for free.

```
      %form registry  (415 forms — the single grammar authority)
              │
      ┌───────┴───────┐
   .st text        EDN text          ← two concrete syntaxes, peer status
      └───────┬───────┘
         Vec<FormMatch>              ← the waist; both directions meet here
              │
    pipeline (resolve→sort→expand→emit)   ← UNTOUCHED
```

### Why `FormMatch` is the waist

| stage | type | lossless | serde |
|---|---|---|---|
| 0 bootstrap | `SyntaxRegistry` ← `%form` | grammar authority | `FormClause` ✓ |
| 1 text→CST | `ParseResult{root: SyntaxNode}` (rowan) | ✓ trivia + comments | ✗ |
| 2 CST→AST | `parser::ast::StFile` | ✗ drops trivia | partial |
| **3 match** | **`StFile.matches: Vec<FormMatch>`** | semantic only | **✓ full** |
| 4 resolve | `ResolvedPrimitive` | ✗ | ✓ |
| 5 emit | `PipelineOutput{js,css,html}` | terminal | ✓ |

Two verified facts pin the choice:

1. `FormMatch` derives `Serialize, Deserialize` (`src/syntax/form_match.rs:20`
   and `:505`).
2. `pipeline::compile(matches: &[FormMatch], ctx)`
   (`src/pipeline/mod.rs:1227`) consumes **exactly this** and nothing else from
   the parse.

∴ an EDN-origin program compiles through the **identical** backend. There is no
second pipeline and therefore no possibility of divergence.

---

## 2. Bijection class: A (semantic)

**Definition.** For any Spacetime program `P`:

```
compile(read_edn(write_edn(parse(P))))  ≡  compile(parse(P))
```

Round-trip preserves **meaning**, proven by identical compiler output.

**Preserved:** every `FormMatch` (macro identity, all captures, selector,
namespace qualifier), all sibling `StFile` fields (§9), source spans (so
diagnostics point at real source).

**NOT preserved:** comments, whitespace, formatting, the author's choice among
equivalent spellings.

**Rejected: class B (textual bijection).** Byte-identical round-trip would
require anchoring to the rowan CST plus a trivia model plus a formatter — roughly
3–4× the work — and buys only clean git diffs for `.st` edited as EDN. Class A
does not foreclose B; if B is ever wanted, it is CST-anchored, not
`FormMatch`-anchored.

### Proof obligations

| # | obligation | where |
|---|---|---|
| P1 | `∀ v: CapturedValue. from_edn(to_edn(v)) == v` | W1 proptest |
| P2 | `∀ f ∈ corpus. parse(f).matches == parse(print(parse(f).matches)).matches` | W2 corpus |
| P3 | paired `X.st` / `X.edn` ⇒ **byte-identical** `PipelineOutput` | W3 fixtures |
| P4 | whole-file bijection incl. sibling fields | W4 |
| P5 | same function via `.st` and via EDN ⇒ identical bundle hash | W5 |
| P6 | existing verse test suite green | W5 gate |

---

## 3. Wire format

**One rule, 415 forms:**

```clojure
(<macro-name> :<capture> <value> …)
```

- `<macro-name>` is the registry's own key — `RegisteredForm.macro_name`,
  looked up with `SyntaxRegistry::get_by_macro_name`.
- `:<capture>` keys are the capture names declared in that form's `%form`
  pattern.
- **Literals are never written.** `inline`, `from`, `:`, `;` come from
  `form.inline_elements` at print time.

### Why keyed on the macro, not the directive

*(measured)*

| metric | value |
|---|---|
| total registered forms | 415 |
| distinct `directive_name` | 268 |
| directives with >1 macro | **37** |
| `@data` alone | **34 macros** |
| `macro_name` distinct | **414 / 415** |
| macro names needing quoting | 0 |

`@data inline` is **two literals inside one form pattern**, not a "`data`
directive taking an `:inline` argument":

```
%macro data-inline {
  %form { @data inline $name:binding $type:typeref? : $value:expr ; }
}
```

Probe output, showing literals live in the pattern *(measured)*:

```
data-inline               directive=@data  literals=["inline", ":", ";"]
                            inline_caps=["name","type","value"] params=[] body=None
data-stream-event-source  directive=@data  literals=["stream","from",";"]
                            inline_caps=["name","type","url"] params=[] body=None
mcp-action                directive=@mcp-action  literals=[]
                            params=["action","target","value","targetAttr","valueAttr","on"]
```

A `(st/data :inline …)` design would need a hand-authored kind-word → macro table
covering 37 directives / ~120 macros, duplicating knowledge `%form` already
holds, and rotting on every new `%macro`. **Rejected.**

### Two capture surfaces, both from the pattern

`%form` distinguishes inline captures from parenthesised params. Both become
keyword args; the printer knows which is which from the pattern:

```clojure
(data-inline :name $mcpPrompt :value "")              ; inline captures
(mcp-action  :action "kit-confirm" :value "confirm")  ; params
(fn :name add :params [$a $b] :returnType "number" :body [:st/expr "$a + $b"])
```

### Mechanical fallback form

Every form additionally has a total, generated representation. Use it when a form
has no idiomatic spelling, and as the canonical debugging view:

```clojure
{:st/form     "data-inline"
 :st/matched  "data-inline"
 :st/captures {"name"  [:st/binding "$mcpPrompt"]
               "value" [:st/expr "\"\""]}
 :st/selector nil
 :st/span     [28 57]}
```

`:st/form` ↔ `FormMatch.macro_name`, `:st/matched` ↔ `FormMatch.matched_macro`.
**Both are required**: parsing `@data inline $x : "";` yields
`macro_name: "data"` with `matched_macro: "data-inline"` *(measured)*. Resolve
honours `matched_macro`; dropping it changes program meaning.

---

## 4. `CapturedValue` ⟷ EDN — complete mapping

All 24 variants of `src/syntax/form_match.rs::CapturedValue`. This table is
normative and exhaustive.

| variant | Rust payload | EDN encoding |
|---|---|---|
| `Ident` | `String` | symbol — `add` |
| `String` | `String` | string — `"hello"` |
| `Number` | `f64` | number — `3.14` |
| `Bool` | `bool` | `true` / `false` |
| `Time` | `u32` (ms) | `[:st/time 500]` |
| `Length` | `LengthValue{value,unit}` | `[:st/len 20 :px]` |
| `Selector` | `String` | `[:st/sel ".kit-btn"]` |
| `Binding` | `String` | symbol, `$`-prefixed — `$mcpPrompt` |
| `Element` | `String` | symbol, `&`-prefixed — `&main` |
| `Expr` | `String` | `[:st/expr "$a + $b"]` |
| `TypeRef` | `String` | `[:st/type "Message[]"]` |
| `Preset` | `String` | `[:st/preset :ease-out]` |
| `Color` | `String` | `[:st/color "#f0e6d6"]` |
| `Json` | `JsonValue` | `[:st/json <edn>]` |
| `Block` | `Vec<FormMatch>` | `[:st/block (form…) (form…)]` |
| `Array` | `Vec<CapturedValue>` | `[:st/array v1 v2 …]` |
| `Named` | `HashMap<String,CapturedValue>` | `[:st/named {"k" v …}]` |
| `Properties` | `Vec<PropertyDef>` | `[:st/props [name type optional?] …]` |
| `Params` | `Vec<ParamDef>` | `[:st/params [name type default?] …]` |
| `Keyframes` | `Vec<KeyframeDef>` | `[:st/keyframes [prop [vals] sel?] …]` |
| `ParamList` | `Vec<TemplateParamDef>` | `[:st/paramlist [name kind opt? type? default? coll?] …]` |
| `StyleProperties` | `Vec<(String,String)>` | `[:st/styles ["k" "v"] …]` |
| `PatternMatch` | `{signal,variant,bindings}` | `[:st/pmatch sig variant [binds]]` |
| `ComponentBody` | *(unit marker)* | `[:st/component-body]` |

Notes:

- `ComponentBody` is a **bare presence marker** (`form_match.rs:102`). It carries
  no data — the payload lives in the `@template:<name>` scope and body
  diagnostics land in `StFile.diagnostics`. Encode the marker only.
- `Json` nests an arbitrary EDN value; JSON objects become EDN maps with **string**
  keys (not keywords) to preserve exact key bytes.
- The `[:st/…]` head keyword makes every non-scalar self-describing, so decoding
  is a single dispatch with no ambiguity against user data.

---

## 5. Scalar encodings — and why

Each rule below exists because the naive alternative was **measured to fail**.

### 5.1 Units are vectors, never tagged literals

```clojure
[:st/time 500]        ; NOT #st/time 500
[:st/len 20 :px]      ; NOT #st/len [20 :px]
```

*(measured)* `BeamLisp.Reader` has **no EDN tagged literals**. `#st/time 500`
raises `"expected one form, got 2"` — it reads `#st/time` as a symbol and `500`
as a separate form. beam-lisp has `#Name{...}` **record** literals (a `{` map
body is mandatory), which is a different construct.

Vectors need **zero** reader support in either runtime. Verified in both.

> Adding EDN tagged literals to `BeamLisp.Reader` is worthwhile on its own merits
> (reading real-world EDN), but this specification **must not depend on it**.

### 5.2 Typerefs are strings

```clojure
:type "Message[]"     ; NOT :type Message[]
```

*(measured)* **This is a silent-corruption trap.** `:type Message[]` parses
**without error** in `clojure-reader` and yields *two* values —
`Symbol("Message")` followed by `Vector([])`. The `[]` is lost and no diagnostic
fires. Always quote typerefs.

### 5.3 Selectors are strings

```clojure
[:st/sel ".kit-btn"]
(sel "#hero" …)
```

*(measured)* `.kit-btn` reads as a symbol in both runtimes, but `#hero` is
**EOF-error in `clojure-reader`** while reading fine in beam-lisp. Strings are
total; bare selector symbols are not.

### 5.4 Bare unit numbers are never valid

*(measured)* `500ms`, `20px`, `50%` all raise `InvalidNumber` in
`clojure-reader`. Use §5.1 vectors.

---

## 6. Sigils

### 6.1 Collision matrix *(measured, both readers)*

| sigil | `.st` means | clojure-reader | `BeamLisp.Reader` | verdict |
|---|---|---|---|---|
| `$x` | binding | `Symbol("$x")` | `{:symbol,"$x"}` | **safe verbatim** |
| `&x` | element ref | `Symbol("&main")` | `{:symbol,"&main"}` | **safe verbatim** |
| `.cls` | selector | `Symbol(".kit-btn")` | `{:symbol,".kit-btn"}` | safe; prefer string |
| `%x` | metasystem | `Symbol("%macro")` | `{:symbol,"%macro"}` | safe outside `#()` |
| **`@x`** | directive | `Symbol("@foo")` | **`(deref foo)`** | **COLLIDES** |
| **`~x`** | preset | `Symbol("~ease-out")` | **`(unquote ease-out)`** | **COLLIDES** |
| `#hero` | id selector | **EOF error** | `{:symbol,"#hero"}` | divergent |
| `500ms` | time | **InvalidNumber** | `{:symbol,"500ms"}` | divergent |

### 6.2 `$` and `&` pass through verbatim

Both readers treat them as ordinary symbol constituents. This is a genuine gift:
bindings and element refs are the most frequent tokens in `.st`, and they need no
encoding at all.

### 6.3 `@` is banished — and that is what makes deref work

Because directives are keyed by **macro name** (§3), no EDN form begins with `@`.
`@` is therefore free to mean exactly what a Lisp programmer expects:

```clojure
(let [c @count] (bind-text c))
;; BEAM: {:list,[symbol:"let", vector:[symbol:"c",
;;               list:[symbol:"deref", symbol:"count"]], …]}   (measured)
```

`@count` derefs an atom. `data-inline` names a form. No escaping, no reader
configuration, no ambiguity. **Deref interop is a consequence of the naming
decision, not a feature bolted on.**

### 6.4 `~` never appears

Presets are `[:st/preset :ease-out]`. A bare `~ease-out` would read as
`(unquote ease-out)` on the BEAM.

### 6.5 Module-qualified macro names are safe

Names such as `on-cutover#@click-timeline` exist in the registry *(measured)*.
Despite containing both `#` and `@`, they read correctly as **bare symbols** in
both runtimes — `#` dispatch is prefix-only, and `@` is only special in leading
position. This looks alarming and is fine; it is stated here so no one "fixes"
it.

---

## 7. Sugar — derivable only

**Rule.** Sugar is admissible **only if it is derivable from the `%form` pattern
or from a `FormMatch` field.** Any sugar requiring a hand-authored table is
rejected, because such a table duplicates registry knowledge and rots when a
`%macro` is added.

### Admitted

**(a) Positional captures.** The pattern declares capture order, so positional
args map mechanically:

```clojure
(data-inline $mcpPrompt "")   ≡  (data-inline :name $mcpPrompt :value "")
```

**(b) `sel` grouping.** `selector` is a `FormMatch` **field**, not a capture, so
it factors out of a group of forms:

```clojure
(sel ".kit-btn-confirm"
  (mcp-action :action "kit-confirm" :value "confirm"))
```

### Rejected

- `(st/data :inline …)` — kind-word-as-argument. Requires the 37-directive
  mapping table (§3).
- Per-directive bespoke spellings. Each is hand-written wiring.
- Bare `~preset` / `@directive` — reader collisions (§6).

### On aesthetics

`data-inline` is less pretty than `@data inline`. That is **correct**: EDN is the
programmatic surface, `.st` remains the human reference (§10). Authors wanting
beauty write `.st`; code generating forms wants uniformity.

---

## 8. Defaults, and the fail-loud contract

### 8.1 Defaults are materialised at parse time

*(measured)* `@mcp-action(action: "go", value: "v", on: "hover")` yields **six**
captures — `target`, `targetAttr`, `valueAttr` were never written by the author.
Printing all captures would emit noise no one typed.

**`capture_spans` is NOT a reliable authored-marker.** `@fn`'s `body` and
`@type`'s `fields` are author-written yet span-less *(measured)*.

**Normative rule:** suppress a capture when its value equals the **registry's
declared `ParamDefault`** for that capture. Never infer authorship from spans.

### 8.2 Fail loud — there is no fallback policy

EDN **never reconstructs an `Expr` it did not author**. In the EDN surface a JS
expression is an explicit opaque leaf — `[:st/expr "$a + $b"]` — a *string*,
carried byte-verbatim in both directions. Faithful by construction.

∴ printing difficulty arises **only** for a `FormMatch` parsed from existing
`.st` whose capture text is mangled. That is a **verse parser bug**, not a
translator gap.

**The three rules:**

1. Pattern-print every form. If a form cannot be printed from its `%form` +
   captures → **hard error naming the macro**.
2. Raw captures (`Expr`, `Balanced`, opaque bodies) print **verbatim** from
   capture text.
3. Verbatim text that fails to re-parse → **hard error + reproducer filed as a
   verse bug**.

The corpus run therefore emits **a list of verse defects**, not a fudge factor.

**Known defect (already observed).** Parsing
`@fn add($a: number, $b: number) : number { $a + $b }` yields
`body = Expr("{ $a + $b")` — unbalanced brace — and `params` **drops `$b`**
*(measured)*. Both silent today.

### 8.3 Sizing *(measured)*

Across 415 forms / ~1150 captures, the non-structured remainder is ~10%,
clustered on two concepts:

| capture | count |
|---|---|
| `Expr` | 71 |
| `Balanced(';')` | 4 |
| body `block` | 13 |
| body `balanced` | 10 |
| body `expr` | 7 |
| body `template` | 6 |
| body `component_body` | 5 |

Making these structured is **FUP-188** (research spike), deliberately **not** a
dependency of this work.

---

## 9. Sibling `StFile` fields

`FormMatch` is not the whole file. *(measured)* a 6-line probe file produced
`scopes: 2`, `imports: 1`, and the CSS declaration `text: $mcpPrompt` produced
**zero** `FormMatch`es — CSS declarations live in `StFile.scopes`.

An EDN **document** is therefore a map, with forms as one member:

```clojure
{:st/imports   ["stdlib/__mcp__"]
 :st/forms     [(data-inline :name $mcpPrompt :value "")
                (mcp-input)]
 :st/scopes    [{:selector ".kit-prompt" :decls {"text" "$mcpPrompt"}}]
 :st/html      [{:skeleton "<main class=\"kit\">[:st/hole 0]</main>"
                 :holes    [[:st/expr "$title"]]}]
 :st/css       [[:st/raw "@media (min-width: 40em) { … }"]]
 :st/presets   []
 :st/patterns  []
 :st/meta-defs []
 :st/exports   []}
```

Covering `imports`, `presets`, `patterns`, `meta_defs`, `scopes`, `html_blocks`,
`raw_css_blocks`, `file_exports`. A bare top-level sequence of forms is shorthand
for `{:st/forms [...]}`.

### Markup is a plane, not a payload

`:st/html` carries the **skeleton/holes split the parser already makes**
(`HtmlBlockAst`), never a flat string. A hole is a BINDING, so a document must be
able to answer *"which signals does this markup read?"* without re-parsing HTML —
and a program patching markup must be able to address a node rather than run a
regex over text (BUG-369).

On the wire a hole is spelled `[:st/hole N]`, indexing the ordered `:holes` list.
In memory the same position uses the parser's private-use sentinels
(`\u{E000}N\u{E001}`), which are correct for an internal skeleton and wrong for a
document humans read and programs rewrite — nobody can address a character they
cannot see.

`[:st/raw "…"]` remains available as an explicit escape for markup that does not
round-trip structurally; it is a deliberate fallback, never the default.

---

## 10. Zero-change guarantee

`.st` remains **the reference syntax**. This is structural, not a promise:

| axis | effect |
|---|---|
| `.st` lexer / parser / CST | untouched — EDN never enters this path |
| `%form` / stdlib | untouched — EDN **reads** the registry, never mutates it |
| `pipeline` / `compiler` | untouched — receives `Vec<FormMatch>` exactly as today |
| new failure modes for `.st` users | none — `src/edn/` is additive; no existing call site changes |
| build cost | one small pure-Rust dep (`clojure-reader`), no C, no V8 |

Consequences:

- EDN **cannot express anything `.st` cannot** — the registry is the ceiling.
- A construct exists because a `%form` says so. EDN can never define syntax.
- A new `%macro` reaches EDN with **no EDN-side change**, so divergence is
  impossible by construction.

### The single-registry rule (load-bearing)

**EDN MUST consult the same `STDLIB_REGISTRY` instance**
(`src/syntax/stdlib_registry.rs:50`). Two registry instances would drift silently
on custom macros. Assert single-instance in test.

---

## 11. beam-lisp interop

### 11.1 Wire parity

The `.bl` reader/writer consumes the **same** wire format defined here. There is
**no second codec** — verse remains the only implementation, because two
implementations of a bijection drift, and the drift is silent.

### 11.2 `add-watch!` — the missing primitive

*(measured)* beam-lisp atoms are `Agent`-backed with `deref` / `swap!` /
`reset!` / `compare_and_set!` (`lib/beam_lisp/refs.ex`), but **`add-watch` does
not exist**. It is the one primitive the connector needs.

Add to `lib/beam_lisp/refs.ex`, Clojure semantics:

```clojure
(add-watch!    ref key (fn [key ref old new] …))
(remove-watch! ref key)
```

~40 lines over the existing Agent. Independently valuable — a genuine
Clojure-compat hole, not scaffolding for this project.

### 11.3 `st/connect!` — the automatic connector

Lives in **beam-lisp** (`spell.st`), not verse. It declares which atom keys back
which Spacetime signals:

```clojure
(def state (atom {:messages [] :status "idle"}))

(st/connect! state {:messages $messages :status $status})
```

Expands to an `add-watch!` that diffs old→new per declared key and pushes **only
changed keys** as `st-set` assigns:

```clojure
(add-watch! state ::ui
  (fn [_k _r old new]
    (doseq [[k sig] bindings]
      (when (not= (get old k) (get new k))
        (st/push! sig (get new k))))))
```

∴ **ordinary `swap!` drives the browser.** Fully async: the watch fires from
whichever process mutated the atom; the LiveView diff channel handles delivery.

Inbound is the mirror:

```clojure
(st/on :send (fn [payload]
  (swap! state update :messages conj {:body (:body payload) :role "user"})
  (swap! state assoc :status "thinking")
  (stream-reply! (:body payload))))
```

This mirrors the **already-shipped** Elixir bridge
(`elixir/spacetime_lv/lib/spacetime_lv_web/live/counter_live.ex`):
`events` / `assigns` / `pushes` + `{:reply, …}`, assigns flowing browser-ward as
diffed `st-set` payloads (templates are never diffed, only data). It invents no
transport.

---

## 12. Worked examples

### 12.1 `stdlib/__mcp__/kit/confirm.st`

```spacetime
@import "stdlib/__mcp__";
@data inline $mcpPrompt : "";
@mcp-input;

.kit-prompt      { text: $mcpPrompt; }
.kit-btn-confirm { @mcp-action(action: "kit-confirm", value: "confirm") }
```

```clojure
{:st/imports ["stdlib/__mcp__"]
 :st/forms   [(data-inline :name $mcpPrompt :value "")
              (mcp-input)
              (sel ".kit-btn-confirm"
                (mcp-action :action "kit-confirm" :value "confirm"))]
 :st/scopes  [{:selector ".kit-prompt" :decls {"text" "$mcpPrompt"}}]}
```

### 12.2 Async chat over Spacetime Live

```spacetime
@host $spell : live("Spell.ChatLive");

@data subscribe $messages Message[] from $spell : messages ;
@data subscribe $status   string    from $spell : status ;

@data stream $tokens Token from $spell {
  receive to Tok { "token" => Chunk($.payload.text); "done" => Done() final; }
}

@data signal $send($body) to $spell {
  send emit "send"
  receive to Sent { "ok" => Ok($.reply); _ => Failed($.reply); }
  policy queue
}

@handle $send { optimistic { $messages = [...$messages, {body: $draft}] } }
```

```clojure
(host-live      :name $spell :module "Spell.ChatLive")
(data-subscribe :name $messages :type "Message[]" :host $spell :assign messages)
(data-subscribe :name $status   :type "string"    :host $spell :assign status)

(data-live-stream :name $tokens :type "Token" :host $spell
  :receive [[:st/arm "token" [:st/cons "Chunk" [:st/expr "$.payload.text"]]]
            [:st/arm "done"  [:st/cons "Done" []] :final true]])

(data-signal :name $send :params [$body] :host $spell
  :send    [:st/emit "send"]
  :receive [[:st/arm "ok" [:st/cons "Ok" [:st/expr "$.reply"]]]
            [:st/arm "_"  [:st/cons "Failed" [:st/expr "$.reply"]]]]
  :policy  [:st/policy :queue])

(sel ".log"
  (each :collection $messages :as $m
    (sel ".msg" (bind-text [:st/expr "$m.body"]))))

(handle :signal $send
  :optimistic [:st/expr "$messages = [...$messages, {body: $draft}]"])
```

Every head — `host-live`, `data-subscribe`, `data-live-stream`, `data-signal`,
`each`, `handle` — is a **real registry macro name** read from the registry
*(measured)*, not invented. All forms above verified to read in **both** runtimes
with identical structure.

### 12.3 Mixing notations

```clojure
;; layout generated in EDN by ordinary Lisp
(for [f (:fields form)]
  (sel (str ".field-" (:id f))
    (mcp-action :action (:id f))))

;; styling stays in .st — untouched, reference syntax
(st-import "./theme.st")
```

Both compile to `Vec<FormMatch>`, concatenate, and enter the same
`pipeline::compile`.

### 12.4 Mini-DSLs

A mini-DSL is a `%form` declaration. It lands in **both** notations at once, with
no EDN-side work:

```spacetime
%macro card {
  %form { @card $title:string { $body:component_body } }
  %binds { card-impl(title: $title) }
}
```

```clojure
(card :title "Recovery" :body [:st/component-body])
```

---

## 13. Module layout

```
src/edn/
  mod.rs      public API
  codec.rs    CapturedValue ⟷ Edn            (W1)
  print.rs    Vec<FormMatch> → .st text       (W2)
  read.rs     EDN text → Vec<FormMatch>       (W3)
  doc.rs      whole-document sibling fields   (W4)
```

Ingress (W5) adds a `lang: "st" | "edn"` discriminator at
`LiveServer::put_source_with_entry` (`src/mcp/live.rs:939`) — the convergence
point for `st_fn_put`, `st_tab_open`, inline `st_mount`, and browser/workbench
source creation. **Patching only `st_fn_put` would miss four other ingress
paths.**

CLI verbs `spacetime edn <f.st>` / `spacetime st <f.edn>` make the translator
inspectable from a shell without an MCP client.

---

## 14. Open questions

1. **Duplicate `macro_name`.** `assert` ×2 is the sole collision in 415 forms
   *(measured)*. Qualify by defining module; assert in test that no new
   collisions appear as stdlib grows.
2. **EDN tagged literals in `BeamLisp.Reader`.** Worth adding on its own merits;
   this spec must not depend on it (§5.1).
3. **Class B textual bijection.** Deferred; would be CST-anchored (§2).
