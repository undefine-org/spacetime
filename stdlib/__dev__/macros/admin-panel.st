/// Admin Panel Macro
///
/// Composes the content tree and data editor primitives into the
/// dev panel tab system. Provides structured editing for JSON data
/// sources with schema-driven forms.

%macro admin-panel {
  %scope file
  %order 901

  %form {
    @admin-panel
  }

  %binds {
    dev-content-tree()
    dev-data-editor()
  }
}
