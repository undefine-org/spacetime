defmodule Mix.Tasks.Spacetime.Build do
  @shortdoc "Compile Spacetime entries into priv/static"

  @moduledoc """
  Compile each configured `.st`/`.edn` entry to `spacetime.js` + `spacetime.css`.

      mix spacetime.build

  ## Configuration

  In the consuming app's `mix.exs`:

      def project do
        [
          # ...
          spacetime: [
            bin: "spacetime",
            entries: [
              {"frontend/board.st", "priv/static/spacetime/board"},
              {"ui/index.edn", "priv/static/spacetime"}
            ]
          ]
        ]
      end

  `bin` is optional and defaults to `spacetime` on `PATH`; `SPACETIME_BIN`
  overrides it, which is what a checkout with a `cargo build --release` binary
  wants.

  ## Why a build task and not a dev server

  The compile is deterministic: the same entry produces the same bundle, byte
  for byte, whether it came from `spacetime build` or from a running
  `spacetime serve`. So the output is an ARTIFACT, and an artifact belongs in
  `priv/static` where `Plug.Static` serves it — one origin, one process, and a
  production deployment that needs no dev server at all.

  Treating `serve` as the source of the bundle makes a dev tool a runtime
  dependency: no serve, no page, and a second origin the browser can be pointed
  at by mistake (where the page's assigns resolve to nothing and every derived
  signal throws on undefined). `serve` remains exactly what it should be — a
  watcher for live reload while you iterate.

  ## Failure is loud

  A compile error raises rather than leaving the previous bundle in place. A
  page that silently serves yesterday's compile is worse than one that refuses
  to build, because the failure surfaces in a browser, later, to someone else.
  """

  use Mix.Task

  @impl Mix.Task
  def run(_args) do
    config = Mix.Project.config()[:spacetime] || []
    entries = Keyword.get(config, :entries, [])

    if entries == [] do
      Mix.raise("""
      no Spacetime entries configured.

      Add to mix.exs:

          spacetime: [entries: [{"frontend/page.st", "priv/static/spacetime/page"}]]
      """)
    end

    bin = binary(config)
    Enum.each(entries, &build_entry(&1, bin))
  end

  defp binary(config) do
    System.get_env("SPACETIME_BIN") || Keyword.get(config, :bin, "spacetime")
  end

  defp build_entry({entry, out}, bin) do
    unless File.exists?(entry) do
      Mix.raise("Spacetime entry not found: #{entry}")
    end

    File.mkdir_p!(out)

    case System.cmd(bin, ["build", entry, "-o", out], stderr_to_stdout: true) do
      {output, 0} ->
        Mix.shell().info("spacetime build #{entry} → #{out}")
        # The CLI reports what it wrote; pass it through so a surprising size or
        # a "copied N static assets" line is visible in the build log.
        Mix.shell().info(indent(output))

      {output, status} ->
        Mix.raise("""
        spacetime build failed for #{entry} (exit #{status}):

        #{indent(output)}
        """)
    end
  rescue
    e in ErlangError ->
      # `:enoent` from System.cmd means the BINARY is missing, not the entry —
      # a distinction worth making, because the two have different fixes and
      # the raw error names neither.
      case e.original do
        :enoent ->
          Mix.raise("""
          spacetime binary not found: #{bin}

          Install it, put it on PATH, or point SPACETIME_BIN at a built one:

              SPACETIME_BIN=/path/to/target/release/spacetime mix spacetime.build
          """)

        other ->
          reraise(e, [{:original, other}] ++ __STACKTRACE__)
      end
  end

  defp indent(text) do
    text
    |> String.trim_trailing()
    |> String.split("\n")
    |> Enum.map_join("\n", &("    " <> &1))
  end
end
