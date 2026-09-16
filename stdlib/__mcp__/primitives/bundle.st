/// MCP region bundle mount primitives (FEAT-126).
///
/// A region is a DOM subtree tagged with `data-st-region="..."`. The page
/// declares one (or many) region roots, and the `@mcp-region` selector-scoped
/// macro starts polling `/__mcp/instance/{id}/bundles` for that region's
/// bundle set. When a bundle arrives, `Spacetime.registerBundle` registers the
/// templates, scopes the CSS, and sets the region's stage signal — which an
/// `@view $stageTpl { _ => &$stageTpl(); }` inside the region reacts to.
///
/// Example host page:
///
/// ```st
/// @import "stdlib/__mcp__";
///
/// <div data-st-region="stage">
///   @view $stageTpl {
///     _ => &$stageTpl();
///   }
/// </div>
///
/// [data-st-region] {
///   @mcp-region
/// }
/// ```
///
/// The instance id is read from the nearest `[data-st-instance]` ancestor of the
/// region root (falling back to `document.documentElement.dataset.stMcpInstance`
/// for the legacy single-instance page). The region id is read from the matched
/// element's `data-st-region`. No custom page JS is required.

%primitive mcp-region-poll(&root) {
  %emit js {
    (() => {
      var regionRoot = %&root;
      var regionId = regionRoot && regionRoot.dataset ? regionRoot.dataset.stRegion : '';
      // Resolve the instance id from the NEAREST `[data-st-instance]` ancestor
      // (the unified one-page environment hosts many regions, each under its own
      // instance host — same convention as `mcp-host`). Fall back to the
      // document-level `data-st-mcp-instance` for the legacy single-instance page.
      var hostEl = regionRoot && regionRoot.closest ? regionRoot.closest('[data-st-instance]') : null;
      var instanceId = hostEl ? (hostEl.getAttribute('data-st-instance') || '') : '';
      if (!instanceId) {
        instanceId = document.documentElement && document.documentElement.dataset
          ? (document.documentElement.dataset.stMcpInstance || '') : '';
      }
      if (!instanceId || !regionId) {
        console.warn('[ST] @mcp-region: missing instance or region id', instanceId, regionId);
        return;
      }
      if (window.Spacetime && window.Spacetime.pollBundles) {
        window.Spacetime.pollBundles(instanceId, regionId, 'stageTpl', 1000);
        // Stop the poll loop when this region root is disposed so the persistent
        // page does not accumulate orphaned fetch loops.
        if (window.ST && ST.onCleanup) {
          ST.onCleanup(regionRoot, function() {
            if (window.Spacetime && Spacetime.stopPolling) {
              Spacetime.stopPolling(instanceId, regionId);
            }
          });
        }
      } else {
        console.warn('[ST] @mcp-region: Spacetime.pollBundles not loaded');
      }
    })();
  }
}

%macro mcp-region {
  %scope selector
  %order 70

  %form {
    @mcp-region
  }

  %binds {
    mcp-region-poll(&self) -> {}
  }
}
