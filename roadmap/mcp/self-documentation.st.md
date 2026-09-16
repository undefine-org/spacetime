# How Spacetime documents itself — and the tool that searches it

> **What this is.** A from-zero explanation of how Spacetime describes its own
> language back to you, plus the concrete design for `spacetime_query_docs`: an
> MCP tool that turns a plain-English *"I want to…"* into the exact Spacetime
> construct that does it. This document commits to the **syntax I intend to
> ship** and tells you **how to build it on the Rust side**, alternatives and all.
>
> **Status legend.** ✅ built & runnable today · 🧭 proposed (this folder's work).
> A ` ```st ` fence below is a *running program* (it compiles when this page is
> served); a ` ```spacetime ` fence is *inert* — it shows syntax I have **not**
> shipped yet, so it is quoted, never executed. That split is the honesty
> contract of a literate `.st.md`: nothing here pretends a roadmap piece is real.

---

## 1 · The gap: intention vs. construct

You sit down to build a page and you think in **intentions**:

- *"I want a number that goes up when I click a button."*
- *"I want to load a list from a URL and re-fetch it every 5 minutes."*
- *"I want this section to fade in as I scroll to it."*

Spacetime offers **constructs** — precise syntax: `@data inline`,
`@data fetch … { refresh: 5m }`, `@on &.scroll { … }`, a few hundred more. Each
is small and learnable, but there are a lot of them.

The **gap** is the distance between *what you meant* and *which construct says
it*. Most languages bridge that gap with a hand-written manual you search by eye.
Spacetime bridges it differently: the language **knows its own construct list**,
keeps it perfectly current, and — with the tool this doc designs — lets you
search that list by describing what you want in ordinary words.

"Self-documenting" here does not mean "we wrote nice docs." It means **the
system's description of itself is generated from the system, so it cannot be
wrong.**

---

## 2 · The key idea: constructs are *data*, not prose ✅

One idea carries everything else.

In most languages a feature lives as compiler code in one place and as a
paragraph in a manual elsewhere. The two drift the moment someone edits one and
forgets the other.

In Spacetime, **every construct declares itself in one place**, and that
declaration is the *only* source. This is the real block that teaches the
compiler the `@data fetch` form (lightly trimmed from `stdlib/macros/data-kind.st`):

```spacetime
/// @data fetch — remote/async source. Loads a URL, exposes
/// $x / $x_loading / $x_error / $x_refetch.
%macro data-fetch-kind {
  %form {
    @data fetch $name:binding $type:typeref? : $src:expr ;
  }
  %binds {
    data-source(name: $name, src: $src) -> { … }
  }
}
```

Three things in that block *are* the documentation:

- the `///` line — a one-sentence **doc**,
- the `%form { … }` — the exact **shape** you type,
- the name + bound exports (`$x_loading`, `$x_error`, …) — recorded too.

The compiler reads all of these into one in-memory catalog, the **registry**.
Every macro, primitive, and form has a row: `{ name, kind, doc, params,
exports }`. Because the description lives *inside* the construct, there is no
second copy to sync. **The manual is generated from the machine.**

---

## 3 · Reading the catalog from inside the language ✅

The registry would be useless if only Rust could see it. The turn is that a
Spacetime **page** reads it too, as ordinary data. This fence is **live** — when
this page is served, these directives hand your page the compiler's real catalog:

```st src
@import "stdlib/dispatch";

/* The compiler's own macro forms, as rows: { directive, name, signature,
   tokens, doc, kind, scopes, binds, overloads }. */
@data dispatch $macros;

/* Count them — a derived signal over the live catalog. */
@data derive $count number : $macros.length;

<p class="tally">Spacetime knows <b>`$count`</b> directive forms right now.</p>
```

When the compiler sees `@data dispatch $macros`, it takes the *live registry*,
turns it into rows, and hands `$macros` to the page as data. From there you loop
with `@each` and draw a card per construct — filtered reactively by a search
signal, zero custom JS:

```st src
@data inline $q : "";

<input class="find" type="text" placeholder="Search directives…" />
<section class="cards"></section>

/* A directive binds through a SELECTOR, never through markup nesting: the
   <section> already exists once `.cards` is bound, so `@each` attaches to it. */
.cards {
  @each($macros as $m, when $q == "" || ($m.directive + " " + $m.doc).toLowerCase().indexOf($q.toLowerCase()) >= 0) {
    <article class="card">
      <code class="card__sig">`$m.directive`</code>
      <p class="card__doc">`$m.doc`</p>
    </article>
  }
}

.find { value <- $q; @on &.input { $q <- $.value; } }
```

That is exactly how the real page `demos/spacetime-docs/reference.st` works. Its
cards are **not** hand-written descriptions — they are registry rows, rendered.
Add a construct to stdlib and a card appears **next build**, zero page edits. The
reference cannot go stale, because it is not a description of the language — it
*is* the language, drawn.

> The thesis in one line: **reflection renders.** A dead system prints its
> catalog as JSON on a terminal. Spacetime renders it as a live page, built by
> the same compiler that owns the catalog.

But notice what section 3 gives you: the **whole** catalog, filtered by *letters*
you type. `@each … when … indexOf` is substring matching. Search "reload" and
you miss `@data fetch`'s "refresh." That last gap — *meaning*, not letters — is
what the next section closes.

---

## 4 · `spacetime_query_docs` — the surface I intend to ship 🧭

Two surfaces, one engine. Same idea seen from the agent side and the page side.

### 4a · The MCP tool (primary deliverable)

An agent (or you, at the MCP layer) hands it a **declaration of intent** and gets
back the **constructs most likely to express it**, ranked:

```jsonc
// tools/call → spacetime_query_docs
{
  "query": "load a list from a url and refresh it every few minutes",
  "kind":  "data",   // optional pre-filter: data | events | animation | …
  "limit": 5          // optional, default 8
}
```

returns `structuredContent`:

```jsonc
{
  "query": "load a list from a url and refresh it every few minutes",
  "matches": [
    { "name": "@data fetch",      "kind": "data", "score": 0.86,
      "signature": "@data fetch $name : $src { refresh: $d }",
      "doc": "remote/async source with refresh",
      "exports": ["$x", "$x_loading", "$x_error", "$x_refetch"] },
    { "name": "@data collection", "kind": "data", "score": 0.71,
      "signature": "@data collection $name from $src via $t { … }",
      "doc": "mutable remote array with optimistic CRUD",
      "exports": ["$x", "$x_loading", "$x_error"] },
    { "name": "@data source",     "kind": "data", "score": 0.64,
      "signature": "@data source $name from $urlSignal ;",
      "doc": "refetch when a URL signal changes", "exports": ["$x"] }
  ]
}
```

The rows are exactly the registry rows from §2–3, plus a `score`. **The corpus
the tool searches is the same catalog the reference page renders** — one source,
two consumers.

### 4b · The in-language mirror (follows the family)

Because `@data registry` / `@data dispatch` / `@data declarations` already exist
(§3), the semantic search *should* be reachable the same way — one more arm of
the family, learnable by pattern. The keyword is `match`, sitting where `from`
sits in the siblings:

```spacetime
/* PROPOSED — not shipped. Shown, not run. */
@data docs $hits match "fade a section in as I scroll to it" ;

<section class="suggestions">
  @each($hits as $h) {
    <article class="hit" data-score="`$h.score`">
      <code>`$h.name`</code>
      <p>`$h.doc`</p>
    </article>
  }
}
</section>
```

With a **literal** query string the match is resolved at *build time* — the
compiler embeds the query once, ranks, and rewrites `@data docs … match` into a
plain `@data inline` array, flowing through the *identical* `%pipeline-consumed`
rail the other `@data <reflective>` arms use. A **signal** query
(`match $q`, live search box) needs a runtime endpoint and is a later rung.

### Why semantic, why fuzzy, why cheap

- **Semantic, not keyword.** A keyword search fails the way manuals fail: you
  searched *"reload,"* the doc says *"refresh,"* you find nothing and conclude the
  feature is missing. Semantic search compares *meaning*: each row becomes a
  compact numeric fingerprint of its name + doc; your sentence becomes one too;
  "closest fingerprints" = "closest in meaning." *"make it appear as I scroll"*
  lands on `@on &.scroll` with zero shared words.
- **Fuzzy = near-miss is a feature.** You will not phrase things the way the docs
  do. The tool is *forgiving of how you said it* and *precise about what it
  returns*.
- **Cheap — and this follows directly from §2.** The corpus is *small*
  (a few hundred one-line rows, not prose), *structured* (filter on
  `kind`/`exports` first, rank second), and *stable* (it only changes when the
  language does). So fingerprints are computed **once per build** and cached —
  a query is a single warm comparison, never a crawl. **The self-description is
  what makes the search cheap.** They are the same idea twice.

---

## 5 · How to build it on the Rust side 🧭

The tool has four moving parts. Three already exist; only the ranking core is new.

```
  registry rows            embed               score + top-k
  (§2, EXISTS)  ──────▶  (query + rows)  ──────▶  matches   ──────▶  MCP result
   build_*_json          NEW ranking core                          tools.rs arm
```

### Step 1 — the corpus (reuse, don't rebuild) ✅ exists

The rows are already produced. Do **not** write a parallel catalog — call the
existing builders over the cached registry:

```rust
// A doc row = one searchable construct. Fields already emitted by
// build_registry_json / build_dispatch_json in src/compiler.rs.
struct DocRow {
    name: String,        // "@data fetch"
    kind: String,        // "data"
    doc: String,         // one-line summary
    signature: String,   // "@data fetch $name : $src ;"
    exports: Vec<String>,
}

fn corpus() -> Vec<DocRow> {
    // cached_stdlib_registry() already memoizes the full stdlib registry.
    let (registry, _errs) = crate::compiler::cached_stdlib_registry();
    // Parse the two JSON arrays the reference page already consumes and merge
    // by construct name. (Or: refactor build_dispatch_json to return Vec<Row>
    // and reuse it directly — preferred, one source.)
    merge(
        crate::compiler::build_registry_json(&registry, None),
        crate::compiler::build_dispatch_json(&registry, None),
    )
}
```

The searchable "document" per row is `format!("{name}. {doc}")` — optionally the
signature too. Short, dense, already curated.

### Step 2 — the ranking core (the only new logic)

This is the one design decision, so **three alternatives**, weighed:

| | Engine | Recall on paraphrase | Deps / weight | Offline | Determinism |
|---|---|---|---|---|---|
| **A** | `fastembed` (BGE-M3, local ONNX) | **best** | heavy (`ort`, ~2 GB model) | yes | yes (fixed model) |
| **B** | BM25 + curated synonym expansion | good on the corpus | **none** (pure Rust) | yes | yes |
| **C** | Remote embedding API (OpenAI/Voyage) | best | HTTP client + key | no | no (model drifts) |

**A — local neural embeddings (`fastembed`).** The `.fastembed_cache/models--BAAI--bge-m3`
already in the tree is a prior spike, so the model is on disk. Embed each row
once, cache the vectors keyed by a blake3 of the corpus text (invalidate when the
registry changes — the same trigger `IncrementalStdlibCache` already watches).
Query → embed → cosine → top-k.
*Pros:* true paraphrase recall; the "reload"→"refresh" jump works with no hand
table. *Cons:* pulls `onnxruntime` into the build, ~2 GB model, cold-start load,
fatter binary. Gate it behind `--features semantic-docs` so the default `cargo
build` stays lean.

```rust
#[cfg(feature = "semantic-docs")]
fn rank(query: &str, rows: &[DocRow], limit: usize) -> Vec<Scored> {
    use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::BGEM3).with_cache_dir(".fastembed_cache".into())
    ).unwrap();
    let docs: Vec<String> = rows.iter().map(|r| format!("{}. {}", r.name, r.doc)).collect();
    let row_vecs = embed_cached(&model, &docs);          // computed once/build, cached
    let q = model.embed(vec![query.to_string()], None).unwrap().remove(0);
    let mut scored: Vec<_> = rows.iter().zip(row_vecs)
        .map(|(r, v)| Scored { row: r.clone(), score: cosine(&q, &v) })
        .collect();
    scored.sort_by(|a, b| b.score.total_cmp(&a.score));
    scored.truncate(limit);
    scored
}
```

**B — lexical + synonyms, no ML.** BM25 over the row documents plus a small
curated expansion map (`reload→refresh`, `popup/modal→dialog`, `fade→transition`,
`list→array`). Because the corpus is tiny and technical, BM25 already ranks well;
the synonym map buys most of the "semantic" recall for the vocabulary that
actually recurs in intents.
*Pros:* **zero** heavy deps, instant, deterministic, ~150 lines, trivially unit-
tested (`assert query "reload list" ranks @data fetch #1`). *Cons:* "semantic"
only as far as the table reaches; a wholly novel paraphrase it was never taught
can miss.

**C — remote embedding API.** Best raw quality, nothing local. *Rejected as the
default:* it needs network + an API key, which breaks offline compiler dev and
the deterministic test story, and sends the query off-box. Fine as an optional
`--features remote-docs` backend for someone who wants it; never the floor.

**Recommendation: ship B now, wire A behind a flag.** B is the honest floor — no
2 GB dependency to search 300 one-liners, deterministic gates, works on a plane.
A is the paraphrase upgrade for when the synonym table stops keeping up, already
de-risked by the existing model cache. C stays a niche opt-in. This also honors
the elegance bar: B adds a self-contained ranker that adds no subsystem; A is a
*data* swap behind one trait, not a second search path.

Make the boundary a trait so A/B/C are interchangeable and the tool never knows
which is live:

```rust
trait DocRanker {
    fn rank(&self, query: &str, rows: &[DocRow], limit: usize) -> Vec<Scored>;
}
// Bm25Ranker (default) · EmbeddingRanker (feature "semantic-docs") · RemoteRanker (opt-in)
```

### Step 3 — register the MCP tool ✅ pattern exists

Add one `tool(…)` entry to the list in `src/mcp/tools.rs` and one arm in `call()`
— identical to every existing `st_*` tool:

```rust
tool(
    "spacetime_query_docs",
    "Search Spacetime's own construct catalog by INTENT. Given a plain-language \
description of what you want a page to do, returns the macros/primitives most \
likely to express it — ranked, with each one's form, doc line, and exported \
signals. The corpus is the live compiler registry, so it can never drift from \
the language.",
    json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "What you want to do, in plain language." },
            "kind":  { "type": "string", "description": "Optional family pre-filter: data | events | animation | motion | scene | text | test." },
            "limit": { "type": "integer", "description": "Max results (default 8)." }
        },
        "required": ["query"],
        "additionalProperties": false
    }),
),
```

```rust
"spacetime_query_docs" => query_docs(args, state),   // in call()

fn query_docs(args: &Value, _state: &mut McpState) -> Result<(String, Value), String> {
    let query = non_empty_arg(args, "query").ok_or("missing `query`")?;
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(8) as usize;
    let mut rows = corpus();
    if let Some(k) = non_empty_arg(args, "kind") {
        rows.retain(|r| r.kind == k);               // structured pre-filter (cheap)
    }
    let matches = RANKER.rank(query, &rows, limit);  // §2 ranking core
    let summary = format!("{} matches for “{query}”", matches.len());
    Ok((summary, json!({ "query": query, "matches": matches })))
}
```

### Step 4 — the in-language arm (optional, follows §3) 🧭

If the `@data docs … match "literal"` surface (§4b) is wanted, it is *not* new
machinery — it is one more `%pipeline-consumed` macro plus one injector:

1. add `data-docs-match` to `stdlib/macros/data-kind.st` mirroring
   `data-registry-all` (recognise the surface only), and
2. add `inject_docs_data` in `src/compiler.rs` beside `inject_registry_data` /
   `inject_dispatch_data`: rank at build time, rewrite the match into
   `@data inline` with the ranked JSON.

Runtime `match $q` (live search box) is the only piece that needs a real dev
endpoint; defer it until the tool proves the ranking.

### Verification (write these first — a gate never seen to fail is not a gate)

- **Unit (ranking):** `query "reload the list" ⇒ @data fetch in top-2` for the B
  ranker; a table of intent→expected-top-k. Deterministic, no browser.
- **Corpus parity:** the tool's `corpus()` length == the reference page's card
  count for the same registry — proves one source, not a fork.
- **MCP smoke:** `tools/call spacetime_query_docs {query}` returns `matches[]`
  with `score` descending and every `name` present in the registry.

---

## 6 · The rest of the staircase — intention → *result* ✅/🧭

Semantic search answers only *"which construct?"* — the **first** rung. Getting
from *"I want X"* to *"X works on my page"* needs the rest. Each is a tool that
closes part of the distance:

1. **Describe** intent → `spacetime_query_docs` 🧭 — finds the construct (§4).
2. **Check** it compiles → `st_tab_open` ✅ — hand it a snippet, get diagnostics
   instantly, no file on disk. Candidate → *verified* candidate.
3. **Disambiguate** when several forms match → `@dispatch-probe` ✅ — watch the
   compiler's own scorer: winner glows, losers show *why they lost*. When the
   tool's top pick is not what you meant, this explains the near-miss.
4. **Recover** from a near-miss → **error siblings** ✅ — many constructs ship a
   decoy that catches the common mistake. `@data fetch $x 5 : "/url"` (a number
   where a type goes) fires *"a bare number is not a valid type — write `number`,
   `string`, or `Product[]`."* The manual meets you at the point of error.
5. **Inspect** structure without a file → `st_outline` / `st_profile` 🧭 — the
   same registry-derived outline the reference page uses, on demand.
6. **Run** it and confirm behavior → `st_mount` / `st_await` ✅ — mount the
   candidate, click it, `st_await` returns the event it emitted. "It compiles"
   and "it does what I pictured" are different claims; this proves the second.
7. **Learn** the idiom in context → `@example` / literate `.st.md` ✅ — many
   constructs carry an `@example`; whole pages (this one) are literate: the code
   fences *are* the running program.

The shape matters: search is rung 1 of 7. It answers *"which construct?"*. The
rest answer *"is it right, is it the one I meant, does it really do what I
pictured?"* — the questions that actually stand between an intention and a result.

---

## 7 · Why this is unusual

Most languages document themselves *badly* because the docs are a second artifact
that rots. Spacetime documents itself *well* because the documentation is a
**projection of the system**, not a copy:

- the construct list is **generated** from the constructs (§2),
- the reference page is **rendered** from that list and cannot drift (§3),
- the search index is **built** from the same list, which is why it is cheap (§4),
- the verification tools use the **same compiler** that owns the list (§6).

One source of truth, seen many ways: the compiler owns it, a page renders it, a
search ranks it, a probe explains it, a mount runs it. Every one is the same
registry wearing a different hat. That is why the gap between *what you meant* and
*what Spacetime does* can be made small — and kept small as the language grows.
