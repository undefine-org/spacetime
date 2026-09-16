// Metasystem syntax reference file
// Shows %primitive, %macro, and all metasystem clauses

// ============================================
// %emit blocks
// ============================================

// Emit blocks allow embedding raw code in specific languages.
// The parser handles nested braces correctly.

// Note: These are typically inside %primitive definitions,
// but shown here for syntax reference.

// %emit js {
//     console.log("hello");
//     const x = 42;
//     if (condition) {
//         nested();
//     }
// }

// %emit css {
//     .generated { color: red; }
// }

// %emit glsl {
//     void main() {
//         gl_FragColor = vec4(1.0);
//     }
// }

// ============================================
// %primitive definitions
// ============================================

// Primitives are low-level reactive building blocks that emit
// platform-specific code (JS, CSS, GLSL).

// Simple primitive with element reference and typed output
%primitive fade(&el, $opacity: number) {
    %emit css {
        {{&el}} {
            opacity: {{$opacity}};
            transition: opacity 0.3s;
        }
    }

    %exports {
        $opacity: number
    }
}

// Primitive with cleanup block
%primitive scroll-driver(&element, $progress: number, threshold: number = 0.5) {
    %emit js {
        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                {{$progress}} = entry.intersectionRatio;
            });
        }, { threshold: {{threshold}} });
        observer.observe({{&element}});
    }

    %cleanup {
        observer.disconnect();
    }

    %exports {
        $progress: number
    }
}

// Primitive with inline cleanup inside emit
%primitive timer($elapsed: number, interval: number = 1000) {
    %emit js {
        let id = setInterval(() => {
            {{$elapsed}}++;
        }, {{interval}});

        %cleanup {
            clearInterval(id);
        }
    }

    %exports {
        $elapsed: number
    }
}

// Primitive with multiple parameters
%primitive spring-animation(&target, $value: number, stiffness: number = 100, damping: number = 10, mass: number = 1) {
    %emit js {
        const spring = createSpring({
            stiffness: {{stiffness}},
            damping: {{damping}},
            mass: {{mass}}
        });
        spring.onUpdate(v => {{$value}} = v);
    }

    %cleanup {
        spring.destroy();
    }

    %exports {
        $value: number
    }
}

// Primitive with function type export
%primitive event-listener(&el, event: string = "click") {
    %emit js {
        const handler = (e) => {
            {{$callback}}?.(e);
        };
        {{&el}}.addEventListener({{event}}, handler);
    }

    %cleanup {
        {{&el}}.removeEventListener({{event}}, handler);
    }

    %exports {
        $callback: fn(event: any) ~> void
        $lastEvent: any?
    }
}

// Primitive with %if conditional
%primitive conditional-observer(&el, $active: number, useResize: bool = false) {
    %emit js {
        let observer = null;
    }

    %if useResize {
        %emit js {
            observer = new ResizeObserver((entries) => {
                {{$active}} = 1;
            });
            observer.observe({{&el}});
        }

        %cleanup {
            observer?.disconnect();
        }
    } %else {
        %emit js {
            observer = new IntersectionObserver((entries) => {
                {{$active}} = entries[0].isIntersecting ? 1 : 0;
            });
            observer.observe({{&el}});
        }

        %cleanup {
            observer?.disconnect();
        }
    }

    %exports {
        $active: number
    }
}

// ============================================
// %macro definitions
// ============================================

// Macros define high-level reactive patterns that get expanded
// into primitives and directives.

// Simple macro with %creates and %form
%macro on-hover {
    %creates @on-hover

    %form {
        @on-hover { $body:properties }
    }

    %scope selector

    %binds {
        hover-state(&self) -> { $isHovered }
    }

    %derives {
        $active: $isHovered
    }

    %animates {
        opacity: $opacity
    }

    %applies {
        cursor: pointer;
    }
}

// Macro with states
%macro toggle {
    %creates @toggle

    %form {
        @toggle(initial: $initial:("on" | "off")?) { $body:properties? }
    }

    %scope selector

    %states {
        on {
            opacity: 1
        }
        off {
            opacity: 0
        }
    }

    %on $state -> on {
        $active <- true
    }

    %on $state -> off {
        $active <- false
    }
}

