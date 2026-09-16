# Spacetime Dev Tools — WebSocket Edit Protocol

The Spacetime dev server supports live content editing via a WebSocket protocol. When running in debug mode, the dev tools UI is injected into pages and communicates with the server to persist edits in real time.

## Enabling Dev Tools

```bash
cargo run -- serve projects/<name>/ --debug
```

This compiles and injects `stdlib/__dev__/index.st` into every page, providing:

- A floating **ST Dev** indicator badge
- An interactive **dev panel** with Data, Timelines, and Errors tabs
- **Click-to-edit** support for data-bound elements
- A persistent **WebSocket connection** for sending edits to the server

## Architecture

```
Browser                             Dev Server
───────                             ──────────
stdlib/__dev__/
├─ dev-ws.st        ◄─── ws:// ───► src/dev_server.rs
│  (WebSocket)                       ├─ handle_client_message()
├─ dev-editable.st                   └─ dispatches to:
│  (click-to-edit)                       src/sync/handlers.rs
├─ dev-save.st                           ├─ handle_edit_json()
│  (orchestrator)                        ├─ handle_edit_html()
├─ dev-panel.st                          └─ handle_edit_ast()
│  (UI shell)                        src/sync/protocol.rs
├─ dev-inspect.st                        └─ message type definitions
│  (runtime data)
└─ macros/
   ├─ data-tree.st
   ├─ timelines-errors.st
   └─ st-properties.st
```

## WebSocket Protocol

All messages are JSON objects with a `type` field. The connection is established at `ws://<host>/ws`.

### Client → Server Messages

#### EditJson

Update a value in a JSON data file by path.

```json
{
  "type": "EditJson",
  "file": "data/products.json",
  "path": "[2].price",
  "value": 29.99,
  "op_id": "op_1"
}
```

| Field    | Type            | Description                                           |
|----------|-----------------|-------------------------------------------------------|
| `file`   | string          | Relative path to the JSON file from site root         |
| `path`   | string          | JSON path expression (e.g., `[0]`, `name`, `[2].price`, `a[0].b`) |
| `value`  | any JSON value  | New value to set at the path                          |
| `op_id`  | string          | Client-generated ID for tracking ack/reject           |

**Path syntax:** `[N]` for array indices, `field` for object keys, chained with dots. Examples: `[0].name`, `products[2].price`, `settings.theme.primary`.

#### EditHtml

Update an HTML element's inner content, targeted by `data-st-id` attribute.

```json
{
  "type": "EditHtml",
  "file": "index.html",
  "element_id": "hero-title",
  "content": "<h1>New Title</h1>",
  "op_id": "op_2"
}
```

| Field        | Type   | Description                                            |
|--------------|--------|--------------------------------------------------------|
| `file`       | string | Relative path to the HTML file from site root          |
| `element_id` | string | Value of the `data-st-id` attribute on the target element |
| `content`    | string | New innerHTML for the element                          |
| `op_id`      | string | Client-generated operation ID                          |

