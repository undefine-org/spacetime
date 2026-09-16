defmodule SpacetimeLvWeb.BoardLive do
  @moduledoc false

  use Spacetime.LiveView,
    bundle: "/spacetime/board/spacetime.js",
    root: ".board",
    # The skeleton `frontend/board.st` binds onto by selector. It lived in a
    # `cond` arm inside `render_host/1` until the bridge became a library; it
    # belongs with the page that owns it. FUP-121 replaces this with the
    # compiler's own emission.
    shell: ~s|<div class="board">
      <div id="board-columns">
        <section class="column" data-column="todo"><div class="column__cards"></div></section>
        <section class="column" data-column="doing"><div class="column__cards"></div></section>
        <section class="column" data-column="done"><div class="column__cards"></div></section>
      </div>
      <div id="board-cursor-layer" class="cursor-layer"></div>
      <div id="board-toasts"><div class="toast"></div></div>
      <input class="new-card__input" aria-label="New card" />
      <button class="new-card__submit" type="button">Add card</button>
    </div>|

  alias SpacetimeLv.Board
  alias SpacetimeLvWeb.Presence

  @topic "board"

  events do
    event(:move_card, card_id: :string, column: :string, version: :integer)
    event(:add_card, title: :string)
    event(:lock_card, card_id: :string)
    event(:cursor, x: :float, y: :float)
  end

  assigns do
    assign(:items, :list)
    assign(:presence, :list)
  end

  pushes do
    push(:flash, message: :string, kind: :atom)
  end

  def topic, do: @topic
  def reset!, do: Board.reset!()

  @impl true
  def mount(_params, _session, socket) do
    socket = assign(socket, items: Board.items(), presence: [])

    if connected?(socket) do
      Phoenix.PubSub.subscribe(SpacetimeLv.PubSub, @topic)
      {:ok, _} = Presence.track(self(), @topic, socket.id, presence_meta(socket.id))
      assign(socket, :presence, remote_presence(socket.id))
    else
      socket
    end
    |> then(&{:ok, &1})
  end

  @impl true
  def handle_event("move_card", params, socket) do
    with {:ok, card_id} <- required(params, "card_id"),
         {:ok, column} <- required(params, "column"),
         {:ok, version} <- integer(params["version"]),
         {:ok, items, card} <- Board.move(card_id, column, version) do
      flash = "Moved #{card.title} to #{column}"
      broadcast(items, flash, socket.id)

      socket =
        socket
        |> assign(:items, items)
        |> Spacetime.LiveView.push_event(:flash, %{
          message: flash,
          kind: :info
        })

      {:reply, %{tag: "ok", reply: %{id: card.id, version: card.version}}, socket}
    else
      {:error, reason} -> {:reply, %{tag: "err", reply: reason}, socket}
    end
  end

  def handle_event("cursor", params, socket) do
    with {:ok, x} <- coordinate(params["x"]),
         {:ok, y} <- coordinate(params["y"]),
         {:ok, _} <- Presence.update(self(), @topic, socket.id, cursor_meta(socket.id, x, y)) do
      {:noreply, socket}
    else
      {:error, _reason} -> {:noreply, socket}
    end
  end

  def handle_event("lock_card", %{"card_id" => card_id}, socket) do
    case Board.lock(card_id) do
      {:ok, items, card} ->
        broadcast(items)

        {:reply, %{tag: "ok", reply: %{id: card.id, version: card.version}},
         assign(socket, :items, items)}

      {:error, reason} ->
        {:reply, %{tag: "err", reply: reason}, socket}
    end
  end

  def handle_event("lock_card", _params, socket),
    do: {:reply, %{tag: "err", reply: "Missing card id"}, socket}

  def handle_event("add_card", %{"title" => title}, socket) do
    case Board.add(title) do
      {:ok, items, card} ->
        broadcast(items)

        {:reply, %{tag: "ok", reply: %{id: card.id, version: card.version}},
         assign(socket, :items, items)}
    end
  end

  def handle_event("add_card", _params, socket),
    do: {:reply, %{tag: "err", reply: "Missing title"}, socket}

  @impl true
  def handle_info({:board_changed, items}, socket), do: {:noreply, assign(socket, :items, items)}

  # A move broadcast carries the flash for OBSERVERS only: the mover already
  # received it as the direct push_event of its own handle_event (fire-once).
  def handle_info({:board_changed, items, flash, from}, socket) do
    socket = assign(socket, :items, items)

    socket =
      if from == socket.id do
        socket
      else
        Spacetime.LiveView.push_event(socket, :flash, %{message: flash, kind: :info})
      end

    {:noreply, socket}
  end

  def handle_info(%Phoenix.Socket.Broadcast{event: "presence_diff"}, socket),
    do: {:noreply, assign(socket, :presence, remote_presence(socket.id))}

  # Presence is a LIST of peer metas (each carries :id as the @each key):
  # the .st page iterates it directly — @each does not walk maps.
  defp remote_presence(client_id) do
    @topic
    |> Presence.list()
    |> Map.delete(client_id)
    |> Enum.map(fn {_id, %{metas: [meta | _]}} -> meta end)
    # A peer renders once their cursor has MOVED. Until then their meta is the
    # x: 0.0/y: 0.0 seed — drawing it puts a frozen "Collaborator" dot in the
    # top-left corner, which reads as a broken remote cursor.
    |> Enum.reject(&(&1.x == 0.0 and &1.y == 0.0))
  end

  defp broadcast(items, flash, from),
    do: Phoenix.PubSub.broadcast(SpacetimeLv.PubSub, @topic, {:board_changed, items, flash, from})

  defp broadcast(items),
    do: Phoenix.PubSub.broadcast(SpacetimeLv.PubSub, @topic, {:board_changed, items})

  defp required(params, key) do
    case params[key] do
      value when is_binary(value) and value != "" -> {:ok, value}
      _ -> {:error, "Missing #{String.replace(key, "_", " ")}"}
    end
  end

  defp integer(value) when is_integer(value), do: {:ok, value}

  defp integer(value) when is_binary(value) do
    case Integer.parse(value) do
      {integer, ""} -> {:ok, integer}
      _ -> {:error, "Invalid card version"}
    end
  end

  defp integer(_value), do: {:error, "Invalid card version"}

  defp coordinate(value) when is_integer(value), do: {:ok, value * 1.0}
  defp coordinate(value) when is_float(value), do: {:ok, value}

  defp coordinate(value) when is_binary(value) do
    case Float.parse(value) do
      {coordinate, ""} -> {:ok, coordinate}
      _ -> {:error, :invalid_coordinate}
    end
  end

  defp coordinate(_value), do: {:error, :invalid_coordinate}

  defp presence_meta(id) do
    %{id: id, name: "Collaborator", color: "#8bb7ff", x: 0.0, y: 0.0}
  end

  defp cursor_meta(id, x, y) do
    @topic
    |> Presence.list()
    |> Map.get(id, %{})
    |> Map.get(:metas, [])
    |> List.first()
    |> then(&if &1, do: Map.take(&1, [:id, :name, :color]), else: presence_meta(id))
    |> Map.merge(%{x: x, y: y})
  end
end
