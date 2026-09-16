defmodule SpacetimeLvWeb.SignupLive do
  @moduledoc """
  SignupLive — FORMS-ACROSS-THE-SEAM SPIKE (FUP-142).

  The server half of the forms vertical. It proves the *envelope* thesis: a
  form's authoritative state crosses the bridge as DATA — `%{values, errors,
  valid?}` — never as a rendered `<form>`. The `.st` page (`frontend/signup.st`)
  subscribes to the `:form` assign and renders inputs + inline errors from that
  envelope, with zero HEEx and zero page JS.

  NB the spike deliberately validates with a plain function rather than Ecto:
  this app has no Ecto dep, and the point under test is the *envelope shape +
  the bridge path*, not Ecto specifically. In the real feature `form_envelope/1`
  would take an `%Ecto.Changeset{}` and derive `values/errors/valid?` from it;
  the wire shape — and therefore the whole `.st` side — is identical.

  What the spike SETTLES (recorded in FUP-142):
    * The client form compiles from PLAIN `@data subscribe` + existing grammar —
      no new `@data form` macro is *required* to make it work.
    * BUT `@data subscribe` carries no type, so the envelope `@type` is
      decorative and every nested read (`$form.errors.email`) compiles to a
      guarded `try/catch` against a `null` seed. THAT is the seam a `@data form`
      macro would close (typed envelope + `.values/.errors/.valid` accessors +
      a non-null seed). The gap is real but narrow → more spikes, per the FUP.
  """
  use Spacetime.LiveView,
    bundle: "/spacetime/signup/spacetime.js",
    root: ".signup",
    shell: SpacetimeLvWeb.SignupShell.shell()

  # Declared surface — checked against frontend/SignupLive.contract.json (E0928
  # from the .st side). `:form` is a `:changeset`-shaped assign; here we hand-roll
  # the envelope, but the assign TYPE is what a future reconciler would serialise.
  events do
    event(:validate, form: :map)
    event(:submit, form: :map)
  end

  assigns do
    assign(:form, :changeset)
  end

  @empty %{"email" => "", "password" => ""}

  @impl true
  def mount(_params, _session, socket) do
    {:ok, assign(socket, form: envelope(@empty, %{}, submitted: false))}
  end

  # Per-keystroke recast: validate the incoming params, reply with the fresh
  # envelope under a tag the page's `receive to Validated` arm decodes.
  @impl true
  def handle_event("validate", %{"form" => params}, socket) do
    {errors, valid?} = validate(params)
    env = envelope(params, errors, valid?: valid?)
    tag = if valid?, do: "ok", else: "invalid"
    {:reply, %{tag: tag, reply: env}, assign(socket, form: env)}
  end

  def handle_event("submit", %{"form" => params}, socket) do
    case validate(params) do
      {_errors, true} ->
        # A real feature would persist here; the spike just confirms the ok path.
        {:reply, %{tag: "ok", reply: %{email: params["email"]}}, socket}

      {errors, false} ->
        env = envelope(params, errors, valid?: false)
        {:reply, %{tag: "invalid", reply: env}, assign(socket, form: env)}
    end
  end

  # ── validation (stands in for a changeset) ──────────────────────────────────
  defp validate(params) do
    email = Map.get(params, "email", "")
    password = Map.get(params, "password", "")

    errors =
      %{}
      |> put_error(:email, email_error(email))
      |> put_error(:password, password_error(password))

    {errors, map_size(errors) == 0}
  end

  defp email_error(""), do: "can't be blank"

  defp email_error(email) do
    if Regex.match?(~r/^[^@\s]+@[^@\s]+\.[^@\s]+$/, email), do: nil, else: "must be a valid email"
  end

  defp password_error(password) when byte_size(password) < 8, do: "must be at least 8 characters"
  defp password_error(_), do: nil

  defp put_error(errors, _field, nil), do: errors
  defp put_error(errors, field, message), do: Map.put(errors, field, message)

  # The ONLY thing that crosses the wire: values + per-field error strings + valid?.
  # (In the real feature: `form_envelope(%Ecto.Changeset{})`.)
  defp envelope(params, errors, opts) do
    %{
      values: %{
        email: Map.get(params, "email", ""),
        password: Map.get(params, "password", "")
      },
      errors: %{
        email: Map.get(errors, :email, ""),
        password: Map.get(errors, :password, "")
      },
      valid: Keyword.get(opts, :valid?, false),
      submitted: Keyword.get(opts, :submitted, false)
    }
  end
end
