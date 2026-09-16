/// BUG: `expr | filterName(args)` is silently ignored inside `@template` /
/// `@each` body interpolation — it leaks as a LITERAL trailing text node
/// instead of being applied. The same filter syntax works correctly through
/// `text <-` derived bindings (those route through `ST.filters`); template
/// body interpolation does not.
///
/// Root cause: two independent interpolation emitters exist for the same
/// `` `expr` | filter(args) `` grammar. `text <-` bindings correctly call
/// `ST.filter("currency(\"$\")", __v)` before writing `textContent`. Template
/// slot/text interpolation instead emits the bound VALUE node, then appends a
/// SEPARATE `document.createTextNode(" | currency(\"$\")")` sibling — the
/// pipe expression is treated as trailing literal text, never parsed as a
/// filter call. Confirmed via `cargo run -- inspect --layer emit
/// examples/data-binding/styles.st`: the `<span slot="price">` builder emits
///   `__n22.appendChild(__t25)` where `__t25 = document.createTextNode(" |
///   currency(\"$\")")` — a raw sibling text node, not a filter application.
///
/// Impact: a website visitor sees literal `2400 | currency("$")` instead of
/// `$2,400`. Silent — no compile error, no warning. The shipped
/// `examples/data-binding/` demo exercises exactly this broken path, so
/// anyone copying the official example inherits the bug.
///
/// This test PINS the correct behavior (filter applied, no literal ` | name`
/// suffix in the rendered DOM) across the two interpolation paths that accept
/// a filter pipe in a template body: an @template slot (via ST.resolve(...,'p')
/// scoped binding) and a bare @each (non-@template) inline row body, contrasted
/// against the ALREADY-CORRECT `text <-` derived-binding path as a control.
/// Both @template/@each cases below are expected to go RED until the
/// template-body interpolation emitter is rewired through `ST.filters` (see
/// BUG-188.org).

@import "stdlib/testing/test"

@data inline $price number : 2400;
@data inline $eachItems : [];

// @template slot form: params arrive via ST.resolve(__node, 'p') — mirrors
// examples/data-binding/styles.st's &product-card exactly (price/currency,
// category/uppercase).
@template &price-card($p) {
    <div class="price-card">
        <span class="price-slot">`$p.price` | currency("$")</span>
        <span class="upper-slot">`$p.category` | uppercase</span>
    </div>
}

@template &price-list() {
    <ul class="price-each-list"></ul>

    .price-each-list {
        @each($eachItems as $p) {
            &price-card($p);
        }
    }
}

// Separate data + template for the uppercase test: re-invoking 'price-list'
// in a second host leaves its @each bound to the FIRST instance's updates
// (a second invokeTemplate of the same template does not re-render on a
// later global setData — filed as FUP-119), so each test gets its own list.
@data inline $upperItems : [];

@template &upper-card($p) {
    <span class="upper-slot">`$p.category` | uppercase</span>
}

@template &upper-list() {
    <ul class="upper-each-list"></ul>

    .upper-each-list {
        @each($upperItems as $p) {
            &upper-card($p);
        }
    }
}

// Bare @each (no @template) row body with a filter pipe on interpolated
// text — same grammar, the each-inline (non-template) codegen path, to pin
// whether the leak is template-specific or shared across all @each bodies.
@template &raw-each-list() {
    <ul class="raw-each-list"></ul>

    .raw-each-list {
        @each($eachItems as $p) {
            <li class="raw-price">`$p.price` | currency("$")</li>
        }
    }
}

// Control: `text <-` derived binding — the CONFIRMED-WORKING path (routes
// through ST.filter). Must stay green; a regression here would mean the fix
// broke the already-correct path.
.cart-total-control {
    text <- $price | currency("$");
}

@test "CONTROL: text <- binding applies the filter, no literal pipe text (must stay green)" {
    @fixture {
        <div class="control-host">
            <div class="cart-total-control">x</div>
        </div>
    }
    @wait 60ms
    @let (t = document.querySelector('.cart-total-control').textContent)
    @assert (t.indexOf('|') === -1) "control: no literal pipe char in text <- output (got: "
    @assert (t.indexOf('currency') === -1) "control: no literal 'currency' text in text <- output (got: "
    @assert (t.indexOf('$2,400') !== -1 || t.indexOf('2400') !== -1) "control: filter or raw value present (got: "
}

@test "@template slot interpolation applies pipe currency(...) filter, no literal pipe text (RED, BUG-188)" {
    @fixture {
        <div class="host-price-list"></div>
    }
    @eval (document.querySelector('.host-price-list').appendChild(Spacetime.invokeTemplate('price-list')))
    @eval (ST.initElement(document.querySelector('.host-price-list .price-each-list')))
    @wait 60ms
    @eval (ST.setData('eachItems', [{ price: 2400, category: 'electronics' }]))
    @wait 120ms
    @let (priceText = document.querySelector('.host-price-list .price-slot').textContent)
    @assert (priceText.indexOf('|') === -1)
        "@template slot: filter pipe must not leak as literal text (got: "
    @assert (priceText.indexOf('currency') === -1)
        "@template slot: filter name must not leak as literal text (got: "
    @assert (priceText.indexOf('$2,400') !== -1)
        "@template slot: currency filter must actually format the value (got: "
}

@test "@template slot interpolation applies pipe uppercase filter, no literal pipe text (RED, BUG-188)" {
    @fixture {
        <div class="host-upper-list"></div>
    }
    @eval (document.querySelector('.host-upper-list').appendChild(Spacetime.invokeTemplate('upper-list')))
    @eval (ST.initElement(document.querySelector('.host-upper-list .upper-each-list')))
    @wait 60ms
    @eval (ST.setData('upperItems', [{ price: 100, category: 'widgets' }]))
    @wait 120ms
    @let (upperText = document.querySelector('.host-upper-list .upper-slot').textContent)
    @assert (upperText.indexOf('|') === -1)
        "@template slot: filter pipe must not leak as literal text (got: "
    @assert (upperText === 'WIDGETS')
        "@template slot: uppercase filter must actually uppercase the value (got: "
}

@test "bare @each (non-template) row body interpolation applies pipe currency(...) filter too (RED, BUG-188)" {
    @fixture {
        <div class="host-raw-list"></div>
    }
    @eval (document.querySelector('.host-raw-list').appendChild(Spacetime.invokeTemplate('raw-each-list')))
    @eval (ST.initElement(document.querySelector('.host-raw-list .raw-each-list')))
    @wait 60ms
    @eval (ST.setData('eachItems', [{ price: 500, category: 'z' }]))
    @wait 120ms
    @let (rawText = document.querySelector('.host-raw-list .raw-price').textContent)
    @assert (rawText.indexOf('|') === -1)
        "bare @each row: filter pipe must not leak as literal text (got: "
    @assert (rawText.indexOf('currency') === -1)
        "bare @each row: filter name must not leak as literal text (got: "
}
