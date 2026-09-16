/// Headless Tests: Component Model — @exports Cross-Instance State Access (CMHT-005)
///
/// Tests that @exports metadata is set on template root elements,
/// cross-instance state is readable/writable via named refs, and
/// templates without exports have no __stExports overhead.

@import "stdlib/testing/test"

// =============================================================================
// Template Definitions
// =============================================================================

@template &export-counter() {
    <div class="export-counter">
        <span class="export-counter__value"></span>
        <button class="export-counter__btn">+</button>
    </div>
    $count number: 0;
    @exports { $count }
    @on &.click(.export-counter__btn) { $count <- $count + 1; }
    .export-counter__value { text <- $count; }
}

@template &mut-counter() {
    <div class="mut-counter">
        <span class="mut-counter__value"></span>
    </div>
    $count number: 0;
    @exports { $count: mut }
    .mut-counter__value { text <- $count; }
}

@template &no-export-box() {
    <div class="no-export-box">
        <span class="no-export-box__label">plain</span>
    </div>
    $open bool: false;
}

// =============================================================================
// Test 1: Read Exported State from Outside
// =============================================================================

@test "exported state readable from outside via named ref" {
  @fixture { <div class="host-read"></div> }
  @eval (document.querySelector('.host-read').appendChild(Spacetime.invokeTemplate('export-counter')))
  @eval (ST.set(document.querySelector('.host-read'), 'counterRef', document.querySelector('.host-read .export-counter')))
  @wait 50ms
  @assert (ST.get(ST.get(document.querySelector('.host-read'), 'counterRef'), 'count') === 0)
    "exported count should be readable and equal 0"
}

// =============================================================================
// Test 2: Cross-Instance Read After Mutation
// =============================================================================

@test "cross-instance read reflects mutated state after click" {
  @fixture { <div class="host-mutate"></div> }
  @eval (document.querySelector('.host-mutate').appendChild(Spacetime.invokeTemplate('export-counter')))
  @eval (ST.set(document.querySelector('.host-mutate'), 'counterRef', document.querySelector('.host-mutate .export-counter')))
  @wait 50ms
  @when .export-counter__btn click
  @wait 50ms
  @assert (ST.get(ST.get(document.querySelector('.host-mutate'), 'counterRef'), 'count') === 1)
    "exported count should be 1 after internal click"
}

// =============================================================================
// Test 3: Mutable Export Writable from Outside
// =============================================================================

@test "mutable export writable from outside updates display" {
  @fixture { <div class="host-write"></div> }
  @eval (document.querySelector('.host-write').appendChild(Spacetime.invokeTemplate('mut-counter')))
  @eval (ST.set(document.querySelector('.host-write'), 'counterRef', document.querySelector('.host-write .mut-counter')))
  @wait 50ms
  @eval (ST.set(ST.get(document.querySelector('.host-write'), 'counterRef'), 'count', 99))
  @wait 50ms
  @assert (ST.get(ST.get(document.querySelector('.host-write'), 'counterRef'), 'count') === 99)
    "exported count should be 99 after external write"
  @then .mut-counter__value should have_text "99"
}

// =============================================================================
// Test 4: Template Without @exports Has No __stExports
// =============================================================================

@test "template without exports has no __stExports metadata" {
  @fixture { <div class="host-noexport"></div> }
  @eval (document.querySelector('.host-noexport').appendChild(Spacetime.invokeTemplate('no-export-box')))
  @wait 50ms
  @assert (document.querySelector('.no-export-box').__stExports === undefined)
    "__stExports should be undefined on template without @exports"
}

// =============================================================================
// Test 5: Named Ref Stores Element
// =============================================================================

@test "named ref stores template element accessible via ST.get" {
  @fixture { <div class="host-namedref"></div> }
  @eval (document.querySelector('.host-namedref').appendChild(Spacetime.invokeTemplate('export-counter')))
  @eval (ST.set(document.querySelector('.host-namedref'), 'myRef', document.querySelector('.host-namedref .export-counter')))
  @wait 50ms
  @assert (ST.get(document.querySelector('.host-namedref'), 'myRef') !== null)
    "named ref should resolve to a non-null element"
  @assert (ST.get(document.querySelector('.host-namedref'), 'myRef') !== undefined)
    "named ref should not be undefined"
  @assert (ST.get(document.querySelector('.host-namedref'), 'myRef') === document.querySelector('.host-namedref .export-counter'))
    "named ref should point to the template root element"
}
