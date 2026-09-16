// Pattern syntax reference file
// Shows @pattern definitions with all control flow variants

// ============================================
// Basic Pattern Definition
// ============================================

// Simple pattern with parameters
// Note: Parameter defaults use plain numbers; duration units like "ms" are
// only allowed in pattern call arguments, not in pattern parameter defaults
@pattern fadeIn($duration: 300, $easing: ease-out) {
    .is-visible: $shown;
    opacity: 0;
}

// Pattern with no parameters
@pattern reset() {
    opacity: 1;
}

// Pattern with various default types
@pattern complex($name, $count: 5, $label: "default", $mode: auto) {
    .is-active: $active;
    opacity: 1;
}

// ============================================
// Pattern with reactive toggle state
// ============================================

@pattern toggle() {
    .is-on: $on;
    @on &.click { $on <- !$on; }
}
.toggle-on { display: block; }
.toggle:not(.is-on) { display: none; }

// ============================================
// Pattern with @for loop
// ============================================

@pattern staggered($items, $delay: 50) {
    @for $item in $items {
        @state(when: "hidden") {
            opacity: 0;
            transform: translateY(20px);
        }
        @state(when: "visible") {
            opacity: 1;
            transform: translateY(0);
        }
    }
}

// Pattern @for with literal list
@pattern sequence() {
    @for $step in [1, 2, 3, 4, 5] {
        @state(when: "active") {
            order: 1;
        }
    }
}

// Pattern @for with string list
@pattern directions() {
    @for $dir in ["north", "south", "east", "west"] {
        @state(when: "heading") {
            cursor: pointer;
        }
    }
}

// ============================================
// Pattern with @if conditional
// ============================================

@pattern responsive($breakpoint: 768) {
    @if $breakpoint {
        @state(when: "expanded") {
            width: 100%;
        }
    }
}

// Pattern @if with equality check
@pattern themed($theme: "light") {
    @if $theme == "dark" {
        @state(when: "active") {
            background: #333;
        }
    }
    @if $theme == "light" {
        @state(when: "active") {
            background: #fff;
        }
    }
}

// Pattern @if with inequality check
@pattern conditional($mode: auto) {
    @if $mode != none {
        @state(when: "enabled") {
            pointer-events: auto;
        }
    }
}

// ============================================
// Pattern with @match
// ============================================

@pattern direction($dir) {
    @match $dir {
        "left" => {
            @state(when: "active") {
                transform: translateX(-100%);
            }
        }
        "right" => {
            @state(when: "active") {
                transform: translateX(100%);
            }
        }
        "up" => {
            @state(when: "active") {
                transform: translateY(-100%);
            }
        }
        "down" => {
            @state(when: "active") {
                transform: translateY(100%);
            }
        }
        _ => {
            @state(when: "active") {
                transform: none;
            }
        }
    }
}

// Pattern @match with identifier patterns
@pattern align($position) {
    @match $position {
        start => {
            @state(when: "aligned") {
                justify-content: flex-start;
            }
        }
        center => {
            @state(when: "aligned") {
                justify-content: center;
            }
        }
        end => {
            @state(when: "aligned") {
                justify-content: flex-end;
            }
        }
        _ => {
            @state(when: "aligned") {
                justify-content: stretch;
            }
        }
    }
}

// ============================================
// Pattern with @include
// ============================================

@pattern composite($duration) {
    @include fadeIn(duration: $duration);
    @include toggle(initial: on);
}

// Pattern @include with multiple args
@pattern enhanced($speed: 200, $dir: "left") {
    @include direction(dir: $dir);
}

// ============================================
// Pattern with Nested Control Flow
// ============================================

@pattern matrix($rows, $cols) {
    @for $row in $rows {
        @for $col in $cols {
            @state(when: "cell") {
                display: grid;
            }
        }
    }
}

// @if inside @for
@pattern filtered($items, $showAll: false) {
    @for $item in $items {
        @if $showAll {
            @state(when: "visible") {
                opacity: 1;
            }
        }
    }
}

// @match inside @for
@pattern grid($cells) {
    @for $cell in $cells {
        @match $cell {
            "header" => {
                @state(when: "layout") {
                    grid-row: 1;
                }
            }
            "content" => {
                @state(when: "layout") {
                    grid-row: 2;
                }
            }
            _ => {
                @state(when: "layout") {
                    grid-row: auto;
                }
            }
        }
    }
}

// ============================================
// Pattern Calls (in scope context)
// ============================================

// Pattern calls are used within scope blocks:
//
// .element {
//     @fadeIn(duration: 500ms, easing: ease-in-out);
//     @toggle(initial: on);
//     @direction(dir: "up");
// }

// ============================================
// Pattern Parameter Types
// ============================================

// Number defaults
@pattern numericDefaults($count: 10, $ratio: 0.5, $negative: -20) {
    @state(when: "set") {
        order: 1;
    }
}

// String defaults
@pattern stringDefaults($name: "default", $label: "text") {
    @state(when: "labeled") {
        content: "label";
    }
}

// Identifier defaults
@pattern identDefaults($easing: ease-in-out, $mode: auto, $position: center) {
    @state(when: "styled") {
        display: flex;
    }
}

// List defaults
@pattern listDefaults($items: [1, 2, 3], $names: ["a", "b"], $empty: []) {
    @state(when: "populated") {
        display: block;
    }
}

// Mixed defaults
@pattern mixedDefaults($required, $num: 100, $str: "hello", $ident: auto, $list: [1, 2]) {
    @state(when: "configured") {
        visibility: visible;
    }
}

// ============================================
// Pattern with reactive data state
// ============================================

@pattern asyncFlow($endpoint: "/api/data") {
    .is-loading: $loading;
    .is-error: $error != null;
    opacity: 1;
}
.asyncFlow.is-loading { opacity: 0.5; }

// ============================================
// Pattern Syntax Summary
// ============================================

// @pattern name($param, $param: default) { body }
//
// Parameters (defaults use plain values, not durations):
// - $name             Required parameter
// - $name: 100        Number default (plain number, no units)
// - $name: "string"   String default
// - $name: ident      Identifier default
// - $name: [a, b]     List default
//
// Body items:
// - .cls: $signal              reactive class
// - @state(when: ...) { properties }   data-st-state → CSS
// - @on event { $sig <- expr }  event mutation
// - @for $var in $source { body }
// - @for $var in [literal, list] { body }
// - @if $condition { body }
// - @if $var == value { body }
// - @if $var != value { body }
// - @match $var { pattern => { body } }
// - @include name(args)
// - @patternName(args)  (nested pattern call)
