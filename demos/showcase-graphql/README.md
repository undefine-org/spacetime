# A GraphQL feed, declared

```
cargo run -- serve demos/showcase-graphql/
```

One file, no `index.html`, no build config, no custom JavaScript.

> **Note on format.** This was drafted as a literate `index.st.md` — prose and
> program in one artifact, which is what this demo deserves. It ships as `.st` +
> this README because every literate document currently fails to compile
> (**BUG-338**: `@doc` resolves to a nonexistent primitive, which breaks even the
> shipped `demos/starter-pack` tutorial). When that lands, the two files collapse
> back into one and this prose moves inline where it belongs.

## Why this demo exists

`@data graphql` did not exist last week. The *capability* did: a `graphql`
primitive sat in `stdlib/primitives/fetch.st` — complete, tested, and shipping
inside every bundle — with no way to write it. No macro bound it, so no page
could reach it.

It was found by a lint that asks a question worth stealing: **which declared
capabilities can nothing reach?**

A primitive nothing can reach is not tidiness debt. It is either work the
language did and never exposed — this case — or a patch of surface a matcher bug
can land on. The second one is not hypothetical: a single mis-parse landed on one
such primitive, emitted a duplicate variable declaration, and killed *every page
that imported the standard library* (BUG-328).

So this demo shows a feature, and the shape of thinking that produced it.

## 1 · The query is a data source

```spacetime
@data graphql $repos : "https://api.github.com/graphql",
  query: "{ viewer { repositories(first: 5) { nodes { name } } } }";
```

That single declaration gives you four bindings, and you already know their names
if you have used `@data fetch`:

| binding | meaning |
| --- | --- |
| `$repos` | the query result |
| `$repos_loading` | true while in flight |
| `$repos_error` | the failure, or nothing |
| `$repos_refetch` | run it again |

Here is the point worth carrying: **`@data graphql` is not a new concept.** It is
an arm of a family you already know.

```spacetime
@data inline  $items : ["ready", "set", "go"];
@data fetch   $posts : "./posts.json";
@data graphql $repos : "/graphql", query: "{ … }";
```

`inline`, `fetch`, `derive`, `fold`, `graphql` — same head, same binding shape,
same `_loading` / `_error` suffixes. Knowing one tells you the next. A language
earns that by *refusing* to give each source its own bespoke spelling, which is
the harder discipline: the tempting move is always one more special case.

## 2 · Rendering it

```spacetime
.repo-list {
  @each($items as $item) {
    <li class="repo">`$item`</li>
  }
}
```

State drives markup directly — no fetch call, no effect hook, no loading flag you
maintain by hand. The bindings already exist, so the page reacts.

The list is driven by `$items` (the inline source) so the demo renders with no
network call and no GitHub token. **Point `@each` at `$repos` and the same markup
renders live data; nothing else changes.** That substitutability is the family
working.

Check `view-source` on the served page and the rows are *already there*:

```html
<li class="repo">ready</li>
<li class="repo">set</li>
<li class="repo">go</li>
```

A static source is unrolled at build time, so the HTML is real content for
crawlers and for readers with JavaScript off — then the same list hydrates and
becomes reactive. One declaration, both renderings, guaranteed to agree because
one definition produces both.

## 3 · Motion is declared

```spacetime
.repo {
  @on &.visible(420ms) {
    opacity: 0 -> 1;
  }
}
```

Note what this does **not** need: no observer to construct, no cleanup to
remember, no intersection ratio threaded through a callback.

`@on &.visible(420ms)` names a driver and a duration. A driver's whole job is to
turn *something* — visibility, scroll position, elapsed time, pointer movement —
into a progress number between 0 and 1. Every driver answers that one question,
which is why they all share one head:

```spacetime
@on &.visible(420ms) { … }
@on &.scroll(...)    { … }
@on &.time(2s)       { … }
```

Same shape, different source of progress.

## 4 · Things to try

- **Change `420ms` to `2s`.** Durations accept `ms`, `s`, `m`, `h`, and `f`
  (frames). They mean the same thing everywhere because *one* parser reads them —
  there used to be two, and `2m` quietly meant 300 milliseconds through one of
  them (PLAN-141 W4).
- **Point `@each` at `$repos`**, add an auth header, and watch the same markup
  render live data.
- **Misspell a directive**, then run `cargo run -- check demos/showcase-graphql/`.
  Both kinds of mistake are now caught, and the two diagnostics differ in a way
  worth noticing:

  ```
  .r { @on-visible(420ms) { … } }   error[E0946]: directive does not match its declared grammar
  .r { @notarealthing(x: 1) }        warning[W0714]: `@notarealthing` is not a known directive
  ```

  `@on-visible` is an ERROR because `@on` *is* a real directive and the compiler
  can say exactly how the call disagrees with its grammar. `@notarealthing`
  matches nothing at all, so it is a warning that names it. Both used to compile
  in silence and render nothing — which is the failure mode this whole arc of
  work exists to remove (PLAN-141 W2).

That last one is the honest summary: a compiler's job is not only to accept what
is right, but to **refuse quietly-wrong things loudly**. A page that silently
does nothing is the most expensive output a toolchain can produce.
