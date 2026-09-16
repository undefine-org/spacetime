/// BUG: ID selectors starting with hex-valid characters are misinterpreted as colors
///
/// The .st parser greedily interprets `#` followed by hex-valid characters (a-f, 0-9)
/// as a CSS hex color value, even when the token appears in selector position.
///
/// This means ID selectors like `#case-cosmetics`, `#facade`, `#beef-section`
/// cause parse errors, while `#story-cosmetics`, `#test`, `#ora-glow` work fine
/// because their first character after `#` is NOT a hex digit.
///
/// Root cause: the tokenizer/parser does not distinguish context (selector position
/// vs property-value position) when encountering `#`. It sees `#ca...` and eagerly
/// lexes a hex color token, consuming characters that should form an ID selector.
///
/// Expected: all of the blocks below should parse without errors.
/// Actual: blocks whose ID starts with [a-fA-F0-9] fail to parse.
///
/// Real-world impact: ora-ventures.com/_clients.st needed `#story-*` instead of
/// the natural `#case-*` to work around this bug.

// =============================================================================
// CURRENTLY FAILING — IDs that start with hex-valid chars
// =============================================================================

// First char 'c' (hex valid)
#case-cosmetics {
    top: 10px;
}

// First char 'f' (hex valid)
#facade-element {
    display: block;
}

// First char 'a' (hex valid)
#abc-container {
    margin: 0;
}

// First char 'b' (hex valid)
#beef-section {
    padding: 1rem;
}

// First char 'd' (hex valid)
#dead-panel {
    opacity: 1;
}

// First char 'e' (hex valid)
#event-handler {
    cursor: pointer;
}

// Pure hex word (looks most like a color)
#facade {
    color: red;
}

// 3-char pure hex (ambiguous with #RGB shorthand)
#ace {
    font-size: 16px;
}

// 6-char pure hex (ambiguous with #RRGGBB)
#decade {
    line-height: 1.5;
}

// Starts with digit
#1st-item {
    order: 1;
}

// =============================================================================
// CURRENTLY PASSING — IDs that start with non-hex chars
// These demonstrate that the parser handles ID selectors fine when
// the first character is NOT in [a-fA-F0-9].
// =============================================================================

#story-cosmetics {
    top: 10px;
}

#test-anchor {
    position: absolute;
}

#ora-glow {
    z-index: -1;
}

#main-content {
    width: 100%;
}

#navigation {
    position: fixed;
}

// =============================================================================
// CONTEXT CONTRAST — same hex-prefixed name used as color vs selector
// The parser needs to distinguish these two contexts:
// =============================================================================

// As selector: #cafe should start a scope block
#cafe {
    background: white;
}

// As color value: #cafe is a hex color inside a property
.some-class {
    // Here #cafe IS a color value (in property-value position)
    color: #cafe;
}
