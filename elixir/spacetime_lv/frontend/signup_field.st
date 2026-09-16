/* ============================================================================
   signup_field.st — SPIKE 4 (FUP-142): the SAME form as signup.st, but every
   field reduced to a single `@field` / `@field-error` call.

   Compare the field-binding block against signup.st's hand-written version:
   4-6 lines/field (with the stale-read footgun) collapse to one call each, and
   the stale-read bug is impossible by construction (the sugar writes the LOCAL
   $values signal and reads the CURRENT value for siblings, never a server echo).
   ============================================================================ */
@import "stdlib/macros/data-kind"
@import "stdlib/macros/handle"
@import "stdlib/macros/on-event"
@import "./field.st"
@import "./$hosts.st"

@data subscribe $form from $authField : form ;
@data inline $submitted boolean : false ;

/* Local client-authoritative values. Quoted keys: BUG-228 workaround. */
@data inline $values : { "email": "", "password": "" } ;

@data signal $validate($vals map) to $authField {
  send emit "validate" { form: $vals }
  receive to Validated {
    "invalid" => Invalid($.reply);
    _ => Ok($.reply);
  }
  policy latest
}

@data signal $submit($vals map) to $authField {
  send emit "submit" { form: $vals }
  receive to Submitted {
    "ok" => Saved($.reply);
    "invalid" => Rejected($.reply);
    _ => Rejected($.reply);
  }
  policy latest
}

.signup {
  @handle $validate {
    receive {
      Invalid(env) => { $form <- env; }
      Ok(env) => { $form <- env; }
    }
  }
  @handle $submit {
    receive {
      Saved(_) => { $submitted <- true; }
      Rejected(env) => { $form <- env; }
    }
  }
}

/* ── The whole point of SPIKE 4: one call per field. ────────────────────────
   value-bind + local-values write + validate-fire, all encoded — and the
   stale-read is structurally impossible. */
.signup .field-email input      { @field($values, email, on: $validate) }
.signup .field-email .error     { @field-error($form, email) }
.signup .field-password input   { @field($values, password, on: $validate) }
.signup .field-password .error  { @field-error($form, password) }

.signup button.submit { @on click { $submit($values); } }
.signup .success { text: $submitted; }

body {
  margin: 0;
  min-height: 100vh;
  display: grid;
  place-items: center;
  background: radial-gradient(120% 120% at 50% 0%, #1a1f2e 0%, #0a0b0f 70%);
  font-family: "Inter", system-ui, sans-serif;
  color: #e8eaf2;
}
.signup {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: min(22rem, 90vw);
  padding: 2rem;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 1rem;
  background: rgba(255, 255, 255, 0.035);
}
.signup label { font-size: 0.8rem; color: #9fb0d0; }
.signup input {
  width: 100%;
  padding: 0.6rem 0.75rem;
  border-radius: 0.6rem;
  border: 1px solid rgba(255, 255, 255, 0.14);
  background: #141a28;
  color: #e8eaf2;
  font: inherit;
}
.signup .error { min-height: 1rem; font-size: 0.78rem; color: #fe6e8e; }
.signup button.submit {
  padding: 0.7rem;
  border: 0;
  border-radius: 0.6rem;
  background: #6ea8fe;
  color: #0b1220;
  font-weight: 700;
  cursor: pointer;
}
