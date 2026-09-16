defmodule SpacetimeLvWeb.SignupFieldLive do
  @moduledoc """
  SPIKE 4 (FUP-142): identical server behaviour to SignupLive, but hosts the
  `@field`-sugar view (frontend/signup_field.st). Exists only to A/B the sugar
  against the hand-written form in a live browser. Not production.

  NB it declares `SpacetimeLvWeb.SignupLive` as its bundle host implicitly via
  the .st page's `@host $auth : live("SpacetimeLvWeb.SignupLive")` — so the
  page's `send emit` still targets SignupLive's contract. To keep the wire
  self-consistent we simply reuse SignupLive's event handlers here by delegation.
  """
  use Spacetime.LiveView,
    bundle: "/spacetime/signup_field/spacetime.js",
    root: ".signup",
    # The same skeleton SignupLive hydrates — these two spikes A/B two ways of
    # writing one page, so a second copy would defeat the comparison.
    shell: SpacetimeLvWeb.SignupShell.shell()

  alias SpacetimeLvWeb.SignupLive

  events do
    event(:validate, form: :map)
    event(:submit, form: :map)
  end

  assigns do
    assign(:form, :changeset)
  end

  @impl true
  defdelegate mount(params, session, socket), to: SignupLive

  @impl true
  defdelegate handle_event(event, params, socket), to: SignupLive
end
