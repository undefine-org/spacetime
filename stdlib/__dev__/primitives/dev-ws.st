/// Dev WebSocket Primitive
///
/// Establishes a WebSocket connection to the Spacetime dev server.
/// Handles JSON message parsing/serialization, reconnection with
/// exponential backoff, and live-reload integration.
///
/// This is a global singleton — it doesn't attach to any element.
/// Other dev tool primitives (dev-save, dev-editable) communicate
/// with the server through `window.__stDevWs`.
///
/// @yields $connected - Whether the WebSocket is currently connected
/// @yields $lastMessage - The last received message object
/// @yields $error - Last error message, if any

%primitive dev-ws() {
  %emit js {
    (function() {
      var ws = null;
      var connected = false;
      var lastMessage = null;
      var error = null;
      var reconnectDelay = 1000;
      var maxReconnectDelay = 30000;
      var reconnectTimer = null;
      var messageHandlers = {};
      var ackCallbacks = {};
      var rejectCallbacks = {};

      function getWsUrl() {
        return (location.protocol === 'https:' ? 'wss://' : 'ws://') + location.host + '/ws';
      }

      function yieldState() {
        %yield connected -> $connected;
        %yield lastMessage -> $lastMessage;
        %yield error -> $error;
      }

      function handleMessage(data) {
        lastMessage = data;

        var type = data.type;
        if (!type) return;

        // Live-reload: Reload message triggers page refresh
        if (type === 'Reload') {
          location.reload();
          return;
        }

        // Ack callback
        if (type === 'Ack' && data.op_id && ackCallbacks[data.op_id]) {
          ackCallbacks[data.op_id](data);
          delete ackCallbacks[data.op_id];
          delete rejectCallbacks[data.op_id];
          return;
        }

        // Reject callback
        if (type === 'Reject' && data.op_id && rejectCallbacks[data.op_id]) {
          rejectCallbacks[data.op_id](data);
          delete ackCallbacks[data.op_id];
          delete rejectCallbacks[data.op_id];
          return;
        }

        // Type-based handlers
        if (messageHandlers[type]) {
          var handlers = messageHandlers[type];
          for (var i = 0; i < handlers.length; i++) {
            handlers[i](data);
          }
        }
      }

      function connect() {
        if (ws && (ws.readyState === WebSocket.CONNECTING || ws.readyState === WebSocket.OPEN)) {
          return;
        }

        try {
          ws = new WebSocket(getWsUrl());
        } catch (e) {
          error = e.message;
          yieldState();
          scheduleReconnect();
          return;
        }

        ws.onopen = function() {
          connected = true;
          error = null;
          reconnectDelay = 1000;
          yieldState();
        };

        ws.onmessage = function(e) {
          try {
            var data = JSON.parse(e.data);
            handleMessage(data);
            yieldState();
          } catch (err) {
            // Ignore unparseable messages
          }
        };

        ws.onclose = function() {
          connected = false;
          ws = null;
          yieldState();
          scheduleReconnect();
        };

        ws.onerror = function() {
          error = 'WebSocket error';
          yieldState();
        };
      }

      function scheduleReconnect() {
        if (reconnectTimer) return;
        reconnectTimer = setTimeout(function() {
          reconnectTimer = null;
          connect();
          // Exponential backoff: 1s, 2s, 4s, 8s, ... max 30s
          reconnectDelay = Math.min(reconnectDelay * 2, maxReconnectDelay);
        }, reconnectDelay);
      }

      // Global API for other dev primitives
      window.__stDevWs = {
        send: function(msg) {
          if (ws && ws.readyState === WebSocket.OPEN) {
            ws.send(typeof msg === 'string' ? msg : JSON.stringify(msg));
          }
        },

        onMessage: function(type, callback) {
          if (!messageHandlers[type]) {
            messageHandlers[type] = [];
          }
          messageHandlers[type].push(callback);
          // Return unsubscribe function
          return function() {
            var handlers = messageHandlers[type];
            if (handlers) {
              var idx = handlers.indexOf(callback);
              if (idx !== -1) handlers.splice(idx, 1);
            }
          };
        },

        onAck: function(opId, callback) {
          ackCallbacks[opId] = callback;
        },

        onReject: function(opId, callback) {
          rejectCallbacks[opId] = callback;
        },

        get connected() {
          return connected;
        }
      };

      // Start connection
      connect();

      %cleanup {
        if (reconnectTimer) {
          clearTimeout(reconnectTimer);
          reconnectTimer = null;
        }
        if (ws) {
          ws.onclose = null;
          ws.close();
          ws = null;
        }
        connected = false;
        window.__stDevWs = null;
      }
    })();
  }

  %exports {
    $connected: bool
    $lastMessage: object
    $error: string
  }
}
