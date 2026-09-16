// Assertion claim grammar (PLAN-027 W3).
//
// Powers `@then <subject> { <claim>; * }`. A claim is `<prop> <op> <value>;` or a
// bare `<prop>;` (truthy/exists). Props are flat idents resolved at runtime by
// the then-claims __claimProp resolver:
//   DOM        : exist · count · text · value · state · <attr>
//   layout     : be_visible · be_hidden            (real-browser only)
//   signal     : $<name>                            (via ST.get)
//   machine    : state_history · transition_count · previous_state ·
//                has_state_machine · animating
//   timeline   : timeline_running · timeline_progress
// Ops: == != >= <= < > ~=(contains) ~>(sequence/history) matches(regex).
//
// Claims are `;`-terminated so the `value:expr` capture has a clean boundary.
//
// Example:
//   @then .wizard {
//     state            == "complete";
//     transition_count >= 3;
//     state_history    ~> ["a", "b"];
//     $count           == 5;
//     exist;
//   }

%capture_type claim_op {
    "==" | "!=" | ">=" | "<=" | "<" | ">" | "~=" | "~>" | "matches"
}

// A prop is either a `$signal` reference or a bare ident. Captured into the SAME
// `prop` field so the resolver sees `$count` or `state_history` uniformly.
%capture_type claim_prop {
    ( "$" $sig:ident ) | ( $id:ident )
}

%capture_type claim_block {
    ( ( $prop:claim_prop $op:claim_op $value:expr ";" ) | ( $prop:claim_prop ";" ) )*
}
