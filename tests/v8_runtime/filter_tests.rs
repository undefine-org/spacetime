//! Content Injection Filter Tests (SC-003)
//!
//! Tests that ST.filters and ST.filter work correctly for the built-in
//! filter registry: uppercase, lowercase, number(format).
//! Also tests that content injections from rawBody.injections are wired
//! with filter application.

use super::context::V8TestContext;

const ST_JS: &str = include_str!("../../public/runtime/st.js");

fn create_context() -> V8TestContext {
    let mut ctx = V8TestContext::new();
    ctx.eval(ST_JS).expect("Failed to load st.js");
    ctx
}

// =============================================================================
// ST.filter() — Built-in Filter Registry
// =============================================================================

#[test]
fn test_filter_uppercase() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const v1 = ST.filter('uppercase', 'hello world');
            const v2 = ST.filter('uppercase', 'Test 123');
            const v3 = ST.filter('uppercase', '');
            return JSON.stringify({ v1, v2, v3 });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["v1"], "HELLO WORLD");
    assert_eq!(obj["v2"], "TEST 123");
    assert_eq!(obj["v3"], "");
}

#[test]
fn test_filter_lowercase() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const v1 = ST.filter('lowercase', 'HELLO WORLD');
            const v2 = ST.filter('lowercase', 'Test 123');
            const v3 = ST.filter('lowercase', '');
            return JSON.stringify({ v1, v2, v3 });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["v1"], "hello world");
    assert_eq!(obj["v2"], "test 123");
    assert_eq!(obj["v3"], "");
}

#[test]
fn test_filter_number_no_format() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const v1 = ST.filter('number', 42);
            const v2 = ST.filter('number', '99.5');
            const v3 = ST.filter('number', 'abc');
            return JSON.stringify({ v1, v2, v3 });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["v1"], "42");
    assert_eq!(obj["v2"], "99.5");
    assert_eq!(obj["v3"], "abc"); // NaN falls back to String(v)
}

#[test]
fn test_filter_number_with_format() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const v1 = ST.filter('number(".2")', 42);
            const v2 = ST.filter('number(".3")', 3.14159);
            const v3 = ST.filter('number(".0")', 99.7);
            return JSON.stringify({ v1, v2, v3 });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["v1"], "42.00");
    assert_eq!(obj["v2"], "3.142");
    assert_eq!(obj["v3"], "100");
}

#[test]
fn test_filter_unknown_returns_value() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const v = ST.filter('nonexistent', 'hello');
            return JSON.stringify({ v });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["v"], "hello"); // Unknown filter passes through
}

#[test]
fn test_filter_null_name_returns_value() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const v1 = ST.filter(null, 'hello');
            const v2 = ST.filter('', 'world');
            return JSON.stringify({ v1, v2 });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["v1"], "hello");
    assert_eq!(obj["v2"], "world");
}

// =============================================================================
// Content Injection Wiring — simulates what template.st does
// =============================================================================

#[test]
fn test_injection_text_with_filter() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const element = document.createElement('div');
            element.textContent = 'initial';

            // Set the reactive value FIRST (before watch)
            ST.set(element, 'name', 'hello world');

            // Wire injection — ST.watch fires immediately with current value
            const tgt = { type: 'Text' };
            const filter = 'uppercase';
            ST.watch(element, 'name', v => {
                const filtered = filter ? ST.filter(filter, v) : v;
                element.textContent = filtered !== undefined ? String(filtered) : String(v);
            });

            return JSON.stringify({ text: element.textContent });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["text"], "HELLO WORLD");
}

#[test]
fn test_injection_text_without_filter() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const element = document.createElement('div');

            // Set value FIRST
            ST.set(element, 'count', 42);

            // Watch fires immediately with current value
            ST.watch(element, 'count', v => {
                element.textContent = v !== undefined ? String(v) : '';
            });

            return JSON.stringify({ text: element.textContent });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["text"], "42");
}

