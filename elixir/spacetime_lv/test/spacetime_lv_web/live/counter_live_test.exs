defmodule SpacetimeLvWeb.CounterLiveTest do
  use SpacetimeLvWeb.ConnCase

  import Phoenix.LiveViewTest

  # The view is the compiled Spacetime bundle (frontend/counter.st); the
  # LiveView owns only state + replies. These tests pin the server half of
  # the live-transport loop (FEAT-137): host markup the bundle binds onto,
  # no Phoenix-owned controls, and the {:reply, %{tag, reply}} contract the
  # page's `receive to IncResult` arms decode.

  test "GET / serves the Spacetime host markup, no phx-click shim", %{conn: conn} do
    {:ok, view, _html} = live(conn, ~p"/")

    assert has_element?(view, "#st-host[phx-hook='SpacetimeBridge'][data-root='.counter']")
    assert has_element?(view, "#st-mount[phx-update='ignore']")
    assert has_element?(view, ".counter .value")
    assert has_element?(view, ".counter .controls button.inc")
    assert has_element?(view, ".counter .controls button.dec")

    # The .st page owns the controls (its @on click fires the signals) —
    # no Phoenix event bindings may remain anywhere in the host subtree.
    refute has_element?(view, "#st-mount [phx-click]")

    # The compiled bundle + stylesheet are referenced.
    assert has_element?(view, ~s|script[src='/spacetime/counter/spacetime.js']|)
    assert has_element?(view, ~s|link[href='/spacetime/counter/spacetime.css']|)
  end

  test "inc/dec events answer {:reply, %{tag, reply}} with the server count", %{conn: conn} do
    {:ok, view, _html} = live(conn, ~p"/")

    render_click(view, "inc")
    assert_reply(view, %{tag: "ok", reply: 1})

    render_click(view, "inc")
    assert_reply(view, %{tag: "ok", reply: 2})

    render_click(view, "dec")
    assert_reply(view, %{tag: "ok", reply: 1})
  end

  test "dec at zero returns the tagged rejection used by optimistic rollback", %{conn: conn} do
    {:ok, view, _html} = live(conn, ~p"/")

    render_click(view, "dec")
    assert_reply(view, %{tag: "err", reply: "Count cannot go below zero"})
  end

  test "connected seed and server ticks push module-scoped count diffs", %{conn: conn} do
    {:ok, view, _html} = live(conn, ~p"/")

    assert_push_event(view, "st-set", %{
      module: "SpacetimeLvWeb.CounterLive",
      assigns: %{"count" => 0}
    })

    send(view.pid, :tick)

    assert_push_event(view, "st-set", %{
      module: "SpacetimeLvWeb.CounterLive",
      assigns: %{"count" => 1}
    })
  end
end
