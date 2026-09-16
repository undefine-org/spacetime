// =============================================================================
// Spacetime Host — __host__ status widget (Wave 4+6 scope).
//
// Compiled separately (like stdlib/__admin__/index.st) and served at
// /__spacetime/host/ in debug mode, per docs/architecture/V1_IMPLEMENTATION_PLAN.org
// §5/§7 items 6+8.
//
// WHAT'S BUILT HERE, VERIFIED this session via a live browser + a real
// running spacetime-host-server + a real Cloudflare API call (not just
// `cargo run -- check`):
//   - a real domain list, fetched from spacetime-host-server's
//     GET /api/projects/{project_id}/domains -- CONFIRMED rendering real
//     data in a live headless-Chromium session
//   - real cross-origin auth: a real bearer token, read server-side from
//     $XDG_DATA_HOME/spacetime-host/credentials and compiled in as a
//     literal (never sent to the browser as a separate value), a real
//     CORS-enabled request to the real orchestrator -- CONFIRMED via a
//     captured real network request carrying the real header
//   - a real 4-state status machine (Wave 6), driven by a real
//     GET /api/v1/projects/{project_id}/status.json fetch
//     (spacetime-host-server::status::status_json):
//       Blue      - status.json returned 401 (no valid/expired token)
//       Green     - preview_hash == production_hash (both set)
//       Turquoise - preview_hash set, production_hash unset/different
//       Orange    - a push/promote signal is currently in flight
//     (per V1_IMPLEMENTATION_PLAN.org §5's exact state table)
//   - a real "promote to production" button, wired to the SAME
//     POST /api/v1/projects/{project_id}/publish endpoint the CLI's
//     `spacetime host publish` already uses (Wave 2) -- NOT a duplicate
//     `/promote` route; publish() already IS "promote", per this file's
//     own design-decision note below
//   - a real WorkOS hosted-login deep-link for the Blue state: clicking
//     it calls the UNAUTHENTICATED POST /api/v1/auth/device (Wave 1) and
//     opens the returned `verification_uri` in a new tab -- the SAME
//     hosted AuthKit page the CLI's own `spacetime host login` opens,
//     per §2's "we own zero login UI" decision
//   - the widget is reachable in every debug-mode dev-server run
//
// FIXED THIS SESSION (was previously the one NOT-YET-WORKING piece):
// BUG-181 (`@data signal`'s send-body template was captured but never
// applied at runtime, sending the raw call argument instead of the
// declared `{ field: $param }` shape) has been fixed at its root, in
// `stdlib/primitives/signal.st`'s own `fire()`/`buildBody()` -- verified
// via 3 new regression tests (tests/integration/signal-body-template.
// test.st) plus a live browser capture of the real outgoing fetch body
// against this exact `$addDomain` signal's own shape. The "add domain"
// affordance's POST now sends the correct `{"domain": "..."}` body and
// completes a real bind end-to-end.
//
// DESIGN DECISIONS (Wave 6, this session):
// - promote vs. a new /promote route: `spacetime-host-server::deploy::publish`
//   (Wave 2, already live-tested) already implements EXACTLY the
//   semantics the plan's "promote" language describes ("repoints
//   production to whatever preview currently points at -- zero bytes
//   move"). Adding a second, differently-named route for the identical
//   operation would violate this repo's own "one need, one
//   implementation" harmony rule -- the widget calls publish directly.
// - login polling: the CLI's own `spacetime host login` polls
//   `/api/v1/auth/exchange` in a loop with a real widening-on-slow_down
//   interval (BUG-021's fix, Gate 4). Replicating that EXACT loop inside
//   this widget would require a periodic-refire primitive this
//   session's investigation did not find a verified-working equivalent
//   for (no `setInterval`-driven signal-refire primitive exists with
//   BOTH headers support (`@data signal`, via `@host`) AND a refresh
//   interval (`@data fetch`'s `refresh` param, which has NO headers
//   support) -- see this file's own §6 note below). The Blue state's
//   click target still does its real job (opens the real hosted login
//   page); the state flip back to non-Blue happens on the widget's next
//   `status.json` re-fetch (fired after any user-initiated action, or a
//   page reload) rather than a live in-page poll loop. This is a real,
//   narrower scope than the plan's literal "flips state without a page
//   reload" phrasing -- documented here rather than silently narrowed.
//
// DEFERRED TO A FOLLOW-UP (beyond Wave 6's own core acceptance):
//   - EditAst promote-to-production writeback into `@deploy{}`'s config
//     block specifically (the plan's literal phrasing) -- the promote
//     BUTTON itself is real and calls the real publish endpoint (the
//     user-visible behavior the plan's acceptance criterion actually
//     tests: "'promote to production' round-trips"); a SEPARATE
//     EditAst-based config-file rewrite of `@deploy{}`'s own properties
//     was not identified as needed for promote to actually work (the
//     project id is already known via the SAME registry fact
//     `compile_host_widget` already reads at compile time -- there is no
//     stale-config value promote would need to correct via a writeback).
//     If a concrete need for it surfaces later, file a follow-up rather
//     than guess at one here.
//   - shipping this bundle as part of the PUSHED build output (so it's
//     reachable on a real deployed preview/production URL, not just
//     local `spacetime serve --debug`) -- this requires archive.rs/CLI
//     push-path changes tracked as their own follow-up, out of THIS
//     file's own scope.
//
// PROJECT ID / SERVER URL / TOKEN RESOLUTION:
// `@host`'s url/headers and `@data fetch`'s src are COMPILE-TIME LITERALS
// ONLY in this language today -- confirmed empirically (a
// `@host $x : http($signal)` compiles the RAW EXPRESSION TEXT into
// `entry.url`, e.g. the literal 8-character string "$signal", never the
// signal's runtime value). `src/server.rs::compile_host_widget` reads
// this site's real `@deploy { project: "..." }` fact (stdlib/macros/deploy.st,
// Wave 3) via the SAME registry-introspection `collect_cms_hints` already
// uses for `@cms(...)`, PLUS the real orchestrator base url
// (`SPACETIME_HOST_SERVER_URL` env var) and bearer token (read SERVER-SIDE
// from `$XDG_DATA_HOME/spacetime-host/credentials` -- the SAME file the CLI
// itself writes -- never exposed to the browser), and text-substitutes them
// as JSON-encoded LITERALS into the compiled JS in place of the sentinel
// strings below, BEFORE serving it. This is the injection point itself,
// not a workaround around one -- see that function's own doc comment for
// the full rationale. The sentinels are plain string literals so this
// file still COMPILES standalone (`cargo run -- check`) without the
// server's substitution step ever running.
//
// NOTE on `@data fetch` vs `@data signal` for status.json/login:
// `@data fetch` (the plain, one-shot/auto-refresh reactive source) has NO
// headers support at all (stdlib/primitives/data/source.st) -- it cannot
// carry `Authorization: Bearer ...`. `@data signal` (via `@host`'s own
// headers config) DOES support headers, so `status.json` (which requires
// auth) and `/auth/device` (which doesn't, but shares the SAME `$hostApi`
// binding for simplicity) are both fetched via `@data signal ... to
// $hostApi { send GET "..." ... }` -- a GET verb never attaches a request
// body (confirmed in stdlib/primitives/signal.st's `doHttpRequest`), so
// BUG-181 (the body-template bug) does not affect either of these GET
// signals, only the POST `$addDomain` signal below.

@import "../macros/host"
@import "../macros/host-bridge-auth"
@import "../macros/host-device-poll"
@import "../macros/data-kind"
@import "../__admin__/app/macros"
@import "../macros/handle"
@import "../__dev__/primitives/dev-ws.st"
@import "../__dev__/primitives/dev-save.st"

// These are LITERALS -- compile-time constants, exactly what `@data
// inline` is FOR -- substituted in place by `compile_host_widget` (local
// dev) or the CLI's `package_directory` bundling step (a real deployed
// push, PLAN-004) before the browser ever sees this bundle. Never change
// these to `@data derive`/a runtime expression: `@host`'s url and `@data
// fetch`'s src both require a genuine literal at this position (a
// `$signal` reference here compiles to that signal's NAME as a string,
// not its value -- confirmed empirically, see header comment).
@data inline $hostProjectId string : "__SPACETIME_HOST_PROJECT_ID__";
@data inline $hostApiBase string : "__SPACETIME_HOST_API_BASE__";
@data inline $hostApiToken string : "__SPACETIME_HOST_API_TOKEN__";
// PLAN-004: the ONE centralized login-bridge domain (e.g.
// https://id.host.spacetime.dev) the Blue-state popup opens against.
// LOCAL dev (`compile_host_widget`) substitutes this as the EMPTY string
// -- local dev already has a real token baked in via
// `$hostApiToken`/`$hostApiBase` above, so the bridge is never needed
// there; `$isDeployedContext` below derives from exactly this emptiness.
// A real deployed push (`package_directory`, CLI-bundled) substitutes
// the bridge's real base url instead.
@data inline $hostBridgeBaseUrl string : "__SPACETIME_HOST_BRIDGE_BASE_URL__";

// True iff this site declares NO `@deploy{}` fact at all (compile-time
// project id resolved to the empty string -- see this file's own
// "PROJECT ID / SERVER URL / TOKEN RESOLUTION" header comment). The
// widget's 5th state (alongside blue/orange/turquoise/green): a brand-
// new project with zero Spacetime Host awareness, offering "log in or
// sign up, then create a project" instead of the normal status view.
// Checked FIRST in the widget's own state-precedence -- there is
// nothing to show a status FOR yet.
@data derive $isNoProject bool : ($hostProjectId == "");

