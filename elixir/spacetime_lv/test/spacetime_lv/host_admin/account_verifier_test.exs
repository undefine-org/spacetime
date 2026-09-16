# This reference test demonstrates "mock from API contracts": Mox generates a double from the behaviour, so WorkOS, GitHub commit APIs, and future integrations can be tested without network calls while preserving the explicit contract.
defmodule SpacetimeLv.HostAdmin.AccountVerifierTest do
  use ExUnit.Case, async: true
  import Mox
  alias SpacetimeLv.HostAdmin.MockAccountVerifier
  setup :verify_on_exit!

  test "verifies through the configured contract mock" do
    expect(MockAccountVerifier, :verify_token, fn "token" -> {:ok, "acct_123"} end)
    verifier = Application.get_env(:spacetime_lv, :account_verifier)
    assert {:ok, "acct_123"} = verifier.verify_token("token")
  end
end