// Macro with %binds using complex arguments
%macro scroll-fade {
    %creates @scroll-fade

    %form {
        @scroll-fade(start: $start:number?, end: $end:number?) { }
    }

    %scope selector

    %binds {
        scroll-driver(&self, threshold: 0) -> { $progress }
    }

    %derives {
        $opacity: lerp(0, 1, clamp($progress, $start ?? 0, $end ?? 1))
    }

    %animates {
        opacity: $opacity
    }
}

// Macro with %when conditional
%macro conditional-style {
    %creates @conditional-style

    %form {
        @conditional-style(condition: $cond:binding) { $styles:properties }
    }

    %scope selector

    %when $cond {
        %applies {
            $styles
        }
    }
}

// Macro with %for loop
%macro staggered {
    %creates @staggered

    %form {
        @staggered(items: $items:element*, delay: $delay:number?) { $body:properties }
    }

    %scope selector

    %for $item in $items {
        %animates {
            opacity: $opacity
            transform: $transform
        }
    }
}

// Macro with %if/%elif/%else
%macro theme-aware {
    %creates @theme-aware

    %form {
        @theme-aware { $body:properties }
    }

    %scope selector

    %binds {
        media-query("(prefers-color-scheme: dark)") -> { $isDark }
    }

    %if $isDark {
        %applies {
            background: #1a1a1a;
            color: #ffffff;
        }
    } %elif $isHighContrast {
        %applies {
            background: #000000;
            color: #ffffff;
        }
    } %else {
        %applies {
            background: #ffffff;
            color: #1a1a1a;
        }
    }
}

// Macro with %includes
%macro combined-effect {
    %creates @combined-effect

    %form {
        @combined-effect(duration: $dur:time?) { }
    }

    %scope selector

    %includes {
        @fade-in(duration: $dur);
        @slide-up(distance: 20px);
    }
}

// Macro with %registers
%macro type-definition {
    %creates @type

    %form {
        @type $name:ident { $fields:fields }
    }

    %scope file

    %registers type($name) {
        fields: $fields
        mutable: false
    }
}

// Macro with %resolves
%macro function-call {
    %creates @fn

    %form {
        @fn $name:ident($params:params) : $ret:typeref { $body:expr }
    }

    %scope file

    %resolves {
        $name -> ST.functions
    }

    %registers function($name) {
        params: $params
        returnType: $ret
    }
}

// Macro with %mutate
%macro editable-text {
    %creates @editable

    %form {
        @editable { $body:template }
    }

    %scope selector

    %mutate on_enter("editing") {
        focus(target: selection);
    }

    %mutate on_exit("editing") {
        blur(target: selection);
    }
}

// Macro with %trigger
%macro click-action {
    %creates @click-action

    %form {
        @click-action(action: $action:ident) { }
    }

    %scope selector

    %on click {
        %trigger $action
    }
}

// Macro with %iterates and %template
%macro list-renderer {
    %creates @each

    %form {
        @each $item:ident in $source:binding { $template:template }
    }

    %scope selector

    %iterates $source {
        item: $
        index: $_index
    }

    %template {
        $template
    }

    %transitions {
        enter { opacity: 0 -> 1 }
        exit { opacity: 1 -> 0 }
    }
}

// Macro with multiple scopes
%macro universal {
    %creates @universal

    %form {
        @universal { $body:properties }
    }

    %scope selector | property-value | template
}

// Macro with exports
%macro data-fetcher {
    %creates @fetch

    %form {
        @fetch(url: $url:string) { }
    }

    %scope selector

    %binds {
        fetch-data(url: $url) -> { $data, $loading, $error }
    }

    %exports {
        $data: any?
        $loading: boolean
        $error: string?
    }
}

// ============================================
// %on clauses - event/trigger handling
// ============================================

// %on with variable transition
// %on $state -> "active" {
//     $opacity <- 1
// }

// %on with variable event
// %on $value.change {
//     $dirty <- true
// }

// %on with event name
// %on click {
//     $count <- $count + 1
// }

// %on with dynamic trigger
// %on state_$timing($state) {
//     $active <- true
// }

