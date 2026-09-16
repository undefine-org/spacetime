defmodule Spacetime.LiveViewTest do
  use ExUnit.Case, async: true

  alias Spacetime.LiveView

  defmodule DemoLive do
    use Spacetime.LiveView,
      bundle: "/spacetime/demo/spacetime.js",
      root: ".demo",
      shell: ~s|<div class="demo"><span class="value"></span></div>|

    events do
      event(:inc)
    end

    assigns do
      assign(:count, :integer)
    end

    pushes do
      push(:flash, message: :string)
    end

    def mount(_params, _session, socket), do: {:ok, Phoenix.Component.assign(socket, count: 0)}

    def handle_event("inc", _payload, socket),
      do: {:reply, %{tag: "ok", reply: 1}, socket}
  end

  describe "the contract" do
    test "carries every declaration, sorted and typed" do
      contract = DemoLive.__spacetime_contract__()

      assert contract.module == "Spacetime.LiveViewTest.DemoLive"
      assert contract.events == [%{name: "inc"}]
      assert contract.assigns == [%{name: "count", type: "integer"}]
      assert contract.pushes == [%{name: "flash", fields: %{"message" => "string"}}]
    end

    test "fingerprints the contract, not the module" do
      %{fingerprint: fingerprint} = DemoLive.__spacetime_contract__()

      assert String.length(fingerprint) == 64
      assert fingerprint =~ ~r/^[0-9a-f]+$/
    end
  end

  describe "stylesheet_url/1" do
    test "a built bundle's stylesheet sits beside it" do
      assert LiveView.stylesheet_url("/spacetime/demo/spacetime.js") ==
               "/spacetime/demo/spacetime.css"
    end

    test "a serve origin names the stylesheet differently, and keeps the query" do
      # The trap this rule exists for: a naive `.js`→`.css` yields `runtime.css`,
      # which 404s and renders the page unstyled with no useful console error.
      assert LiveView.stylesheet_url(
               "http://127.0.0.1:4455/__spacetime/runtime.js?entry=index.edn"
             ) ==
               "http://127.0.0.1:4455/__spacetime/styles.css?entry=index.edn"
    end

    test "a query string on a built bundle survives" do
      assert LiveView.stylesheet_url("/spacetime/demo/spacetime.js?v=2") ==
               "/spacetime/demo/spacetime.css?v=2"
    end
  end

  describe "render_host/1" do
    test "emits the caller's shell, never an invented one" do
      html = render(DemoLive)

      assert html =~ ~s|<div class="demo">|
      assert html =~ ~s|<span class="value">|
    end

    test "fences the Spacetime subtree off from Phoenix's differ" do
      html = render(DemoLive)

      # Two writers on one subtree is a fight nobody wins: the page's signal
      # renderer owns everything under st-mount.
      assert html =~ ~s|phx-update="ignore"|
      assert html =~ ~s|id="st-mount"|
    end

    test "carries the data attributes the hook routes on" do
      html = render(DemoLive)

      assert html =~ ~s|phx-hook="SpacetimeBridge"|
      assert html =~ ~s|data-root=".demo"|
      assert html =~ ~s|data-module="Spacetime.LiveViewTest.DemoLive"|
    end

    test "loads the bundle and its stylesheet" do
      html = render(DemoLive)

      assert html =~ ~s|src="/spacetime/demo/spacetime.js"|
      assert html =~ ~s|href="/spacetime/demo/spacetime.css"|
    end
  end

  describe "E0928: a declared event must reach the server" do
    test "a declared event with no literal handle_event head fails the compile" do
      # The check that makes the contract worth declaring: an event the page can
      # emit and the server cannot answer is a runtime silence, so it is a
      # COMPILE error instead.
      source = """
      defmodule MissingMoveCardHandlerLive do
        use Spacetime.LiveView,
          bundle: "/spacetime/test/spacetime.js",
          root: ".test",
          shell: "<div class='test'></div>"

        events do
          event(:move_card)
        end
      end
      """

      error = assert_raise CompileError, fn -> Code.compile_string(source) end

      assert error.description =~ "move_card"
      assert error.description =~ "MissingMoveCardHandlerLive"
    end

    test "a dynamic handle_event head downgrades the error to a warning" do
      # A catch-all MAY handle it; that cannot be proven either way, so refusing
      # to compile would be wrong.
      source = """
      defmodule DynamicHandlerLive do
        use Spacetime.LiveView,
          bundle: "/spacetime/test/spacetime.js",
          root: ".test",
          shell: "<div class='test'></div>"

        events do
          event(:move_card)
        end

        def handle_event(_name, _payload, socket), do: {:noreply, socket}
      end
      """

      warning = ExUnit.CaptureIO.capture_io(:stderr, fn -> Code.compile_string(source) end)

      assert warning =~ "move_card"
      assert warning =~ "cannot be proven"
    end
  end

  describe "shell: is required" do
    test "a module without it does not compile" do
      # Required rather than defaulted: a default would be a second, invented
      # skeleton — the drift this library exists to remove.
      assert_raise KeyError, fn ->
        defmodule NoShellLive do
          use Spacetime.LiveView, bundle: "/b.js", root: ".x"
        end
      end
    end
  end

  # Render through the module's GENERATED `render/1` rather than calling
  # `render_host/1` with hand-built assigns: that is the real surface, and it
  # proves the bundle/root/shell given to `use` actually travel.
  defp render(module) do
    %{}
    |> module.render()
    |> Phoenix.LiveViewTest.rendered_to_string()
  end
end