#[test]
fn test_injection_attr_with_filter() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const element = document.createElement('img');

            // Set value FIRST
            ST.set(element, 'desc', 'A Beautiful SUNSET');

            // Watch fires immediately with current value
            const filter = 'lowercase';
            ST.watch(element, 'desc', v => {
                const filtered = filter ? ST.filter(filter, v) : v;
                const val = filtered !== undefined ? String(filtered) : String(v);
                element.setAttribute('alt', val);
            });

            return JSON.stringify({ alt: element.getAttribute('alt') });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["alt"], "a beautiful sunset");
}

#[test]
fn test_injection_slot_with_filter() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            const element = document.createElement('div');
            element.innerHTML = '<span slot=\"price\">-</span>';

            // Set value FIRST
            ST.set(element, 'amount', 9.5);

            // Watch fires immediately with current value
            const filter = 'number(\".2\")';
            ST.watch(element, 'amount', v => {
                const slotEl = element.querySelector('[slot="price"]');
                if (!slotEl) return;
                const filtered = filter ? ST.filter(filter, v) : v;
                slotEl.textContent = filtered !== undefined ? String(filtered) : String(v);
            });

            const slotText = element.querySelector('[slot="price"]').textContent;
            return JSON.stringify({ slotText });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["slotText"], "9.50");
}

// =============================================================================
// Custom Filter Registration
// =============================================================================

#[test]
fn test_custom_filter_registration() {
    let mut ctx = create_context();

    let result = ctx.eval(
        r#"
        (() => {
            // Register a custom filter
            ST.filters.reverse = function(v) {
                return String(v).split('').reverse().join('');
            };

            const v = ST.filter('reverse', 'hello');
            return JSON.stringify({ v });
        })();
    "#,
    );

    let val = result.expect("JS evaluation failed");
    let obj: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(obj["v"], "olleh");
}

// =============================================================================
// FEAT-108 — Color algebra filters (darken / lighten / alpha / mix)
// =============================================================================

#[test]
fn test_filter_darken() {
    let mut ctx = create_context();
    let result = ctx.eval(
        r#"
        (() => JSON.stringify({
            // #FF0020 darkened 20% toward black: each channel scaled by 0.8.
            d20: ST.filters.darken('#FF0020', 0.2),
            // 0% = unchanged.
            d0:  ST.filters.darken('#FF0020', 0),
            // 100% = black.
            d100: ST.filters.darken('#FF0020', 1),
            // shorthand hex.
            short: ST.filters.darken('#f02', 0.5),
            // non-color passes through.
            pass: ST.filters.darken('not-a-color', 0.2),
        }))();
    "#,
    );
    let val = result.expect("JS evaluation failed");
    let o: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(o["d20"], "#cc001a", "FF*0.8=CC, 00, 20*0.8=1A");
    assert_eq!(o["d0"], "#ff0020");
    assert_eq!(o["d100"], "#000000");
    assert_eq!(
        o["short"], "#800011",
        "#f02 -> #ff0022, *0.5 (ff*0.5=127.5 rounds to 128=0x80)"
    );
    assert_eq!(o["pass"], "not-a-color");
}

#[test]
fn test_filter_lighten() {
    let mut ctx = create_context();
    let result = ctx.eval(
        r#"
        (() => JSON.stringify({
            // black lightened 50% = mid grey.
            l50: ST.filters.lighten('#000000', 0.5),
            l0:  ST.filters.lighten('#0a0a0a', 0),
            l100: ST.filters.lighten('#0a0a0a', 1),
        }))();
    "#,
    );
    let val = result.expect("JS evaluation failed");
    let o: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(o["l50"], "#808080");
    assert_eq!(o["l0"], "#0a0a0a");
    assert_eq!(o["l100"], "#ffffff");
}

#[test]
fn test_filter_alpha() {
    let mut ctx = create_context();
    let result = ctx.eval(
        r#"
        (() => JSON.stringify({
            // alpha<1 -> rgba()
            a60: ST.filters.alpha('#0a0a0a', 0.6),
            // alpha 1 -> hex.
            a1: ST.filters.alpha('#0a0a0a', 1),
        }))();
    "#,
    );
    let val = result.expect("JS evaluation failed");
    let o: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(o["a60"], "rgba(10, 10, 10, 0.6)");
    assert_eq!(o["a1"], "#0a0a0a");
}