// ============================================
// %if/%elif/%else conditions
// ============================================

// Simple truthy: %if $variable
// Falsy: %if !$variable
// Equality: %if $var == "value"
// Inequality: %if $var != 0
// Comparison: %if $var > 10
// Comparison: %if $var <= 100
// Logical OR: %if $a || $b
// Logical AND: %if $a && $b
// Combined: %if ($a && $b) || $c
// Property access: %if $obj.prop == true
// Array access: %if $arr[0] > 0

// ============================================
// %resolves clause
// ============================================

// Maps symbols to runtime registries for code generation
// %resolves {
//     $lerp -> ST.functions
//     $spring -> ST.easings
//     $fetchApi -> ST.network
// }

// ============================================
// %registers clause
// ============================================

// Registers definitions in compile-time registries
// %registers type($name) {
//     fields: $fields
//     mutable: false
// }

// %registers binding($name, type: $type) {
//     value: $value
//     dependencies: [$dep1, $dep2]
// }

// ============================================
// @preset definitions (parsed at file level)
// ============================================

// @preset category ~name is RETIRED (SIP-001c / BUG-263). The `~` preset
// mechanism never resolved at runtime: a `~name` in an easing/duration slot
// was emitted verbatim and the runtime fell back to the default (the arc's
// banned silent-acceptance class). A `~name` reference now FAILS LOUDLY with
// the "preset references are retired" diagnostic; the parser-level
// discrimination is asserted in src/parser/tests.rs (assert_preset_retired).
//
// Named curves are now @form declarations; easings in particular are
// `@form easing --name { cubic-bezier(...) }` and are referenced with the
// form sigil `easing: --name` (D12 strips `--` at the emission boundary so
// the runtime easing map, keyed BARE, resolves the name). See
// stdlib/macros/presets.st for the standard curve library.
//
// @preset easing ~smooth: cubic-bezier(0.4, 0, 0.2, 1)      -> @form easing --smooth { cubic-bezier(0.4, 0, 0.2, 1) }
// @preset duration ~fast: 150ms                              -> retired (durations are now explicit 150ms literals)
// @preset animation ~fade-in { ... }                         -> retired (use @form / a scoped construct)
// @preset scroll ~fade-section { ... }                       -> retired
// @preset load ~lazy { ... }                                 -> retired

// ============================================
// %runtime-registry definitions
// ============================================

// Runtime registries define how symbols are resolved at runtime
// for different compilation targets (JS, WASM, etc.)

// %runtime-registry functions {
//     %target js {
//         %namespace { ST.functions }
//         %init {
//             window.ST = window.ST || {};
//             ST.functions = {};
//         }
//         %call($name, $args) {
//             ST.functions[{{$name}}]({{$args}})
//         }
//         %register($name, $impl) {
//             ST.functions[{{$name}}] = {{$impl}}
//         }
//         %access($name) {
//             ST.functions[{{$name}}]
//         }
//     }
// }

// ============================================
// Metasystem Syntax Summary
// ============================================

// %primitive name(&el, $out: type, param: type = default) {
//     %emit js { code }
//     %emit css { code }
//     %emit glsl { code }
//     %cleanup { code }
//     %exports { $var: type }
//     %if condition { ... } %else { ... }
// }
//
// %macro name {
//     %creates @directive-name
//     %form { @directive(params) { body } }
//     %scope file | selector | property-value | ...
//     %binds { primitive(args) -> { $outputs } }
//     %derives { $name: expression }
//     %states { state { properties } }
//     %resolves { $symbol -> registry }
//     %registers name(args) { fields }
//     %when $condition { body }
//     %on trigger { body }
//     %for $item in $source { body }
//     %if condition { body } %elif { } %else { }
//     %animates { property: $var }
//     %applies { styles }
//     %includes { @patterns }
//     %mutate timing { operations }
//     %trigger event
//     %iterates $source { mappings }
//     %template { $template }
//     %transitions { enter/exit/move }
//     %exports { $var: type }
// }
//
// @preset category ~name RETIRED (SIP-001c / BUG-263) — ~ refs error loudly;
// named easings are @form easing --name (see the @preset section above).
//
// %runtime-registry name {
//     %target target { ... }
// }
