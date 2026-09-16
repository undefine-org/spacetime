defmodule Mix.Tasks.Spacetime.Bridge do
  @shortdoc "Generate the classic-script build of the bridge hook"

  @moduledoc """
  Derive `spacetime_bridge.global.js` from `spacetime_bridge.js`.

  ## Why a generated second file rather than a second source

  The hook has two consumers with incompatible module systems:

    * a bundler app imports `SpacetimeBridge` and passes it in `hooks:`;
    * an app with no bundler loads a plain `<script>` — where the ESM keyword
      `export` is a syntax error, and the whole file fails to parse.

  Two hand-maintained copies is what this library exists to end, so the classic
  build is DERIVED: the ESM file is the single source, and this task rewrites
  only its module boundary — the `export` keywords come off and a self-boot call
  goes on. The body is copied verbatim, so the two can never disagree about
  retry policy, module routing, or anything else that matters.

  Run it after editing the hook; `mix test` asserts the two are in step, so a
  forgotten run fails the suite rather than shipping a stale bridge.
  """

  use Mix.Task

  @source "priv/static/js/spacetime_bridge.js"
  @target "priv/static/js/spacetime_bridge.global.js"

  @impl Mix.Task
  def run(_args) do
    File.write!(target_path(), derive(File.read!(source_path())))
    Mix.shell().info("wrote #{@target}")
  end

  @doc false
  def source_path, do: Application.app_dir(:spacetime_phoenix, @source)

  @doc false
  def target_path, do: Application.app_dir(:spacetime_phoenix, @target)

  @doc """
  The classic-script text for a given ESM source.

  Pure, so the drift test can compare `derive(read(source))` against the file on
  disk without running the task.
  """
  def derive(esm) do
    banner = """
    // GENERATED from spacetime_bridge.js by `mix spacetime.bridge` — do not edit.
    //
    // The classic-script build: same hook, no ESM. Load it with a plain <script>
    // after phoenix.js and phoenix_live_view.js and it boots its own LiveSocket.
    """

    body =
      esm
      |> String.replace("export const SpacetimeBridge = {", "const SpacetimeBridge = {")
      |> String.replace("export function autoConnect() {", "function autoConnect() {")
      |> String.replace("export default SpacetimeBridge;\n", "")
      |> String.trim_trailing()

    banner <> "\n(function () {\n" <> indent(body) <> "\n\n  autoConnect();\n})();\n"
  end

  defp indent(text) do
    text
    |> String.split("\n")
    |> Enum.map(fn
      "" -> ""
      line -> "  " <> line
    end)
    |> Enum.join("\n")
  end
end
