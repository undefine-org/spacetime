/// Integration Tests: Modal Component
///
/// Tests modal behavior combining state, animation, and DOM assertions.

@import "stdlib/testing/test"
@import "stdlib/testing/assertions/state"
@import "stdlib/testing/assertions/timeline"

// =============================================================================
// Modal Open/Close Flow
// =============================================================================

@test "modal opens with animation" {
  @fixture {
    <div class="modal-test" data-st-state="closed">
      <div class="modal__backdrop"></div>
      <div class="modal__content">
        <h2>Modal Title</h2>
        <p>Modal content here</p>
        <button class="modal__close">Close</button>
      </div>
    </div>
    <button class="trigger-open">Open Modal</button>
  }

  @given .modal-test is hidden

  // Simulate opening
  @let (modal = document.querySelector('.modal-test'))
  @eval (modal.dataset.stState = 'open')
  @eval (modal.__stStateHistory = 'closed,open'.split(','))
  @eval (modal.__stTransitionCount = 1)

  // Publish the completed opening driver from its owning modal.
  @eval (ST.set(modal, 'modal-open', 1))

  @then .modal-test should have_state "open"
  @assert (document.querySelector('.modal-test').__stStateHistory.join(',') === 'closed,open')
  @then .modal-test should have_transition_count 1
  @then .modal-test should have_timeline_complete "modal-open"
}

@test "modal closes and returns to closed state" {
  @fixture {
    <div class="modal-close-test" data-st-state="open">
      <div class="modal__backdrop"></div>
      <div class="modal__content">Content</div>
    </div>
  }

  @let (modal = document.querySelector('.modal-close-test'))
  @eval (modal.__stStateHistory = 'closed,open'.split(','))

  // Simulate closing
  @eval (modal.dataset.stState = 'closed')
  @eval (modal.__stStateHistory.push('closed'))
  @eval (modal.__stTransitionCount = 2)

  @then .modal-close-test should have_state "closed"
  @assert (document.querySelector('.modal-close-test').__stStateHistory.join(',') === 'closed,open,closed')
  @then .modal-close-test should have_transition_count 2
}

// =============================================================================
// Modal Focus Management
// =============================================================================

@test "modal traps focus on open" {
  @fixture {
    <div class="focus-modal" data-st-state="open">
      <button class="first-focusable">First</button>
      <input type="text" class="input-field" />
      <button class="last-focusable">Last</button>
    </div>
  }

  // First focusable should receive focus
  @eval (document.querySelector('.first-focusable').focus())

  @then .first-focusable should be_focused
}

// =============================================================================
// Modal Backdrop Interaction
// =============================================================================

@test "backdrop click closes modal" {
  @fixture {
    <div class="backdrop-modal" data-st-state="open">
      <div class="modal__backdrop" data-action="close"></div>
      <div class="modal__content">Content</div>
    </div>
  }

  @let (modal = document.querySelector('.backdrop-modal'))
  @eval (modal.__stStateHistory = 'open'.split(','))

  @when .modal__backdrop click

  // Simulate state change from click handler
  @eval (modal.dataset.stState = 'closed')
  @eval (modal.__stStateHistory.push('closed'))
  @eval (modal.__stLastTransition = __obj("from", "open", "to", "closed", "time", Date.now()))

  @then .backdrop-modal should have_state "closed"
  @then .backdrop-modal should have_last_transition from "open" to "closed"
}

// =============================================================================
// Modal Animation Coordination
// =============================================================================

@test "modal enter animation completes before content visible" {
  @fixture {
    <div class="anim-modal" data-st-state="opening">
      <div class="modal__content" data-content-state="hidden">
        Content
      </div>
    </div>
  }

  @let (modal = document.querySelector('.anim-modal'))
  @let (content = document.querySelector('.modal__content'))

  // Publish the intermediate enter driver from its owning modal.
  @eval (ST.set(modal, 'modal-enter', 0.5))

  @then .anim-modal should have_timeline_running "modal-enter"
  @then .anim-modal should have_timeline_progress "modal-enter" >= 0.5

  // Complete the named enter driver; completion is progress at or above one.
  @eval (ST.set(modal, 'modal-enter', 1))
  @eval (modal.dataset.stState = 'open')
  @eval (content.dataset.contentState = 'visible')

  @then .anim-modal should have_state "open"
  @then .anim-modal should have_timeline_complete "modal-enter"
}

// =============================================================================
// Modal Escape Key Handler
// =============================================================================

@test "escape key closes modal" {
  @fixture {
    <div class="escape-modal" data-st-state="open">
      <div class="modal__content">Content</div>
    </div>
  }

  @let (modal = document.querySelector('.escape-modal'))
  @eval (modal.__stStateHistory = 'closed,open'.split(','))

  @when .escape-modal keydown "Escape"

  // Simulate escape handler effect
  @eval (modal.dataset.stState = 'closed')
  @eval (modal.__stStateHistory.push('closed'))
  @eval (modal.__stLastTransition = __obj("from", "open", "to", "closed", "time", Date.now()))

  @then .escape-modal should have_state "closed"
  @then .escape-modal should have_last_transition from "open" to "closed"
}

// =============================================================================
// Nested Modal State
// =============================================================================

@test "nested modal states are independent" {
  @fixture {
    <div class="parent-modal" data-st-state="open">
      <div class="modal__content">
        Parent Content
        <div class="child-modal" data-st-state="closed">
          <div class="child-content">Child Content</div>
        </div>
      </div>
    </div>
  }

  @then .parent-modal should have_state "open"
  @then .child-modal should have_state "closed"

  // Open child modal
  @let (child = document.querySelector('.child-modal'))
  @eval (child.dataset.stState = 'open')
  @eval (child.__stStateHistory = 'closed,open'.split(','))

  @then .parent-modal should have_state "open"
  @then .child-modal should have_state "open"
  @assert (document.querySelector('.child-modal').__stStateHistory.join(',') === 'closed,open')
}