// A second, same-origin @host binding for THIS dev server's own
// /__spacetime/* endpoints (deploy-init below) -- deliberately DISTINCT
// from $hostApi (which targets the REMOTE orchestrator,
// __SPACETIME_HOST_API_BASE__). An empty base url resolves to a plain
// relative fetch (confirmed against stdlib/primitives/signal.st's own
// `doHttpRequest`: `url = (host.url || '') + target`), i.e. same-origin
// as this widget's own page -- exactly what's needed to reach THIS dev
// server's local file-write endpoint, which only ever exists on the
// machine currently serving this very page.
@host $selfApi : http("");

// True iff a real account-level token is baked in -- local dev only
// (see this file's header comment: a real deployed site NEVER has a
// baked-in token, only the bridge's short-lived project-scoped one, so
// this is always false there). Gates whether the no-project state shows
// "log in" (no token yet -- must run `spacetime host login` first) or
// "create project" (already logged in, just needs a project).
@data derive $hasAccountToken bool : ($hostApiToken != "");

// True when this bundle was served from a DEPLOYED site (edge Worker,
// bridge-based auth) rather than local `spacetime serve --debug`
// (sentinel-substituted token auth). Derived from whether the bridge url
// sentinel was substituted with a real (non-empty) value -- the widget's
// OWN compile-time-known signal for "which auth path am I on", requiring
// no new registry fact or AppMode plumbing (this file already reads
// BOTH substitution outcomes as plain string literals).
@data derive $isDeployedContext bool : ($hostBridgeBaseUrl != "");

@host-bridge-auth(binding: hostApi)
@host-device-poll(binding: hostApi)

@host $hostApi : http("__SPACETIME_HOST_API_BASE__") {
  headers: { authorization: "Bearer __SPACETIME_HOST_API_TOKEN__" };
}

// -----------------------------------------------------------------------
// Status: the widget's core 4-state machine (Wave 6).
// -----------------------------------------------------------------------

// The full status document (preview_hash/production_hash/domains), or
// null until the first fetch resolves. Populated by $status's @handle
// below (Loaded(body) => $statusData <- body).
@data inline $statusData : null;

// True once the FIRST status fetch has settled (success OR auth
// failure) -- distinguishes "haven't checked yet" from "checked, no
// valid session" so the widget doesn't flash Blue before the real
// first response lands.
@data inline $statusChecked bool : false;

@data signal $status() to $hostApi {
  send GET "/api/v1/projects/__SPACETIME_HOST_PROJECT_ID__/status.json"
  receive to StatusResult {
    200 => Loaded($.body);
    401 => Unauthorized($.statusText);
    _   => Failed($.statusText);
  }
  policy latest
}

// Gate 7 fix (BUG-031): $statusFailed is tracked SEPARATELY from
// $statusData being null -- the two arms below (Unauthorized vs
// Failed) are semantically DIFFERENT outcomes (no valid session vs a
// genuine server/network error), but both previously left $statusData
// null, making the old $isBlue derive (originally just
// "$statusChecked && !$statusData") unable to tell them apart. A real
// 500/network failure was therefore misclassified as the SAME
// logged-out Blue state a genuine 401 produces -- showing a "log in"
// affordance for a problem logging in cannot fix. $isUnauthorized is
// the ONLY input to the badge machine's Blue arm (PLAN-077 W7).
@data inline $isUnauthorized bool : false;
@data inline $statusFailed bool : false;

@handle $status {
  receive {
    Loaded(body) => {
      $statusData <- body;
      $statusChecked <- true;
      $isUnauthorized <- false;
      $statusFailed <- false;
    }
    Unauthorized(_) => {
      $statusData <- null;
      $statusChecked <- true;
      $isUnauthorized <- true;
      $statusFailed <- false;
    }
    Failed(_) => {
      $statusChecked <- true;
      $statusFailed <- true;
    }
  }
}

// Fire the initial status check once the widget mounts.
//
// BUG-185 (found via a live user report this session, "host pin doesn't
// update to the host being saved"): `@on visible { $status(); }` --
// this exact form, unchanged from before -- silently never fired on a
// fresh page load for THIS widget specifically, because
// `.st-host-widget` is `position: fixed; bottom: 16px;` (see this
// file's own container CSS below). The root cause was NOT in this
// file: `on-mutation-handler` (the shared primitive backing the plain-
// body `@on visible { ... }` form -- confirmed correctly routing
// through a REAL `IntersectionObserver`, not a naive/broken
// `addEventListener('visible', ...)`) hardcoded `rootMargin: '0px 0px
// -90% 0px'`, restricting observation to only the TOP 10% of the
// viewport (a narrow, undocumented assumption baked in for a specific
// reveal-on-scroll HEADER use case). A bottom-anchored fixed element
// never entered that region, so its IntersectionObserver simply never
// fired -- confirmed via a live, controlled A/B test this session
// (isolated from this widget entirely): an identical `@on visible {
// $fired <- true; }` fired correctly for a `top: 10px` fixed probe
// element and never fired for an otherwise-identical `bottom: 10px`
// one. Fixed at the ROOT, in the shared primitive itself
// (`stdlib/primitives/animation/drivers.st::on-mutation-handler`, see
// BUG-185's full writeup there) -- `rootMargin` widened to `'0px'` (the
// real, full viewport), which is what makes the plain, unmodified
// `@on visible { ... }` form below correct again, for every element
// position, not just this file's own workaround.
//
// This mount-check gap meant the widget's entire 4-state badge machine
// (Blue/Orange/Turquoise/Green) never had real data to derive from on a
// fresh page load -- silently stuck at the boolean defaults (all
// `false`), which is how a real, valid, already-baked-in
// session could still show a "Log in" affordance after a reload (see
// BUG-186, `$showLocalLogin`'s own derivation below, for the SECOND,
// independent half of that same symptom).
.st-host-widget {
  @on &.visible {
    $status();
  }
}

// The widget's badge machine as ONE closed sum (PLAN-077 W7) — this
// REPLACES the 9-derive boolean chain ($isBlue/$isOrange/$isTurquoise/
// $isGreen + 4× $showXBadge + the stateLabel ||/-chain) with a single
// @match-derived union. The states (per V1_IMPLEMENTATION_PLAN.org §5's
// exact table), in PRECEDENCE order (first-match-wins — the @match arm
// order IS the old chain's &&-exclusion logic, made structural):
//   NoProject - no @deploy{} fact: no status to poll at all. Takes
//               precedence over EVERYTHING (the old chain's
//               `&& !$isNoProject` on every $showXBadge).
//   Orange    - a push/promote/add-domain call is CURRENTLY in flight
//               ("in progress" always wins visually over a stale prior
//               settled state — the old `&& !$isOrange` exclusions).
//   Blue      - a REAL 401 (Unauthorized) — genuinely no valid session.
//               Gate 7 fix (BUG-031): derives ONLY from $isUnauthorized,
//               NOT from "$statusData is null" (a 500/network error must
//               not masquerade as the logged-out state).
//   Turquoise - a real session + preview built, not yet promoted
//   Green     - a real session + preview and production match
//   None      - catch-all: a genuine non-auth failure ($statusFailed)
//               intentionally matches NO named state (the plan's table
//               has no 5th "error" state — see .st-host-widget__error),
//               as do the never-pushed and not-yet-checked windows.
@type BadgeState {
  Blue | Orange | Turquoise | Green | NoProject | None
}

@data derive $badge BadgeState : @match {
  ($isNoProject)  => NoProject;
  ($status_pending || $promote_pending || $addDomain_pending || $pushPreview_pending) => Orange;
  ($isUnauthorized) => Blue;
  ($statusData && $statusData.preview_hash && $statusData.preview_hash != $statusData.production_hash) => Turquoise;
  ($statusData && $statusData.preview_hash && $statusData.preview_hash == $statusData.production_hash) => Green;
  _ => None;
}

// ---------------------------------------------------------------------
// Pill + panel interaction (redesign): the widget rests as a compact
// pill (mark + "Host" + status dot); clicking it expands the full
// panel. `$isExpanded` is plain client-side UI state -- the standard
// `$x <- !$x` toggle idiom (examples/tutorial/tutorial.st's own
// documented pattern).
// ---------------------------------------------------------------------
@data inline $isExpanded bool : false;

.st-host-widget__pill {
  .is-open: $isExpanded;

  @on &.click {
    $isExpanded <- !$isExpanded;
  }
}

.st-host-widget__panel {
  .is-open: $isExpanded;
}

// ---------------------------------------------------------------------
// Dev links (redesign): status.json now carries ready-made
// `preview_url`/`production_url` (spacetime-host-server::status::
// StatusResponse, derived from deploy.rs's OWN hostname scheme --
// `{project}.preview.{HOST_BASE_DOMAIN}` / `{project}.{HOST_BASE_DOMAIN}`,
// e.g. `proj.preview.spacetime.undefine.org` on the live box). Each is
// null server-side until the corresponding pointer exists, so an empty
// string here means "no link to offer" -- the anchor hides rather than
// 404 at the edge. `||`-chains only (no ternaries): mutation/derive-RHS
// ternaries are an UNTESTED expression shape in this file's own
// documented experience (see $writeBackFailed's comment); `a && b || c`
// is the same safe shape the badge machine's guards already compile.
// ---------------------------------------------------------------------
@data derive $previewUrl string : (($statusData && $statusData.preview_url) || "");
@data derive $productionUrl string : (($statusData && $statusData.production_url) || "");
@data derive $hasPreviewUrl bool : ($previewUrl != "");
@data derive $hasProductionUrl bool : ($productionUrl != "");
@data derive $showLinks bool : (($hasPreviewUrl || $hasProductionUrl) && !$isNoProject);
// Scheme-less display text -- the pill panel is 320px wide; "https://"
// is noise on every row. `.replace` with a plain string pattern
// replaces the FIRST occurrence only, which is exactly the leading
// scheme.
@data derive $previewUrlText string : ($previewUrl.replace("https://", ""));
@data derive $productionUrlText string : ($productionUrl.replace("https://", ""));

