defmodule Spacetime.BridgeTest do
  use ExUnit.Case, async: true

  alias Mix.Tasks.Spacetime.Bridge

  @source Path.expand("../../priv/static/js/spacetime_bridge.js", __DIR__)
  @target Path.expand("../../priv/static/js/spacetime_bridge.global.js", __DIR__)

  describe "the two builds are one file" do
    test "the classic build is in step with its ESM source" do
      # The whole point of deriving rather than hand-maintaining: if someone
      # edits the hook and forgets `mix spacetime.bridge`, this fails HERE
      # rather than in a browser that answers no signal.
      assert File.read!(@target) == Bridge.derive(File.read!(@source)),
             "spacetime_bridge.global.js is stale — run `mix spacetime.bridge`"
    end

    test "the classic build carries no ESM syntax" do
      # `export` in a plain <script> is a syntax error that kills the whole
      # file, so the page hydrates and then answers nothing.
      global = File.read!(@target)

      refute global =~ ~r/^export /m
      refute global =~ "export default"
    end

    test "the ESM build exports the hook a bundler imports" do
      esm = File.read!(@source)

      assert esm =~ "export const SpacetimeBridge"
      assert esm =~ "export function autoConnect"
    end

    test "the classic build self-boots; the ESM build does not" do
      # A bundler app owns its LiveSocket and passes the hook in beside its own;
      # a no-bundler app has nobody to do that, so the classic build calls
      # autoConnect itself.
      assert File.read!(@target) =~ "autoConnect();"
      refute File.read!(@source) =~ ~r/^autoConnect\(\);/m
    end
  end

  describe "the hook's contract with the compiled page" do
    test "routes both directions through the names the page registered" do
      esm = File.read!(@source)

      # server → page, and page → server. Named explicitly because these four
      # globals ARE the seam: rename one here and the page stops answering.
      assert esm =~ "__stLiveSubscriptions"
      assert esm =~ "__stLiveStreams"
      assert esm =~ "__stLiveBridge"
      assert esm =~ "st-set"
    end

    test "cleans up after itself" do
      # A LiveView that navigates away must not leave a bridge behind for the
      # next page to find and push into.
      assert File.read!(@source) =~ "destroyed()"
    end
  end
end
