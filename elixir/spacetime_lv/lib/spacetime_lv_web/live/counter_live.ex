defmodule SpacetimeLvWeb.CounterLive do
  @moduledoc """
  CounterLive — the native live-transport vertical (FEAT-137).

  The view is `frontend/counter.st` (compiled to `/spacetime/counter/spacetime.js`),
  NOT HEEx — and the page now drives the wire itself: its buttons fire
  `@data signal … send emit "inc"`, which the live transport routes over this
  LiveView's own channel as a phx event. Each `handle_event/3` answers
  `{:reply, %{tag, reply}}` — the correlated one-shot the page's
  `receive to IncResult { "ok" => Bumped($.reply) }` decodes, and its `@handle`
  applies to the `$count` signal.

  The `st-set` push channel reconciles each changed assign (including the
  connected-mount seed); the event loop remains reply-based for client fires.
  The `inc`/`dec` event names match `frontend/CounterLive.contract.json` — the
  contract the build-time E0928 check verifies the .st page against.
  """
  use Spacetime.LiveView,
    bundle: "/spacetime/counter/spacetime.js",
    root: ".counter",
    # The skeleton `frontend/counter.st` binds onto — the buttons are plain, the
    # page wires `@on click` to its own signals and no `phx-click` appears.
    shell: ~s|<div class="counter">
      <span class="value"></span>
      <span class="error"></span>
      <div class="controls">
        <button class="inc">+</button>
        <button class="dec">−</button>
      </div>
    </div>|

  events do
    event(:inc)
    event(:dec)
  end

  assigns do
    assign(:count, :integer)
  end

  pushes do
    push(:flash, message: :string, kind: :atom)
  end

  @impl true
  def mount(_params, _session, socket) do
    if connected?(socket), do: Process.send_after(self(), :tick, 1_000)
    {:ok, assign(socket, count: 0)}
  end

  @impl true
  def handle_event("inc", _params, socket) do
    {count, socket} = bump(socket, +1)
    {:reply, %{tag: "ok", reply: count}, socket}
  end

  def handle_event("dec", _params, %{assigns: %{count: 0}} = socket) do
    {:reply, %{tag: "err", reply: "Count cannot go below zero"}, socket}
  end

  def handle_event("dec", _params, socket) do
    {count, socket} = bump(socket, -1)
    {:reply, %{tag: "ok", reply: count}, socket}
  end

  @impl true
  def handle_info(:tick, socket) do
    Process.send_after(self(), :tick, 1_000)
    {count, socket} = bump(socket, +1)

    socket =
      if rem(count, 5) == 0 do
        Spacetime.LiveView.push_event(socket, :flash, %{
          message: "Count reached #{count}",
          kind: :info
        })
      else
        socket
      end

    {:noreply, socket}
  end

  # Mutate the server count; the reconciler streams this changed assign while
  # event callers also receive the correlated reply.
  defp bump(socket, delta) do
    count = socket.assigns.count + delta
    {count, assign(socket, count: count)}
  end
end