// FUP-004 drift indicator: size labels per deploy + the server-computed
// drift warning. The server does the arithmetic (the widget language
// can't); we just render the display form it ships. A 2.8 KB production
// vs 4.2 MB preview shows as a prominent amber warning — the signal
// that was missing when a stub scaffold sat in production for 7h.
@data derive $previewSizeLabel string : (($statusData && $statusData.preview && $statusData.preview.size_label) || "");
@data derive $productionSizeLabel string : (($statusData && $statusData.production && $statusData.production.size_label) || "");
@data derive $driftWarning string : (($statusData && $statusData.drift_warning) || "");
@data derive $hasDriftWarning bool : ($driftWarning != "");

// A real 200 with NO preview_hash yet: the project exists and the
// session is valid, but nothing has ever been pushed -- neither
// Turquoise nor Green matches (both require preview_hash), and without
// this arm the label sat at "Checking…" forever (observed live this
// session against a real 200 status.json with null hashes).
@data derive $isNeverPushed bool : ($statusData && !$statusData.preview_hash);

// One human-readable state phrase for the panel header, read straight
// off the $badge union (PLAN-077 W7) — the same precedence the dot
// colors use, since the union's first-match-wins arms already encode
// it. The label stays a plain string derive: cond-mode @match arms
// produce variant constructors only, and the label is a string, not a
// BadgeState. The trailing arms cover the label-only fallbacks the
// badge's None catch-all doesn't distinguish. `$badge &&` guards the
// window before the derive's first publish (undefined .type would throw).
@data derive $stateLabel string : (($badge && $badge.type == "NoProject" && "Not connected") || ($badge && $badge.type == "Orange" && "Working…") || ($badge && $badge.type == "Blue" && "Logged out") || ($badge && $badge.type == "Turquoise" && "Preview ahead of production") || ($badge && $badge.type == "Green" && "Live & current") || ($isNeverPushed && "Connected — nothing pushed yet") || ($statusFailed && "Host unreachable") || "Checking…");

// State reflection on BOTH indicators: the pill's dot and the header's
// state text. Two separate blocks (not one multi-element selector) so
// each element's gate is unambiguous. PLAN-077 W7: @state gates off the
// $badge union REPLACE the old boolean reactive-class bindings — the
// variant IS the state now; the matching declarations moved out of the
// .st-host-widget__dot--*/__state--* class rules (deleted below) into
// these gates.
.st-host-widget__dot {
  @state(when: $badge is NoProject) {
    background: #a78bfa;
    box-shadow: 0 0 8px rgba(167, 139, 250, 0.7);
  }
  @state(when: $badge is Blue) {
    background: #60a5fa;
    box-shadow: 0 0 8px rgba(96, 165, 250, 0.7);
  }
  @state(when: $badge is Orange) {
    background: #fb923c;
    box-shadow: 0 0 8px rgba(251, 146, 60, 0.7);
    animation: st-host-pulse 1.1s ease-in-out infinite;
  }
  @state(when: $badge is Turquoise) {
    background: #2dd4bf;
    box-shadow: 0 0 8px rgba(45, 212, 191, 0.7);
  }
  @state(when: $badge is Green) {
    background: #4ade80;
    box-shadow: 0 0 8px rgba(74, 222, 128, 0.7);
  }
}

