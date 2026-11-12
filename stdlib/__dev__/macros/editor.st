/// Dev Editor Macro
///
/// Formally composes the dev-editable, dev-save, and dev-ws primitives
/// into a single declarative unit for inline content editing in dev mode.
///
/// The three primitives communicate via global APIs:
///   - window.__stDevWs — WebSocket connection to dev server
///   - window.__stDevEditable — click-to-edit on data-bound elements
///   - window.__stDevSave — save orchestration (edit → ws → ack/reject)
///
/// Dependency order: dev-ws → dev-editable → dev-save
/// (dev-save subscribes to events from dev-editable and sends via dev-ws)

%macro dev-editor {
  %scope file
  %order 50

  %form {
    @dev-editor
  }

  %binds {
    dev-hooks()

    dev-flag()

    dev-ws() -> {
      $connected,
      $lastMessage,
      $error as $wsError
    }

    dev-editable() -> {
      $activeEdit,
      $editCount
    }

    dev-save() -> {
      $saving,
      $lastSaveError,
      $savedCount
    }
  }

  %example "@dev-editor" {
    @dev-editor
  }
}
