/**
 * Spacetime Realtime Runtime
 *
 * Backend-agnostic WebSocket layer for real-time data sync and presence.
 * Works with any WebSocket server via message configuration.
 * Automatic protocol when using Spacetime dev server.
 *
 * Features:
 * - Connection pooling (multiple subscriptions share one socket)
 * - Auto-reconnect with exponential backoff
 * - Optimistic updates with rollback
 * - Presence with throttled broadcasts
 * - Protocol abstraction for any backend
 */

const SpacetimeRealtime = {
  // Connection pool: URL → RealtimeConnection
  connections: new Map(),

  /**
   * Get or create a connection to the given URL
   * @param {string} url - WebSocket URL (default: '/ws' → ws://host/ws)
   * @returns {RealtimeConnection}
   */
  connect(url = '/ws') {
    // Normalize URL
    const wsUrl = url.startsWith('ws')
      ? url
      : `${location.protocol === 'https:' ? 'wss:' : 'ws:'}//${location.host}${url}`;

    if (this.connections.has(wsUrl)) {
      return this.connections.get(wsUrl);
    }

    const conn = new RealtimeConnection(wsUrl);
    this.connections.set(wsUrl, conn);
    return conn;
  },

  /**
   * Disconnect all connections
   */
  disconnectAll() {
    for (const conn of this.connections.values()) {
      conn.disconnect();
    }
    this.connections.clear();
  }
};

/**
 * A single WebSocket connection with multiplexed subscriptions
 */
class RealtimeConnection {
  constructor(url) {
    this.url = url;
    this.ws = null;
    this.connected = false;
    this.reconnecting = false;
    this.reconnectAttempts = 0;
    this.maxReconnectDelay = 30000;
    this.baseReconnectDelay = 1000;

    // Subscriptions by channel
    this.subscriptions = new Map();

    // Presence rooms
    this.presenceRooms = new Map();

    // Pending operations (for optimistic updates)
    this.pendingOps = new Map();
    this.opIdCounter = 0;

    // Message handlers for custom protocols
    this.messageHandlers = new Map();

    // Connect
    this._connect();
  }

  // ===========================================================================
  // Connection Management
  // ===========================================================================

  _connect() {
    try {
      this.ws = new WebSocket(this.url);

      this.ws.onopen = () => {
        this.connected = true;
        this.reconnecting = false;
        this.reconnectAttempts = 0;
        this._emit('connected', true);

        // Resubscribe to all channels
        for (const [channel, sub] of this.subscriptions) {
          this._sendSubscribe(channel, sub.query);
        }

        // Rejoin presence rooms
        for (const [room, presence] of this.presenceRooms) {
          this._sendPresenceJoin(room, presence.state);
        }
      };

      this.ws.onmessage = (event) => {
        try {
          const msg = JSON.parse(event.data);
          this._handleMessage(msg);
        } catch (e) {
          console.error('[SpacetimeRealtime] Failed to parse message:', e);
        }
      };

      this.ws.onclose = () => {
        this.connected = false;
        this._emit('connected', false);
        this._scheduleReconnect();
      };

      this.ws.onerror = (error) => {
        console.error('[SpacetimeRealtime] WebSocket error:', error);
      };
    } catch (e) {
      console.error('[SpacetimeRealtime] Failed to connect:', e);
      this._scheduleReconnect();
    }
  }

  _scheduleReconnect() {
    if (this.reconnecting) return;
    this.reconnecting = true;
    this._emit('reconnecting', true);

    const delay = Math.min(
      this.baseReconnectDelay * Math.pow(2, this.reconnectAttempts),
      this.maxReconnectDelay
    );
    this.reconnectAttempts++;

    setTimeout(() => {
      this.reconnecting = false;
      this._connect();
    }, delay);
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.connected = false;
    this.subscriptions.clear();
    this.presenceRooms.clear();
  }

  // ===========================================================================
  // Event Emitter
  // ===========================================================================

  _listeners = new Map();

  on(event, fn) {
    if (!this._listeners.has(event)) {
      this._listeners.set(event, new Set());
    }
    this._listeners.get(event).add(fn);
    return () => this._listeners.get(event).delete(fn);
  }

  _emit(event, data) {
    const listeners = this._listeners.get(event);
    if (listeners) {
      for (const fn of listeners) {
        try { fn(data); } catch (e) { console.error(e); }
      }
    }
  }