The handler uses [lol_html](https://github.com/nickel-org/lol_html) for streaming HTML rewriting — it finds the element matching `[data-st-id="<element_id>"]` and replaces its inner content.

#### EditAst

Patch properties of a Spacetime directive in a `.st` file.

```json
{
  "type": "EditAst",
  "file": "styles.st",
  "selector": ".hero @scroll",
  "patch": { "duration": "1200ms" },
  "op_id": "op_3"
}
```

| Field      | Type   | Description                                               |
|------------|--------|-----------------------------------------------------------|
| `file`     | string | Relative path to the `.st` file from site root            |
| `selector` | string | `<css-selector> @<directive>` format (e.g., `.hero @scroll`) |
| `patch`    | object | Key-value pairs of property names to new values           |
| `op_id`    | string | Client-generated operation ID                             |

The handler parses the `.st` file, locates the scope matching the CSS selector, finds the directive (FormMatch) by name, then performs text-based patching within the directive's source span. This preserves formatting unlike an AST roundtrip.

### Server → Client Messages

#### Ack

Confirms a successful edit.

```json
{
  "type": "Ack",
  "op_id": "op_1"
}
```

#### Reject

Reports a failed edit with a reason.

```json
{
  "type": "Reject",
  "op_id": "op_1",
  "reason": "File not found: data/missing.json"
}
```

Common rejection reasons:

| Reason                          | Cause                                       |
|---------------------------------|---------------------------------------------|
| `File not found: <path>`        | Target file does not exist                  |
| `Path traversal not allowed`    | Path escapes site directory (e.g., `../../`) |
| `No site directory configured`  | Server started without a site directory     |
| `Invalid JSON: <error>`         | Target file is not valid JSON               |
| `Field not found: <path>`       | JSON path navigates to a missing key        |
| `Index out of bounds: <path>`   | JSON path index exceeds array length        |
| `Element with data-st-id="..." not found` | No matching element in HTML file  |
| `Scope '<sel>' not found`       | CSS selector not found in .st file          |
| `Directive '@<name>' not found` | Directive not found in matched scope        |
| `Property '<key>' not found in directive body` | Patch key doesn't exist in directive |

#### DataUpdate

Broadcast to ALL connected clients after a successful EditJson. Contains the full updated file contents so clients can refresh their views without reloading.

```json
{
  "type": "DataUpdate",
  "source": "data/products.json",
  "data": [
    { "id": "1", "name": "Ceramic Mug", "price": 29.99 }
  ]
}
```

> **Note:** EditHtml and EditAst do NOT broadcast DataUpdate. HTML/AST edits write to disk, and the file watcher detects the change and sends a `Reload` message to all clients instead.

#### Reload

Sent when watched files change on disk (not part of the edit protocol, but relevant for the full picture).

```json
{
  "type": "Reload",
  "files": ["styles.st"]
}
```

Watched extensions: `.st`, `.edn`, and `*.bundle.js`.

### Proactive compile + `build-status.json`

On boot and on every watched file change, the dev server compiles the site's
default entry (`index.st` / `index.st.md` / `index.edn`) BEFORE sending
`Reload`, and writes the verdict to `<site>/build-status.json`:

```json
{
  "ok": true,
  "entry": "index.edn",
  "mtime_ms": 1787009560656,
  "content_sha256": "9f2c…",
  "diagnostics": []
}
```

This is the machine-readable seam for external processes (another compiler, an
agent harness): write a page source, hash it (sha256 hex), then poll
`build-status.json` until `content_sha256` matches — `ok` and `diagnostics`
are the compile verdict for THAT content. Hashing (not `mtime_ms`) is the
handshake: mtime truncation can alias two rapid writes, the hash cannot.

No browser needs to be connected; a missing file means "no compile happened",
never "compile failed" (failures write a status too).

EDN pages (PLAN-148) are first-class entries: `foo.edn` serves `foo.html`
exactly like `foo.st`, and EDN source enters the pipeline through
`Compiler::from_file`'s ingress, never through the printer.

## Security

All file operations validate paths against the site directory using `canonicalize()`:

1. The relative path is joined to the site root
2. Both are canonicalized to absolute paths
3. The file path must start with the site directory path
4. Path traversal attempts (e.g., `../../../etc/passwd`) are rejected

## Dev Tools Primitives

The browser-side dev tools live in `stdlib/__dev__/` and are compiled separately from user code.

| Primitive       | File                          | Purpose                                |
|-----------------|-------------------------------|----------------------------------------|
| `dev-flag`      | `index.st` (inline)           | Sets `window.__ST_DEV__ = true`        |
| `dev-ws`        | `primitives/dev-ws.st`        | WebSocket client with reconnection     |
| `dev-panel`     | `primitives/dev-panel.st`     | Shadow DOM panel UI shell              |
| `dev-inspect`   | `primitives/dev-inspect.st`   | Runtime data inspector (polls timelines, errors, data) |
| `dev-editable`  | `primitives/dev-editable.st`  | Click-to-edit for `[data-st-bind]` elements |
| `dev-save`      | `primitives/dev-save.st`      | Orchestrates editable → ws → server flow |

### Global APIs

Each primitive exposes a global for cross-primitive communication:

| Global                   | Key Methods / Properties                              |
|--------------------------|-------------------------------------------------------|
| `window.__stDevWs`       | `send(msg)`, `onMessage(type, cb)`, `onAck(opId, cb)`, `onReject(opId, cb)`, `connected` |
| `window.__stDevPanel`    | `shadow`, `getPane(name)`, `setTab(name)`             |
| `window.__stDevInspect`  | `getTimelines()`, `getErrors()`, `clearErrors()`, `refresh()` |
| `window.__stDevEditable` | `onEdit(cb)`, `getActiveEdit()`, `cancelEdit()`, `markSaving(el)`, `clearSaving(el)` |
| `window.__stDevSave`     | `save(editEvent)`                                     |

### Composition Macros

| Macro               | File                            | Purpose                                |
|----------------------|---------------------------------|----------------------------------------|
| `data-tree`          | `macros/data-tree.st`           | Interactive data tree in panel Data tab |
| `timelines-errors`   | `macros/timelines-errors.st`    | Timeline progress bars + error list    |
| `st-properties`      | `macros/st-properties.st`       | Editable animation properties          |
| `panel` (macro)      | `macros/panel.st`               | Declarative panel composition          |
| `editor` (macro)     | `macros/editor.st`              | Declarative editor composition         |

## Provenance Attributes

In debug mode, data-bound elements receive provenance attributes for dev tools targeting:

| Attribute         | Example Value           | Set By     | Purpose                          |
|-------------------|-------------------------|------------|-----------------------------------|
| `data-st-source`  | `data/products.json`    | `@each`    | Source data file path             |
| `data-st-index`   | `2`                     | `@each`    | Array index of this item          |
| `data-st-key`     | `prod-3`                | `@each`    | Key field value (keyed iteration) |
| `data-st-bind`    | `$.price`               | `@each`    | Data binding expression           |

These attributes are only injected when `window.__ST_DEV__` is `true` (set by `dev-flag`), ensuring zero overhead in production.

The `dev-editable` primitive uses these attributes to determine which file, index, and field to edit when a user clicks on a data-bound element.

## Edit Flow (End to End)

1. User clicks a `[data-st-bind]` element in the page
2. `dev-editable` makes the element `contenteditable`, reads provenance attributes
3. User types a new value, presses Enter
4. `dev-editable` fires an edit event with `{ source, index, field, value }`
5. `dev-save` receives the event, constructs an `EditJson` message, sends via `dev-ws`
6. Server receives `EditJson`, validates path, reads file, applies change, writes file
7. Server sends `Ack` to the editing client
8. Server broadcasts `DataUpdate` with full updated data to ALL clients
9. `dev-save` receives `DataUpdate`, calls `ST.setData()` to refresh bindings
10. All connected clients see the updated data without page reload
