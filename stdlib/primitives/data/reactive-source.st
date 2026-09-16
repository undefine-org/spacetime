/// Reactive Source Primitive (FEAT-076)
///
/// Backs `@data source $name from $urlSignal` — a fetch whose URL is itself a
/// SIGNAL. Unlike `data-source` (a compile-time-constant URL), this re-fetches
/// every time the URL signal changes, republishing the result under $name. It
/// is the keystone for meta-renderers like the CMS admin, whose "active
/// collection" is whichever file the rail selected ($activeFile).
///
/// Publishes via the same dual channel as data-source (ST.setData + the global
/// signal store + local:<name>:updated), so @each / : / <- consumers react
/// identically whether the source is static or dynamic.
///
/// @param name     - signal name to publish the fetched array/value under
/// @param urlName  - name of the signal holding the URL to fetch
///
/// @yields $value   - the fetched data (null until the first load resolves)
/// @yields $loading - whether a fetch is in flight
%primitive reactive-source(
  name: ident,
  urlName: ident
) %uses data-store, data-url, data-channel {
  %emit js {
    (function () {
      const sourceName = '%name';
      const urlSignal = '%urlName';

      const store = window.__spacetimeData[sourceName] = {
        data: [],
        loading: false,
        error: null,
      };

      // Read the current URL from the global signal store; the template →
      // fetchable-URL transform ({locale} + root-relative normalization) is
      // owned by the data-url module (`%uses data-url`).
      const currentUrl = () => {
        const sl = window.SpacetimeLocal || {};
        return window.__stResolveUrl(sl[urlSignal]);
      };

      // Publish a value through the dual channel — the channel itself is
      // owned by the data-channel module (`%uses data-channel`); the store
      // bookkeeping stays local.
      const publish = (value) => {
        store.data = value;
        store.loading = false;
        window.__stPublish(sourceName, value);
      };

      let inflight = 0;
      const load = async () => {
        const url = currentUrl();
        // Bump the ticket FIRST so an empty-URL change also invalidates any
        // in-flight fetch — otherwise a pending response could resolve after we
        // cleared to [] and clobber it (stale-clobber on rapid select → deselect).
        const ticket = ++inflight;
        if (!url) { publish([]); return; }
        store.loading = true;
        try {
          const res = await fetch(url);
          const data = res.ok ? await res.json() : [];
          // Drop stale responses: only the latest request may publish.
          if (ticket !== inflight) return;
          publish(Array.isArray(data) ? data : []);
        } catch (err) {
          if (ticket !== inflight) return;
          store.error = String(err);
          publish([]);
        }
      };

      // Re-fetch whenever the URL signal changes; load once for the initial value.
      document.addEventListener('local:' + urlSignal + ':updated', load);
      load();

      store.reload = load;
    })();
  }
}