  // ===========================================================================
  // Data Subscriptions
  // ===========================================================================

  /**
   * Subscribe to a data channel
   * @param {string} channel - Channel name
   * @param {Object} handlers - { onSnapshot, onInsert, onUpdate, onDelete }
   * @param {Object} options - { query, messages, actions }
   * @returns {Function} Unsubscribe function
   */
  subscribe(channel, handlers, options = {}) {
    const sub = {
      handlers,
      query: options.query,
      messages: options.messages || this._defaultMessages(),
      actions: options.actions || this._defaultActions(channel),
      items: [],
      pending: new Map() // opId → { type, item, prevItems }
    };

    this.subscriptions.set(channel, sub);

    if (this.connected) {
      this._sendSubscribe(channel, sub.query);
    }

    return () => {
      this.subscriptions.delete(channel);
      if (this.connected) {
        this._send({ type: 'Unsubscribe', channel });
      }
    };
  }

  _defaultMessages() {
    // Spacetime protocol message types
    return {
      snapshot: 'Snapshot',
      inserted: 'ItemInserted',
      updated: 'ItemUpdated',
      deleted: 'ItemDeleted',
      ack: 'Ack',
      reject: 'Reject'
    };
  }

  _defaultActions(channel) {
    // Spacetime protocol actions
    return {
      insert: (item, opId) => ({ type: 'Insert', channel, item, op_id: opId }),
      update: (id, patch, opId) => ({ type: 'Update', channel, id, patch, op_id: opId }),
      delete: (id, opId) => ({ type: 'Delete', channel, id, op_id: opId })
    };
  }

  _sendSubscribe(channel, query) {
    this._send({ type: 'Subscribe', channel, query });
  }

  /**
   * Insert item with optimistic update
   */
  async insert(channel, item) {
    const sub = this.subscriptions.get(channel);
    if (!sub) throw new Error(`Not subscribed to channel: ${channel}`);

    const opId = `op_${++this.opIdCounter}_${Date.now()}`;
    const tempId = item.id || `temp_${opId}`;
    const optimisticItem = { ...item, id: tempId, _pending: true };

    // Optimistic update
    const prevItems = [...sub.items];
    sub.items = [...sub.items, optimisticItem];
    sub.pending.set(opId, { type: 'insert', tempId, prevItems });
    sub.handlers.onInsert?.(optimisticItem);

    // Send to server
    const action = sub.actions.insert(item, opId);
    this._send(action);

    return opId;
  }

  /**
   * Update item with optimistic update
   */
  async update(channel, id, patch) {
    const sub = this.subscriptions.get(channel);
    if (!sub) throw new Error(`Not subscribed to channel: ${channel}`);

    const opId = `op_${++this.opIdCounter}_${Date.now()}`;
    const idx = sub.items.findIndex(i => i.id === id);

    if (idx >= 0) {
      // Optimistic update
      const prevItems = [...sub.items];
      const prevItem = sub.items[idx];
      sub.items[idx] = { ...prevItem, ...patch, _pending: true };
      sub.pending.set(opId, { type: 'update', id, prevItem, prevItems });
      sub.handlers.onUpdate?.(id, patch);
    }

    // Send to server
    const action = sub.actions.update(id, patch, opId);
    this._send(action);

    return opId;
  }

  /**
   * Delete item with optimistic update
   */
  async delete(channel, id) {
    const sub = this.subscriptions.get(channel);
    if (!sub) throw new Error(`Not subscribed to channel: ${channel}`);

    const opId = `op_${++this.opIdCounter}_${Date.now()}`;
    const idx = sub.items.findIndex(i => i.id === id);

    if (idx >= 0) {
      // Optimistic update
      const prevItems = [...sub.items];
      const removedItem = sub.items[idx];
      sub.items = sub.items.filter(i => i.id !== id);
      sub.pending.set(opId, { type: 'delete', id, removedItem, prevItems });
      sub.handlers.onDelete?.(id);
    }

    // Send to server
    const action = sub.actions.delete(id, opId);
    this._send(action);

    return opId;
  }

  // ===========================================================================
  // Presence
  // ===========================================================================

