// =============================================================================
// Spacetime CMS — Admin entry point (FEAT-076: declarative multi-file rebuild).
//
// Compiled separately from user code, served at /__spacetime/admin/ in debug
// mode. The full-Spacetime replacement for the former 1349-line admin-app.st
// %emit js monolith: the admin is now a META-RENDERER built ENTIRELY from
// platform constructs (@data / @each / @template / @match / @portal / @on $sig.change /
// :  / <-) across the app/ tree. ZERO %emit js in the admin tree — the ONLY
// imperative module kept is the dev WebSocket persistence bridge (reused from
// __dev__), exactly as the acceptance criteria require.
//
// Served by compile_admin() as a real .st page: the file-scope shell markup
// (app.st) mounts to document.body, like demos/app-shell/index.st.
// =============================================================================

@import "../__dev__/primitives/dev-ws.st"
@import "../__dev__/primitives/dev-save.st"
@import "./app/macros.st"
@import "./app/styles.st"
@import "./app/app.st"
@import "./app/rail.st"
@import "./app/list.st"
@import "../fields/index.st";
@import "../editable/index.st"
@import "./app/drawer.st"
@import "./app/media.st"
@import "./app/graph.st"
@import "./app/tools.st"

// Wire the dev bridge (window.__stDevWs / __stDevSave) for persistence.
@dev-bridge
