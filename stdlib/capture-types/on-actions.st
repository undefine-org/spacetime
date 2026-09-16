// On-action capture types for @on handler body parsing
// Enables free interleaving of keyframes and mutations
//
// Example @on body:
//   @on click press(150ms) {
//       scale: 1 -> 0.95 -> 1;      // keyframe
//       $clicks <- $clicks + 1;     // mutation
//       opacity: 0 -> 1;            // keyframe
//       @emit clicked;              // emit
//   }

/// Raw value until semicolon
/// Captures everything until a semicolon is encountered
/// Used for keyframe values like "0 -> 1" or "scale(0.9) -> scale(1)"
///
%capture_type raw_until_semicolon {
    [^;]+
}

/// Keyframe property line
/// Matches: property: value -> value;
///
/// Examples:
///   opacity: 0 -> 1;
///   scale: 1 -> 0.95 -> 1;
///   transform: scale(0.9) -> scale(1);
///
%capture_type keyframe_prop {
    $prop:ident ":" $value:raw_until_semicolon ";"?
}

/// Mutation assignment
/// Matches: $var <- expr;
///
/// Examples:
///   $count <- $count + 1;
///   $name <- $.textContent;
///   $visible <- true;
///
%capture_type mutation {
    "$" $target:ident "<-" $expr:expr ";"?
}

/// A PROPERTY written where a signal mutation belongs (BUG-346).
///
/// Matches the same shape WITHOUT the `$`, so the mistake is caught as itself
/// instead of falling through every `on_motion_body` alternative and reporting
/// as E0946 "expected capture `$body:body` but no tokens remaining" — a message
/// about the parser's position rather than the author's line.
///
/// This is a MATCH-ONLY type: it exists to be named by an error sibling, never
/// to lower. `<-` in an `@on` body means "assign this cell once", and a CSS
/// property is not a cell — `ST.runMutations` has no style-write path at all.
///
/// Examples:
///   opacity <- 0.8;
///   background-color <- red;
///
%capture_type property_mutation_invalid {
    $prop:ident "<-" $expr:expr ";"?
}

/// Emit action
/// Matches: @emit event_name;
///
/// Examples:
///   @emit clicked;
///   @emit open_modal;
///   @emit close;
///
%capture_type emit_action {
    "@emit" $event:ident ";"?
}

/// Signal method call
/// Matches: $signal.method(args);
///
/// Examples:
///   $todos.insert({ text: "hello" });
///   $analytics.track("click");
///   $items.remove(0);
///
%capture_type signal_call {
    "$" $signal:ident "." $method:ident "(" $args:expr? = undefined ")" ";"?
}

/// Single action in @on body
/// Can be any of: emit, mutation, signal call, or keyframe
///
%capture_type on_action {
    ( $emit:emit_action )
    | ( $mutation:mutation )
    | ( $signal:signal_call )
    | ( $keyframe:keyframe_prop )
}

/// Full @on body - sequence of actions
/// Allows free interleaving of keyframes and mutations
///
%capture_type on_body {
    ( $action:on_action )*
}
