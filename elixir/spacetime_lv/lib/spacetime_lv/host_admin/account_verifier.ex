defmodule SpacetimeLv.HostAdmin.AccountVerifier do
  @behaviour SpacetimeLv.HostAdmin.AccountVerifierBehaviour
  @moduledoc "Reference implementation: future Rust host endpoint; Req is the preferred HTTP client."
  @impl true
  def verify_token(token) do
    endpoint =
      Application.get_env(
        :spacetime_lv,
        :host_token_endpoint,
        "http://localhost:4000/api/tokens/verify"
      )

    case Req.post(endpoint, json: %{token: token}) do
      {:ok, %{status: 200, body: %{"account_id" => account_id}}} -> {:ok, account_id}
      _ -> {:error, :invalid_token}
    end
  end
end
