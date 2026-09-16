/// @host — a transport+config binding (PLAN-038 W1, pillar 1).
///
/// A `@host` declares a NAMED transport endpoint that `@data signal` / `@data
/// stream` send to and receive from. It binds three things: the TRANSPORT kind
/// (http | ws), its CONFIG (base url, headers, socket), and \u2014 implicitly \u2014 the
/// wire ENVELOPE type that types the decode subject `$.` inside a `receive` block.
///
///   @host $api : http("https://api.example.com") { headers: { authorization: $token }; }
///   @host $api : http("https://api.example.com")
///   @host $mcp : ws($hostSocket)
///
/// ENVELOPE TYPES (what `$.` is inside a `receive` over this host):
///   http  $. = Response { status int, ok bool, body Json, headers }   discriminator: status
///   ws    $. = Message  { type string, payload Json }                 discriminator: type
///
/// THE KIND IS THE MACRO (mirrors @data's kind-word dispatch): each transport is
/// its own %macro whose %form carries the literal kind-word (`http` / `ws`), so
/// parse disambiguates by specificity \u2014 no `%if $kind==` string-switch. The binding
/// name is captured as `:binding` (the `$`-prefixed sigil token); the leading `$`
/// is stripped for the emitted JS name. Each registers a `host(...)` fact carrying
/// `transport` + config, which `@data signal` reads to choose its transport
/// primitive (the http/ws %primitive is bound by the SIGNAL, not the host \u2014 the
/// host is config; the signal is the call site that drives the wire).
///
/// The `:` is value-position (sigil law): it introduces the transport VALUE
/// (`http(...)` / `ws(...)`). The transport's argument (url string / socket
/// binding) and the optional `{ headers: ... }` body are pure config.

/// @host (http) with a config body \u2014 base url + headers.
///   @host $api : http("https://api.example.com") { headers: { authorization: $t }; }
%macro host-http-opts {
  %scope file
  %order 250

  %form {
    @host $name:binding : http($url:expr) {
      headers: $headers:object?
    }
  }

  %binds {
    host-register(name: $name, transport: "http", url: $url, headers: $headers)
  }

  %registers host($name) {
    transport: "http"
    url: $url
    headers: $headers
    envelope: "Response"
  }
}

/// @host (http) bodyless \u2014 base url only.
///   @host $api : http("https://api.example.com")
%macro host-http {
  %scope file
  %order 250

  %form {
    @host $name:binding : http($url:expr) ;
  }

  %binds {
    host-register(name: $name, transport: "http", url: $url)
  }

  %registers host($name) {
    transport: "http"
    url: $url
    envelope: "Response"
  }
}

/// @host (ws) \u2014 a websocket endpoint bound to a socket expression.
///   @host $mcp : ws($hostSocket)
/// The socket arg is an EXPR (a signal/binding or a url string), so both
/// `ws($sock)` and `ws("/ws")` are accepted.
%macro host-ws {
  %scope file
  %order 250

  %form {
    @host $name:binding : ws($socket:expr) ;
  }

  %binds {
    host-register(name: $name, transport: "ws", socket: $socket)
  }

  %registers host($name) {
    transport: "ws"
    socket: $socket
    envelope: "Message"
  }
}

/// @host (live) — a Phoenix LiveView endpoint (PLAN: spacetime-phoenix-liveview-bridge).
///
///   @host $board : live("MyAppWeb.BoardLive")
///
/// A LiveView is the THIRD transport (after http/ws): a stateful socket whose
/// wire is an assign-diff protocol IN and a phx-event protocol OUT. The arg is
/// the server module name (a string); the build reads that module's
/// `__spacetime_contract__/0` to type the page's reads/fires (self-aware host).
///
/// ENVELOPE (what `$.` is inside a `receive` over a live host):
///   live  reply  $. = Reply { reply Json, ok bool }   discriminator: ok / reply tag
///   live  push   $. = Push  { event string, payload Json }   discriminator: event
///   (assign changes are an inbound SOURCE, not a receive subject — see the page's
///    dotted reads, which lower onto the inbound assign channel.)
///
/// Like http/ws this is config only: the signal/stream call site drives the wire
/// (over the existing Phoenix channel), reading this `host(...)` fact to choose
/// the live transport. `module` carries the server LiveView the contract check
/// resolves against.
%macro host-live {
  %scope file
  %order 250

  %form {
    @host $name:binding : live($module:expr) ;
  }

  %binds {
    host-register(name: $name, transport: "live", socket: $module)
  }

  %registers host($name) {
    transport: "live"
    module: $module
    envelope: "Reply"
  }
}

