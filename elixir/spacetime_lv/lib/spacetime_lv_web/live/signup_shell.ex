defmodule SpacetimeLvWeb.SignupShell do
  @moduledoc """
  The skeleton both signup spikes hydrate.

  `SignupLive` (hand-written form) and `SignupFieldLive` (the `@field` sugar)
  exist to A/B two ways of writing the SAME page, so they render the same
  markup by definition — that is the point of the comparison. One module holds
  it, because two copies of the thing being A/B'd would make the A/B
  meaningless the first time one drifted.

  This lived in a `cond` arm inside `Spacetime.LiveView.render_host/1` and moved
  out when the bridge became a library. FUP-121 replaces it with the compiler's
  own emission from the `.st` page.
  """

  @shell ~s|<div class="signup">
  <div class="field-email">
    <label>Email</label>
    <input type="email" aria-label="Email" />
    <div class="error"></div>
  </div>
  <div class="field-password">
    <label>Password</label>
    <input type="password" aria-label="Password" />
    <div class="error"></div>
  </div>
  <button class="submit" type="button">Create account</button>
  <div class="success"></div>
</div>|

  @doc "The markup `frontend/signup.st` and `frontend/signup_field.st` bind onto."
  def shell, do: @shell
end
