defmodule SpacetimeLvWeb.Presence do
  @moduledoc false

  use Phoenix.Presence,
    otp_app: :spacetime_lv,
    pubsub_server: SpacetimeLv.PubSub
end
