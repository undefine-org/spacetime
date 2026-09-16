/// Host-device-poll macro — file-scope declarative wrapper.
///
/// The `host-device-poll` %primitive (stdlib/primitives/host-device-poll.st)
/// can only be invoked from inside a `%macro`'s `%binds` block (the SAME
/// constraint `@host-bridge-auth`'s own macro wrapper documents). This macro
/// is that thin wrapper, mirroring `@host-bridge-auth`'s exact shape.
///
/// @example
/// @host-device-poll(binding: hostApi)
%macro host-device-poll {
  %scope file
  %order 50

  %form {
    @host-device-poll(
      binding: $binding:ident
    )
  }

  %binds {
    host-device-poll(bindingName: $binding)
  }
}
