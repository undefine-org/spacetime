/// Dev Panel Composition Macro
///
/// Formally declares the composition of the dev tools system:
///   dev-panel → dev-inspect → data-tree → timelines-errors
///
/// The primitives already self-compose via global APIs:
///   - window.__stDevPanel: shadow, getPane(), switchTab(), host
///   - window.__stDevInspect: refresh(), getTimelines(), getErrors(),
///     getDataSources(), clearErrors()
///
/// data-tree and timelines-errors inject their content into the
/// panel's Shadow DOM panes. Import order in index.st handles
/// initialization sequencing.
///
/// This macro serves as the formal declaration of that composition,
/// making the dependency graph explicit in the metasystem.
///
/// @yields $activeTab - Currently selected tab name (from dev-panel)
/// @yields $minimized - Whether panel is minimized (from dev-panel)
/// @yields $timelines - Array of timeline objects (from dev-inspect)
/// @yields $errors - Array of error objects (from dev-inspect)
/// @yields $timelineCount - Number of active timelines (from dev-inspect)
/// @yields $errorCount - Number of errors (from dev-inspect)
/// @yields $dataSources - Array of data source info objects (from dev-inspect)

%macro dev-panel {
  %scope file
  %order 900

  %form {
    @dev-panel
  }

  %binds {
    dev-panel() -> {
      $activeTab,
      $minimized
    }

    dev-inspect() -> {
      $timelines,
      $errors,
      $timelineCount,
      $errorCount,
      $dataSources
    }

    data-tree()

    timelines-errors()

    dev-indicator()

    dev-selector()

    st-properties()

    admin-panel()

    dev-review()

    dev-review-tab()
  }
}
