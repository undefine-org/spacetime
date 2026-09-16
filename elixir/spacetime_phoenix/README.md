# spacetime_phoenix

Render a Phoenix LiveView's body with a compiled Spacetime bundle.

One typed state machine, split at the network boundary — not two apps with a
protocol between them. The LiveView owns the socket, assigns and effects; the
`.st`/`.edn` page owns rendering, local state and motion; data crosses, never
templates.

```elixir
def deps do
  [{:spacetime_phoenix, path: "../verse/elixir/spacetime_phoenix"}]
end
```

## A page

```elixir
defmodule MyAppWeb.CounterLive do
  use Spacetime.LiveView,
    bundle: "/spacetime/counter/spacetime.js",
    root: ".counter",
    shell: ~s|<div class="counter"><span class="value"></span></div>|

  events do
    event(:inc)
  end

  assigns do
    assign(:count, :integer)
  end

  def mount(_params, _session, socket), do: {:ok, assign(socket, count: 0)}

  def handle_event("inc", _payload, socket) do
    count = socket.assigns.count + 1
    {:reply, %{tag: "ok", reply: count}, assign(socket, count: count)}
  end
end
```

No `render/1` — it is generated. `handle_event/3` answers `{:reply, %{tag:, reply:}}`
and the page's `receive` arms decode it through the same matcher family as
`@match`.

## Building the bundle

```elixir
# mix.exs
spacetime: [
  entries: [{"frontend/counter.st", "priv/static/spacetime/counter"}]
]
```

```sh
mix spacetime.build
```

Then serve it like any other static asset:

```elixir
plug Plug.Static, at: "/", from: :my_app, only: ~w(assets spacetime ...)
```

**Why a build task, not a dev server.** The compile is deterministic — the same
entry produces the same bundle, byte for byte, whether it came from
`spacetime build` or a running `spacetime serve`. So the output is an artifact
and belongs in `priv/static`: one origin, one process, and a production
deployment that needs no dev server. `serve` stays what it should be, a watcher
for live reload while you iterate.

`SPACETIME_BIN=/path/to/target/release/spacetime` points the task at a checkout
build.

## The client half

The `SpacetimeBridge` hook ships in this library's `priv/static/js`, in two
builds derived from one source:

| your app | file | how |
|---|---|---|
| has a bundler | `spacetime_bridge.js` | `import { SpacetimeBridge }`, pass in `hooks:` |
| has none | `spacetime_bridge.global.js` | plain `<script>` after `phoenix.js` + `phoenix_live_view.js`; self-boots |

Edit the ESM file, then `mix spacetime.bridge` to regenerate the classic build.
A test asserts the two are in step, so a forgotten regeneration fails the suite
rather than shipping a stale bridge.

## `shell:` is required

It is the markup the bundle hydrates. Required rather than defaulted, because a
default would be a second invented skeleton — and a copy of markup the compiler
already emits will drift. An earlier version carried one hard-coded DOM arm per
page and said so in its own comments.

A generator can lift it from the page (beam-lisp does, from the view's `&shell`
template). When the compiler emits `skeleton.html` beside the bundle (FUP-121),
this option gains a source rather than a replacement.

## Tasks

| task | does |
|---|---|
| `mix spacetime.build` | compile configured entries into `priv/static` |
| `mix spacetime.contract` | write `<Module>.contract.json` sidecars |
| `mix spacetime.bridge` | regenerate the classic-script bridge build |
