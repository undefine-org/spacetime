/// Browser automation (PLAN-027 W7): @drive a LIVE page.
///
/// Test ≡ automation. A @drive block is a @test whose WORLD is a live URL
/// instead of a mounted fixture; the SAME probes (@when / @then / @observe /
/// @wait-for / @capture) run against it. Requires a real-browser backend (CDP):
/// the runner navigates the page to <url> before running the body.
///
/// @example
///   @drive "https://example.com" {
///     @then h1 { exist; text ~= "Example"; }
///     @capture screenshot "example.png";
///   }
///
/// `via kernel <wsUrl>` (remote CDP, kernel.sh :9222) is the same surface with a
/// swapped endpoint — selected at the CLI/runner level, not in the macro.
%macro drive {

  %form {
    @drive $url:string {
      $body:block
    }
  }

  %emit js {
    window.__spacetime_register_test('drive ' + %$url, async () => {
      const __ctx = { startTime: Date.now() };
      // The CDP runner navigates the page to this URL BEFORE the body runs
      // (it reads the `drive` option). On a non-CDP backend the body still runs
      // against whatever is loaded, so @drive degrades to a no-network check.
      %$body.js
    }, { drive: %$url });
  }
}


/// @capture screenshot "<path>" — capture a screenshot of the live page.
/// Records a request the CDP runner fulfils (page.screenshot -> file). No-op on
/// the V8 backend (no real rendering).
%macro capture {

  %form {
    @capture $kind:ident $path:string
  }

  %emit js {
    {
    if (typeof window !== 'undefined') {
      window.__stCaptures = window.__stCaptures || [];
      window.__stCaptures.push({ kind: '%$kind', path: %$path });
    }
    }
  }
}


/// @wait-for <selector> — wait until an element appears (polls in-page).
/// Macro name `wait-for-appears` is distinct from testing/test.st's `wait-for`
/// (which backs the `@wait_for ... timeout:` directive). Both load into the same
/// stdlib registry, so the MACRO names must be unique (E0812) even though the
/// DIRECTIVE surfaces differ (`@wait-for` vs `@wait_for`). Directive dispatch keys
/// on the %form first token, not the macro name (see resolve.rs).
%macro wait-for-appears {

  %form {
    @wait-for $target:selector
  }

  %emit js {
    {
    const __sel = %$target;
    const __deadline = Date.now() + 5000;
    while (!document.querySelector(__sel) && Date.now() < __deadline) {
      await new Promise(r => setTimeout(r, 25));
    }
    if (!document.querySelector(__sel)) {
      throw new Error('@wait-for: ' + __sel + ' never appeared within 5s');
    }
    }
  }
}


/// @observe <selector> as $handle — bind a stable handle to live DOM you did not
/// author (the agent-handle pattern). Stores the matched element(s) under
/// window.__stHandles[name] so later claims can reference $handle.
%macro observe {

  %form {
    @observe $target:selector as $name:ident
  }

  %emit js {
    {
    if (typeof window !== 'undefined') {
      window.__stHandles = window.__stHandles || {};
      window.__stHandles['%$name'] = ST.subjectAll(%$target);
    }
    }
  }
}
