/// WebSocket Primitives
///
/// Real-time communication primitives using SpacetimeRealtime runtime.
/// Provides connection pooling, auto-reconnect, optimistic updates, and presence.
///
/// Primitives:
/// - websocket: Basic WebSocket with message routing
/// - realtime-source: Data collection with CRUD and optimistic updates
/// - presence: Multiplayer presence with throttled broadcasts

// =============================================================================
// Base WebSocket Primitive (backward compatible)
// =============================================================================

/// WebSocket Primitive
///
/// Thin wrapper around WebSocket API using SpacetimeRealtime for connection pooling.
/// Yields signals for connection state and incoming messages.
///
/// @param &el - Element to bind lifecycle to
/// @param url - WebSocket URL (default: '/ws')
/// @param reconnect - Auto-reconnect delay (0 to disable)
/// @yields $connected - Connection state
/// @yields $message - Last received message (parsed JSON)
/// @yields send - Function to send messages

%primitive websocket(&el, url: string = "/ws", reconnect: time = 2000ms) {
  %emit js {
    // Use SpacetimeRealtime for connection pooling
    const conn = SpacetimeRealtime.connect('%url');

    // Track connection state
    const offConnected = conn.on('connected', (state) => {
      %yield state -> $connected;
      if (state) {
        %&el.dispatchEvent(new CustomEvent('ws.open', { bubbles: false }));
      } else {
        %&el.dispatchEvent(new CustomEvent('ws.close', { bubbles: false }));
      }
    });

    // Register a custom message handler that dispatches events
    const offMessage = conn.onMessage('*', (msg) => {
      %yield msg -> $message;
      // Dispatch typed events for state machine routing
      %&el.dispatchEvent(new CustomEvent('ws.message', {
        detail: msg,
        bubbles: false
      }));
      if (msg.type) {
        %&el.dispatchEvent(new CustomEvent(`ws.${msg.type}`, {
          detail: msg,
          bubbles: false
        }));
      }
    });

    // Send function
    const send = (msg) => {
      conn._send(typeof msg === 'object' ? msg : { data: msg });
    };

    %yield conn.connected -> $connected;
    %yield send -> $send;

    %cleanup {
      offConnected();
      offMessage();
    }
  }

  %exports {
    $connected: bool
    $message: object
    $send: fn(object) ~> void
  }
}

// =============================================================================
// Realtime Source Primitive
// =============================================================================

/// Realtime Data Source Primitive
///
/// Subscribe to a real-time data collection with optimistic CRUD operations.
/// Automatically syncs with server and handles optimistic updates with rollback.
///
/// @param &el - Element to bind lifecycle to
/// @param channel - Channel/collection name to subscribe to
/// @param url - WebSocket URL (default: '/ws')
/// @param query - Optional query/filter for the subscription
/// @param optimistic - Enable optimistic updates (default: true)
/// @param messages - Custom message type mapping for backend-agnostic mode
/// @param actions - Custom action creators for backend-agnostic mode
///
/// @yields $items - Current collection items
/// @yields $loading - True while loading initial data
/// @yields insert - Function to insert new item
/// @yields update - Function to update item by id
/// @yields remove - Function to remove item by id

%primitive realtime-source(
  &el,
  channel: string,
  url: string = "/ws",
  query: object? = null,
  optimistic: bool = true,
  messages: object? = null,
  actions: object? = null
) {
  %emit js {
    const conn = SpacetimeRealtime.connect('%url');
    let items = [];
    let loading = true;

    %yield true -> $loading;
    %yield [] -> $items;

    const handlers = {
      onSnapshot: (data) => {
        items = data;
        loading = false;
        %yield items -> $items;
        %yield false -> $loading;
        %&el.dispatchEvent(new CustomEvent('realtime.snapshot', {
          detail: { channel: '%channel', items },
          bubbles: false
        }));
      },
      onInsert: (item) => {
        // Already handled by subscription's optimistic update
        // Dispatch event for state machine integration
        %&el.dispatchEvent(new CustomEvent('item.inserted', {
          detail: item,
          bubbles: true  // Bubble to @each item elements
        }));
      },
      onUpdate: (id, patch) => {
        %&el.dispatchEvent(new CustomEvent('item.updated', {
          detail: { id, patch },
          bubbles: true
        }));
      },
      onDelete: (id) => {
        %&el.dispatchEvent(new CustomEvent('item.removing', {
          detail: { id },
          bubbles: true
        }));
      }
    };

    const options = {
      query: %query,
      messages: %messages,
      actions: %actions
    };

    const unsubscribe = conn.subscribe('%channel', handlers, options);

    // CRUD operations
    const insert = (item) => {
      conn.insert('%channel', item);
    };

    const update = (id, patch) => {
      conn.update('%channel', id, patch);
    };

    const remove = (id) => {
      conn.delete('%channel', id);
    };

    %yield insert -> $insert;
    %yield update -> $update;
    %yield remove -> $remove;

    %cleanup {
      unsubscribe();
    }
  }

  %exports {
    $items: array
    $loading: bool
    $insert: fn(object) ~> void
    $update: fn(string, object) ~> void
    $remove: fn(string) ~> void
  }
}

