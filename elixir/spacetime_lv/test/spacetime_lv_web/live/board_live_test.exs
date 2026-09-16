defmodule SpacetimeLvWeb.BoardLiveTest do
  use SpacetimeLvWeb.ConnCase, async: false

  import Phoenix.LiveViewTest

  alias SpacetimeLvWeb.{BoardLive, Presence}

  setup do
    :ok = BoardLive.reset!()
    :ok
  end

  test "GET /board serves the Spacetime board host without Phoenix controls", %{conn: conn} do
    {:ok, view, _html} = live(conn, ~p"/board")

    assert has_element?(view, "#st-host[phx-hook='SpacetimeBridge'][data-root='.board']")
    assert has_element?(view, "#st-mount[phx-update='ignore']")
    assert has_element?(view, ".board")
    assert has_element?(view, "#board-columns")
    assert has_element?(view, "#board-cursor-layer")
    assert has_element?(view, "#board-toasts")
    refute has_element?(view, "#st-mount [phx-click]")
  end

  test "cursor events reconcile to peers but exclude the local cursor", %{conn: conn} do
    {:ok, view, _html} = live(conn, ~p"/board")
    _ = initial_items(view)

    render_click(view, "cursor", %{"x" => "120.5", "y" => 64})

    assert [%{metas: [%{name: "Collaborator", color: "#8bb7ff", x: 120.5, y: 64.0}]}] =
             BoardLive.topic()
             |> Presence.list()
             |> Map.values()
  end

  test "presence joins and leaves reconcile through the presence assign", %{conn: conn} do
    {:ok, view, _html} = live(conn, ~p"/board")
    _ = initial_items(view)

    {:ok, _} =
      Presence.track(self(), BoardLive.topic(), "test-peer", %{
        id: "test-peer",
        name: "Test peer",
        color: "#facc15",
        # non-zero coords: peers at the 0.0/0.0 seed (never moved) are filtered
        # from remote_presence so they don't render as frozen corner dots
        x: 12.0,
        y: 34.0
      })

    # presence is a LIST of peer metas (the .st page iterates it with @each)
    assert 1 == view |> presence_assign() |> length()

    :ok = Presence.untrack(self(), BoardLive.topic(), "test-peer")

    assert 0 == view |> presence_assign() |> length()
  end

  test "move_card returns tagged success and stale-version rejection", %{conn: conn} do
    {:ok, view, _html} = live(conn, ~p"/board")
    %{id: card_id, version: version} = initial_card(view)

    render_click(view, "move_card", %{
      "card_id" => card_id,
      "column" => "done",
      "version" => version
    })

    assert_reply(view, %{tag: "ok"})

    render_click(view, "move_card", %{
      "card_id" => card_id,
      "column" => "doing",
      "version" => version
    })

    assert_reply(view, %{tag: "err", reply: "Card version is stale"})
  end

  test "concurrent moves reject the stale loser", %{conn: conn} do
    {:ok, first_view, _html} = live(conn, ~p"/board")
    {:ok, second_view, _html} = live(conn, ~p"/board")
    %{id: card_id, version: version} = initial_card(first_view)
    _ = initial_items(second_view)

    render_click(first_view, "move_card", %{
      "card_id" => card_id,
      "column" => "doing",
      "version" => version
    })

    assert_reply(first_view, %{tag: "ok"})

    render_click(second_view, "move_card", %{
      "card_id" => card_id,
      "column" => "done",
      "version" => version
    })

    assert_reply(second_view, %{tag: "err", reply: "Card version is stale"})
  end

  test "move_card returns tagged lock rejection", %{conn: conn} do
    {:ok, view, _html} = live(conn, ~p"/board")
    %{id: card_id, version: version} = initial_card(view)

    render_click(view, "lock_card", %{"card_id" => card_id})
    assert_reply(view, %{tag: "ok"})

    render_click(view, "move_card", %{
      "card_id" => card_id,
      "column" => "done",
      "version" => version
    })

    assert_reply(view, %{tag: "err", reply: "Card is locked"})
  end

  test "board PubSub changes reconcile as an items-only st-set diff", %{conn: conn} do
    {:ok, view, _html} = live(conn, ~p"/board")
    items = initial_items(view)
    [first | rest] = items
    updated_items = [%{first | column: "done", version: first.version + 1} | rest]

    Phoenix.PubSub.broadcast(
      SpacetimeLv.PubSub,
      BoardLive.topic(),
      {:board_changed, updated_items}
    )

    assert_push_event(view, "st-set", %{
      module: "SpacetimeLvWeb.BoardLive",
      assigns: %{"items" => ^updated_items}
    })
  end

  defp presence_assign(view) do
    assert_push_event(view, "st-set", %{
      module: "SpacetimeLvWeb.BoardLive",
      assigns: %{"presence" => presence}
    })

    presence
  end

  defp initial_card(view) do
    view
    |> initial_items()
    |> List.first()
  end

  defp initial_items(view) do
    assert_push_event(view, "st-set", %{
      module: "SpacetimeLvWeb.BoardLive",
      assigns: %{"items" => items}
    })

    items
  end
end
