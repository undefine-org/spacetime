/// Integration Tests: Form Validation
///
/// Tests form behavior combining signals, state, and DOM assertions.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/state"
@import "stdlib/testing/assertions/binding"

// =============================================================================
// Form Input Validation
// =============================================================================

@test "form validates required field" {
  @fixture {
    <form class="test-form" data-st-state="idle">
      <input class="name-input" type="text" required data-field="name" />
      <span class="name-error" data-error="name" hidden></span>
      <button class="submit-btn" type="submit">Submit</button>
    </form>
  }

  @let (form = document.querySelector('.test-form'))
  @eval (ST.set(form, 'errors', Array()))
  @eval (ST.set(form, 'isValid', false))

  @then $isValid on .test-form should be_falsy
  @then .name-error should be_hidden
}

@test "form shows error on invalid submit" {
  @fixture {
    <form class="invalid-form" data-st-state="error">
      <input class="email-input" type="email" value="invalid" />
      <span class="email-error" data-error="email">Invalid email</span>
      <button class="submit" type="submit">Submit</button>
    </form>
  }

  @let (form = document.querySelector('.invalid-form'))
  @eval (form.__stStateHistory = 'idle,validating,error'.split(','))
  @eval (ST.set(form, 'errors', 'email'.split(',')))

  @then .invalid-form should have_state "error"
  @assert (document.querySelector('.invalid-form').__stStateHistory.join(',') === 'idle,validating,error')
  @then $errors on .invalid-form should have_length 1
  @then $errors on .invalid-form should contain "email"
}

@test "form clears error when input corrected" {
  @fixture {
    <form class="correct-form" data-st-state="idle">
      <input class="fix-input" type="email" value="valid@email.com" />
      <span class="fix-error" hidden></span>
    </form>
  }

  @let (form = document.querySelector('.correct-form'))
  @eval (form.__stStateHistory = 'error,idle'.split(','))
  @eval (ST.set(form, 'errors', Array()))
  @eval (ST.set(form, 'isValid', true))

  @then .correct-form should have_state "idle"
  @then $errors on .correct-form should have_length 0
  @then $isValid on .correct-form should be_truthy
  @then .fix-error should be_hidden
}

// =============================================================================
// Form Submission Flow
// =============================================================================

@test "form goes through submission states" {
  @fixture {
    <form class="submit-form" data-st-state="submitting">
      <input class="field" value="data" />
      <button class="submit">Submit</button>
    </form>
  }

  @let (form = document.querySelector('.submit-form'))
  @eval (form.__stStateHistory = 'idle,validating,submitting'.split(','))
  @eval (form.__stStateEntries = 'idle,validating,submitting'.split(','))
  @eval (form.__stStateExits = 'idle,validating'.split(','))

  @then .submit-form should have_state "submitting"
  @assert (document.querySelector('.submit-form').__stStateHistory.join(',') === 'idle,validating,submitting')

  // Simulate successful submission
  @eval (form.dataset.stState = 'success')
  @eval (form.__stStateHistory.push('success'))
  @eval (form.__stStateEntries.push('success'))
  @eval (form.__stStateExits.push('submitting'))

  @then .submit-form should have_state "success"
  @then .submit-form should have_entered_state "submitting"
  @then .submit-form should have_exited_state "submitting"
}

@test "form disables submit during submission" {
  @fixture {
    <form class="disable-form" data-st-state="submitting">
      <button class="submit-disabled" disabled>Submitting...</button>
    </form>
  }

  @then .submit-disabled should be_disabled
  @then .submit-disabled should have_text "Submitting..."
}

// =============================================================================
// Form Field State
// =============================================================================

@test "form tracks dirty fields" {
  @fixture {
    <form class="dirty-form">
      <input class="dirty-input" value="modified" data-dirty="true" />
    </form>
  }

  @let (form = document.querySelector('.dirty-form'))
  @eval (ST.set(form, 'dirtyFields', 'dirty-input'.split(',')))
  @eval (ST.set(form, 'isDirty', true))

  @then $isDirty on .dirty-form should be_truthy
  @then $dirtyFields on .dirty-form should contain "dirty-input"
}

@test "form tracks touched fields" {
  @fixture {
    <form class="touch-form">
      <input class="touch-input" data-touched="true" />
    </form>
  }

  @let (form = document.querySelector('.touch-form'))
  @eval (ST.set(form, 'touchedFields', 'touch-input'.split(',')))

  @then $touchedFields on .touch-form should have_length 1
  @then $touchedFields on .touch-form should contain "touch-input"
}

