defmodule SpacetimeLv.HostAdmin.AccountVerifierBehaviour do
  @callback verify_token(token :: String.t()) ::
              {:ok, account_id :: String.t()} | {:error, :invalid_token}
end