#[test]
fn test_filter_mix() {
    let mut ctx = create_context();
    let result = ctx.eval(
        r#"
        (() => JSON.stringify({
            // mix black & white 50/50 = grey.
            half: ST.filters.mix('#000000', '#ffffff', 0.5),
            // weight toward first when 0 (stays at first color... w=0 -> all c2? check semantics)
            // _stScale(c1, w, c2): w=0 -> c1, w=1 -> c2.
            w0: ST.filters.mix('#000000', '#ffffff', 0),
            w1: ST.filters.mix('#000000', '#ffffff', 1),
        }))();
    "#,
    );
    let val = result.expect("JS evaluation failed");
    let o: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    assert_eq!(o["half"], "#808080");
    assert_eq!(o["w0"], "#000000", "weight 0 = first color");
    assert_eq!(o["w1"], "#ffffff", "weight 1 = second color");
}

#[test]
fn test_resolve_derived_value_with_filter_name_not_corrupted() {
    // FEAT-108 review P2: a brand value that contains a filter-name + '(' must not
    // be rewritten inside its string literal. adResolveDerived rewrites bare
    // filter calls on the RAW formula BEFORE substituting values.
    let mut ctx = create_context();
    let result = ctx.eval(
        r#"
        (() => JSON.stringify({
            // value 'mix(x)' is a plain string fed to darken (not a color) -> passes through.
            safe: ST.adResolveDerived('$brand.note | darken(0.1)', { note: 'mix(x)' }),
            // real formula still resolves.
            real: ST.adResolveDerived('$brand.scarlet | darken(0.2)', { scarlet: '#FF0020' }),
            // member call (.toUpperCase) is NOT rewritten as a filter.
            member: ST.adResolveDerived('$brand.scarlet', { scarlet: '#abcdef' }),
        }))();
    "#,
    );
    let val = result.expect("JS evaluation failed");
    let o: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    // 'mix(x)' isn't a parseable color, darken returns it unchanged (no corruption).
    assert_eq!(
        o["safe"], "mix(x)",
        "value with filter-name substring must pass through intact"
    );
    assert_eq!(o["real"], "#cc001a");
    assert_eq!(o["member"], "#abcdef");
}

// =============================================================================
// FEAT-107 — Color picker HSV round-trip (adColorToHsv / adHsvToHex)
// =============================================================================

#[test]
fn test_color_picker_hsv_roundtrip() {
    let mut ctx = create_context();
    let result = ctx.eval(
        r#"
        (() => {
            // hex -> hsv -> hex must round-trip (within integer rounding).
            const cases = ['#FF0020', '#0a0a0a', '#ffffff', '#000000', '#3366cc', '#00ff00'];
            const out = {};
            for (const hex of cases) {
                const hsv = ST.adColorToHsv(hex);
                out[hex] = ST.adHsvToHex(hsv.h, hsv.s, hsv.v, hsv.a);
            }
            // pure hue/sat/val compose: red.
            out.red = ST.adHsvToHex(0, 100, 100, 1);
            // white = v100 s0.
            out.white = ST.adHsvToHex(0, 0, 100, 1);
            // alpha<1 -> rgba.
            out.alpha = ST.adHsvToHex(0, 100, 100, 0.5);
            return JSON.stringify(out);
        })();
    "#,
    );
    let val = result.expect("JS evaluation failed");
    let o: serde_json::Value = serde_json::from_str(val.as_str().unwrap()).unwrap();
    // Round-trips (allow the parser's lowercase hex output).
    assert_eq!(o["#FF0020"], "#ff0020");
    assert_eq!(o["#0a0a0a"], "#0a0a0a");
    assert_eq!(o["#ffffff"], "#ffffff");
    assert_eq!(o["#000000"], "#000000");
    assert_eq!(o["#3366cc"], "#3366cc");
    assert_eq!(o["#00ff00"], "#00ff00");
    assert_eq!(o["red"], "#ff0000");
    assert_eq!(o["white"], "#ffffff");
    assert_eq!(o["alpha"], "rgba(255, 0, 0, 0.5)");
}
