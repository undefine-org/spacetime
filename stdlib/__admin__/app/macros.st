// =============================================================================
// Admin composition macros (FEAT-076 rebuild).
//
// @dev-bridge composes ONLY the persistence substrate — the dev WebSocket and
// save orchestration primitives (reused from __dev__) — without the legacy
// admin-app() monolith. The declarative app tree (app.st · rail.st · …) supplies
// the UI; this macro just ensures window.__stDevWs / window.__stDevSave exist.
// Mirrors the @admin / @dev-editor composition pattern: primitives emit only
// when invoked from a macro's %binds.
// =============================================================================

%macro dev-bridge {
  %scope file
  %order 50

  %form {
    @dev-bridge
  }

  %binds {
    dev-ws() -> {
      $connected,
      $lastMessage,
      $error as $wsError
    }

    dev-save() -> {
      $saving,
      $lastSaveError,
      $savedCount
    }
  }
}