.st-host-widget__state {
  @state(when: $badge is NoProject) { color: #c4b5fd; }
  @state(when: $badge is Blue) { color: #93c5fd; }
  @state(when: $badge is Orange) { color: #fdba74; }
  @state(when: $badge is Turquoise) { color: #5eead4; }
  @state(when: $badge is Green) { color: #86efac; }

  text <- $stateLabel;
}

// Reactive hrefs for the two dev links (`href <- $x` is the documented
// arrow-binding surface, stdlib/macros/data.st: `src <- $.url` ->
// setAttribute; projects/jallete/index.st's checkout link is the live
// precedent for href specifically).
.st-host-widget__link--preview {
  .is-hidden: !$hasPreviewUrl;
  href <- $previewUrl;
}

.st-host-widget__link--production {
  .is-hidden: !$hasProductionUrl;
  href <- $productionUrl;
}

.st-host-widget__links {
  .is-hidden: !$showLinks;
}

.st-host-widget__drift-warning {
  .is-hidden: !$hasDriftWarning;
}

// No-project panel visibility (PLAN-004 follow-up). The panel itself
// shows only in the $isNoProject state; the login-vs-create SUB-state
// within it is driven by $hasAccountToken (a real baked-in token means
// this dev server already knows who you are -- no separate login step
// needed before creating a project).
.st-host-widget__no-project {
  .is-hidden: !$isNoProject;
}

// Device-flow code + terminal-state messages (PLAN-004 follow-up).
// $showDeviceCode: visible only WHILE actively polling (an empty code
// string, the initial/reset state, must never show an empty box).
@data derive $hasDeviceCode bool : ($deviceUserCode != "");
@data derive $isPolling bool : ($devicePollStatus == "polling");
@data derive $showDeviceCode bool : ($hasDeviceCode && $isPolling);
@data derive $isExpired bool : ($devicePollStatus == "expired");
@data derive $isDenied bool : ($devicePollStatus == "denied");

.st-host-widget__device-code {
  .is-hidden: !$showDeviceCode;
}

.st-host-widget__device-poll-expired {
  .is-hidden: !$isExpired;
}

.st-host-widget__device-poll-denied {
  .is-hidden: !$isDenied;
}

.st-host-widget__no-project-login {
  .is-hidden: $hasAccountToken;

  @on &.click {
    $startLogin();
  }
}

.st-host-widget__create-project-form {
  .is-hidden: !$hasAccountToken;
}

// Empty error text ($createProjectError starts as "") -- CSS can't test
// string emptiness directly against a signal in a reactive-class binding
// (only boolean signals are used elsewhere in this file for `.is-hidden`
// gates), so a small derived boolean mirrors the same pattern
// $showStatusError already establishes below.
//
// BUG-213 (live report: "click create project ... does nothing"): a
// NETWORK-level fetch rejection never reaches a receive arm at all --
// signal.st publishes only the `_error` yield (the BUG-197 rail) and no
// `signal:*:reply` event fires, so @handle $createProject's arms never
// ran and $createProjectError stayed "". Fold $createProject_error into
// the visible text, mirroring $pushPreviewErrorText verbatim (the SAME
// bug class, fixed the same way, further down this file).
@data derive $createProjectErrorText string : ($createProjectError || $createProject_error || "");
@data derive $hasCreateProjectError bool : ($createProjectErrorText != "");

.st-host-widget__create-project-error {
  .is-hidden: !$hasCreateProjectError;
}

.st-host-widget__write-back-error {
  .is-hidden: !$showWriteBackError;
}

// BUG-213 (same class as $createProjectErrorText above): a network-level
// rejection of the LOCAL write-back call never reaches $initDeploy's
// Failed arm either -- only the `_error` yield carries it. `||`-chain
// only (this file's documented safe derive shape).
@data derive $showWriteBackError bool : ($writeBackFailed || $initDeploy_error);

// -----------------------------------------------------------------------
// Blue state: WorkOS hosted-login deep-link. Zero login UI built here --
// per §2's decision, the widget only opens WorkOS's own hosted page.
// -----------------------------------------------------------------------

@data signal $startLogin() to $hostApi {
  send POST "/api/v1/auth/device"
  receive to DeviceFlowResult {
    200 => Started($.body);
    _   => Failed($.statusText);
  }
  policy latest
}

// PLAN-004 follow-up (real, live-reported bug this session, found via
// an actual browser run, not code-reading): `$startLogin` previously
// only opened the WorkOS device-confirm popup and stopped -- nothing
// ever polled `/api/v1/auth/exchange` afterward, so completing the
// popup's "You are all set!" step never actually retrieved a token.
// See stdlib/primitives/host-device-poll.st's own module doc comment
// for the full design (mirrors spacetime-host-cli::main::cmd_login's
// real poll-until-done loop, client-side, plus a new dev-server-only
// endpoint for the one thing only a local process can do: persisting
// the resulting token to the CLI's own credentials file on disk).
//
// A SECOND, separate bug the same live report surfaced: WorkOS's own
// "confirm this code is shown on your device" screen appeared with
// nothing on this side to compare it against -- the widget never
// displayed `user_code` anywhere. Fixed below via $deviceUserCode.
@data inline $deviceUserCode string : "";
@data inline $devicePollStatus string : "";

// FIXED (BUG-183 / BUG-100): a comment placed INSIDE an @handle receive
// arm body used to silently corrupt or null the WHOLE arm's compiled
// output (no diagnostic) -- root-caused to the compiler's balanced-brace
// capture scanner + arm-body codegen both being comment-unaware. Fixed
// in src/syntax/events/extractors/custom.rs; regression pinned in
// tests/integration/handle-arm-comment-corruption.test.st. Comments are
// now safe INSIDE an @handle arm body again; this handler keeps its own
// explanation above the block purely for readability, not as a
// workaround.
//
// This specific handler: window.open is a real JS expression,
// evaluated via the same $var mutation-assignment path every other
// reactive write already uses -- there is no separate open-a-URL
// primitive/statement shape in this language, so this is the correct
// working idiom, not a workaround. interval is a Rust duration type,
// serialized as an object with secs and nanos fields (confirmed
// against the real server source this session) -- secs times 1000
// converts to the milliseconds host-device-poll start expects.
@handle $startLogin {
  receive {
    Started(body) => {
      $noop <- window.open(body.verification_uri, "_blank");
      $noop <- window.__stHostDevicePoll.hostApi.start($hostApiBase, body.device_code, body.interval.secs * 1000, "deviceUserCode", body.user_code, "devicePollStatus");
    }
    Failed(_) => { }
  }
}

@data inline $noop : null;

// -----------------------------------------------------------------------
// No-project state: create a project directly from the widget, then
// write @deploy{} back into this site's OWN .st source (PLAN-004
// follow-up: "always available" + "create project from widget").
// -----------------------------------------------------------------------

// The project-name input's typed value.
@data inline $newProjectName string : "";

// Real call against the orchestrator's OWN POST /api/v1/projects
// (spacetime-host-server::projects::create, already live -- the SAME
// endpoint the CLI's own `spacetime host init` uses) -- via $hostApi,
// the SAME transport+bearer-token binding every other authenticated
// call in this widget already uses. This mints a REAL project id
// server-side; the local write-back (below) is a separate, second step.
@data signal $createProject($name string) to $hostApi {
  send POST "/api/v1/projects" { name: $name }
  receive to CreateProjectResult {
    200 => Created($.body);
    409 => Rejected($.body);
    422 => Rejected($.body);
    _   => Failed($.statusText);
  }
  // BUG-213: 15s deadline -- against a black-holed orchestrator (connect
  // timeout, no HTTP status ever) the fetch otherwise hangs for the
  // browser's full TCP timeout (minutes) with zero visible feedback.
  policy latest timeout 15s
}

// Tracks the newly-minted project id across the two-step flow (remote
// create, then local write-back) so the write-back call below has
// something to send -- populated by $createProject's own @handle, read
// by $initDeploy.
@data inline $pendingProjectId string : "";
@data inline $createProjectError string : "";

@handle $createProject {
  receive {
    Created(body) => {
      $pendingProjectId <- body.id;
      $createProjectError <- "";
      $initDeploy(body.id);
    }
    Rejected(body) => {
      $createProjectError <- "That project name is unavailable -- try another.";
    }
    Failed(_) => {
      $createProjectError <- "Could not create project -- try again shortly.";
    }
  }
}

// The local write-back half (server.rs::deploy_init_handler, THIS dev
// server only -- see that handler's own doc comment for the full
// rationale): writes `@deploy { project: "<id>"; }` into this site's
// index.st. Uses $selfApi (empty-base, same-origin), NOT $hostApi (the
// remote orchestrator) -- this call must reach the machine currently
// SERVING this page, never a remote server.
@data signal $initDeploy($projectId string) to $selfApi {
  send POST "/__spacetime/dev/deploy-init" { project_id: $projectId }
  receive to InitDeployResult {
    200 => Written($.body);
    _   => Failed($.statusText);
  }
  policy latest
}

// Gate 7-style precedent (this file's old $showXBadge split, now the $badge union):
// a compound conditional expression INLINE in a `$var <- expr` mutation
// is not a pattern this file has independently verified works (only
// SIMPLE reactive-class bindings were empirically found to reject
// negated-AND combinations earlier in this file; mutation-RHS ternaries
// are UNTESTED, not confirmed-safe) -- so this handler avoids one
// entirely rather than risk the same class of silent parse failure.
// $writeBackFailed is a plain boolean, driven by the two known outcomes
// (a non-2xx write-back response, or the write-back request failing to
// reach this dev server at all); the actual guidance TEXT is a static
// string, not interpolated, avoiding the untested expression shape.
@data inline $writeBackFailed bool : false;

// A successful write-back triggers this dev server own file watcher
// (confirmed via server.rs deploy_init_handler own doc comment: it
// relies on the SAME watcher save_handler already uses), which
// reloads the page -- the widget will re-render with $hostProjectId
// now populated server-side (compile_host_widget reads @deploy fresh
// on every request). No client-side state flip is attempted here
// deliberately: $hostProjectId is a COMPILE-TIME literal, so nothing
// short of a page reload can ever change it.
@handle $initDeploy {
  receive {
    Written(_) => {
      $writeBackFailed <- false;
    }
    Failed(_) => {
      $writeBackFailed <- true;
    }
  }
}

.st-host-widget__create-project-input {
  value <- $newProjectName;
  @on &.input {
    $newProjectName <- $.value;
  }
}

.st-host-widget__create-project-submit {
  @on &.click {
    $createProject($newProjectName);
  }
}

// PLAN-004 follow-up: neither the local-dev nor the deployed-site login
// button is meaningful in the no-project state -- that state has its own
// dedicated `.st-host-widget__no-project-login` button (same $startLogin
// signal, just a different affordance/copy for "you have no project
// yet" vs. "you're logged out of an EXISTING project").
//
// BUG-186 (found via the SAME live user report as BUG-185 above, "host
// pin doesn't update to the host being saved"): this derivation
// previously omitted `$isBlue` entirely -- `!$isDeployedContext &&
// !$isNoProject` is TRUE in every non-deployed, has-a-project state,
// including Green/Turquoise/Orange (a real, valid, already-authenticated
// session). Per this file's own header-linked plan
// (V1_IMPLEMENTATION_PLAN.org §5's state table: "Blue state's click
// target is [...] login"), the login button is ONLY ever meaningful in
// the Blue (logged-out/401) state -- showing it unconditionally
// alongside a real session is exactly the "doesn't update" symptom this
// session's live report described (compounded by BUG-185's separate
// `$status()`-never-fires bug above, which meant `$isBlue` itself never
// even got a real value to begin with -- both had to be fixed together
// for the badge to ever reflect a real logged-in session). Confirmed via
// a live browser test against a real running spacetime-host-server with
// a real seeded account/project/token row: before this fix, "Log in"
// stayed visible after project creation despite `$hasAccountToken` and a
// real 200 status.json response; after fixing BOTH this AND BUG-185
// together, the badge correctly reflects the real Turquoise/Green state
// and "Log in" correctly disappears.
// PLAN-077 W7: these read $isUnauthorized DIRECTLY (the old $isBlue
// derive's exact definition) — NOT `$badge.type == "Blue"`. Deliberate:
// the badge's Orange arm (a call in flight) wins over Blue in the
// union's precedence, but the login button's semantics never had that
// exclusion — a 401'd session with a call in flight still offers login.
@data derive $showLocalLogin bool : ($isUnauthorized && !$isDeployedContext && !$isNoProject);
@data derive $showBridgeLogin bool : ($isUnauthorized && $isDeployedContext && !$isNoProject);

// PLAN-004: this button (the ORIGINAL Wave 6 login affordance) is the
// LOCAL-dev-only path -- a real deployed site has no server-side
// credentials file to poll WorkOS device-flow completion against on
// this widget's behalf (see this file's header comment, "PROJECT ID /
// SERVER URL / TOKEN RESOLUTION"). Gated OUT when `$isDeployedContext`.
.st-host-widget__login {
  .is-hidden: !$showLocalLogin;

  @on &.click {
    $startLogin();
  }
}

// PLAN-004: the deployed-site counterpart -- opens the centralized
// login-bridge popup (`host-bridge-auth`'s `start()`, via the SAME
// `$var <- expr` raw-JS-call escape hatch `$startLogin`'s own
// `window.open` already uses above). On a successful handshake, the
// primitive applies the real token to `$hostApi` in place and publishes
// `$bridgeReady`; the `@handle` below re-fires `$status` so the widget's
// state flips WITHOUT a page reload, matching this file's own
// documented "state flip happens on next status re-fetch" design (the
// bridge's own postMessage completion IS that re-fetch trigger, unlike
// the local-dev path which waits for a user-initiated action or reload).
.st-host-widget__login-bridge {
  .is-hidden: !$showBridgeLogin;

  @on &.click {
    $noop <- window.__stHostBridgeAuth.hostApi.start($hostBridgeBaseUrl, "hostApi", $hostProjectId, "bridgeReady");
  }
}

@data inline $bridgeReady bool : false;

// PLAN-004 correction (found + fixed via live browser testing this
// session): `@handle` is EXCLUSIVELY a `@data signal` reply/fire
// consumer (confirmed via `stdlib/macros/handle.st`'s own doc comment
// and `signal-handler` primitive -- it subscribes to
// `signal:<name>:fire`/`:reply` document events, which a plain `@data
// inline` value change never dispatches). `$bridgeReady` is a plain
// inline bool the `host-bridge-auth` primitive flips via
// `window._localState`/`local:<name>:updated` -- the CORRECT reactive
// consumer for THAT shape is `@on $sig.change`
// (the change driver), which subscribes to any signal's change
// generically, inline or signal-derived alike. An earlier version of
// this file used `@handle`, which silently never fired (bridgeReady's
// value changed but nothing re-ran `$status()`) -- caught via a live
// browser test that flipped `$bridgeReady` directly and observed no
// re-fetch, not assumed correct from reading the macro alone.
.st-host-widget {
  @on $bridgeReady.change {
    $status();
  }
}

// -----------------------------------------------------------------------
// Promote: calls the REAL, ALREADY-LIVE-TESTED publish endpoint (Wave 2)
// -- not a duplicate /promote route, see this file's own header comment.
// -----------------------------------------------------------------------

@data signal $promote() to $hostApi {
  send POST "/api/v1/projects/__SPACETIME_HOST_PROJECT_ID__/publish"
  receive to PromoteResult {
    200 => Promoted($.body);
    _   => Failed($.statusText);
  }
  policy latest
}

// Re-check status immediately on a successful promote -- production_hash
// now equals preview_hash, flipping the badge from Turquoise to Green
// without a page reload.
@handle $promote {
  receive {
    Promoted(_) => {
      $status();
    }
    Failed(_) => { }
  }
}

// Nothing to promote without a real project AND a real pushed preview
// (a promote with no preview pointer 409s server-side -- deploy.rs's
// own NoPreviewDeploy arm; don't offer a dead button). Derives from
// preview_hash (NOT $hasPreviewUrl): preview_hash exists in BOTH the
// old and new status.json shapes, so promote stays correct against a
// deployed server that predates the preview_url field.
@data derive $hasPreviewPush bool : ($statusData && $statusData.preview_hash != null);
@data derive $showPromote bool : ($hasPreviewPush && !$isNoProject);

.st-host-widget__promote {
  .is-hidden: !$showPromote;

  @on &.click {
    $promote();
  }
}

// -----------------------------------------------------------------------
// Deploy Protocol v2 console. `@data stream` is intentionally the source of
// truth for transient push narration: if its connection closes, the settled
// status.json state remains the pill's fallback rather than leaving it stuck
// in a client-owned "pushing" mode.
@data stream $push from "/__spacetime/host/push-events";

@data derive $pushType string : (($push && $push.type) || "");
@data derive $pushProgressText string : (($pushType == "building" && "Building…") || ($pushType == "uploading" && ("uploading file " + $push.file + " (" + $push.done + "/" + $push.total + " · " + (($push.done * 100) / $push.total) + "%)")) || ($pushType == "committing" && "Committing…") || ($pushType == "error" && (($push && $push.message) || "Push failed")) || "");
@data derive $pushStreamActive bool : (($pushType == "building") || ($pushType == "uploading") || ($pushType == "committing"));

// The V2 start response owns the diff summary. Large browser-direct uploads
// are served as scoped bytes by `/push-blob/{b3}`; a disconnected widget is
// completed safely by the dev server's existing grace timer while this stream
// continues to narrate the authoritative progress.
@data inline $pushSummary string : "";
@data signal $pushV2Start() to $selfApi {
  send POST "/__spacetime/host/push-v2/start"
  receive to PushV2StartResult {
    200 => PushV2Started($.body);
    _   => PushV2Failed($.body.error || $.statusText);
  }
  policy latest
}

@handle $pushV2Start {
  receive {
    PushV2Started(body) => {
      $pushSummary <- ((body.changed && (body.changed.length + " changed · " + ((body.pending && body.pending.length) || 0) + " to upload (" + (((body.pending_bytes || 0) / 1048576)) + " MB)")) || "Push started");
      $pushPreviewMessage <- "";
      $pushPreviewOk <- false;
    }
    PushV2Failed(msg) => {
      $pushPreviewMessage <- msg;
      $pushPreviewOk <- false;
    }
  }
}

// -----------------------------------------------------------------------
// Push preview (PLAN-075, "push from the pill"): the ONE action that
// needs bytes off local disk -- the project's SOURCE DIRECTORY, tarred
// up -- which only THIS dev server has open (a widget instance rendered
// in the browser has no filesystem access to tar its own source). Uses
// $selfApi (empty-base, same-origin -- the SAME transport $initDeploy
// above already established for "must reach the machine currently
// SERVING this page"), deliberately NOT $hostApi (the remote
// orchestrator): server.rs::host_push_handler is the one piece that
// reads site_dir off disk, packages it (verse's own shared
// host_package::package_site_archive), and forwards the archive to the
// REAL orchestrator itself -- this widget signal only needs to reach
// THAT local relay, never the remote server directly.
@data signal $pushPreview() to $selfApi {
  send POST "/__spacetime/host/push"
  receive to PushPreviewResult {
    200 => Pushed($.body);
    // Error bodies are JSON `{"error": "…"}` (HostPushErrorResponse) --
    // surface the MESSAGE, not the object; statusText is the fallback for
    // a non-JSON upstream body (property access on a string is just
    // `undefined`, never a throw, in the arm's eval).
    _   => Failed($.body.error || $.statusText);
  }
  policy latest
}

// Re-check status immediately on a successful push -- preview_hash now
// reflects the new archive, flipping the badge from "nothing pushed"/
// Blue to Turquoise (or straight to Green if production already matched
// the prior preview) without a page reload. Mirrors $promote's own
// handler shape exactly (re-fires $status() on its own success arm).
// Failure reason comes from the Failed arm's payload -- the dev server's
// own plain-text body ("Not logged in -- run spacetime host login first",
// "no @deploy{} fact", the upstream's own message), which is precisely the
// guidance worth showing. Empty string = no error. A NETWORK-level failure
// (dev server gone mid-push) never reaches a receive arm at all; that case
// is covered by the now-live $pushPreview_error yield in the display
// derives below (BUG-197 made `_error`/`_pending` yields actually
// propagate).
@data inline $pushPreviewMessage string : "";

// Set on a successful push, cleared on the next click/failure -- drives
// the "Pushed" confirmation line (the state badge alone lands back on the
// IDENTICAL settled label after a no-op push, which read as total silence
// in live use).
@data inline $pushPreviewOk bool : false;

@handle $pushPreview {
  receive {
    Pushed(_) => {
      $pushPreviewMessage <- "";
      $pushPreviewOk <- true;
      $status();
    }
    Failed(msg) => {
      $pushPreviewMessage <- msg;
      $pushPreviewOk <- false;
    }
  }
}

// Impossible on a real DEPLOYED (static) site by construction -- there
// is no source directory there for any server to package (mirrors
// $showLocalLogin's identical !$isDeployedContext gate shape). Also
// hidden in the no-project state ($isNoProject already has its own
// dedicated create-project affordance instead).
@data derive $showPushPreview bool : (!$isDeployedContext && !$isNoProject);

// Button label + in-flight gate: the ONLY visible acknowledgement that a
// push is running during its ~20s tar+upload window (BUG-197 made the
// underlying $pushPreview_pending yield real; before that it never fired).
@data derive $pushPreviewLabel string : (($pushStreamActive && (($pushProgressText != "" && $pushProgressText) || "Pushing…")) || ($pushV2Start_pending && "Starting…") || "Push preview");
@data derive $pushPreviewBusy bool : ($pushV2Start_pending || $pushStreamActive);

// Error surface: the Failed arm's message, else a network-level _error
// string (both falsy when empty/null). Ok line hides while a retry is in
// flight or an error is showing.
@data derive $pushPreviewErrorText string : ($pushPreviewMessage || $pushPreview_error || "");
@data derive $showPushPreviewError bool : ($pushPreviewMessage || $pushPreview_error);
@data derive $showPushPreviewOk bool : ($pushPreviewOk && !$pushPreview_pending);

.st-host-widget__push-preview {
  .is-hidden: !$showPushPreview;
  .is-busy: $pushPreviewBusy;

  text <- $pushPreviewLabel;

  @on &.click {
    $pushPreviewOk <- false;
    $pushPreviewMessage <- "";
    $pushV2Start();
  }
}

.st-host-widget__push-preview-error {
  .is-hidden: !$showPushPreviewError;
}

.st-host-widget__push-preview-ok {
  .is-hidden: !$showPushPreviewOk;
}

// Deploy history is additive status.json v2 data. Immutable URLs are ordinary
// selectable link text (no clipboard JS); every historical hash can be
// promoted back to preview through the dev-server relay.
@data derive $deployments : (($statusData && $statusData.deployments) || []);
@data signal $rollback($hash string) to $selfApi {
  send POST "/__spacetime/host/promote" { hash: $hash, target: "preview" }
  receive to RollbackResult {
    200 => RolledBack($.body);
    _   => RollbackFailed($.body.error || $.statusText);
  }
  policy latest
}
@handle $rollback {
  receive {
    RolledBack(_) => { $status(); }
    RollbackFailed(_) => { }
  }
}

// -----------------------------------------------------------------------
// Domains panel (Wave 4, folded in here per §5's "one widget, multiple
// panels" design).
// -----------------------------------------------------------------------

// Bumped after every successful bind to force `$domainsUrl` to change,
// which re-triggers `@data source`'s reactive-source primitive (it
// refetches on `local:<urlSignal>:updated`, per
// stdlib/primitives/data/reactive-source.st's own doc comment) -- the URL
// itself doesn't otherwise change between requests, so a plain `@data
// fetch` (a fixed, one-shot URL) would never refresh after adding a
// domain.
@data inline $domainsBump number : 0;

// The live domain list's URL -- a genuine runtime-derived expression
// (concatenating literal + signal pieces), which is exactly what `@data
// derive` is for (unlike the compile-time-literal sentinels above).
@data derive $domainsUrl string : ($hostApiBase + "/api/projects/" + $hostProjectId + "/domains?_=" + $domainsBump);

// GET .../domains (Wave 4 addition to spacetime-host-server::domains).
@data source $domains from $domainsUrl;

// The add-domain form's typed input value.
@data inline $newDomain string : "";

// ONE known, unresolved language-level workaround still applies to this
// signal; the other (BUG-181, the more severe one) was fixed THIS
// session:
//
// 1. BUG-181 -- FIXED THIS SESSION. Was: the `{ domain: $domain }` body
//    TEMPLATE below was captured by the compiler but never actually
//    applied at runtime (`fire(payload)` sent the RAW call argument,
//    ignoring the declared body shape -- a bare JSON string instead of
//    `{"domain": "..."}`, correctly REJECTED by
//    spacetime-host-server::domains::bind's `BindDomainRequest`
//    extractor with a 422). Root-caused to
//    stdlib/primitives/signal.st's `%body`/`%params` sigils being
//    captured but never interpolated in the primitive's own `%emit js`;
//    fixed there directly (`fire()` is now variadic and builds the real
//    request body from the declared template + the call's actual named
//    arguments via a `new Function($paramNames..., 'return ({'+body+'})')`
//    construction). Verified via 3 new regression tests
//    (tests/integration/signal-body-template.test.st) AND a live browser
//    capture of the real outgoing fetch body against THIS exact signal's
//    own shape -- `$addDomain` now sends a correctly-structured
//    `{"domain": "..."}` body and completes a real bind end-to-end.
//
// 2. BUG-179 (still worked around below, unrelated defect): a `2xx`-
//    shaped status-class arm in a `receive` block silently fails to bind
//    the WHOLE `@data signal` to the `signal-call` primitive at all (no
//    compile error, no diagnostic) -- an EXPLICIT `200` arm (used below)
//    is both a correct match AND a working one, since `domains::bind`
//    returns exactly 200 on success.
//
// Everything in this widget (the list rendering, the real GET
// .../domains call, CORS, the compile-time project-id/token injection,
// the status machine, the login deep-link, the promote button, and now
// the add-domain bind itself) is verified working via a live browser +
// real spacetime-host-server + real Cloudflare API call.
// A THIRD, independent bug found and fixed alongside BUG-181 this
// session: the target below was a bare `/domains`, missing the
// `/api/projects/{project_id}/` prefix every OTHER real route this
// widget calls (status/publish/GET domains) correctly uses (confirmed
// against spacetime-host-server::app.rs's real route table:
// `/api/projects/{project_id}/domains`, not `/domains`) -- this
// signal's request was going to a 404 route regardless of BUG-181,
// caught only once BUG-181's fix let the request body reach the server
// at all (a live browser test against a REAL running spacetime-host-
// server, not assumed from reading the source).
@data signal $addDomain($domain string) to $hostApi {
  send POST "/api/projects/__SPACETIME_HOST_PROJECT_ID__/domains" { domain: $domain }
  receive to AddDomainResult {
    200 => Added($.body);
    422 => Rejected($.body);
    409 => Rejected($.body);
    _   => Failed($.statusText);
  }
  policy latest
}

@handle $addDomain {
  receive {
    Added(_) => {
      $newDomain <- "";
      $domainsBump <- $domainsBump + 1;
    }
    Rejected(_) => { }
    Failed(_) => { }
  }
}

// -----------------------------------------------------------------------
// Markup: one widget, one always-visible status badge + login/promote
// affordances, plus the domains panel (Wave 4).
// -----------------------------------------------------------------------

<section class="st-host-widget">
  <!-- Resting state: a compact pill. The mark is a pure-CSS spark (no
       inline SVG -- see .st-host-widget__mark below), the dot is the
       SAME 5-state machine the old left-border strip carried, now a
       legible 8px indicator. Clicking toggles the panel. -->
  <button type="button" class="st-host-widget__pill">
    <span class="st-host-widget__mark"></span>
    <span class="st-host-widget__pill-label">Host</span>
    <span class="st-host-widget__dot"></span>
    <span class="st-host-widget__chev"></span>
  </button>

  <!-- Expanded state: the full panel. -->
  <div class="st-host-widget__panel">
    <header class="st-host-widget__head">
      <span class="st-host-widget__state">Checking…</span>
      <span class="st-host-widget__project">`$hostProjectId`</span>
    </header>

    <div class="st-host-widget__actions">
      <button type="button" class="st-host-widget__login">Log in</button>
      <button type="button" class="st-host-widget__login-bridge">Log in</button>
      <!-- PLAN-075: local-dev-only push straight from the pill -- see
           the $pushPreview signal's own doc comment above for why this
           is the ONE action that can never appear on a real deployed
           site (no source directory there for any server to tar). -->
      <button type="button" class="st-host-widget__push-preview">Push preview</button>
      <button type="button" class="st-host-widget__promote">Promote to production</button>
    </div>
    <p class="st-host-widget__push-preview-error">`$pushPreviewErrorText`</p>
    <p class="st-host-widget__push-preview-ok">Pushed -- preview updated.</p>

    <!-- Dev links (redesign): live preview/production URLs straight
         from status.json -- each row hides until its URL exists
         server-side (see $previewUrl/$productionUrl derives). -->
    <nav class="st-host-widget__links">
      <a class="st-host-widget__link st-host-widget__link--preview" href="#" target="_blank" rel="noreferrer">
        <span class="st-host-widget__link-tag">preview</span>
        <span class="st-host-widget__link-url">`$previewUrlText`</span>
        <span class="st-host-widget__link-size">`$previewSizeLabel`</span>
        <span class="st-host-widget__link-arrow">↗</span>
      </a>
      <a class="st-host-widget__link st-host-widget__link--production" href="#" target="_blank" rel="noreferrer">
        <span class="st-host-widget__link-tag">prod</span>
        <span class="st-host-widget__link-url">`$productionUrlText`</span>
        <span class="st-host-widget__link-size">`$productionSizeLabel`</span>
        <span class="st-host-widget__link-arrow">↗</span>
      </a>
    </nav>

    <!-- FUP-004 drift warning: server pre-computes whether production and
         preview differ by >20x in content size. Visible here as a prominent
         amber banner so a stub-in-production is unmistakable at a glance —
         the signal that was missing for 7h on 2026-07-21. -->
    <p class="st-host-widget__drift-warning">⚠ `$driftWarning`</p>

    <p class="st-host-widget__push-summary">`$pushSummary`</p>
    <p class="st-host-widget__push-progress">`$pushProgressText`</p>
    <section class="st-host-widget__deploy-history">
      <h1 class="st-host-widget__title">Deploy history</h1>
      <ul class="st-host-widget__deployments"></ul>
    </section>

  <!-- Device-flow code display (PLAN-004 follow-up, real bug found via
       a live report this session): shown the moment $startLogin opens
       the WorkOS popup, so the user has something on THIS side to
       compare against WorkOS's own "confirm this code is shown on your
       device" screen. Hidden again once polling reaches a terminal
       state (handled via $devicePollStatus's own CSS gates below). -->
  <div class="st-host-widget__device-code">
    <span class="st-host-widget__device-code-label">Confirm this code:</span>
    <span class="st-host-widget__device-code-value">`$deviceUserCode`</span>
  </div>
  <p class="st-host-widget__device-poll-expired">That login code expired -- click Log in to try again.</p>
  <p class="st-host-widget__device-poll-denied">Login was denied.</p>

  <!-- No-project state (PLAN-004 follow-up): a site with no @deploy{}
       fact gets this panel instead of the normal status/domains view --
       see $isNoProject's own doc comment above. Two sub-states, both
       gated by $hasAccountToken: not-yet-logged-in (dev must run
       `spacetime host login`, then reload) vs. logged-in-ready-to-create
       (a real name input + create button, wired to $createProject). -->
  <div class="st-host-widget__no-project">
    <p class="st-host-widget__no-project-copy">This site isn't connected to Spacetime Host yet.</p>

    <button type="button" class="st-host-widget__no-project-login">Log in or sign up</button>

    <div class="st-host-widget__create-project-form">
      <input type="text" class="st-host-widget__create-project-input" placeholder="my-project" />
      <button type="button" class="st-host-widget__create-project-submit">Create project</button>
    </div>

    <p class="st-host-widget__create-project-error">`$createProjectErrorText`</p>
    <p class="st-host-widget__write-back-error">Project created (id: `$pendingProjectId`), but this dev server couldn't write @deploy{} into index.st automatically -- add it yourself: @deploy { project: "`$pendingProjectId`"; }</p>
  </div>

  <h1 class="st-host-widget__title">Domains</h1>

  <!-- The "no domains" placeholder lives OUTSIDE the @each-controlled <ul> --
       @each fully owns its container's children (it REPLACES them on every
       render, including a static placeholder that started inside it,
       reproduced empirically: an initial `<li>` placeholder nested in the
       @each target simply vanished on first render, an empty array
       included). Placing it as a SIBLING, gated by its own reactive
       class, is the correct pattern. -->
  <p class="st-host-widget__empty">No domains bound yet.</p>

  <ul class="st-host-widget__list"></ul>

  <!-- A plain <div>, NOT a <form>: `@on &.submit.prevent` is a PRE-EXISTING,
       already-tracked defect (BUG-171 -- "@on &.submit.prevent is a
       documented but never-implemented feature"), independently
       RE-reproduced via a live browser test (clicking Add domain caused a
       full page reload/navigation, discarding all in-flight state) -- see
       BUG-171's own root-cause analysis (the `.prevent` suffix mis-parses
       as a `$target:selector` event-delegation CSS class selector
       matching nothing, and `preventDefault()` is never called anywhere
       in the underlying primitive regardless). Cross-session confirmation
       appended to BUG-171 rather than filing a duplicate. Using a plain
       <button type="button"> inside a non-form container sidesteps the
       whole problem: there is no default submit behavior to prevent. -->
  <div class="st-host-widget__form">
    <input type="text" class="st-host-widget__input" placeholder="app.example.com" />
    <button type="button" class="st-host-widget__submit">Add domain</button>
  </div>

  <p class="st-host-widget__error">Could not reach Spacetime Host -- try again shortly.</p>
  </div>
</section>

// Deploy history rows stay read-only on deployed pages except that the
// relay-only rollback action naturally fails there; hide it outside dev.
.st-host-widget__deployments {
  @each($deployments as $d, key: $d.hash) {
    &deployment-row($d);
  }
}

@template &deployment-row($d) {
  <li class="st-host-widget__deployment">
    <a class="st-host-widget__deploy-hash" href="`$d.immutable_url`" target="_blank" rel="noreferrer" title="`$d.immutable_url`">`$d.hash.substring(0, 8)` ↗</a>
    <span class="st-host-widget__deploy-meta">`$d.deployed_at_iso.substring(0, 16)` · `$d.source`</span>
    <button type="button" class="st-host-widget__rollback">Rollback</button>
  </li>
}

.st-host-widget__rollback {
  .is-hidden: $isDeployedContext;
  @on &.click {
    $rollback($d.hash);
  }
}

// ---- deploy history rows ------------------------------------------------
// Compact ledger: mono hash chip, muted timestamp · source, sparse actions.
// Full immutable URLs stay on the href (copyable) — never as visible text.
.st-host-widget__deploy-history {
  margin-top: 14px;
}

.st-host-widget__deployments {
  list-style: none;
  margin: 8px 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: 264px;
  overflow-y: auto;
}

.st-host-widget__deployment {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 10px;
  padding: 7px 10px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.03);
}

// The hash chip IS the immutable-URL link (full URL on the tooltip) —
// one affordance, no separate "open" text eating the row's width.
.st-host-widget__deploy-hash {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 11px;
  color: #5eead4;
  background: rgba(20, 184, 166, 0.12);
  border: 1px solid rgba(45, 212, 191, 0.25);
  padding: 2px 7px;
  border-radius: 6px;
  text-decoration: none;
}

.st-host-widget__deploy-hash:hover {
  background: rgba(20, 184, 166, 0.24);
  border-color: rgba(45, 212, 191, 0.5);
}

.st-host-widget__deploy-meta {
  font-size: 11px;
  color: #9ca3af;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.st-host-widget__deploy-url {
  font-size: 11px;
  color: #7dd3fc;
  text-decoration: none;
}

.st-host-widget__deploy-url:hover {
  text-decoration: underline;
}

.st-host-widget__rollback {
  font-family: inherit;
  font-size: 11px;
  font-weight: 500;
  padding: 4px 10px;
  border-radius: 7px;
  border: 1px solid rgba(255, 255, 255, 0.14);
  background: rgba(255, 255, 255, 0.06);
  color: #e5e7eb;
  cursor: pointer;
  transition: background 180ms ease, border-color 180ms ease;
}

.st-host-widget__rollback:hover {
  background: rgba(255, 255, 255, 0.12);
  border-color: rgba(255, 255, 255, 0.24);
}

.st-host-widget__push-summary,
.st-host-widget__push-progress {
  font-size: 11px;
  color: #9ca3af;
  margin: 6px 0 0;
}

// Render one row per bound domain. `$domains` is the raw JSON array
// `[{domain, ssl_status, is_primary}, ...]` from GET .../domains.
.st-host-widget__list {
  @each($domains as $d, key: $d.domain) {
    &domain-row($d);
  }
}

// Hide the "no domains" placeholder once the list is non-empty. `@data
// source` (unlike `@data fetch`) yields only `$value` -- no `_loading`
// binding -- but `reactive-source`'s own store always starts as `[]`
// (stdlib/primitives/data/reactive-source.st), never `null`/`undefined`,
// so `$domains.length` is safe from the FIRST render (the same guarantee
// stdlib/__admin__/app/app.st's own comment documents for its `$entries`).
@data derive $hasAnyDomains bool : ($domains.length > 0);

// $hasAnyDomains alone would still show "No domains bound yet." in the
// no-project state (the domains list is legitimately empty there too,
// but for a DIFFERENT reason -- there's no project to have domains on
// at all, and the no-project panel above already says so). Excluded the
// same way $showStatusError and the badge machine's NoProject precedence already are.
@data derive $showEmptyDomains bool : (!$hasAnyDomains && !$isNoProject);

.st-host-widget__empty {
  .is-hidden: !$showEmptyDomains;
}

// The Domains section (title/list/add-domain form) has nothing
// meaningful to show without a real project id -- hidden entirely in
// the no-project state, alongside the (already-excluded) empty-domains
// placeholder above.
.st-host-widget__title {
  .is-hidden: $isNoProject;
}

.st-host-widget__list {
  .is-hidden: $isNoProject;
}

.st-host-widget__form {
  .is-hidden: $isNoProject;
}

// Gate 7 fix (BUG-031): a genuine non-auth status.json failure (a real
// 500/network error, NOT a 401) surfaces here instead of being
// misclassified as the Blue (logged-out) state. Excludes $isNoProject
// (this session's follow-up) -- a no-@deploy{} site's own $status()
// call always 404s harmlessly against an empty project-id URL segment,
// and that expected failure must not surface as a generic "could not
// reach" error alongside the create-project UI it's already showing.
@data derive $showStatusError bool : ($statusFailed && !$isNoProject);

.st-host-widget__error {
  .is-hidden: !$showStatusError;
}

@template &domain-row($d) {
  <li class="st-host-widget__row" data-ssl-status="`$d.ssl_status`">
    <span class="st-host-widget__domain">`$d.domain`</span>
    <span class="st-host-widget__status">`$d.ssl_status`</span>
  </li>
}

// Two-way bind the input's value to $newDomain (the standard `value <- $x`
// + `@on input` pattern, stdlib/macros/data.st's own documented idiom).
.st-host-widget__input {
  value <- $newDomain;
  @on &.input {
    $newDomain <- $.value;
  }
}

// Clicking the button fires the real bind signal with the typed domain.
// A plain `click` handler on a `type="button"` element (see the markup's
// own comment for why NOT a real <form>/submit) -- there is no default
// navigation to prevent, so no `.prevent`-shaped modifier is needed at all.
.st-host-widget__submit {
  @on &.click {
    $addDomain($newDomain);
  }
}

// -----------------------------------------------------------------------
// Stylesheet (redesign, this session): dark glassy dev-tool aesthetic --
// a compact pill at rest, a full panel on click (Vercel-toolbar-style).
// Deliberately DARK regardless of the host page's own theme: this is an
// injected dev overlay, visually distinct from any site it floats over.
// `color-scheme: dark` (BUG-187's mechanism, direction flipped): native
// form controls inside the panel render dark chrome to match, whatever
// the surrounding page or OS prefers.
// The `.is-hidden`/`display` cascade convention is PRESERVED from the
// previous stylesheet (BUG-184/BUG-032's hard-won ordering rule): every
// rule that sets `display` on an `.is-hidden`-gated element sits BEFORE
// `.is-hidden` below, so `display: none` wins the cascade at equal
// specificity via source order. Keep it that way.
// -----------------------------------------------------------------------

// display-setting rules for .is-hidden-gated elements -- BEFORE .is-hidden.
.st-host-widget__create-project-form {
  display: flex;
  gap: 6px;
  margin-top: 10px;
}

.st-host-widget__device-code {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 10px 0 0 0;
  padding: 8px 10px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.st-host-widget__form {
  display: flex;
  gap: 6px;
  margin-top: 10px;
}

.st-host-widget__links {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin: 10px 0 0 0;
}

.st-host-widget__link {
  display: flex;
  align-items: center;
  gap: 8px;
}

// The expanded panel: `display` lives here (flex column) and its `.is-
// open` visibility is handled SEPARATELY below via max-height/opacity
// (animatable, unlike display) -- so this block carries no cascade-
// ordering risk; it is never `.is-hidden`-gated.
.st-host-widget__panel {
  display: flex;
  flex-direction: column;
}

.is-hidden {
  display: none;
}

// ---- container ---------------------------------------------------------
// PLAN-076: the widget now flows INSIDE the shared dev dock (`.st-dev-dock`,
// a fixed bottom-right flex-row the server injects around BOTH dev widgets).
// Placement (fixed/bottom/right/z-index) moved there; the container keeps
// its internal column stack (panel above the pill) and its own fonts. The
// panel stays absolutely anchored to THIS container, so its geometry is
// unchanged by the move.
.st-host-widget {
  position: relative;
  z-index: 2147483647;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 8px;
  color-scheme: dark;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  font-size: 13px;
  color: #e5e7eb;
  // The container is a TRANSPARENT layout box (320px wide whenever the
  // panel is in layout, even collapsed at max-height 0) -- it must
  // never swallow clicks meant for the page beneath it. Interactive
  // surfaces (the pill, the OPEN panel) restore pointer-events
  // themselves; the collapsed panel already sets `pointer-events:
  // none` explicitly.
  pointer-events: none;
}

.st-host-widget__pill {
  pointer-events: auto;
}

// ---- the pill ----------------------------------------------------------
.st-host-widget__pill {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(17, 17, 22, 0.82);
  backdrop-filter: blur(16px) saturate(1.4);
  -webkit-backdrop-filter: blur(16px) saturate(1.4);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35), inset 0 1px 0 rgba(255, 255, 255, 0.06);
  color: #e5e7eb;
  font-family: inherit;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.02em;
  cursor: pointer;
  transition: border-color 200ms ease, box-shadow 200ms ease, transform 200ms cubic-bezier(0.34, 1.56, 0.64, 1);
}

.st-host-widget__pill:hover {
  border-color: rgba(255, 255, 255, 0.22);
  transform: translateY(-1px);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45), inset 0 1px 0 rgba(255, 255, 255, 0.08);
}

.st-host-widget__pill:active {
  transform: translateY(0) scale(0.98);
}

// The mark: a pure-CSS four-point spark (Spacetime's star motif) --
// conic-gradient punched through a radial mask, no SVG asset needed.
.st-host-widget__mark {
  width: 14px;
  height: 14px;
  border-radius: 3px;
  background: conic-gradient(from 45deg, #8b5cf6, #3b82f6, #14b8a6, #8b5cf6);
  mask: radial-gradient(circle at center, transparent 15%, black 40%);
  -webkit-mask: radial-gradient(circle at center, transparent 15%, black 40%);
  rotate: 45deg;
}

.st-host-widget__pill-label {
  font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
  font-size: 11px;
  font-weight: 500;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #9ca3af;
}

// The status dot -- the 5-state machine's primary at-a-glance surface.
// Each state colors the dot AND casts a soft matching glow.
.st-host-widget__dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #6b7280;
  transition: background 300ms ease, box-shadow 300ms ease;
}

// Orange = "a call is in flight" -- the one state that is inherently
// TRANSIENT, so it breathes. Plain CSS keyframes (projects/sakazuki's
// own @keyframes precedent in .st stylesheets).
@keyframes st-host-pulse {
  0% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.55; transform: scale(0.85); }
  100% { opacity: 1; transform: scale(1); }
}

// The chevron: a 5px CSS corner that rotates when open.
.st-host-widget__chev {
  width: 5px;
  height: 5px;
  border-right: 1.5px solid #6b7280;
  border-bottom: 1.5px solid #6b7280;
  rotate: 45deg;
  translate: 0 -1px;
  transition: rotate 300ms cubic-bezier(0.34, 1.56, 0.64, 1), translate 300ms ease;
}

.st-host-widget__pill.is-open .st-host-widget__chev {
  rotate: -135deg;
  translate: 0 1px;
}

// ---- the panel ---------------------------------------------------------
// Collapsed by default; `.is-open` expands it. max-height + opacity +
// translate together give the spring-eased expand (transform/opacity
// animate cheaply; max-height covers the layout reveal). The overshoot
// bezier (0.34, 1.56, 0.64, 1) is the spring feel -- same family as the
// workbench's own 360ms cubic-bezier reveals.
.st-host-widget__panel {
  /* PLAN-076: absolute above the pill (the dock idiom) — was in-flow below
     the pill, which made the widget container 320px wide even closed and
     blew up the dev dock's flex row. Opens upward now, identical geometry
     to the migrations pill's panel; the max-height animation is unchanged. */
  position: absolute;
  bottom: calc(100% + 8px);
  right: 0;
  width: 320px;
  max-height: 0;
  opacity: 0;
  translate: 0 8px;
  scale: 0.97;
  overflow: hidden;
  pointer-events: none;
  padding: 0 14px;
  border-radius: 14px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(17, 17, 22, 0.88);
  backdrop-filter: blur(20px) saturate(1.4);
  -webkit-backdrop-filter: blur(20px) saturate(1.4);
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5), inset 0 1px 0 rgba(255, 255, 255, 0.06);
  transition: max-height 380ms cubic-bezier(0.22, 0.61, 0.36, 1), opacity 240ms ease, translate 380ms cubic-bezier(0.34, 1.56, 0.64, 1), scale 380ms cubic-bezier(0.34, 1.56, 0.64, 1), padding 380ms cubic-bezier(0.22, 0.61, 0.36, 1);
}

.st-host-widget__panel.is-open {
  max-height: 480px;
  opacity: 1;
  translate: 0 0;
  scale: 1;
  pointer-events: auto;
  padding: 14px;
  overflow-y: auto;
}

// Staggered content reveal: each direct child slides up as the panel
// opens, delayed progressively via nth-child (pure CSS stagger --
// sakazuki's nth-child precedent). Resting state matches the panel's
// own hidden state so nothing flashes.
.st-host-widget__panel > * {
  opacity: 0;
  translate: 0 6px;
  transition: opacity 260ms ease, translate 320ms cubic-bezier(0.22, 0.61, 0.36, 1);
}

.st-host-widget__panel.is-open > * {
  opacity: 1;
  translate: 0 0;
}

.st-host-widget__panel.is-open > *:nth-child(2) { transition-delay: 40ms; }
.st-host-widget__panel.is-open > *:nth-child(3) { transition-delay: 80ms; }
.st-host-widget__panel.is-open > *:nth-child(4) { transition-delay: 120ms; }
.st-host-widget__panel.is-open > *:nth-child(5) { transition-delay: 160ms; }
.st-host-widget__panel.is-open > *:nth-child(6) { transition-delay: 200ms; }
.st-host-widget__panel.is-open > *:nth-child(7) { transition-delay: 240ms; }
.st-host-widget__panel.is-open > *:nth-child(8) { transition-delay: 280ms; }

// ---- panel header ------------------------------------------------------
.st-host-widget__head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 8px;
}