  /**
   * Join a presence room
   * @param {string} room - Room name
   * @param {Object} handlers - { onJoin, onLeave, onUpdate, onState }
   * @param {Object} options - { messages, initialState, throttle }
   * @returns {Object} { update, leave }
   */
  joinPresence(room, handlers, options = {}) {
    const throttleMs = options.throttle || 50;

    const presence = {
      handlers,
      messages: options.messages || this._defaultPresenceMessages(),
      state: options.initialState || {},
      users: [],
      myId: null,
      throttleTimer: null,
      pendingState: null
    };

    this.presenceRooms.set(room, presence);

    if (this.connected) {
      this._sendPresenceJoin(room, presence.state);
    }

    // Throttled state update
    const update = (state) => {
      presence.pendingState = { ...presence.pendingState, ...state };

      if (!presence.throttleTimer) {
        presence.throttleTimer = setTimeout(() => {
          if (presence.pendingState && this.connected) {
            this._send({
              type: presence.messages.update || 'PresenceUpdate',
              room,
              state: presence.pendingState
            });
            presence.state = { ...presence.state, ...presence.pendingState };
            presence.pendingState = null;
          }
          presence.throttleTimer = null;
        }, throttleMs);
      }
    };

    const leave = () => {
      if (presence.throttleTimer) {
        clearTimeout(presence.throttleTimer);
      }
      this.presenceRooms.delete(room);
      if (this.connected) {
        this._send({ type: 'PresenceLeave', room });
      }
    };

    return { update, leave, getUsers: () => presence.users, getMyId: () => presence.myId };
  }

  _defaultPresenceMessages() {
    return {
      state: 'PresenceState',
      joined: 'PresenceJoined',
      left: 'PresenceLeft',
      updated: 'PresenceUpdated',
      update: 'PresenceUpdate'
    };
  }

  _sendPresenceJoin(room, initialState) {
    this._send({ type: 'PresenceJoin', room, state: initialState });
  }

  // ===========================================================================
  // Custom Message Handlers (for backend-agnostic mode)
  // ===========================================================================

  /**
   * Register a handler for a specific message type
   * @param {string} type - Message type
   * @param {Function} handler - Handler function
   */
  onMessage(type, handler) {
    if (!this.messageHandlers.has(type)) {
      this.messageHandlers.set(type, new Set());
    }
    this.messageHandlers.get(type).add(handler);
    return () => this.messageHandlers.get(type).delete(handler);
  }

  // ===========================================================================
  // Message Handling
  // ===========================================================================

  _handleMessage(msg) {
    const type = msg.type;

    // Custom handlers first
    const handlers = this.messageHandlers.get(type);
    if (handlers) {
      for (const fn of handlers) {
        try { fn(msg); } catch (e) { console.error(e); }
      }
    }

    // Built-in protocol handling
    switch (type) {
      // Data sync
      case 'Snapshot':
        this._handleSnapshot(msg);
        break;
      case 'ItemInserted':
        this._handleItemInserted(msg);
        break;
      case 'ItemUpdated':
        this._handleItemUpdated(msg);
        break;
      case 'ItemDeleted':
        this._handleItemDeleted(msg);
        break;
      case 'Ack':
        this._handleAck(msg);
        break;
      case 'Reject':
        this._handleReject(msg);
        break;

      // Presence
      case 'PresenceState':
        this._handlePresenceState(msg);
        break;
      case 'PresenceJoined':
        this._handlePresenceJoined(msg);
        break;
      case 'PresenceLeft':
        this._handlePresenceLeft(msg);
        break;
      case 'PresenceUpdated':
        this._handlePresenceUpdated(msg);
        break;

      // Legacy (for existing @websocket usage)
      case 'Reload':
      case 'TestResults':
      case 'Ping':
        // Let custom handlers deal with these
        break;

      default:
        // Check if any subscription has custom message mapping
        for (const [channel, sub] of this.subscriptions) {
          const messages = sub.messages;
          if (type === messages.snapshot) {
            this._handleSnapshot({ ...msg, channel });
          } else if (type === messages.inserted) {
            this._handleItemInserted({ ...msg, channel });
          } else if (type === messages.updated) {
            this._handleItemUpdated({ ...msg, channel });
          } else if (type === messages.deleted) {
            this._handleItemDeleted({ ...msg, channel });
          }
        }

        // Check presence rooms for custom messages
        for (const [room, presence] of this.presenceRooms) {
          const messages = presence.messages;
          if (type === messages.state) {
            this._handlePresenceState({ ...msg, room });
          } else if (type === messages.joined) {
            this._handlePresenceJoined({ ...msg, room });
          } else if (type === messages.left) {
            this._handlePresenceLeft({ ...msg, room });
          } else if (type === messages.updated) {
            this._handlePresenceUpdated({ ...msg, room });
          }
        }
    }
  }

