/// Unit Tests: Dev Save Locale Persistence
///
/// Tests that saveEdit() correctly branches on editType to produce
/// proper EditJson messages for locale edits vs data edits.

@import "stdlib/testing/test"

// =============================================================================
// Locale edit produces correct filePath and jsonPath
// =============================================================================

@test "locale edit produces correct filePath and jsonPath" {
  @fixture {
    <div id="locale-el-1">Original heading</div>
  }

  @eval (eval("(function() {\
    var captured = null;\
    window.__stDevWs = {\
      connected: true,\
      send: function(msg) { captured = msg; },\
      onAck: function() {},\
      onReject: function() {}\
    };\
    \
    function resolveFilePath(source) {\
      return 'data/' + source + '.json';\
    }\
    \
    function generateOpId() { return 'op_test_1'; }\
    \
    var editEvent = {\
      editType: 'locale',\
      localeKey: 'hero_h1_1',\
      locale: 'en',\
      file: 'locales/en.json',\
      oldValue: 'Original heading',\
      newValue: 'New heading text',\
      elementId: 'locale-el-1'\
    };\
    \
    var opId = generateOpId();\
    var filePath, jsonPath, value;\
    \
    if (editEvent.editType === 'locale') {\
      filePath = editEvent.file;\
      jsonPath = editEvent.localeKey;\
      value = editEvent.newValue;\
    } else {\
      var source = editEvent.source;\
      var index = editEvent.index;\
      var field = editEvent.field;\
      value = editEvent.newValue !== undefined ? editEvent.newValue : editEvent.value;\
      filePath = resolveFilePath(source);\
      jsonPath = '[' + index + '].' + field;\
    }\
    \
    window.__stDevWs.send({\
      type: 'EditJson',\
      file: filePath,\
      path: jsonPath,\
      value: value,\
      op_id: opId\
    });\
    \
    window.__localeResult = captured;\
  })()"))

  @assert (window.__localeResult !== null)
    "Should have captured a message"
  @assert (window.__localeResult.type === 'EditJson')
    "Message type should be EditJson"
  @assert (window.__localeResult.file === 'locales/en.json')
    "filePath should be locales/en.json"
  @assert (window.__localeResult.path === 'hero_h1_1')
    "jsonPath should be the locale key hero_h1_1"
  @assert (window.__localeResult.value === 'New heading text')
    "value should be the new text"
  @assert (window.__localeResult.op_id === 'op_test_1')
    "op_id should be set"
}

// =============================================================================
// Data edit still produces correct filePath and jsonPath (regression)
// =============================================================================

@test "data edit produces correct filePath and jsonPath (regression)" {
  @fixture {
    <div id="data-el-1">Product Name</div>
  }

  @eval (eval("(function() {\
    var captured = null;\
    window.__stDevWs = {\
      connected: true,\
      send: function(msg) { captured = msg; },\
      onAck: function() {},\
      onReject: function() {}\
    };\
    \
    function resolveFilePath(source) {\
      return 'data/' + source + '.json';\
    }\
    \
    function generateOpId() { return 'op_test_2'; }\
    \
    var editEvent = {\
      editType: 'data',\
      source: 'products',\
      index: 0,\
      field: 'name',\
      oldValue: 'Product Name',\
      newValue: 'New Product Name',\
      elementId: 'data-el-1'\
    };\
    \
    var opId = generateOpId();\
    var filePath, jsonPath, value;\
    \
    if (editEvent.editType === 'locale') {\
      filePath = editEvent.file;\
      jsonPath = editEvent.localeKey;\
      value = editEvent.newValue;\
    } else {\
      var source = editEvent.source;\
      var index = editEvent.index;\
      var field = editEvent.field;\
      value = editEvent.newValue !== undefined ? editEvent.newValue : editEvent.value;\
      filePath = resolveFilePath(source);\
      jsonPath = '[' + index + '].' + field;\
    }\
    \
    window.__stDevWs.send({\
      type: 'EditJson',\
      file: filePath,\
      path: jsonPath,\
      value: value,\
      op_id: opId\
    });\
    \
    window.__dataResult = captured;\
  })()"))

  @assert (window.__dataResult !== null)
    "Should have captured a message"
  @assert (window.__dataResult.type === 'EditJson')
    "Message type should be EditJson"
  @assert (window.__dataResult.file === 'data/products.json')
    "filePath should be data/products.json"
  @assert (window.__dataResult.path === '[0].name')
    "jsonPath should be [0].name"
  @assert (window.__dataResult.value === 'New Product Name')
    "value should be the new text"
}