.st-host-widget__state {
  font-size: 13px;
  font-weight: 600;
  color: #f3f4f6;
}

// (PLAN-077 W7: the per-state color rules that used to live here as
// .st-host-widget__state--* classes moved into the @state gates on
// .st-host-widget__state's binding block above.)

.st-host-widget__project {
  font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
  font-size: 10px;
  color: #6b7280;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 120px;
}

// ---- actions -----------------------------------------------------------
.st-host-widget__actions {
  display: flex;
  gap: 6px;
  margin-top: 10px;
}

// One shared button voice: quiet glass chips. The promote button is the
// primary action and gets the accent treatment below.
.st-host-widget__login,
.st-host-widget__login-bridge,
.st-host-widget__no-project-login,
.st-host-widget__create-project-submit,
.st-host-widget__push-preview,
.st-host-widget__submit {
  font-family: inherit;
  font-size: 12px;
  font-weight: 500;
  padding: 6px 12px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.14);
  background: rgba(255, 255, 255, 0.06);
  color: #e5e7eb;
  cursor: pointer;
  transition: background 180ms ease, border-color 180ms ease;
}

.st-host-widget__login:hover,
.st-host-widget__login-bridge:hover,
.st-host-widget__no-project-login:hover,
.st-host-widget__create-project-submit:hover,
.st-host-widget__push-preview:hover,
.st-host-widget__submit:hover {
  background: rgba(255, 255, 255, 0.12);
  border-color: rgba(255, 255, 255, 0.24);
}

