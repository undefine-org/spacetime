defmodule Spacetime.LiveView do
  # `render_host/1` builds HEEx via `~H`, so this module needs the component imports.
  use Phoenix.Component

  @moduledoc """
  Render a Phoenix LiveView's body with a Spacetime bundle instead of HEEx.

  ## The bridge thesis

  One typed state machine, split at the network boundary — never two apps with
  a protocol between them:

    * The LiveView owns the **socket, assigns and effects**. `mount/3`,
      `handle_event/3`, `handle_info/2` and PubSub are ordinary LiveView.
    * The **view** is a compiled Spacetime bundle, not HEEx. The LiveView
      supplies DATA; the page owns rendering, local state and animation.
    * A small **bridge** maps the two planes:
      - assign → signal: after each render the changed assigns are pushed as a
        module-scoped `st-set` diff, so the connected mount supplies the seed;
      - signal → event → reply: the page's `@data signal … send emit` fires over
        this LiveView's own channel through `window.__stLiveBridge` (mounted by
        the `SpacetimeBridge` hook), `handle_event/3` answers
        `{:reply, %{tag: …, reply: …}}`, and the page's `receive` arms decode it
        through the same matcher family as `@match`.

  Data crosses, never templates. The Spacetime subtree carries
  `phx-update="ignore"`, so Phoenix's differ never touches it and the page's
  signal renderer is its sole writer.

  ## Usage

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

  A `render/1` is GENERATED: it emits the host element the bundle hydrates, the
  stylesheet and the bundle script. You never write one.

  ## `shell:` is required, and why

  Earlier versions carried a `cond` in `render_host/1` with one hard-coded DOM
  arm per page, and said so themselves: *"every new page grows one more arm here
  that silently drifts from its .st"*. That is a copy of markup the compiler
  already emitted, and a copy drifts.

  So the caller passes the skeleton. A generator (`spell.contract/elixir-module`
  in beam-lisp) lifts it from the view's own `&shell` template; a hand-written
  LiveView passes the same markup its page binds onto. Required rather than
  defaulted, because a default would be a second invented skeleton — the exact
  failure this removes.

  When the compiler emits `skeleton.html` beside the bundle (FUP-121), this
  option gains a source rather than a replacement: the value travels from the
  page either way.
  """

  @doc """
  Inject the Spacetime-rendered LiveView scaffolding.

  Options:
    * `:bundle` (required) — URL of the compiled bundle, normally served from
      `priv/static` (see `mix spacetime.build`).
    * `:root` (required) — CSS selector of the bundle's root element.
    * `:shell` (required) — the host markup the bundle hydrates.
  """
  defmacro __using__(opts) do
    bundle = Keyword.fetch!(opts, :bundle)
    root = Keyword.fetch!(opts, :root)
    shell = Keyword.fetch!(opts, :shell)

    quote do
      use Phoenix.LiveView

      @spacetime_bundle unquote(bundle)
      @spacetime_root unquote(root)
      @spacetime_shell unquote(shell)

      on_mount({Spacetime.LiveView, :reconcile_assigns})

      import Spacetime.LiveView,
        only: [
          assigns: 1,
          events: 1,
          event: 1,
          event: 2,
          pushes: 1,
          push: 1,
          push: 2,
          push_view: 2
        ]

      Module.register_attribute(__MODULE__, :spacetime_events, accumulate: true)
      Module.register_attribute(__MODULE__, :spacetime_assigns, accumulate: true)
      Module.register_attribute(__MODULE__, :spacetime_pushes, accumulate: true)
      Module.register_attribute(__MODULE__, :spacetime_handled_events, accumulate: true)
      @on_definition Spacetime.LiveView
      @before_compile Spacetime.LiveView

      @doc false
      def render(assigns) do
        assigns
        |> Map.put(:__bundle__, @spacetime_bundle)
        |> Map.put(:__root__, @spacetime_root)
        |> Map.put(:__shell__, @spacetime_shell)
        |> Map.put(:__module__, inspect(__MODULE__))
        |> Spacetime.LiveView.render_host()
      end
    end
  end

  @doc """
  The generated host markup + bridge boot.

  Emits the bundle's DOM skeleton (as given by `shell:`), the stylesheet and the
  bundle script. `phx-update="ignore"` keeps Phoenix's DOM patcher OUT of the
  Spacetime-owned subtree: the page's own signal renderer is the sole writer
  there, and two writers on one subtree is a fight nobody wins.

  The host element carries the data attributes the `SpacetimeBridge` hook reads
  (`data-root`, `data-module`); the hook is the legitimate glue layer, never
  page logic.
  """
  def render_host(assigns) do
    ~H"""
    <div id="st-host" phx-hook="SpacetimeBridge" data-root={@__root__} data-module={@__module__}>
      <div phx-update="ignore" id="st-mount">
        {Phoenix.HTML.raw(@__shell__)}
      </div>
    </div>
    <link rel="stylesheet" href={Spacetime.LiveView.stylesheet_url(@__bundle__)} />
    <script type="module" src={@__bundle__}>
    </script>
    """
  end

  @doc """
  The stylesheet URL that accompanies a bundle URL.

  The stylesheet sits beside the bundle, but its NAME is not always the bundle's
  with a different extension. A `spacetime serve` origin answers
  `runtime.js?entry=…` and `styles.css?entry=…`, so a naive `.js`→`.css` replace
  produces `runtime.css` — a 404 that fails silently, rendering the page
  unstyled with no console error worth the name. Two rules, one for each shape.
  """
  def stylesheet_url(bundle) do
    if String.contains?(bundle, "runtime.js") do
      String.replace(bundle, "runtime.js", "styles.css")
    else
      String.replace(bundle, ~r/\.js(\?.*)?$/, ".css\\1")
    end
  end

  @reconciler_private_key :spacetime_live_view_assigns

  @doc false
  def on_mount(:reconcile_assigns, _params, _session, socket) do
    socket =
      Phoenix.LiveView.attach_hook(
        socket,
        :spacetime_live_view_assign_reconciler,
        :after_render,
        &reconcile_assigns/1
      )

    {:cont, socket}
  end

  @doc false
  def reconcile_assigns(socket) do
    if Phoenix.LiveView.connected?(socket) do
      assigns = public_assigns(socket.assigns)
      previous = Map.get(socket.private, @reconciler_private_key, %{})
      diff = changed_assigns(assigns, previous)
      socket = put_in(socket.private[@reconciler_private_key], assigns)

      if map_size(diff) == 0 do
        socket
      else
        push_assign_diff(socket, diff)
      end
    else
      socket
    end
  end

  @doc """
  Push an explicit assign → signal diff to the Spacetime view.

  Shares the reconciler's `st-set` envelope; ordinary assign changes are pushed
  automatically after render, so this is only for the rare explicit case.
  """
  def push_view(socket, assigns) when is_list(assigns) do
    push_assign_diff(socket, Map.new(assigns))
  end

  # LiveView's own bookkeeping must not cross the bridge: `__changed__` is a
  # diff-tracking map, and `flash`/`live_action` are framework state. The page
  # has no signal by any of those names, so pushing them is noise at best and a
  # spurious "changed" diff every render at worst.
  defp public_assigns(assigns) do
    Map.reject(assigns, fn {key, _value} ->
      key == :__changed__ or key == :flash or key == :live_action
    end)
  end

  defp changed_assigns(assigns, previous) do
    Map.filter(assigns, fn {key, value} -> Map.get(previous, key, :__missing__) !== value end)
  end

  defp push_assign_diff(socket, assigns) do
    payload = %{
      module: inspect(socket.view),
      assigns: Map.new(assigns, fn {key, value} -> {to_string(key), value} end)
    }

    Phoenix.LiveView.push_event(socket, "st-set", payload)
  end

  @doc "Declares the events a Spacetime page may emit to this LiveView."
  defmacro events(do: block), do: block

  @doc "Declares the assigns the Spacetime page may subscribe to."
  defmacro assigns(do: block) do
    Macro.prewalk(block, fn
      {:assign, meta, args} -> {{:., meta, [Spacetime.LiveView, :assign]}, meta, args}
      node -> node
    end)
  end

  @doc "Declares the transient effects this LiveView may push to its page."
  defmacro pushes(do: block), do: block

  defmacro event(name), do: declare(:spacetime_events, name, [])
  defmacro event(name, fields), do: declare(:spacetime_events, name, fields)
  defmacro assign(name, type), do: declare(:spacetime_assigns, name, type)
  defmacro push(name), do: declare(:spacetime_pushes, name, [])
  defmacro push(name, fields), do: declare(:spacetime_pushes, name, fields)

  @doc """
  Pushes a declared transient effect to the Spacetime bridge.

  The declaration is checked at runtime because `Phoenix.LiveView.push_event/3`
  can be called directly; use this wrapper so an undeclared push fails loudly
  here rather than arriving at a page with no arm to decode it.
  """
  def push_event(socket, name, payload) when is_atom(name) or is_binary(name) do
    declared = socket.view.__spacetime_contract__().pushes
    name = to_string(name)

    unless Enum.any?(declared, &(&1.name == name)) do
      raise ArgumentError,
            "undeclared Spacetime push #{inspect(name)} for #{inspect(socket.view)}; " <>
              "declare it in pushes do ... end"
    end

    Phoenix.LiveView.push_event(socket, name, payload)
  end

  # Reply tags live in handler bodies and can be runtime values, so
  # `@on_definition` cannot soundly enumerate them; an explicit `replies`
  # declaration waits until the contract sidecar exposes the page's receive
  # discriminators.

  # Literal handle_event/3 heads are proof that a declared page event reaches the
  # server. A dynamic head may be an intentional catch-all; it cannot prove a
  # particular event, so missing literal heads warn rather than fail in that case.
  @doc false
  def __on_definition__(env, _kind, :handle_event, [event, _, _], _guards, _body) do
    handled_event = if is_binary(event), do: event, else: :dynamic
    Module.put_attribute(env.module, :spacetime_handled_events, handled_event)
  end

  def __on_definition__(_env, _kind, _name, _args, _guards, _body), do: :ok

  defmacro __before_compile__(env) do
    declared_events = Module.get_attribute(env.module, :spacetime_events)
    handled_events = Module.get_attribute(env.module, :spacetime_handled_events)
    dynamic_handler? = :dynamic in handled_events

    declared_events
    |> Enum.map(fn {name, _metadata} -> to_string(name) end)
    |> Enum.reject(&(&1 in handled_events))
    |> Enum.each(fn event ->
      message =
        "E0928: declared Spacetime event #{inspect(event)} for #{inspect(env.module)} " <>
          "has no literal handle_event(#{inspect(event)}, _, _) clause"

      if dynamic_handler? do
        IO.warn(
          message <> "; a dynamic handle_event/3 head may handle it, so this cannot be proven"
        )
      else
        raise CompileError, file: env.file, line: env.line, description: message
      end
    end)

    contract = %{
      module: inspect(env.module),
      events: contract_entries(declared_events, :fields),
      assigns: contract_entries(Module.get_attribute(env.module, :spacetime_assigns), :type),
      pushes: contract_entries(Module.get_attribute(env.module, :spacetime_pushes), :fields)
    }

    # OTP's `JSON`, not Jason: a library must not force a JSON dependency on
    # every consumer. The fingerprint is a hash of the encoded contract, so the
    # only requirement is that both sides of a comparison encode the same way.
    fingerprint =
      contract
      |> JSON.encode!()
      |> then(&:crypto.hash(:sha256, &1))
      |> Base.encode16(case: :lower)

    quote do
      @doc false
      def __spacetime_contract__ do
        unquote(Macro.escape(Map.put(contract, :fingerprint, fingerprint)))
      end
    end
  end

  defp declare(attribute, name, metadata) do
    quote bind_quoted: [attribute: attribute, name: name, metadata: metadata] do
      Module.put_attribute(__MODULE__, attribute, {name, metadata})
    end
  end

  defp contract_entries(entries, metadata_key) do
    entries
    |> Enum.reverse()
    |> Enum.map(fn {name, metadata} ->
      entry = %{name: to_string(name)}

      metadata =
        case metadata_key do
          :fields ->
            Enum.into(metadata, %{}, fn {key, type} -> {to_string(key), to_string(type)} end)

          :type ->
            to_string(metadata)
        end

      if metadata in [%{}, ""] do
        entry
      else
        Map.put(entry, metadata_key, metadata)
      end
    end)
    |> Enum.sort_by(& &1.name)
  end
end