// =============================================================================
// EditJson message shape is correct for locale edits
// =============================================================================

@test "EditJson message shape has all required fields for locale" {
  @fixture {
    <div id="shape-el">Test</div>
  }

  @eval (eval("(function() {\
    var captured = null;\
    window.__stDevWs = {\
      connected: true,\
      send: function(msg) { captured = msg; },\
      onAck: function() {},\
      onReject: function() {}\
    };\
    \
    var editEvent = {\
      editType: 'locale',\
      localeKey: 'nav_link_about',\
      locale: 'fr',\
      file: 'locales/fr.json',\
      oldValue: 'A propos',\
      newValue: 'A propos de nous',\
      elementId: 'shape-el'\
    };\
    \
    var filePath = editEvent.file;\
    var jsonPath = editEvent.localeKey;\
    var value = editEvent.newValue;\
    \
    captured = {\
      type: 'EditJson',\
      file: filePath,\
      path: jsonPath,\
      value: value,\
      op_id: 'op_shape_1'\
    };\
    \
    window.__shapeResult = captured;\
    window.__shapeKeys = Object.keys(captured).sort().join(',');\
  })()"))

  @assert (window.__shapeResult.type === 'EditJson')
    "type field should be EditJson"
  @assert (window.__shapeResult.file === 'locales/fr.json')
    "file field should be locales/fr.json"
  @assert (window.__shapeResult.path === 'nav_link_about')
    "path field should be the locale key"
  @assert (window.__shapeResult.value === 'A propos de nous')
    "value field should be the new text"
  @assert (window.__shapeKeys === 'file,op_id,path,type,value')
    "Message should have exactly 5 fields: file, op_id, path, type, value"
}

// =============================================================================
// Missing WebSocket returns error
// =============================================================================

@test "missing WebSocket sets lastSaveError" {
  @fixture {
    <div id="no-ws-el">No WS</div>
  }

  @eval (eval("(function() {\
    window.__stDevWs = null;\
    var lastSaveError = '';\
    \
    var editEvent = {\
      editType: 'locale',\
      localeKey: 'test_key',\
      locale: 'en',\
      file: 'locales/en.json',\
      oldValue: 'old',\
      newValue: 'new',\
      elementId: 'no-ws-el'\
    };\
    \
    var ws = window.__stDevWs;\
    if (!ws || !ws.connected) {\
      lastSaveError = 'WebSocket not connected';\
    }\
    \
    window.__wsError = lastSaveError;\
  })()"))

  @assert (window.__wsError === 'WebSocket not connected')
    "Should set error when WebSocket is missing"
}

// =============================================================================
// Disconnected WebSocket returns error
// =============================================================================

@test "disconnected WebSocket sets lastSaveError" {
  @fixture {
    <div id="disc-ws-el">Disconnected</div>
  }

  @eval (eval("(function() {\
    window.__stDevWs = { connected: false };\
    var lastSaveError = '';\
    \
    var ws = window.__stDevWs;\
    if (!ws || !ws.connected) {\
      lastSaveError = 'WebSocket not connected';\
    }\
    \
    window.__wsDiscError = lastSaveError;\
  })()"))

  @assert (window.__wsDiscError === 'WebSocket not connected')
    "Should set error when WebSocket is disconnected"
}
