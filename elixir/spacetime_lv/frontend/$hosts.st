/* ============================================================================
   $hosts.st — the Phoenix LiveView hosts for this app (FEAT-137).

   One `@host` per LiveView module; pages `@import "./$hosts.st"` and read/fire
   by name — the page redeclares nothing the server already knows. The contract
   sidecar next to each page (`<Module>.contract.json`) is the build-time check
   (E0928) that every `send emit "<event>"` names an event the server handles.
   ============================================================================ */
@import "stdlib/macros/host"

@host $counter : live("SpacetimeLvWeb.CounterLive")

@host $board : live("SpacetimeLvWeb.BoardLive")

@host $auth : live("SpacetimeLvWeb.SignupLive")


@host $authField : live("SpacetimeLvWeb.SignupFieldLive")

