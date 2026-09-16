/// Headless Tests: Body Directives on @each-created Template Instances
///
/// ITEM-BUG-body-directives-002
///
/// Verifies that every body directive type works correctly on template
/// instances created via Spacetime.invokeTemplate (the same path @each uses):
///   - State initialization ($var type: value;)
///   - Click events (@on click { $var <- !$var; })
///   - Class toggles (.class-name: $var;)
///   - Content bindings (text <- $var;)

@import "stdlib/testing/test"

// =============================================================================
// Template Definitions — each exercises a specific body directive
// =============================================================================

// Template with all four directive types combined (mirrors Pulsar pattern)
@template &directive-card() {
    <article class="dir-card">
        <span class="dir-card__label"></span>
        <button class="dir-card__btn">Toggle</button>
    </article>
    $active bool: false;
    @on &.click(.dir-card__btn) { $active <- !$active; }
    .dir-card--active: $active;
    .dir-card__label { text <- $active; }
}

// Minimal template: state init only
@template &state-only() {
    <div class="state-only-el"></div>
    $count number: 42;
    $label string: "hello";
}

// Minimal template: click event only
@template &click-only() {
    <button class="click-only-el">Click</button>
    $toggled bool: false;
    @on &.click { $toggled <- !$toggled; }
}

// Minimal template: class toggle only
@template &class-toggle-only() {
    <div class="cls-toggle-el"></div>
    $highlighted bool: false;
    .cls-toggle-el--on: $highlighted;
}

// Minimal template: content binding only
@template &content-bind-only() {
    <div class="content-el">
        <span class="content-el__text"></span>
    </div>
    $msg string: "initial";
    .content-el__text { text <- $msg; }
}

// =============================================================================
// Test 1: State Initialization per Instance
// =============================================================================

@test "template instance initializes body state variables" {
  @fixture {
    <div class="host-state-a"></div>
    <div class="host-state-b"></div>
  }
  @eval (document.querySelector('.host-state-a').appendChild(Spacetime.invokeTemplate('state-only')))
  @eval (document.querySelector('.host-state-b').appendChild(Spacetime.invokeTemplate('state-only')))
  @wait 50ms
  @assert (ST.get(document.querySelector('.host-state-a .state-only-el'), 'count') === 42)
    "instance A: count should initialize to 42"
  @assert (ST.get(document.querySelector('.host-state-a .state-only-el'), 'label') === 'hello')
    "instance A: label should initialize to 'hello'"
  @assert (ST.get(document.querySelector('.host-state-b .state-only-el'), 'count') === 42)
    "instance B: count should initialize to 42"
  @assert (ST.get(document.querySelector('.host-state-b .state-only-el'), 'label') === 'hello')
    "instance B: label should initialize to 'hello'"
}

@test "template instances have independent state" {
  @fixture {
    <div class="host-indep-a"></div>
    <div class="host-indep-b"></div>
  }
  @eval (document.querySelector('.host-indep-a').appendChild(Spacetime.invokeTemplate('state-only')))
  @eval (document.querySelector('.host-indep-b').appendChild(Spacetime.invokeTemplate('state-only')))
  @wait 50ms
  // Mutate instance A only
  @eval (ST.set(document.querySelector('.host-indep-a .state-only-el'), 'count', 99))
  @wait 50ms
  @assert (ST.get(document.querySelector('.host-indep-a .state-only-el'), 'count') === 99)
    "instance A: count should be mutated to 99"
  @assert (ST.get(document.querySelector('.host-indep-b .state-only-el'), 'count') === 42)
    "instance B: count should remain at 42"
}

// =============================================================================
// Test 2: Click Event -> State Toggle
// =============================================================================

@test "click event toggles state on template instance" {
  @fixture { <div class="host-click"></div> }
  @eval (document.querySelector('.host-click').appendChild(Spacetime.invokeTemplate('click-only')))
  @wait 50ms
  @assert (ST.get(document.querySelector('.click-only-el'), 'toggled') === false)
    "toggled should start false"
  @eval (document.querySelector('.click-only-el').dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true})))
  @wait 50ms
  @assert (ST.get(document.querySelector('.click-only-el'), 'toggled') === true)
    "toggled should be true after click"
}

@test "click event double-toggle returns to original" {
  @fixture { <div class="host-dblclick"></div> }
  @eval (document.querySelector('.host-dblclick').appendChild(Spacetime.invokeTemplate('click-only')))
  @wait 50ms
  @eval (document.querySelector('.click-only-el').dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true})))
  @wait 50ms
  @eval (document.querySelector('.click-only-el').dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true})))
  @wait 50ms
  @assert (ST.get(document.querySelector('.click-only-el'), 'toggled') === false)
    "toggled should return to false after double click"
}

