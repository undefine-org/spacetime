/// WebSocket & Realtime Macros
///
/// Declarative macros for real-time communication.
/// Provides ergonomic sugar over the websocket primitives.
///
/// Macros:
/// - @websocket: Basic WebSocket with message routing
/// - @realtime: Real-time data collections with CRUD
/// - @presence: Multiplayer presence tracking
/// - @channel: Typed message channels

// =============================================================================
// @websocket Macro
// =============================================================================

/// WebSocket Macro
///
/// Declarative WebSocket with message-type routing.
/// Connection lifecycle is reflected via reactive signals; message types are
/// routed via @on directives.
///
/// @example
/// .live-connection {
///   @websocket(url: "ws://localhost:3030/ws") {
///     @ws-on Reload { trigger("reload-tests") }
///     @ws-on TestResults { $testResults <- $.results }
///     @ws-on Ping { send({ type: "Pong" }) }
///     @ws-on Error { $lastError <- $.message }
///   }
/// }

%macro websocket {

  %form {
    @websocket(url: $url:string = "/ws", reconnect: $reconnect:time = 2000ms) {
      $_handlers:message_handlers*
    }
  }

  // Bind the thin primitive - yields $connected, $message, $send
  %binds {
    websocket(&self, url: $url, reconnect: $reconnect) -> { $connected, $message, $send }
  }

  // TODO: Connection lifecycle signal (nested %macro not yet supported)
  // TODO: Route each message type to its handler
}


/// WebSocket Message Handler
///
/// Used within @websocket blocks to define message type handlers.
///
/// @example
/// @ws-on Reload { $files <- $.files }

%macro ws-message-handler {

  %form {
    @ws-on $messageType:ident {
      $actions:statements
    }
  }

  %registers message_handler {
    type: $messageType
    action: $actions
  }
}

// =============================================================================
// @realtime Macro
// =============================================================================

/// Realtime Data Collection Macro
///
/// Subscribe to a real-time data collection with optimistic CRUD.
/// Automatically syncs with server and handles optimistic updates.
///
/// @example
/// @type Todo { id: string; text: string; done: bool }
///
/// @realtime todos: Todo[] {
///   channel: "todos"
///   query: { userId: $user.id }    // optional filter
///   order: $.createdAt desc        // optional sort (client-side)
/// }
///
/// .todo-list {
///   @each($todos, key: $.id) {
///     > .item {
///       [slot=text]: $.text
///       @on click { $todos.update($.id, { done: !$.done }) }
///       @on click [.delete] { $todos.delete($.id) }
///
///       // Item lifecycle via reactive class on a per-item status signal
///       .is-entering: $.status == "entering";
///     }
///   }
///
///   @on submit form { $todos.insert({ text: $input.value, done: false }) }
/// }

%macro realtime {

  %form {
    @realtime $_name:ident : $type:typeref {
      channel: $channel:string
      url: $url:string = "/ws"
      query: $query:expr?
      order: $_order:expr?
      optimistic: $optimistic:bool = true
    }
  }

  // Bind the realtime-source primitive
  %binds {
    realtime-source(&self,
      channel: $channel,
      url: $url,
      query: $query,
      optimistic: $optimistic
    ) -> { $items, $loading, $insert, $update, $remove }
  }
}


/// Realtime Collection (short form)
///
/// Simpler form for common use cases.
///
/// @example
/// @realtime todos: Todo[] on "todos"

%macro realtime-short {

  %form {
    @realtime $_name:ident : $type:typeref on $channel:string
  }

  %binds {
    realtime-source(&self, channel: $channel) -> { $items, $loading, $insert, $update, $remove }
  }
}

// =============================================================================
// @presence Macro
// =============================================================================

/// Presence Macro
///
/// Join a presence room to track and broadcast user state.
/// Great for collaborative features like cursors, selections, typing indicators.
///
/// @example
/// @type Cursor { x: number; y: number; color: string; name: string }
///
/// .canvas {
///   @presence(room: $docId, throttle: 50ms) as cursors: Cursor
///
///   // Render remote cursors
///   @each($cursors.users, key: $.id) {
///     > .cursor {
///       translate-x: $.x
///       translate-y: $.y
///       background: $.color
///       [slot=name]: $.name
///
///       :entering { opacity: 0 -> 1 }
///       :exiting { opacity: 1 -> 0 }
///     }
///   }
///
///   // Broadcast my cursor position (throttled)
///   @on mousemove {
///     $cursors.set({ x: $.clientX, y: $.clientY })
///   }
/// }

%macro presence {

  %form {
    @presence(room: $room:string, throttle: $throttle:time = 50ms, url: $url:string = "/ws")
      as $_name:ident : $type:typeref? = any
  }

  // Bind the presence primitive
  %binds {
    presence(&self,
      room: $room,
      url: $url,
      throttle: $throttle
    ) -> { $users, $myId, $set, $leave }
  }
}