  _handleSnapshot(msg) {
    const sub = this.subscriptions.get(msg.channel);
    if (sub) {
      sub.items = msg.data || [];
      sub.handlers.onSnapshot?.(sub.items);
    }
  }

  _handleItemInserted(msg) {
    const sub = this.subscriptions.get(msg.channel);
    if (sub) {
      // Check if this replaces an optimistic insert
      const item = msg.item;
      const existingIdx = sub.items.findIndex(i => i._pending && i.id?.startsWith('temp_'));

      if (existingIdx >= 0) {
        // Replace optimistic item with server version
        sub.items[existingIdx] = item;
      } else {
        sub.items = [...sub.items, item];
      }

      sub.handlers.onInsert?.(item);
    }
  }

  _handleItemUpdated(msg) {
    const sub = this.subscriptions.get(msg.channel);
    if (sub) {
      const idx = sub.items.findIndex(i => i.id === msg.id);
      if (idx >= 0) {
        sub.items[idx] = { ...sub.items[idx], ...msg.patch, _pending: false };
        sub.handlers.onUpdate?.(msg.id, msg.patch);
      }
    }
  }

  _handleItemDeleted(msg) {
    const sub = this.subscriptions.get(msg.channel);
    if (sub) {
      sub.items = sub.items.filter(i => i.id !== msg.id);
      sub.handlers.onDelete?.(msg.id);
    }
  }

  _handleAck(msg) {
    const opId = msg.op_id;
    // Find and clear pending operation
    for (const [channel, sub] of this.subscriptions) {
      if (sub.pending.has(opId)) {
        const op = sub.pending.get(opId);
        sub.pending.delete(opId);

        // Clear _pending flag on affected items
        if (op.type === 'insert' && op.tempId) {
          const idx = sub.items.findIndex(i => i.id === op.tempId);
          if (idx >= 0) {
            delete sub.items[idx]._pending;
          }
        } else if (op.type === 'update' && op.id) {
          const idx = sub.items.findIndex(i => i.id === op.id);
          if (idx >= 0) {
            delete sub.items[idx]._pending;
          }
        }
        break;
      }
    }
  }

  _handleReject(msg) {
    const opId = msg.op_id;
    // Rollback optimistic update
    for (const [channel, sub] of this.subscriptions) {
      if (sub.pending.has(opId)) {
        const op = sub.pending.get(opId);
        sub.pending.delete(opId);

        // Restore previous state
        sub.items = op.prevItems;
        sub.handlers.onSnapshot?.(sub.items);

        console.warn(`[SpacetimeRealtime] Operation rejected: ${msg.reason}`);
        break;
      }
    }
  }

  _handlePresenceState(msg) {
    const presence = this.presenceRooms.get(msg.room);
    if (presence) {
      presence.users = msg.users || [];
      presence.myId = msg.myId || msg.my_id;
      presence.handlers.onState?.(presence.users, presence.myId);
    }
  }

  _handlePresenceJoined(msg) {
    const presence = this.presenceRooms.get(msg.room);
    if (presence) {
      presence.users = [...presence.users, msg.user];
      presence.handlers.onJoin?.(msg.user);
    }
  }

  _handlePresenceLeft(msg) {
    const presence = this.presenceRooms.get(msg.room);
    if (presence) {
      const userId = msg.userId || msg.user_id;
      presence.users = presence.users.filter(u => u.id !== userId);
      presence.handlers.onLeave?.(userId);
    }
  }

  _handlePresenceUpdated(msg) {
    const presence = this.presenceRooms.get(msg.room);
    if (presence) {
      const userId = msg.userId || msg.user_id;
      const idx = presence.users.findIndex(u => u.id === userId);
      if (idx >= 0) {
        presence.users[idx] = { ...presence.users[idx], ...msg.state };
        presence.handlers.onUpdate?.(userId, msg.state);
      }
    }
  }

  // ===========================================================================
  // Send
  // ===========================================================================

  _send(msg) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(msg));
    }
  }
}

// Export for module systems and global access
if (typeof module !== 'undefined' && module.exports) {
  module.exports = { SpacetimeRealtime, RealtimeConnection };
}
if (typeof globalThis !== 'undefined') {
  globalThis.SpacetimeRealtime = SpacetimeRealtime;
}