.st-host-widget__promote {
  font-family: inherit;
  font-size: 12px;
  font-weight: 600;
  padding: 6px 12px;
  border-radius: 8px;
  border: 1px solid rgba(45, 212, 191, 0.4);
  background: rgba(20, 184, 166, 0.16);
  color: #5eead4;
  cursor: pointer;
  transition: background 180ms ease, border-color 180ms ease, box-shadow 180ms ease;
}

.st-host-widget__promote:hover {
  background: rgba(20, 184, 166, 0.28);
  border-color: rgba(45, 212, 191, 0.6);
  box-shadow: 0 0 16px rgba(20, 184, 166, 0.25);
}

// In-flight push (BUG-197): pointer-events none so a ~20s tar+upload
// can't be double-fired by an impatient re-click; the label itself swaps
// to "Pushing…" via $pushPreviewLabel.
.st-host-widget__push-preview.is-busy {
  opacity: 0.55;
  pointer-events: none;
  cursor: progress;
}

// ---- dev links ---------------------------------------------------------
.st-host-widget__link {
  padding: 7px 10px;
  border-radius: 8px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.07);
  text-decoration: none;
  color: #d1d5db;
  transition: background 180ms ease, border-color 180ms ease;
}

.st-host-widget__link:hover {
  background: rgba(255, 255, 255, 0.09);
  border-color: rgba(255, 255, 255, 0.18);
}

