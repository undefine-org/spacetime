// Keyframe declarations (PLAN-023 W2).
//
// Parses a `;`-separated list of `property: value (-> value)*` chains, used by animation
// directive bodies (@on, @loop, @keyframes, hover/press presets, ...).
//
// Examples:
//   opacity: 0 -> 1
//   scale: 1 -> 0.95 -> 1;
//   translate-y: 20px -> 0
//   box-shadow: none -> 0 4px 20px rgba(0,0,0,0.15)
//
// The property is captured as a balanced run up to `:` (so hyphenated CSS props like
// `translate-y`/`background-color` are captured whole); the value chain is a balanced run
// up to `;`. reify_keyframes splits the value chain on `->` into Vec<String> (pure scalar
// split — the raw run preserves the arrow text verbatim).
//
// BUG-201: a NESTED scope block (`sel { prop: a -> b; }`) is a first-class keyframes
// item (parallax/tilt choreography: one driver, children animate at different rates).
// The scope arm is tried FIRST so an element selector (`div { ... }`) is never eaten
// by the property arm's balanced(':') run; its body re-enters `keyframes` recursively.

%capture_type keyframes {
    ( $selector:selector "{" $scope:keyframes "}" | $property:balanced(':') ":" $values:balanced(';') ";"? )*
}
