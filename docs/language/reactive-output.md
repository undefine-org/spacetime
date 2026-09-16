# Reactive Output — `@host`, `@data signal`/`stream`, `receive`, `@handle`

`@data` is reactive **input** — `SOURCE → STATE → VIEW`. Reactive **output** is
its mirror: the `INTENT → OUT` arc, the half that used to drop to imperative
`@on &.click { fetch() }`. It completes the loop **as `@data`'s own machinery**, not
a parallel construct (one need, one implementation):

```
SOURCE ──→ STATE ──→ VIEW ──→ INTENT ──→ (SOURCE) …
(@data in)          (render)  (@data signal out, @handle)
```

The whole feature is **two new `@data` kinds + one consumer directive**, all
powered by one extended pattern-matcher. Nothing is bolted on.

> Status: PLAN-038 W1. Parse + compile + validation are shipped and tested:
> `@host`, `@data signal`/`@data stream`, the `receive` matcher, `@handle`, and
> tagged-sum `@type` all parse, dispatch, bind their primitives, and emit the
> signal-call runtime (HTTP transport). The per-variant **decode dispatch** (arms
> → constructed variant → typed `@handle` binding) is the next wave — see
> [Runtime status](#runtime-status). The surface below is stable.

---

## 1. The five pillars

| Pillar | Construct | Role |
|--------|-----------|------|
| 1 | `@host` | a transport+config binding; declares the wire envelope type |
| 2 | `@data signal` / `@data stream` | reactive output — one reply / many replies |
| 3 | `receive { … }` | the decode matcher (envelope → variant) |
| 4 | `@handle` | the consumer — reacts to replies in a scope |
| — | `@type Sum { A(T) \| B(U) }` | tagged sums (the response interface) |

---

## 2. `@host` — a transport binding

A `@host` names a transport endpoint that signals send to and receive from. It
declares the **transport** (http | ws), its **config** (base url, headers, socket),
and implicitly the wire **envelope type** that types the decode subject `$.`.

```spacetime
@host $api : http("https://api.example.com") { headers: { authorization: $token }; }
@host $api : http("https://api.example.com")          // bodyless
@host $mcp : ws($hostSocket)
```

The kind word (`http` / `ws`) selects the transport — kind-as-macro dispatch, the
same shape `@data inline`/`fetch`/… use. The `:` is value-position (it introduces
the transport value); the argument and `{ headers }` body are pure config.

| Transport | Envelope `$.` | Discriminator |
|-----------|---------------|---------------|
| `http` | `Response { status int, ok bool, body Json, statusText, headers }` | `status` |
| `ws` | `Message { type string, payload Json }` | `type` |

---

## 3. `@data signal` / `@data stream` — reactive output

`@data signal` (kind 6) is **one correlated reply** (≈ PureScript `Aff`).
`@data stream` (kind 7) is **many replies over time** (≈ `Event`). Both are the
outbound sibling of `@data fetch`: a signal yields a **callable** `$name` plus
`$name_pending` / `$name_error` / `$name_cancel`, exactly as `@data fetch` yields
`$x` / `$x_loading` / `$x_error` / `$x_refetch`.

```spacetime
@data signal $add($text string) to $api {
  send POST "/api/todos" { title: $text }
  receive to AddResult {
    200 => Created($.body as Todo);
    207 => Partial(normalize($.body) as Report);
    422 => Invalid($.body as Errors);
    _   => Failed($.statusText);
  }
  policy latest
}
```

Body clauses (`send` + `receive` required; `policy` + `optimistic` optional):

- **`send`** — the request builder.
  - http: `send POST "/api/todos" { title: $text }` (METHOD url { body })
  - ws: `send emit "proposal" { options: $opts }` (emit type { payload })
- **`receive [to <Sum>]`** — decode the reply (see §4).
- **`policy latest|queue|parallel|drop [timeout T] [retry n]`** — owns races.
  `latest` aborts the in-flight call when a new one fires (search-box classic).
- **`optimistic { … }`** — applied immediately on fire (default; scopes override).

Parameters use the space-form (`$text string`), matching `@fn` and template params.
`to $host` names the `@host` the signal sends to.

---

## 4. `receive` — the decode matcher

`receive` is **the** matcher keyword, shared across three host positions (one
grammar, different consequence):

| Host | Subject | Each arm's consequence |
|------|---------|------------------------|
| `@match $x { }` | a value | `&template(args)` (render) |
| `receive { }` in a signal def | the envelope `$.` | construct a variant (decode) |
| `receive { }` in `@handle` | the variant sum | effect statements (handle) |

An arm pattern is an exact status (`422`), a status-class/tag ident, or a string
(ws message type); `_` is the wildcard. The consequence is a **variant
constructor** carrying an expression, or a bare expression:

```spacetime
200 => Created($.body as Todo);     // construct Created, carrying the decoded body
_   => Failed($.statusText);        // bare expression outcome
```

### `to <Sum>` — the response-type through-line

The whole signal reads as a pipeline; `to` marks each stage's **forward target**:

```
$add(args) ─send to $api─▶ ─receive to AddResult─▶ ─@handle … to AddResult─▶ effects
             target: host        target: sum type         target: sum type
```

One relation everywhere (the stage keyword says whether the target is a host value
or a sum type). The response type becomes the visible spine, threaded end-to-end:

```spacetime
receive to AddResult { 200 => Created($.body as Todo); … }   // decode → AddResult
@handle $add to AddResult { receive { Created(t) => … } }    // same through-line
```

**Position decides role** (no token-dependent flip): a name **right of `to`** is the
sum (the output); a name **left of `to`** is the envelope alias (self-doc, e.g.
`Response.status`). To name the sum you always go through `to`. All four head
shapes are accepted: `receive { … }`, `receive to Sum { … }`,
`receive Alias to Sum { … }`, and `receive Alias { … }` (alias only). The literal
`to` is matched before the alias slot, so `receive to Sum` binds the sum and never
mis-reads `to` as the alias.

### `final` — termination

An arm may carry `final` to mark a **terminal** outcome (ends a stream; triggers
`@handle`'s `final { }`). A one-shot `@data signal` is implicitly all-arms-final.

---

## 5. `@handle` — the consumer

`@handle $sig { … }` consumes a signal in **this scope**, re-opening its clauses
(nearest-scope cascade, CSS-like). One signal, many handlers — each owning its
scope's behavior; a fire resolves to its nearest enclosing `@handle`, else the
def defaults apply.

```spacetime
.quick-add {
  @handle $add {
    optimistic { $todos <- $todos.concat({ title: $draft, pending: true }); }
    receive {
      Created(todo) => { $todos <- $todos.replace(todo); }
      Invalid(errs) => { $errors <- errs; }
    }
    final { $draft <- ""; }
  }
}
```

All three clauses are optional — a bare `@handle $sig {}` adopts the def behavior.
The handle-side `receive` matches over the **variant sum** (not the wire envelope);
each arm binds the carried payload (`(todo)`), typed by the variant's `as` type
(decl↔use, inferred — never restated).

> Mnemonic: `@each` is collection-**in-space**; `@handle` is responses-**in-time**.

---

## 6. Typing — one optional knob, `as Type`

Typing is optional everywhere (parity with `@data`). The single knob is `as Type`
ascription, which **flows** decl → use:

```spacetime
send POST "/url" { title: $text } as CreateReq   // request body type (optional)
receive to AddResult {
  200 => Created($.body as Todo);                 // the response @type
}
@handle $add { receive { Created($todo) => … } }  // $todo : Todo (carried, inferred)
```

`Created(Todo)` is sugar for `Created($.body as Todo)`. The sum is named via `to`
(`receive to AddResult`) and **is** the introspectable response interface.

### Tagged sums (`@type`)

`@type` now carries a tagged-sum body alongside its product form:

```spacetime
@type AddResult {
  Created(Todo)
  | Partial(Report)
  | Invalid(Errors)
  | Failed                  // payload-less variant
}
```

The grammar **is** the discriminator: a sum body requires ≥1 `|`; a product body
(no bars) stays a product. One type-creation mechanism, two scopes — a file-scope
`@type Name { … }` (shareable, recursive) and a signal-scoped `receive to Name { … }`
(the arms are the variant definition) register the same kind of named sum.

### The unified enum signal (PLAN-077)

A variant value is a **plain object**: `{type: 'Created', …payload}` — `type`
names the variant, payload fields sit beside it. Every PRODUCER yields that one
shape and every CONSUMER reads it, so unions flow across the whole language
without adapters:

| producer | yields |
|---|---|
| `@data signal … receive to Sum { … }` (§3-4) | the decoded response variant |
| `@data derive $x Sum : @match { … }` (cond-mode — see data-and-rendering.md §3) | the first truthy guard's constructor |
| `$x <- Created($todo)` (mutation construction) | the built variant |

| consumer | reads |
|---|---|
| `@handle $sig { receive { Created(t) => … } }` (§5) | variant + destructured payload |
| `@match $sig { Created => &card($t); _ => … }` (dispatch-mode) | variant, payload into template scope |
| `@state(when: $sig is Created { $t }) { … }` | variant → `data-st-state`, payload into element scope |

`@state` reflection writes the CURRENT variant name to `data-st-state`
unconditionally — the CSS gate `%self[data-st-state=Created]` activates only
for its own variant, so a catch-all variant (e.g. `None`) reflects but matches
no gate (the "unstyled" outcome). One reactive writer — `ST.bindState` in the
runtime — backs both `@state` and the metasystem `%states` clause, with the
watchScoped global-channel subscription (a signal published before mount still
reflects) and an ownership-guarded falsy clear (two gates sharing one element's
carrier cannot wipe each other's state).

---

## 7. Runtime status

**Fully shipped (PLAN-038 W1 + FUP-080 + FUP-081): the reactive-output DSL is
live end to end** — author surface AND runtime.

The wire path (`@data signal`/`@data stream`):
- **`@host` runtime registration** — `@host` emits JS populating
  `window.__stHosts[<name>]` with the transport + config (http base url + headers,
  or ws socket). `signal-call`'s `resolveHost()` reads it at call time.
- **Decode dispatch** — the `receive` arms compile into a runtime decode table.
  `decodeReply(envelope)` selects the first arm whose pattern matches the
  discriminator (http status / status-class `2xx` / `ok`/`error` / `_` wildcard;
  ws message `type`), evaluates the carried payload expression (`$.body as Todo`
  → `envelope.body`, the `as Type` ascription stripped at runtime), and constructs
  the tagged variant `{ variant, value, final, sum }`, published on
  `signal:<name>:reply`.
- **Policy** — all four race modes: `latest` (abort in-flight), `queue`
  (serialize via a promise tail), `parallel` (concurrent), `drop` (ignore while
  busy). The optional `timeout <dur>` aborts a slow call; `retry <n>` re-fires a
  failed attempt up to N extra times.
- **Transports** — `http` (POST/PUT/… + status decode) and `ws` (correlated
  `{ type, payload, __corr }` frames over a pooled socket; uncorrelated messages
  publish directly for the `@data stream` many-replies case).

The consumer path (`@handle`):
- **Variant dispatch** — `@handle`'s `receive` arms match the decoded
  `reply.variant` to the arm whose constructor matches (or a `_` wildcard), bind
  the carried `value` to the arm's payload name, and run the arm's effect body
  through `ST.runMutations` (the shared `@on`/`@effect` mutation rail, extended
  with a lexically-bound `locals` map so the payload resolves in the RHS).
- **`optimistic`** runs at FIRE time (signal-call dispatches `signal:<name>:fire`
  on invocation; the handler's `onFire` applies the optimistic statements before
  the reply lands).
- **`final`** runs on a terminal outcome (`reply.final`), DRYing cleanup.

Type creation:
- **Scoped sum** — `receive to AddResult { … }` registers a named sum type
  `AddResult` from its arms (ctor = variant, `as <Type>` = payload), so the type
  registry / introspection sees the response interface. An explicit `@type` wins.
- **Envelope alias** — `receive Response to AddResult { … }` is supported; the
  name left of `to` is the envelope alias, the name right of `to` is the sum.

Verification: `cargo test --lib` (emit tests:
`host_emits_runtime_registration`, `signal_emits_decode_table`,
`signal_emits_policy_and_lifecycle`, `handle_emits_variant_dispatch_and_clauses`,
`ws_signal_emits_socket_transport`; type tests:
`test_receive_block_registers_scoped_sum`, `test_receive_*` alias tests) +
`tests/integration/signal-dispatch.test.st` (headless decode→dispatch round-trip).

Remaining niceties (not blocking): the alias is captured but not yet threaded as a
typed envelope binding (it's self-doc only); ws reconnection/backoff is delegated
to the browser; the Phoenix `live` transport is config-only (the wire is a future
plan).

---

## 8. Worked example — optimistic todo add

```spacetime
@type Todo   { id: string; title: string }
@type Errors { message: string }

@host $api : http("https://api.example.com") { headers: { authorization: $token }; }

@data signal $add($text string) to $api {
  send POST "/api/todos" { title: $text }
  receive to AddResult {
    200 => Created($.body as Todo);
    422 => Invalid($.body as Errors);
    _   => Failed($.statusText);
  }
  policy latest
}

.quick-add {
  <input class="field">
  <button class="go">Add</button>

  @on &.click .go { $add($draft); }       // fire the signal

  @handle $add {
    optimistic { $todos <- $todos.concat({ title: $draft, pending: true }); }
    receive {
      Created(todo) => { $todos <- $todos.replace(todo); }
      Invalid(errs) => { $errors <- errs; }
    }
    final { $draft <- ""; }
  }
}
```

One declarative loop: input → fire → optimistic paint → reply → reconcile →
cleanup. No hand-rolled `fetch()`, no per-page transport — the `@host` owns the
wire, the signal owns the correlation, the `@handle` owns the scope's reaction.