// =============================================================================
// Presence Primitive
// =============================================================================

/// Presence Primitive
///
/// Join a presence room to track and broadcast user state.
/// Automatically handles throttled updates and user lifecycle.
///
/// @param &el - Element to bind lifecycle to
/// @param room - Room name to join
/// @param url - WebSocket URL (default: '/ws')
/// @param throttle - Throttle interval for state updates (default: 50ms)
/// @param initial - Initial presence state
/// @param messages - Custom message type mapping for backend-agnostic mode
///
/// @yields $users - Array of users in the room
/// @yields $myId - Current user's ID
/// @yields set - Function to update presence state (throttled)
/// @yields leave - Function to leave the room

%primitive presence(
  &el,
  room: string,
  url: string = "/ws",
  throttle: time = 50ms,
  initial: object? = null,
  messages: object? = null
) {
  %emit js {
    const conn = SpacetimeRealtime.connect('%url');
    let users = [];
    let myId = null;

    %yield [] -> $users;
    %yield null -> $myId;

    const handlers = {
      onState: (userList, id) => {
        users = userList;
        myId = id;
        %yield users -> $users;
        %yield myId -> $myId;
        %&el.dispatchEvent(new CustomEvent('presence.state', {
          detail: { room: '%room', users, myId },
          bubbles: false
        }));
      },
      onJoin: (user) => {
        %&el.dispatchEvent(new CustomEvent('presence.join', {
          detail: user,
          bubbles: false
        }));
      },
      onLeave: (userId) => {
        %&el.dispatchEvent(new CustomEvent('presence.leave', {
          detail: { userId },
          bubbles: false
        }));
      },
      onUpdate: (userId, state) => {
        %&el.dispatchEvent(new CustomEvent('presence.update', {
          detail: { userId, state },
          bubbles: false
        }));
      }
    };

    const options = {
      throttle: %throttle,
      initialState: %initial,
      messages: %messages
    };

    const presence = conn.joinPresence('%room', handlers, options);

    // Expose presence controls
    const set = (state) => {
      presence.update(state);
    };

    const leave = () => {
      presence.leave();
    };

    %yield set -> $set;
    %yield leave -> $leave;

    %cleanup {
      presence.leave();
    }
  }

  %exports {
    $users: array
    $myId: string
    $set: fn(object) ~> void
    $leave: fn() ~> void
  }
}

// =============================================================================
// Channel Primitive
// =============================================================================

/// Channel Primitive
///
/// Subscribe to a typed message channel for custom protocols.
/// Provides structured send/receive for specific message types.
///
/// @param &el - Element to bind lifecycle to
/// @param name - Channel name
/// @param url - WebSocket URL (default: '/ws')
///
/// @yields $connected - Connection state
/// @yields $lastMessage - Last received message
/// @yields send - Function to send typed messages

%primitive channel(
  &el,
  name: string,
  url: string = "/ws"
) {
  %emit js {
    const conn = SpacetimeRealtime.connect('%url');

    %yield conn.connected -> $connected;
    %yield null -> $lastMessage;

    const offConnected = conn.on('connected', (state) => {
      %yield state -> $connected;
    });

    // Listen for messages on this channel
    const offMessage = conn.onMessage('%name', (msg) => {
      %yield msg -> $lastMessage;
      %&el.dispatchEvent(new CustomEvent('channel.' + '%name', {
        detail: msg,
        bubbles: false
      }));
      // Also dispatch by message type if present
      if (msg.type) {
        %&el.dispatchEvent(new CustomEvent('channel.' + '%name' + '.' + msg.type, {
          detail: msg,
          bubbles: false
        }));
      }
    });

    const send = (type, payload = {}) => {
      conn._send({ channel: '%name', type, ...payload });
    };

    %yield send -> $send;

    %cleanup {
      offConnected();
      offMessage();
    }
  }

  %exports {
    $connected: bool
    $lastMessage: object
    $send: fn(string, object) ~> void
  }
}
