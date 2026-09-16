# `:cli` tests shell out to a real `spacetime` binary. They are excluded by
# default so a checkout without one still gets a meaningful green suite, and
# INCLUDED automatically when SPACETIME_BIN names a runnable binary — a test
# that silently skips because a tool is missing proves nothing, but a suite
# that fails for the same reason teaches you to ignore it.
bin = System.get_env("SPACETIME_BIN")

cli? =
  is_binary(bin) and
    match?({_, 0}, System.cmd(bin, ["--help"], stderr_to_stdout: true))

if cli? do
  ExUnit.start()
else
  IO.puts(
    :stderr,
    "note: skipping :cli tests — set SPACETIME_BIN to a spacetime binary to run them"
  )

  ExUnit.start(exclude: [:cli])
end
