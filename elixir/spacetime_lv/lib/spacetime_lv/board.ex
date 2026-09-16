defmodule SpacetimeLv.Board do
  @moduledoc false

  use Agent

  @initial_items [
    %{
      id: "brief",
      title: "Shape the brief",
      column: "todo",
      position: 0,
      version: 1,
      locked: false
    },
    %{
      id: "research",
      title: "Research the problem",
      column: "todo",
      position: 1,
      version: 1,
      locked: false
    },
    %{
      id: "prototype",
      title: "Prototype the flow",
      column: "doing",
      position: 0,
      version: 1,
      locked: false
    },
    %{id: "docs", title: "Write docs", column: "done", position: 0, version: 1, locked: false}
  ]

  def start_link(_opts), do: Agent.start_link(fn -> @initial_items end, name: __MODULE__)

  def items, do: Agent.get(__MODULE__, & &1)
  def reset!, do: Agent.update(__MODULE__, fn _items -> @initial_items end)

  def move(card_id, column, version) do
    Agent.get_and_update(__MODULE__, fn items ->
      case Enum.find(items, &(&1.id == card_id)) do
        nil ->
          {{:error, "Card not found"}, items}

        %{locked: true} ->
          {{:error, "Card is locked"}, items}

        %{version: current} when current != version ->
          {{:error, "Card version is stale"}, items}

        card ->
          next_position = items |> Enum.filter(&(&1.column == column)) |> length()
          moved = %{card | column: column, position: next_position, version: card.version + 1}
          updated = Enum.map(items, fn item -> if item.id == card_id, do: moved, else: item end)
          {{:ok, updated, moved}, updated}
      end
    end)
  end

  def lock(card_id) do
    Agent.get_and_update(__MODULE__, fn items ->
      case Enum.find(items, &(&1.id == card_id)) do
        nil ->
          {{:error, "Card not found"}, items}

        card ->
          locked = %{card | locked: true, version: card.version + 1}
          updated = Enum.map(items, fn item -> if item.id == card_id, do: locked, else: item end)
          {{:ok, updated, locked}, updated}
      end
    end)
  end

  def add(title) when is_binary(title) do
    Agent.get_and_update(__MODULE__, fn items ->
      card = %{
        id: "card-#{System.unique_integer([:positive])}",
        title: title,
        column: "todo",
        position: Enum.count(items, &(&1.column == "todo")),
        version: 1,
        locked: false
      }

      updated = items ++ [card]
      {{:ok, updated, card}, updated}
    end)
  end
end