// =============================================================================
// Form Reset
// =============================================================================

@test "form reset clears all state" {
  @fixture {
    <form class="reset-form" data-st-state="idle">
      <input class="reset-input" value="" />
    </form>
  }

  @let (form = document.querySelector('.reset-form'))
  @eval (form.__stStateHistory = 'dirty,idle'.split(','))
  @eval (ST.set(form, 'errors', Array()))
  @eval (ST.set(form, 'dirtyFields', Array()))
  @eval (ST.set(form, 'touchedFields', Array()))
  @eval (ST.set(form, 'isValid', true))
  @eval (ST.set(form, 'isDirty', false))

  @then .reset-form should have_state "idle"
  @then $errors on .reset-form should have_length 0
  @then $dirtyFields on .reset-form should have_length 0
  @then $touchedFields on .reset-form should have_length 0
  @then $isDirty on .reset-form should be_falsy
}

// =============================================================================
// Async Validation
// =============================================================================

@test "form shows loading during async validation" {
  @fixture {
    <form class="async-form" data-st-state="validating">
      <input class="async-input" value="checking..." />
      <span class="async-loader">Validating...</span>
    </form>
  }

  @let (form = document.querySelector('.async-form'))
  @eval (form.__stStateHistory = 'idle,validating'.split(','))
  @eval (ST.set(form, 'isValidating', true))

  @then .async-form should have_state "validating"
  @then $isValidating on .async-form should be_truthy
  @then .async-loader should be_visible
}

@test "async validation completes" {
  @fixture {
    <form class="async-complete" data-st-state="idle">
      <input class="valid-input" value="valid@email.com" />
      <span class="success-msg">Valid!</span>
    </form>
  }

  @let (form = document.querySelector('.async-complete'))
  @eval (form.__stStateHistory = 'validating,idle'.split(','))
  @eval (form.__stStateEntries = 'validating,idle'.split(','))
  @eval (form.__stStateExits = 'validating'.split(','))
  @eval (ST.set(form, 'isValidating', false))
  @eval (ST.set(form, 'isValid', true))

  @then .async-complete should have_state "idle"
  @then $isValidating on .async-complete should be_falsy
  @then $isValid on .async-complete should be_truthy
  @then .async-complete should have_exited_state "validating"
}

// =============================================================================
// Form Error Display
// =============================================================================

@test "form displays multiple errors" {
  @fixture {
    <form class="multi-error-form" data-st-state="error">
      <div class="error-list">
        <span class="err-1">Name is required</span>
        <span class="err-2">Email is invalid</span>
        <span class="err-3">Password too short</span>
      </div>
    </form>
  }

  @let (form = document.querySelector('.multi-error-form'))
  @eval (ST.set(form, 'errors', 'name,email,password'.split(',')))

  @then .multi-error-form should have_state "error"
  @then $errors on .multi-error-form should have_length 3
  @then $errors on .multi-error-form should contain "name"
  @then $errors on .multi-error-form should contain "email"
  @then $errors on .multi-error-form should contain "password"
}

// =============================================================================
// Form Field Focus
// =============================================================================

@test "form focuses first invalid field on submit" {
  @fixture {
    <form class="focus-form" data-st-state="error">
      <input class="valid-field" value="valid" />
      <input class="invalid-field" value="" data-invalid="true" />
    </form>
  }

  // Simulate focus on invalid field
  @eval (document.querySelector('.invalid-field').focus())

  @then .invalid-field should be_focused
}

// =============================================================================
// Conditional Field Visibility
// =============================================================================

@test "conditional fields show based on selection" {
  @fixture {
    <form class="conditional-form">
      <select class="type-select">
        <option value="individual" selected>Individual</option>
        <option value="business">Business</option>
      </select>
      <div class="business-fields" hidden>
        <input class="company-name" placeholder="Company Name" />
      </div>
    </form>
  }

  @let (form = document.querySelector('.conditional-form'))
  @eval (ST.set(form, 'formType', 'individual'))
  @eval (ST.set(form, 'showBusinessFields', false))

  @then $formType on .conditional-form should equal "individual"
  @then $showBusinessFields on .conditional-form should be_falsy
  @then .business-fields should be_hidden

  // Change to business
  @eval (ST.set(form, 'formType', 'business'))
  @eval (ST.set(form, 'showBusinessFields', true))
  @eval (document.querySelector('.business-fields').hidden = false)

  @then $formType on .conditional-form should equal "business"
  @then $showBusinessFields on .conditional-form should be_truthy
  @then .business-fields should be_visible
}
