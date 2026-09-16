/* ============================================================================
   signup.st — FORMS-ACROSS-THE-SEAM SPIKE (FUP-142).

   Goal of this spike: pressure-test the forms syntax against the REAL compiler
   BEFORE inventing a `@data form` macro. This VARIANT deliberately uses NOTHING
   new — plain `@data subscribe` for the changeset envelope + an `@type` to name
   its shape + the established `value <- $x` / `@on input` binding + `@data
   signal` round-trips + `@handle` continuity.

   If this compiles and expresses the whole form cleanly, then `@data form`
   does NOT earn its place (elegance bar) and the FUP's answer is "no new macro,
   just a stdlib `field()` sugar at most". If a real seam shows up (nested
   access in bindings, envelope typing, field checking) that plain subscribe
   can't carry, THAT gap is what justifies `@data form`.
   ============================================================================ */
@import "stdlib/macros/data-kind"
@import "stdlib/macros/handle"
@import "stdlib/macros/on-event"
@import "./$hosts.st"

/* The changeset envelope shape the server serialises to. This is the shared
   schema — the ONLY thing that crosses the wire (never a rendered <form>). */
@type FormErrors {
  email: string;
  password: string;
}
@type FormValues {
  email: string;
  password: string;
}
@type FormEnvelope {
  values: FormValues;
  errors: FormErrors;
  valid: boolean;
}

/* The changeset-typed assign arrives as a normal subscribed signal, NOW TYPED
   (FUP-144): `$form` carries `FormEnvelope`, so the envelope @types are
   REFERENCED (no W0203 "defined but never used") and the binding is registered
   with its shape — the substrate a future compile-time field-name check reads.
   The reconciler (assign->signal, FEAT-159) streams assign diffs; a changeset
   serialises to {values, errors, valid} first. Still NO new macro — just the
   sibling-parity type slot every other @data kind already had. */
@data subscribe $form FormEnvelope from $auth : form ;

/* Local-only view state the server never owns. */
@data inline $submitted boolean : false ;

/* Live per-keystroke validation + submit are ORDINARY signals (existing
   grammar). The whole values map crosses so the server recasts the changeset. */
@data signal $validate($values map) to $auth {
  send emit "validate" { form: $values }
  receive to Validated {
    "invalid" => Invalid($.reply);
    _ => Ok($.reply);
  }
  policy latest
}

@data signal $submit($values map) to $auth {
  send emit "submit" { form: $values }
  receive to Submitted {
    "ok" => Saved($.reply);
    "invalid" => Rejected($.reply);
    _ => Rejected($.reply);
  }
  policy latest
}

.signup {
  /* @handle is the single continuity surface — server-authoritative envelope
     replaces the local one; no second mutation vocabulary. */
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

/* Field bindings reuse the ESTABLISHED two-way pattern. Nested property access
   ($form.values.email, $form.errors.email) is the real thing under test. */
.signup .field-email input {
  value <- $form.values.email;
  @on input { $validate({ email: $.value, password: $form.values.password }); }
}
.signup .field-email .error { text: $form.errors.email; }

.signup .field-password input {
  value <- $form.values.password;
  @on input { $validate({ email: $form.values.email, password: $.value }); }
}
.signup .field-password .error { text: $form.errors.password; }

.signup button.submit {
  @on click { $submit($form.values); }
}
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
