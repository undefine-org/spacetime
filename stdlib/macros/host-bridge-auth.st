/// Host-bridge-auth macro — file-scope declarative wrapper (PLAN-004).
///
/// The `host-bridge-auth` %primitive (stdlib/primitives/host-bridge-auth.st)
/// can only be invoked from inside a `%macro`'s `%binds` block (the SAME
/// constraint every other bare-primitive-call in this stdlib already has --
/// confirmed via `stdlib/__dev__/macros/editor.st`'s own `dev-hooks()`/
/// `dev-flag()` calls, neither of which is ever invoked directly at file
/// scope in a `.st` entry point). This macro is that thin wrapper, mirroring
/// `@dev-editor`'s exact shape (one macro, one primitive call, no extra
/// logic) so `stdlib/__host__/index.st` can write a single declarative
/// `@host-bridge-auth(binding: hostApi)` line rather than reaching for the
/// primitive directly.
%macro host-bridge-auth {
  %scope file
  %order 50

  %form {
    @host-bridge-auth(
      /// the JS identifier this primitive's start()/
      /// sessionStorage key are exposed under (no `$`) — lets
      /// multiple widgets on one page (unlikely today, but
      /// keeps this primitive instance-scoped rather than a
      /// single hardcoded global name) coexist.
      binding: $binding:ident
    )
  }

  %binds {
    host-bridge-auth(bindingName: $binding)
  }

  %example "@host-bridge-auth" {
    @host-bridge-auth(binding: hostApi)
  }
}
