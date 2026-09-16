defmodule Mix.Tasks.Spacetime.Contract do
  @shortdoc "Emit LiveView contract sidecars for Spacetime pages"

  @moduledoc """
  Write each declared `Spacetime.LiveView` contract beside its page.

      mix spacetime.contract
      mix spacetime.contract MyAppWeb.BoardLive

  With no argument every module that `use Spacetime.LiveView` is written. That
  is the default deliberately: a one-module default silently leaves every other
  contract stale, which has bitten before — a presence assign changed shape and
  the sidecar that would have caught it was never regenerated.

  ## Configuration

      spacetime: [contract_dir: "frontend"]

  Defaults to `frontend`. The app whose modules are scanned is the current Mix
  project, so this works in any consuming app rather than a hardcoded one.
  """

  use Mix.Task

  @impl Mix.Task
  def run(args) do
    Mix.Task.run("compile")

    args
    |> modules()
    |> Enum.each(&write_contract!/1)
  end

  defp modules([]), do: spacetime_modules()
  defp modules(names), do: Enum.map(names, &Module.concat([&1]))

  defp spacetime_modules do
    app = Mix.Project.config()[:app]

    case :application.get_key(app, :modules) do
      {:ok, mods} ->
        Enum.filter(mods, fn mod ->
          Code.ensure_loaded?(mod) and function_exported?(mod, :__spacetime_contract__, 0)
        end)

      :undefined ->
        Mix.raise("could not list modules for #{inspect(app)} — has it compiled?")
    end
  end

  defp write_contract!(module) do
    Code.ensure_loaded!(module)

    unless function_exported?(module, :__spacetime_contract__, 0) do
      Mix.raise("#{inspect(module)} does not use Spacetime.LiveView")
    end

    dir = Keyword.get(Mix.Project.config()[:spacetime] || [], :contract_dir, "frontend")
    File.mkdir_p!(dir)
    path = Path.join(dir, module_filename(module) <> ".contract.json")

    contract = module.__spacetime_contract__()
    File.write!(path, JSON.encode!(contract) <> "\n")
    Mix.shell().info("wrote #{path}")
  end

  defp module_filename(module) do
    module
    |> Module.split()
    |> List.last()
  end
end