@test "click events are independent across instances" {
  @fixture {
    <div class="host-click-a"></div>
    <div class="host-click-b"></div>
  }
  @eval (document.querySelector('.host-click-a').appendChild(Spacetime.invokeTemplate('click-only')))
  @eval (document.querySelector('.host-click-b').appendChild(Spacetime.invokeTemplate('click-only')))
  @wait 50ms
  // Click only instance A
  @eval (document.querySelector('.host-click-a .click-only-el').dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true})))
  @wait 50ms
  @assert (ST.get(document.querySelector('.host-click-a .click-only-el'), 'toggled') === true)
    "instance A: toggled should be true after click"
  @assert (ST.get(document.querySelector('.host-click-b .click-only-el'), 'toggled') === false)
    "instance B: toggled should remain false"
}

// =============================================================================
// Test 3: Class Toggle Reacts to State
// =============================================================================

@test "class toggle adds class when state becomes true" {
  @fixture { <div class="host-cls"></div> }
  @eval (document.querySelector('.host-cls').appendChild(Spacetime.invokeTemplate('class-toggle-only')))
  @wait 50ms
  @assert (!document.querySelector('.cls-toggle-el').classList.contains('cls-toggle-el--on'))
    "class should not be present initially"
  @eval (ST.set(document.querySelector('.cls-toggle-el'), 'highlighted', true))
  @wait 50ms
  @assert (document.querySelector('.cls-toggle-el').classList.contains('cls-toggle-el--on'))
    "class should be added when state becomes true"
}

@test "class toggle removes class when state becomes false" {
  @fixture { <div class="host-cls-off"></div> }
  @eval (document.querySelector('.host-cls-off').appendChild(Spacetime.invokeTemplate('class-toggle-only')))
  @wait 50ms
  @eval (ST.set(document.querySelector('.cls-toggle-el'), 'highlighted', true))
  @wait 50ms
  @eval (ST.set(document.querySelector('.cls-toggle-el'), 'highlighted', false))
  @wait 50ms
  @assert (!document.querySelector('.cls-toggle-el').classList.contains('cls-toggle-el--on'))
    "class should be removed when state becomes false"
}

// =============================================================================
// Test 4: Content Binding Updates Text
// =============================================================================

@test "content binding sets initial text" {
  @fixture { <div class="host-content"></div> }
  @eval (document.querySelector('.host-content').appendChild(Spacetime.invokeTemplate('content-bind-only')))
  @wait 50ms
  @assert (document.querySelector('.content-el__text').textContent === 'initial')
    "text should display initial value"
}

@test "content binding updates when state changes" {
  @fixture { <div class="host-content-upd"></div> }
  @eval (document.querySelector('.host-content-upd').appendChild(Spacetime.invokeTemplate('content-bind-only')))
  @wait 50ms
  @eval (ST.set(document.querySelector('.content-el'), 'msg', 'updated'))
  @wait 50ms
  @assert (document.querySelector('.content-el__text').textContent === 'updated')
    "text should update when state changes"
}

// =============================================================================
// Test 5: Combined Directives (Pulsar-like pattern)
// =============================================================================

@test "combined directives: click toggles class and updates content" {
  @fixture { <div class="host-combined"></div> }
  @eval (document.querySelector('.host-combined').appendChild(Spacetime.invokeTemplate('directive-card')))
  @wait 50ms
  // Initial state
  @assert (ST.get(document.querySelector('.dir-card'), 'active') === false)
    "active should start false"
  @assert (!document.querySelector('.dir-card').classList.contains('dir-card--active'))
    "active class should not be present initially"
  // Click the button
  @eval (document.querySelector('.dir-card__btn').dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true})))
  @wait 50ms
  // After click
  @assert (ST.get(document.querySelector('.dir-card'), 'active') === true)
    "active should be true after click"
  @assert (document.querySelector('.dir-card').classList.contains('dir-card--active'))
    "active class should be present after click"
  @assert (document.querySelector('.dir-card__label').textContent === 'true')
    "label should display 'true' after click"
}

@test "combined directives: two instances are fully independent" {
  @fixture {
    <div class="host-combo-a"></div>
    <div class="host-combo-b"></div>
  }
  @eval (document.querySelector('.host-combo-a').appendChild(Spacetime.invokeTemplate('directive-card')))
  @eval (document.querySelector('.host-combo-b').appendChild(Spacetime.invokeTemplate('directive-card')))
  @wait 50ms
  // Click only instance A's button
  @eval (document.querySelector('.host-combo-a .dir-card__btn').dispatchEvent(new MouseEvent('click',{bubbles:true,cancelable:true})))
  @wait 50ms
  // Instance A should be active
  @assert (ST.get(document.querySelector('.host-combo-a .dir-card'), 'active') === true)
    "instance A: active should be true"
  @assert (document.querySelector('.host-combo-a .dir-card').classList.contains('dir-card--active'))
    "instance A: should have active class"
  // Instance B should remain unchanged
  @assert (ST.get(document.querySelector('.host-combo-b .dir-card'), 'active') === false)
    "instance B: active should remain false"
  @assert (!document.querySelector('.host-combo-b .dir-card').classList.contains('dir-card--active'))
    "instance B: should not have active class"
}
