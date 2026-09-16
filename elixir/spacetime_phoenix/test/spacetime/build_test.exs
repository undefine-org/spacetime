defmodule Spacetime.BuildTest do
  # NOT async: one test deletes SPACETIME_BIN for its duration, and the env is
  # process-global — a concurrent test reading it would see the hole.
  use ExUnit.Case, async: false

  @moduledoc """
  The build task's contract with the CLI.

  The happy path needs a real `spacetime` binary, so it is tagged and skipped
  unless one is reachable — a suite that silently passes because a tool is
  missing proves nothing. Point `SPACETIME_BIN` at a checkout build to run it:

      SPACETIME_BIN=target/release/spacetime mix test --include cli
  """

  @bin System.get_env("SPACETIME_BIN") || "spacetime"

  describe "configuration" do
    test "no entries is a named failure, not a silent no-op" do
      # A build that quietly does nothing ships the previous bundle forever.
      assert_raise Mix.Error, ~r/no Spacetime entries configured/, fn ->
        in_project([], fn -> Mix.Tasks.Spacetime.Build.run([]) end)
      end
    end

    test "a missing entry names the file" do
      assert_raise Mix.Error, ~r/entry not found: frontend\/absent\.st/, fn ->
        in_project([entries: [{"frontend/absent.st", "out"}]], fn ->
          Mix.Tasks.Spacetime.Build.run([])
        end)
      end
    end

    test "a missing binary is distinguished from a missing entry" do
      # Different causes, different fixes; the raw :enoent names neither.
      #
      # SPACETIME_BIN must not leak in here: the env var overrides the config,
      # so on a machine that HAS a spacetime the fake name would never be used
      # and this test would pass for the wrong reason.
      entry = Path.join(tmp("missing-bin"), "page.st")
      File.write!(entry, ".x { color: red; }\n")

      without_env("SPACETIME_BIN", fn ->
        assert_raise Mix.Error, ~r/spacetime binary not found/, fn ->
          in_project(
            [
              bin: "definitely-not-a-real-spacetime-binary",
              entries: [{entry, tmp("missing-bin-out")}]
            ],
            fn -> Mix.Tasks.Spacetime.Build.run([]) end
          )
        end
      end)
    end
  end

  @tag :cli
  describe "against the real CLI" do
    @tag :cli
    test "compiles an entry to spacetime.js + spacetime.css" do
      unless cli_available?(), do: raise("SPACETIME_BIN not runnable: #{@bin}")

      entry = Path.join(tmp(), "mini.st")
      out = tmp("mini-out")
      File.write!(entry, ".mini { background: #111; color: #eee; }\n")

      in_project([bin: @bin, entries: [{entry, out}]], fn ->
        Mix.Tasks.Spacetime.Build.run([])
      end)

      assert File.exists?(Path.join(out, "spacetime.js"))
      assert File.exists?(Path.join(out, "spacetime.css"))
    end

    @tag :cli
    test "a compile error raises rather than leaving a stale bundle" do
      unless cli_available?(), do: raise("SPACETIME_BIN not runnable: #{@bin}")

      entry = Path.join(tmp(), "broken.st")
      out = tmp("broken-out")
      File.write!(entry, "@import \"stdlib/definitely/not/a/real/module\"\n")

      assert_raise Mix.Error, ~r/spacetime build failed/, fn ->
        in_project([bin: @bin, entries: [{entry, out}]], fn ->
          Mix.Tasks.Spacetime.Build.run([])
        end)
      end
    end
  end

  defp without_env(name, fun) do
    previous = System.get_env(name)
    System.delete_env(name)

    try do
      fun.()
    after
      if previous, do: System.put_env(name, previous)
    end
  end

  defp cli_available? do
    match?({_, 0}, System.cmd(@bin, ["--help"], stderr_to_stdout: true))
  rescue
    _ -> false
  end

  # Run `fun` with a project config carrying `spacetime: opts`.
  defp in_project(opts, fun) do
    Mix.Project.in_project(:probe, tmp(), [spacetime: opts], fn _ -> fun.() end)
  end

  defp tmp(sub \\ "") do
    path = Path.join([System.tmp_dir!(), "spacetime_build_test", sub])
    File.mkdir_p!(path)
    path
  end
end