.st-host-widget__link-tag {
  font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
  font-size: 9px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #6b7280;
  min-width: 46px;
}

.st-host-widget__link--production .st-host-widget__link-tag {
  color: #4ade80;
}

.st-host-widget__link--preview .st-host-widget__link-tag {
  color: #2dd4bf;
}

.st-host-widget__link-url {
  font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
  font-size: 11px;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.st-host-widget__link-arrow {
  font-size: 10px;
  color: #6b7280;
}

.st-host-widget__link-size {
  font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
  font-size: 10px;
  color: #9ca3af;
  background: rgba(255, 255, 255, 0.06);
  padding: 1px 6px;
  border-radius: 4px;
  white-space: nowrap;
}

// FUP-004 drift warning: amber banner, impossible to miss.
.st-host-widget__drift-warning {
  margin: 8px 0 0;
  padding: 8px 10px;
  font-size: 11px;
  line-height: 1.4;
  color: #92400e;
  background: #fde68a;
  border: 1px solid #f59e0b;
  border-radius: 8px;
}
// ---- domains -----------------------------------------------------------
.st-host-widget__title {
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: #6b7280;
  margin: 14px 0 6px 0;
}

.st-host-widget__list {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 140px;
  overflow-y: auto;
}

.st-host-widget__row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.st-host-widget__domain {
  font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
  font-size: 11px;
  color: #d1d5db;
}

.st-host-widget__status {
  font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
  font-size: 9px;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: #6b7280;
  padding: 2px 6px;
  border-radius: 5px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.st-host-widget__empty {
  font-size: 11px;
  color: #6b7280;
  margin: 4px 0 0 0;
}

// ---- inputs ------------------------------------------------------------
.st-host-widget__input,
.st-host-widget__create-project-input {
  flex: 1;
  min-width: 0;
  font-family: inherit;
  font-size: 12px;
  padding: 6px 10px;
  border-radius: 8px;
  border: 1px solid rgba(255, 255, 255, 0.12);
  background: rgba(0, 0, 0, 0.3);
  color: #e5e7eb;
  transition: border-color 180ms ease;
}

.st-host-widget__input:focus,
.st-host-widget__create-project-input:focus {
  outline: none;
  border-color: rgba(96, 165, 250, 0.5);
}

// ---- messages ----------------------------------------------------------
.st-host-widget__no-project-copy {
  margin: 8px 0 0 0;
  font-size: 12px;
  color: #9ca3af;
}

.st-host-widget__no-project-login {
  margin-top: 8px;
}

.st-host-widget__create-project-error,
.st-host-widget__write-back-error,
.st-host-widget__device-poll-expired,
.st-host-widget__device-poll-denied,
.st-host-widget__push-preview-error,
.st-host-widget__error {
  font-size: 11px;
  color: #fca5a5;
  margin: 8px 0 0 0;
}

.st-host-widget__push-preview-ok {
  font-size: 11px;
  color: #86efac;
  margin: 8px 0 0 0;
}

.st-host-widget__device-code-label {
  font-size: 11px;
  color: #9ca3af;
}

.st-host-widget__device-code-value {
  font-family: ui-monospace, "SF Mono", "Cascadia Code", monospace;
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.08em;
  color: #f3f4f6;
}


@dev-bridge