/// ERROR SIBLING (BUG-264 class 3): the `: $type:typeref?` slot after `as $name`
/// names the SHAPE each presence user's state takes and must be a TYPE
/// (`Viewer[]`, `Cursor`), not a value. `$name:ident` takes a bare identifier, so
/// `as viewers: 5` matches the productive `presence` form with `$name=viewers`
/// and `$type=any` — the `: 5` becoming a post-inline leftover the form DROPS
/// (a lenient match). This sibling matches the refused value via `$badType:expr`
/// and reports it. Declared AFTER `presence`: on a valid type the productive form
/// matches cleanly and wins; only a refused near-miss leaves the productive match
/// lenient (form_compiler's post-inline trailing-leniency detection), letting
/// this sibling fire.
%macro presence-type-invalid {
  %diagnostic



  %form {
    @presence(room: $room:string, throttle: $throttle:time = 50ms, url: $url:string = "/ws")
      as $name:ident : $badType:expr
  }

  %binds {
    optional-clause-error(
      message: "`as $name: …` expects a TYPE, not a value — got `$badType`",
      hint: "write `as $name: Type[]` (e.g. `as viewers: Viewer[]`) — the type slot names the shape each presence user's state takes; `$name` is the bare alias signal",
      value: $badType
    )
  }
}


/// Presence (short form)
///
/// @example
/// @presence(room: "doc-123") as viewers

%macro presence-short {

  %form {
    @presence(room: $room:string) as $_name:ident
  }

  %binds {
    presence(&self, room: $room) -> { $users, $myId, $set, $leave }
  }
}

// =============================================================================
// @channel Macro
// =============================================================================

/// Channel Macro
///
/// Subscribe to a typed message channel for custom protocols.
/// Provides structured send/receive with type safety.
///
/// @example
/// @channel Notifications {
///   @in Subscribe { topics: string[] }
///   @out Alert { level: "info" | "warn" | "error"; message: string }
///   @out Badge { count: number }
/// }
///
/// .notifications {
///   @connect(Notifications, url: "/ws") as notif
///
///   @on connected {
///     $notif.send(Subscribe { topics: ["orders", "alerts"] })
///   }
///
///   @on $notif.Alert {
///     $alerts <- [...$alerts, $]
///   }
///
///   @on $notif.Badge {
///     $badgeCount <- $.count
///   }
/// }

%macro channel {

  %form {
    @channel $name:ident {
      $messages:channel_messages*
    }
  }

  // Channel definition stores message types
  %registers channel_def {
    name: $name
    messages: $messages
  }
}


/// Channel Message Type (incoming)
%macro channel-in {

  %form {
    @in $type:ident { $fields:type_fields }
  }

  %registers channel_in {
    type: $type
    fields: $fields
  }
}


/// Channel Message Type (outgoing)
%macro channel-out {

  %form {
    @out $type:ident { $fields:type_fields }
  }

  %registers channel_out {
    type: $type
    fields: $fields
  }
}


/// Connect to a Channel
///
/// Binds a channel definition to an element.
///
/// @example
/// @connect(Notifications) as notif

%macro connect {

  %form {
    @connect($channel:ident, url: $url:string = "/ws") as $name:ident
  }

  %binds {
    channel(&self, name: $channel, url: $url) -> { $connected, $lastMessage, $send }
  }
}

// =============================================================================
// Backend-Agnostic Configuration
// =============================================================================

/// Realtime with custom backend protocol
///
/// For servers that don't use the Spacetime sync protocol.
///
/// @example
/// @realtime items: Item[] {
///   channel: "items"
///   messages: {
///     snapshot: "ITEMS_SNAPSHOT"
///     inserted: "ITEM_CREATED"
///     updated: "ITEM_UPDATED"
///     deleted: "ITEM_DELETED"
///   }
///   actions: {
///     insert: (item) => { type: "CREATE_ITEM", payload: item }
///     update: (id, patch) => { type: "UPDATE_ITEM", id, ...patch }
///     delete: (id) => { type: "DELETE_ITEM", id }
///   }
/// }

%macro realtime-custom {

  %form {
    @realtime $_name:ident : $type:typeref {
      channel: $channel:string
      url: $url:string = "/ws"
      messages: $messages:object
      actions: $actions:object
    }
  }

  %binds {
    realtime-source(&self,
      channel: $channel,
      url: $url,
      messages: $messages,
      actions: $actions
    ) -> { $items, $loading, $insert, $update, $remove }
  }
}


/// Presence with custom backend protocol
///
/// @example
/// @presence(room: "doc-123") as cursors {
///   messages: {
///     state: "PRESENCE_STATE"
///     joined: "USER_JOINED"
///     left: "USER_LEFT"
///   }
/// }

%macro presence-custom {

  %form {
    @presence(room: $room:string, throttle: $throttle:time = 50ms) as $_name:ident {
      url: $url:string = "/ws"
      messages: $messages:object
    }
  }

  %binds {
    presence(&self,
      room: $room,
      url: $url,
      throttle: $throttle,
      messages: $messages
    ) -> { $users, $myId, $set, $leave }
  }
}


/// Socket Macro (typed-union state producer) — moved from the retired
/// the retired state-machine macros module (PLAN-077 W6). The union it yields is
/// consumed by @state(when: $ws is V) via stdlib/enum/state.st's state-reflect.


/// Socket Macro
///
/// WebSocket connection with typed union state for pattern matching.
/// Invokes the socket primitive and yields $state as the aliased signal.
///
/// @example
/// .socket-demo {
///   @socket(&self, url: attr(url) or "/ws") as $ws
///
///   @state(when: $ws is Connected { $send, $received }) {
///     opacity: 1;
///   }
/// }

%macro socket {

  %form {
    @socket(url: $url:expr = "/ws") as $state:ident
  }

  // Bind the socket primitive - yields typed union state
  // The $state from the form becomes the output binding name
  %binds {
    socket(&self, url: $url) -> { $state }
  }
}
