/// Stored Sync Primitive
///
/// Keeps one local-state signal in step with localStorage under `st.<name>`:
/// restores a stored value over the declared initial at load, then saves
/// every later write. Pairs with `local-state-impl` in `@data stored`.
/// Storage failures (private mode, quota, bad JSON) leave the signal on its
/// declared value; they never throw into the page.

%primitive stored-sync(name: ident) {
  %emit js {
    (() => {
      const storageKey = `st.${%name}`;
      try {
        const stored = window.localStorage.getItem(storageKey);
        if (stored !== null) window.SpacetimeLocal[%name] = JSON.parse(stored);
      } catch (e) {}
      document.addEventListener(`local:${%name}:updated`, (ev) => {
        try { window.localStorage.setItem(storageKey, JSON.stringify(ev.detail)); } catch (e) {}
      });
    })();
  }

  %exports {}
}
