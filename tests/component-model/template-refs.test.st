/// Headless Tests: Component Model — Template Ref Resolution (CMHT-004)
///
/// Tests that @template definitions compile to factories, template invocations
/// insert HTML into the DOM, multiple instances maintain independent state,
/// named refs store elements via ST.set, and nested template invocations render.

@import "stdlib/testing/test"

// =============================================================================
// Template Definitions (compiled at file scope → registered factories)
// =============================================================================

@template &simple-badge() {
    <span class="simple-badge">Badge</span>
}

@template &click-counter() {
    <div class="click-counter">
        <span class="click-counter__value"></span>
        <button class="click-counter__btn">+1</button>
    </div>
    $count number: 0;
    @on &.click(.click-counter__btn) { $count <- $count + 1; }
    .click-counter__value { text <- $count; }
}

@template &inner-tag() {
    <em class="inner-tag">inner</em>
}

@template &outer-card() {
    <div class="outer-card">
        <span class="outer-card__title">Outer</span>
    </div>
    &inner-tag();
}

@template &card-with-ref() {
    <div class="card-with-ref">
        <span class="card-with-ref__label">Card</span>
    </div>
    &badgeRef &simple-badge();
}

@template &multi-var() {
    <div class="multi-var">
        <span class="multi-var__name"></span>
        <span class="multi-var__score"></span>
    </div>
    $name string: "untitled";
    $score number: 100;
    $enabled bool: true;
    .multi-var__name { text <- $name; }
    .multi-var__score { text <- $score; }
    .multi-var--enabled: $enabled;
}

// =============================================================================
// Test 1: Anonymous Template Invocation
// =============================================================================

@test "anonymous template invocation inserts HTML into DOM" {
  @fixture { <div class="host-anon"></div> }
  @eval (document.querySelector('.host-anon').appendChild(Spacetime.invokeTemplate('simple-badge')))
  @wait 50ms
  @then .simple-badge should exist
  @then .simple-badge should have_text "Badge"
}

// =============================================================================
// Test 2: Multiple Instances with Independent State
// =============================================================================

@test "multiple template instances have independent state" {
  @fixture {
    <div class="host-a"></div>
    <div class="host-b"></div>
  }
  @eval (document.querySelector('.host-a').appendChild(Spacetime.invokeTemplate('click-counter')))
  @eval (document.querySelector('.host-b').appendChild(Spacetime.invokeTemplate('click-counter')))
  @wait 50ms
  // Both should initialize to 0
  @assert (ST.get(document.querySelector('.host-a .click-counter'), 'count') === 0)
    "instance A count should start at 0"
  @assert (ST.get(document.querySelector('.host-b .click-counter'), 'count') === 0)
    "instance B count should start at 0"
  // Click button in instance A only
  @eval (document.querySelector('.host-a .click-counter__btn').click())
  @wait 50ms
  // Instance A should increment, instance B should remain 0
  @assert (ST.get(document.querySelector('.host-a .click-counter'), 'count') === 1)
    "instance A count should be 1 after click"
  @assert (ST.get(document.querySelector('.host-b .click-counter'), 'count') === 0)
    "instance B count should remain 0"
}

// =============================================================================
// Test 3: Nested Template Invocation
// =============================================================================

@test "nested template invocation renders both outer and inner" {
  @fixture { <div class="host-nested"></div> }
  @eval (document.querySelector('.host-nested').appendChild(Spacetime.invokeTemplate('outer-card')))
  @wait 50ms
  @then .outer-card should exist
  @then .outer-card__title should have_text "Outer"
  @then .inner-tag should exist
  @then .inner-tag should have_text "inner"
}

// =============================================================================
// Test 4: Named Ref DOM Insertion
// =============================================================================

@test "named template ref inserts child and stores reference" {
  @fixture { <div class="host-ref"></div> }
  @eval (document.querySelector('.host-ref').appendChild(Spacetime.invokeTemplate('card-with-ref')))
  @wait 50ms
  // Child template HTML should be inserted
  @then .card-with-ref should exist
  @then .simple-badge should exist
  // Named ref should be accessible via ST.get
  @assert (ST.get(document.querySelector('.card-with-ref'), 'badgeRef') !== null)
    "badgeRef should be stored on parent element"
  @assert (ST.get(document.querySelector('.card-with-ref'), 'badgeRef') !== undefined)
    "badgeRef should not be undefined"
}

// =============================================================================
// Test 5: Multiple State Variables in One Template
// =============================================================================

@test "template with multiple state vars initializes all correctly" {
  @fixture { <div class="host-multi"></div> }
  @eval (document.querySelector('.host-multi').appendChild(Spacetime.invokeTemplate('multi-var')))
  @wait 50ms
  @assert (ST.get(document.querySelector('.multi-var'), 'name') === 'untitled')
    "name should initialize to 'untitled'"
  @assert (ST.get(document.querySelector('.multi-var'), 'score') === 100)
    "score should initialize to 100"
  @assert (ST.get(document.querySelector('.multi-var'), 'enabled') === true)
    "enabled should initialize to true"
  @then .multi-var__name should have_text "untitled"
  @then .multi-var__score should have_text "100"
  @then .multi-var should have_class "multi-var--enabled"
}
