defmodule SpacetimePhoenix.MixProject do
  use Mix.Project

  @version "0.1.0"

  def project do
    [
      app: :spacetime_phoenix,
      version: @version,
      elixir: "~> 1.15",
      elixirc_paths: elixirc_paths(Mix.env()),
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      description: description(),
      package: package(),
      docs: docs()
    ]
  end

  # A LIBRARY: no `mod:`, no supervision tree, nothing started on behalf of the
  # host application. It contributes a macro, a Mix task and one JS asset.
  def application do
    [extra_applications: [:logger]]
  end

  defp elixirc_paths(:test), do: ["lib", "test/support"]
  defp elixirc_paths(_), do: ["lib"]

  # Phoenix and LiveView ONLY. The bridge is the seam between Spacetime and
  # Phoenix, so it may depend on Phoenix — and on nothing else, because every
  # further dependency is one the consuming app inherits without asking. In
  # particular there is no JSON library here: OTP ships `JSON` since 27, and a
  # library that drags in Jason forces that choice on its consumers.
  defp deps do
    [
      {:phoenix, "~> 1.7"},
      {:phoenix_live_view, "~> 1.0"},
      {:phoenix_html, "~> 4.1"},
      {:ex_doc, ">= 0.0.0", only: :dev, runtime: false}
    ]
  end

  defp description do
    "Render a Phoenix LiveView's body with a compiled Spacetime bundle: " <>
      "assign→signal diffs, signal→event→reply over the LiveView's own channel."
  end

  defp package do
    [
      files: ~w(lib priv mix.exs README.md),
      licenses: ["MIT"],
      links: %{"Spacetime" => "https://spacetime.st"}
    ]
  end

  defp docs do
    [main: "Spacetime.LiveView", extras: ["README.md"]]
  end
end
