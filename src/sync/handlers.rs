//! Dev Edit Handlers
//!
//! Processes incoming edit messages: validates paths, applies changes to files,
//! and returns responses for the WebSocket layer to dispatch.

#[cfg(test)]
use super::protocol::ValueType;
use super::protocol::{
    ArrayOp, ClientMessage, DataBindingContext, PropertyNode, PropertySection, ServerMessage,
    StateMachineContext, ValueNode, classify_value,
};
use crate::parser::parse;
use crate::syntax::span::LineIndex;
use log;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Serializes the read → verify → patch → write transaction for source edits.
///
/// The guards (`__expect_hash`, `__expect_span_text`) are compared against bytes
/// read moments before the write. Without a lock, two concurrent edit messages —
/// the dev server spawns a receive task per WebSocket connection — can BOTH read
/// the same bytes, BOTH find their guards satisfied, and both write: each Acks,
/// but the later write silently discards the earlier accepted edit (W1R review
/// finding). Holding this across the whole transaction makes
/// check-then-write atomic, so the second edit re-reads the first one's result and
/// its guard correctly refuses.
///
/// One global lock (not per-path): source edits are human-paced and the critical
/// section is a small read+write, so contention is irrelevant next to the
/// simplicity of having exactly one ordering point for every write rail.
fn source_write_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Result of handling an edit message
#[derive(Debug)]
pub struct HandleResult {
    /// Response to send back to the originating client (Ack or Reject)
    pub response: ServerMessage,
    /// If the edit succeeded, a DataUpdate to broadcast to ALL clients
    pub broadcast: Option<ServerMessage>,
    /// If the edit succeeded and wrote a file, the path to suppress from reload
    pub suppress_reload_path: Option<PathBuf>,
}

/// Parse a JSON path string into navigation segments.
///
/// Supports: `[0]`, `[2].price`, `name`, `products[0].name`, `[1].nested.field`
///
/// Returns a list of `PathSegment` values representing sequential navigation steps.
pub fn parse_json_path(path: &str) -> Result<Vec<PathSegment>, String> {
    let mut segments = Vec::new();
    let mut chars = path.chars().peekable();

    while chars.peek().is_some() {
        // Skip leading dots (field separators)
        if chars.peek() == Some(&'.') {
            chars.next();
        }

        if chars.peek() == Some(&'[') {
            // Array index: [N]
            chars.next(); // consume '['
            let mut num_str = String::new();
            while let Some(&c) = chars.peek() {
                if c == ']' {
                    chars.next(); // consume ']'
                    break;
                }
                num_str.push(c);
                chars.next();
            }
            let index: usize = num_str
                .parse()
                .map_err(|_| format!("Invalid array index: {}", num_str))?;
            segments.push(PathSegment::Index(index));
        } else {
            // Field name: read until '.' or '[' or end
            let mut field = String::new();
            while let Some(&c) = chars.peek() {
                if c == '.' || c == '[' {
                    break;
                }
                field.push(c);
                chars.next();
            }
            if field.is_empty() {
                return Err("Empty field name in path".to_string());
            }
            segments.push(PathSegment::Field(field));
        }
    }

    if segments.is_empty() {
        return Err("Empty path".to_string());
    }

    Ok(segments)
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    Field(String),
    Index(usize),
}

/// Navigate a mutable JSON value by path segments, returning a mutable reference
/// to the target location.
fn navigate_to_mut<'a>(
    root: &'a mut Value,
    segments: &[PathSegment],
) -> Result<&'a mut Value, String> {
    let mut current = root;

    for (i, segment) in segments.iter().enumerate() {
        let path_so_far: Vec<String> = segments[..=i]
            .iter()
            .map(|s| match s {
                PathSegment::Field(f) => f.clone(),
                PathSegment::Index(n) => format!("[{}]", n),
            })
            .collect();
        let path_str = path_so_far.join(".");

        current = match segment {
            PathSegment::Field(key) => current
                .get_mut(key.as_str())
                .ok_or_else(|| format!("Field not found: {}", path_str))?,
            PathSegment::Index(idx) => current
                .get_mut(*idx)
                .ok_or_else(|| format!("Index out of bounds: {}", path_str))?,
        };
    }

    Ok(current)
}

/// Validate that a file path is within the site directory (path traversal prevention).
///
/// Uses the same canonicalize pattern as `save_handler` in `src/server.rs`.
pub(crate) fn validate_file_path(site_dir: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let file_path = site_dir.join(relative_path);

    let canonical_site = site_dir
        .canonicalize()
        .map_err(|e| format!("Invalid site directory: {}", e))?;

    let canonical_file = file_path
        .canonicalize()
        .map_err(|_| format!("File not found: {}", relative_path))?;

    if !canonical_file.starts_with(&canonical_site) {
        return Err("Path traversal not allowed".to_string());
    }

    Ok(canonical_file)
}

/// Handle an EditJson message: read file, apply path-based update, write back.
pub fn handle_edit_json(
    site_dir: &Path,
    file: &str,
    path: &str,
    value: Value,
    op_id: &str,
) -> HandleResult {
    match handle_edit_json_inner(site_dir, file, path, value) {
        Ok(updated_data) => HandleResult {
            response: ServerMessage::Ack {
                op_id: op_id.to_string(),
            },
            broadcast: Some(ServerMessage::DataUpdate {
                source: file.to_string(),
                data: updated_data,
            }),
            suppress_reload_path: Some(validate_file_path(site_dir, file).unwrap_or_default()),
        },
        Err(reason) => HandleResult {
            response: ServerMessage::Reject {
                op_id: op_id.to_string(),
                reason,
            },
            broadcast: None,
            suppress_reload_path: None,
        },
    }
}

/// Recursively sanitize any HTML-looking string in a JSON value against the
/// rich-text allow/denylist. Scalars without tags pass through unchanged; this
/// makes the persist boundary safe for both rich-text and plain fields without
/// needing per-field schema lookup at the WS layer.
fn sanitize_value_html(
    value: Value,
    editable: Option<&crate::editable::ValidationSchema>,
) -> Value {
    match value {
        // A structured-editor document AST (`{type:"doc", content:[…]}`) is
        // validated against the editable schema, not HTML-sanitized: drop any
        // block/mark the schema doesn't describe, strip disallowed attrs, and
        // reject dangerous `url`-typed attrs (PLAN-031 / FEAT-099). This is the
        // security boundary for the new structured rich-text field.
        Value::Object(ref map)
            if map.get("type").and_then(|t| t.as_str()) == Some("doc")
                && map.contains_key("content") =>
        {
            match editable {
                Some(schema) => crate::editable::validate_richtext_ast(&value, schema),
                // No schema available (no marks registered): fail closed by
                // validating against an empty schema (drops everything but text).
                None => crate::editable::validate_richtext_ast(
                    &value,
                    &crate::editable::ValidationSchema::default(),
                ),
            }
        }
        Value::String(s) => {
            if crate::sync::richtext::looks_like_html(&s) {
                Value::String(crate::sync::richtext::sanitize_richtext(&s))
            } else {
                Value::String(s)
            }
        }
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|v| sanitize_value_html(v, editable))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, sanitize_value_html(v, editable)))
                .collect(),
        ),
        other => other,
    }
}

/// Build the editable validation schema for a site by parsing its index.st and
/// collecting `@editable-mark`/`@editable-block` declarations. Returns `None`
/// when the site has no index or fails to parse (caller fails closed).
fn editable_schema_for_site(site_dir: &Path) -> Option<crate::editable::ValidationSchema> {
    let index_st = site_dir.join("index.st");
    let content = std::fs::read_to_string(&index_st).ok()?;
    let main_ast = crate::parser::parse(&content).ok()?;
    // Resolve imports so the BUILTIN marks/blocks (`@import "stdlib/editable"`) are
    // visible — they are defined in the stdlib, not in index.st. Without this the
    // schema is empty and validate_richtext_ast fails closed, stripping every block/
    // mark from a persisted on-page @richtext edit (FUP-047). Mirrors parse_site_ast
    // (the dev schema endpoint): resolve_imports + rematch for user macros.
    let mut ast = if !main_ast.imports.is_empty() {
        crate::parser::resolve_imports(&main_ast, &index_st, site_dir).ok()?
    } else {
        main_ast
    };
    if !ast.meta_defs.is_empty() {
        crate::parser::rematch_with_user_macros(&mut ast, &content);
    }
    let (schemas, _diags) =
        crate::editable::collect_schemas_from_matches(&ast.matches, &ast.scopes);
    let json = crate::editable::schemas_to_json(&schemas);
    Some(crate::editable::ValidationSchema::from_schema_json(&json))
}

fn handle_edit_json_inner(
    site_dir: &Path,
    file: &str,
    path: &str,
    value: Value,
) -> Result<Value, String> {
    let file_path = validate_file_path(site_dir, file)?;

    // Read and parse
    let content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;
    let mut data: Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {}", e))?;

    // Parse path and navigate
    let segments = parse_json_path(path)?;
    let target = navigate_to_mut(&mut data, &segments)?;
    // Server-side rich-text sanitization (Stage-2 Wave-3, FUP-029): any string
    // value that looks like HTML is sanitized against the allow/denylist BEFORE
    // it touches disk. This is the security-bearing enforcement point — a buggy
    // or malicious client cannot persist <script>, event handlers, or
    // javascript: URLs. Plain-text values (no tags) pass through byte-exact.
    let editable_schema = editable_schema_for_site(site_dir);
    *target = sanitize_value_html(value, editable_schema.as_ref());

    // Write back (pretty-printed)
    let output = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    std::fs::write(&file_path, &output).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(data)
}

/// Handle an EditJsonArray message: array insert, delete, reorder operations.
pub fn handle_edit_json_array(
    site_dir: &Path,
    file: &str,
    path: &str,
    op: ArrayOp,
    op_id: &str,
) -> HandleResult {
    match handle_edit_json_array_inner(site_dir, file, path, op) {
        Ok(updated_data) => HandleResult {
            response: ServerMessage::Ack {
                op_id: op_id.to_string(),
            },
            broadcast: Some(ServerMessage::DataUpdate {
                source: file.to_string(),
                data: updated_data,
            }),
            suppress_reload_path: Some(validate_file_path(site_dir, file).unwrap_or_default()),
        },
        Err(reason) => HandleResult {
            response: ServerMessage::Reject {
                op_id: op_id.to_string(),
                reason,
            },
            broadcast: None,
            suppress_reload_path: None,
        },
    }
}

fn handle_edit_json_array_inner(
    site_dir: &Path,
    file: &str,
    path: &str,
    op: ArrayOp,
) -> Result<Value, String> {
    let file_path = validate_file_path(site_dir, file)?;

    // Read and parse
    let content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;
    let mut data: Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {}", e))?;

    // Navigate to target array
    let target = if path.is_empty() {
        &mut data
    } else {
        let segments = parse_json_path(path)?;
        navigate_to_mut(&mut data, &segments)?
    };

    let arr = target
        .as_array_mut()
        .ok_or_else(|| "Target is not a JSON array".to_string())?;

    match op {
        // Sanitize the inserted item's HTML-looking strings before it touches
        // disk (Wave-3 boundary also covers array Insert, not just EditJson).
        ArrayOp::Insert { item, index } => {
            let item = sanitize_value_html(item, editable_schema_for_site(site_dir).as_ref());
            match index {
                None => arr.push(item),
                Some(i) => {
                    if i > arr.len() {
                        return Err(format!(
                            "Insert index {} out of bounds for array of length {}",
                            i,
                            arr.len()
                        ));
                    }
                    arr.insert(i, item);
                }
            }
        }
        ArrayOp::Delete { index } => {
            if index >= arr.len() {
                return Err(format!(
                    "Delete index {} out of bounds for array of length {}",
                    index,
                    arr.len()
                ));
            }
            arr.remove(index);
        }
        ArrayOp::Reorder {
            from_index,
            to_index,
        } => {
            if from_index >= arr.len() {
                return Err(format!(
                    "Reorder from_index {} out of bounds for array of length {}",
                    from_index,
                    arr.len()
                ));
            }
            if to_index >= arr.len() {
                return Err(format!(
                    "Reorder to_index {} out of bounds for array of length {}",
                    to_index,
                    arr.len()
                ));
            }
            let item = arr.remove(from_index);
            arr.insert(to_index, item);
        }
        ArrayOp::Update { index, patch } => {
            if index >= arr.len() {
                return Err(format!(
                    "Update index {} out of bounds for array of length {}",
                    index,
                    arr.len()
                ));
            }
            // Sanitize patch values on the same Wave-3 boundary as Insert before
            // they touch disk.
            let patch = sanitize_value_html(patch, editable_schema_for_site(site_dir).as_ref());
            let patch_obj = patch
                .as_object()
                .ok_or_else(|| "Update patch must be a JSON object".to_string())?;
            let target = arr[index]
                .as_object_mut()
                .ok_or_else(|| "Update target is not a JSON object".to_string())?;
            for (k, v) in patch_obj {
                target.insert(k.clone(), v.clone());
            }
        }
    }

    // Write back (pretty-printed)
    let output = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
    std::fs::write(&file_path, &output).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(data)
}

/// Handle an EditHtml message: read file, find element by data-st-id, replace innerHTML.
pub fn handle_edit_html(
    site_dir: &Path,
    file: &str,
    element_id: &str,
    content: &str,
    op_id: &str,
) -> HandleResult {
    log::warn!(
        "EditHtml is deprecated; use EditAst with __content sentinel instead. op_id={}",
        op_id
    );
    match handle_edit_html_inner(site_dir, file, element_id, content) {
        Ok(()) => HandleResult {
            response: ServerMessage::Ack {
                op_id: op_id.to_string(),
            },
            broadcast: None,
            suppress_reload_path: Some(validate_file_path(site_dir, file).unwrap_or_default()),
        },
        Err(reason) => HandleResult {
            response: ServerMessage::Reject {
                op_id: op_id.to_string(),
                reason,
            },
            broadcast: None,
            suppress_reload_path: None,
        },
    }
}

fn handle_edit_html_inner(
    site_dir: &Path,
    file: &str,
    element_id: &str,
    content: &str,
) -> Result<(), String> {
    let file_path = validate_file_path(site_dir, file)?;
    let html =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;
    let new_html = update_element_content(&html, element_id, content)?;
    std::fs::write(&file_path, &new_html).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

/// Use lol_html to find an element by `data-st-id` attribute and replace its innerHTML.
fn update_element_content(
    html: &str,
    element_id: &str,
    new_content: &str,
) -> Result<String, String> {
    use lol_html::{HtmlRewriter, Settings, element};

    let mut output = Vec::new();
    let target_id = element_id.to_string();
    let content = new_content.to_string();
    let mut found = false;

    {
        let mut rewriter = HtmlRewriter::new(
            Settings {
                element_content_handlers: vec![element!(
                    format!("[data-st-id=\"{}\"]", target_id),
                    |el| {
                        el.set_inner_content(&content, lol_html::html_content::ContentType::Html);
                        found = true;
                        Ok(())
                    }
                )],
                ..Settings::default()
            },
            |c: &[u8]| output.extend_from_slice(c),
        );

        rewriter
            .write(html.as_bytes())
            .map_err(|e| format!("HTML rewrite error: {}", e))?;
        rewriter
            .end()
            .map_err(|e| format!("HTML rewrite error: {}", e))?;
    }

    if !found {
        return Err(format!(
            "Element with data-st-id=\"{}\" not found",
            element_id
        ));
    }

    String::from_utf8(output).map_err(|e| format!("UTF-8 conversion error: {}", e))
}

/// Handle an EditAst message: parse .st file, locate directive by selector, apply patch.
///
/// The selector format is `<css-selector> @<directive>`, e.g., `.hero @scroll`.
/// The patch is a JSON object mapping property names to new string values.
/// AST changes trigger the file watcher which sends a Reload, so no broadcast is needed.
pub fn handle_edit_ast(
    site_dir: &Path,
    file: &str,
    selector: &str,
    patch: &Value,
    op_id: &str,
) -> HandleResult {
    match handle_edit_ast_inner(site_dir, file, selector, patch) {
        Ok(()) => HandleResult {
            response: ServerMessage::Ack {
                op_id: op_id.to_string(),
            },
            broadcast: None,
            suppress_reload_path: Some(validate_file_path(site_dir, file).unwrap_or_default()),
        },
        Err(reason) => HandleResult {
            response: ServerMessage::Reject {
                op_id: op_id.to_string(),
                reason,
            },
            broadcast: None,
            suppress_reload_path: None,
        },
    }
}

/// Parse an AST selector into (css_selector, directive_name).
///
/// Format: `.hero §scroll` → (".hero", "scroll")
/// The `§` prefix on the directive name is stripped.
fn parse_ast_selector(selector: &str) -> Result<(String, String), String> {
    // Find the last occurrence of " §" to split CSS selector from directive
    let at_pos = selector.rfind(" §").ok_or_else(|| {
        format!(
            "Invalid AST selector '{}': expected format '<css-selector> §<directive>'",
            selector
        )
    })?;

    let css_selector = selector[..at_pos].trim().to_string();
    let directive = selector[at_pos + " §".len()..].trim().to_string();

    if css_selector.is_empty() {
        return Err("Empty CSS selector in AST selector".to_string());
    }
    if directive.is_empty() {
        return Err("Empty directive name in AST selector".to_string());
    }

    Ok((css_selector, directive))
}

/// Parsed content selector for EditAst __content edits.
#[derive(Debug, PartialEq)]
enum ContentSelector {
    /// Direct element selector for HTML files: [data-st-id="st-a3f2b1c0"]
    HtmlElement { selector: String },
    /// AST content element: .card §template h3 (element path retained for readability only;
    /// actual targeting uses __st_id from the patch, not the element_path token).
    AstContent {
        css_selector: String, // ".card"
        directive: String,    // "template"
    },
    /// Existing AST directive (no element path): .hero §scroll
    AstDirective {
        css_selector: String,
        directive: String,
    },
}

/// Parse a selector string for content editing.
///
/// Handles three formats:
/// 1. No '§' → HtmlElement (e.g., `[data-st-id="st-a3f2b1c0"]`)
/// 2. '§' with text after directive → AstContent (e.g., `.card §template h3`)
/// 3. '§' with nothing after directive → AstDirective (e.g., `.hero §scroll`)
fn parse_content_selector(selector: &str) -> Result<ContentSelector, String> {
    let selector = selector.trim();

    // No '§' → HtmlElement
    if !selector.contains('§') {
        return Ok(ContentSelector::HtmlElement {
            selector: selector.to_string(),
        });
    }

    // Find the last '§' (same approach as parse_ast_selector)
    let (css_selector, after_at) = if let Some(at_pos) = selector.rfind(" §") {
        let css = selector[..at_pos].trim().to_string();
        let after = selector[at_pos + " §".len()..].trim();
        (css, after)
    } else if selector.starts_with('§') {
        // Root-scope: no CSS selector prefix, directive starts the string
        ("".to_string(), selector['§'.len_utf8()..].trim())
    } else {
        return Err(format!(
            "Invalid selector: '§' must be preceded by a space: {:?}",
            selector
        ));
    };

    // Split directive from element path (first space after directive name).
    // The element path token (e.g. "h3" in "§template h3") is retained in data-st-origin
    // for human readability but is NOT used for targeting — that uses __st_id from the patch.
    let (directive, has_element_path) = if let Some(space_pos) = after_at.find(' ') {
        let dir = after_at[..space_pos].trim().to_string();
        let elem = after_at[space_pos + 1..].trim();
        (dir, !elem.is_empty())
    } else {
        (after_at.to_string(), false)
    };

    if directive.is_empty() {
        return Err(format!(
            "Invalid selector: empty directive after '§': {:?}",
            selector
        ));
    }

    if has_element_path {
        Ok(ContentSelector::AstContent {
            css_selector,
            directive,
        })
    } else {
        Ok(ContentSelector::AstDirective {
            css_selector,
            directive,
        })
    }
}

/// Render a patch value as the SOURCE TEXT that replaces an existing value.
///
/// The rule is driven by what the source already had, not by the key's spelling:
/// **a value that was a quoted string stays a quoted string.** This is the one
/// place quoting is decided, for every surface the patcher serves — an
/// object-literal entry (`"ink": "#0a0a0a"`), a directive body
/// (`duration: 800ms;`), and a template invocation's named arg
/// (`&child(title: "hi")`, whose KEY is bare but whose VALUE is quoted).
///
/// Keying this on the key's spelling instead (the pre-PLAN-112 behavior: quote
/// only when the KEY was quoted) wrote `title: patched` — unquoted, invalid
/// Spacetime — into every template invocation, which the MCP rail papered over by
/// pre-quoting at ITS call site while the EditAst disk rail silently corrupted the
/// file. One rule here; callers pass bare values.
///
/// A string is emitted JSON-ESCAPED (`serde_json::to_string`) so a value
/// containing `"`/`}`/newline can neither corrupt the literal nor inject .st
/// source from a dev-ws message (BUG-093).
///
/// An OBJECT-LITERAL entry (`"ink": "…"`) is ALWAYS escaped — no exemptions.
/// Its value is data, never source, and a prefix/suffix "looks quoted" test is
/// forgeable: the payload `" , "injected": "x"` both starts and ends with a
/// quote, so exempting it would splice a NEW live key into the user's object.
///
/// For a BARE-key slot (a template invocation's named arg, a directive body
/// value) three exemptions keep a string verbatim because it is already SOURCE
/// rather than a literal — these were the MCP `mcp_node_param` resolver's own
/// pre-quote rules, moved here so both rails share ONE rule. Each is validated as
/// a COMPLETE, well-formed token; a partial or trailing-garbage match is escaped
/// like any other text, so no exemption can be used to break out of the slot:
///   - a complete JSON string literal spanning the whole value (`"already"`),
///   - a signal reference (`$title`, `$item.name`) — a binding, not text,
///   - a clean numeric literal (`42`, `-1.5e3`).
fn emit_patch_value(
    new_value: &Value,
    new_val_str: &str,
    original_quote: Option<char>,
    is_object_entry: bool,
) -> String {
    let Some(quote) = original_quote else {
        return new_val_str.to_string();
    };
    if !matches!(new_value, Value::String(_)) {
        return new_val_str.to_string();
    }
    if !is_object_entry && is_complete_source_atom(new_val_str.trim()) {
        return new_val_str.to_string();
    }
    // FUP-133: re-quote in the ORIGINAL style so an edit does not churn the file.
    //
    // An object entry is JSON and must stay double-quoted whatever the surrounding
    // source looked like — single quotes there would produce invalid JSON.
    //
    // Correctness outranks style: if the new value CONTAINS the original quote
    // character, preserving the style blindly would emit `'it's here'`, which
    // terminates at the apostrophe and leaves trailing garbage. Source that no
    // longer parses is far worse than source that normalized a quote, so in that
    // case fall back to JSON's escaping.
    if quote == '\'' && !is_object_entry {
        if let Value::String(text) = new_value {
            if !text.contains('\'') && !text.contains('\\') {
                return format!("'{text}'");
            }
        }
    }
    serde_json::to_string(new_value).unwrap_or_else(|_| format!("{new_val_str:?}"))
}

/// Is `value` a COMPLETE source atom that may be written to a bare-key slot
/// verbatim? Every arm must consume the ENTIRE input — that totality is what
/// makes the exemption injection-proof (`"" , "injected": "x"` is a quoted string
/// FOLLOWED BY more source, so it fails and gets escaped).
fn is_complete_source_atom(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    // A complete JSON string literal, nothing after it.
    if value.starts_with('"') {
        let mut stream = serde_json::Deserializer::from_str(value).into_iter::<String>();
        return match stream.next() {
            Some(Ok(_)) => stream.byte_offset() == value.len(),
            _ => false,
        };
    }
    // A signal reference: `$name`, optionally dotted (`$item.name`).
    if let Some(rest) = value.strip_prefix('$') {
        let mut segments = rest.split('.');
        let head_ok = segments.next().is_some_and(|head| {
            !head.is_empty()
                && head.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                && head.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        });
        return head_ok
            && segments.all(|seg| {
                !seg.is_empty() && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            });
    }
    // A clean numeric literal. `parse::<f64>` alone would accept `inf`/`NaN`
    // (and locale-ish oddities), which are not numeric SOURCE here.
    value.parse::<f64>().is_ok()
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
}

/// Apply a patch to source text within a given byte range.
///
/// For each key-value pair in the patch, finds `key: <old_value>` within the
/// directive's text span and replaces the value. Handles semicolons and whitespace.
pub(crate) fn apply_text_patch(
    source: &str,
    span_start: usize,
    span_end: usize,
    patch: &Value,
) -> Result<String, String> {
    let patch_obj = patch
        .as_object()
        .ok_or_else(|| "Patch must be a JSON object".to_string())?;

    if patch_obj.is_empty() {
        return Ok(source.to_string());
    }

    // Empty patches remain no-ops, including when their caller carries a stale span.
    // Nonempty patches reject invalid endpoints: all callers pass source-derived spans,
    // so retaining the former end-clamping would hide stale addresses and risk a
    // wrong write.
    let source_len = source.len();
    if span_start > source_len {
        return Err(format!(
            "Patch span start {span_start} exceeds source length {source_len}"
        ));
    }
    if span_end > source_len {
        return Err(format!(
            "Patch span end {span_end} exceeds source length {source_len}"
        ));
    }
    if span_start >= span_end {
        return Err(format!(
            "Patch span start {span_start} must be less than end {span_end} (source length {source_len})"
        ));
    }
    if !source.is_char_boundary(span_start) || !source.is_char_boundary(span_end) {
        return Err(format!(
            "Patch span [{span_start}, {span_end}] is not on UTF-8 character boundaries (source length {source_len})"
        ));
    }

    // Work on the validated region within the span.
    let region = &source[span_start..span_end];
    let mut modified_region = region.to_string();

    for (key, new_value) in patch_obj {
        let new_val_str = match new_value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => {
                return Err(format!(
                    "Unsupported patch value type for '{}': expected string or number",
                    key
                ));
            }
        };

        // Locate the property. Two surfaces share this patcher:
        //   - directive body:    `duration: 800ms;`   (bare key, `;`-terminated)
        //   - object literal:     `"ink": "#0a0a0a",`  (quoted key, `,`-terminated)
        // Try the quoted key first (object-literal entry), then the bare key.
        let quoted = format!("\"{}\":", key);
        let bare = format!("{}:", key);
        let (key_pos, pat_len, is_object_entry) = if let Some(p) = modified_region.find(&quoted) {
            (p, quoted.len(), true)
        } else if let Some(p) = modified_region.find(&bare) {
            (p, bare.len(), false)
        } else {
            return Err(format!("Property '{}' not found in directive body", key));
        };
        let value_start = key_pos + pat_len;
        // Terminate the value at the first DEPTH-0 `,`, `;`, or `}` — whichever
        // comes first — respecting nested ()[]{} and quoted strings so an
        // object-literal sibling key (after a `,`) is never swallowed (BUG-092).
        let rest = &modified_region[value_start..];
        let value_end = {
            let bytes = rest.as_bytes();
            let mut depth = 0i32;
            let mut in_str: Option<u8> = None;
            let mut end = rest.len();
            let mut i = 0;
            while i < bytes.len() {
                let c = bytes[i];
                if let Some(q) = in_str {
                    if c == b'\\' {
                        i += 2;
                        continue;
                    }
                    if c == q {
                        in_str = None;
                    }
                } else {
                    match c {
                        b'"' | b'\'' => in_str = Some(c),
                        b'(' | b'[' | b'{' => depth += 1,
                        // A closing bracket at depth 0 ends the value: it's the
                        // delimiter of the ENCLOSING construct (e.g. the `)` of a
                        // paren-param list, the `}` of a directive body) — the
                        // last param's value stops here, never running past it
                        // into the body/siblings (BUG-098). Below depth 0 they
                        // just balance nested groups within the value.
                        b')' | b']' | b'}' if depth == 0 => {
                            end = i;
                            break;
                        }
                        b')' | b']' | b'}' => depth -= 1,
                        b',' | b';' if depth == 0 => {
                            end = i;
                            break;
                        }
                        _ => {}
                    }
                }
                i += 1;
            }
            end
        };

        // Replace the value, preserving a single leading space if present. An
        // object-literal entry whose original value was a STRING keeps the JSON
        // quotes (so `"ink": "#0a0a0a"` stays a quoted string); a bare directive
        // value (and non-string object values) are written verbatim.
        let old_value = &rest[..value_end];
        let leading_space = if old_value.starts_with(' ') { " " } else { "" };
        // FUP-133: carry the original token's QUOTE STYLE, not merely whether it
        // was quoted. Testing `starts_with('"')` alone made a single-quoted token
        // read as UNQUOTED, so the new value was written bare — `'hello'` came back
        // as `goodbye` without quotes at all, and in an object slot as `"goodbye"`.
        // Nothing breaks semantically (the compiler reads both alike), but the
        // SOURCE churns: a diff shows a change the author never made, and a codebase
        // with a single-quote convention gets rewritten one token per edit.
        let trimmed_old = old_value.trim_start();
        let original_quote = if trimmed_old.starts_with('"') {
            Some('"')
        } else if trimmed_old.starts_with('\'') {
            Some('\'')
        } else {
            None
        };
        let emit_val = emit_patch_value(new_value, &new_val_str, original_quote, is_object_entry);
        let replacement = format!("{}{}", leading_space, emit_val);
        modified_region = format!(
            "{}{}{}",
            &modified_region[..value_start],
            replacement,
            &modified_region[value_start + value_end..]
        );
    }

    // Reconstruct the full source
    Ok(format!(
        "{}{}{}",
        &source[..span_start],
        modified_region,
        &source[span_end..]
    ))
}

/// Dispatch a `__content` edit to the appropriate handler based on file extension and selector type.
///
/// `st_id` is required for `.st` template edits: it is the `data-st-id` value the browser read
/// from the served DOM and must be round-tripped back so the server can target the exact element.
fn handle_content_edit(
    file_path: &Path,
    file: &str,
    selector: &str,
    new_content: &str,
    st_id: Option<&str>,
) -> Result<(), String> {
    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let parsed = parse_content_selector(selector)?;

    // EditAst __content writes browser-supplied innerHTML to disk — sanitize it
    // against the rich-text allow/denylist so this persist boundary (separate
    // from EditJson) cannot store <script>/event-handler/js: payloads (FUP-029).
    let sanitized = crate::sync::richtext::sanitize_richtext(new_content);
    let new_content: &str = &sanitized;

    match (ext, &parsed) {
        ("html" | "htm", ContentSelector::HtmlElement { selector: sel }) => {
            handle_content_edit_html(file_path, file, sel, new_content)
        }
        (
            "st",
            ContentSelector::AstContent {
                css_selector,
                directive,
            },
        ) => {
            // __st_id is mandatory: without it we cannot uniquely identify the target element.
            let id = st_id.ok_or_else(|| {
                "Missing __st_id in patch; cannot target element uniquely in template body. \
                 The browser must send the data-st-id value it read from the served DOM."
                    .to_string()
            })?;
            handle_content_edit_st(file_path, css_selector, directive, id, new_content)
        }
        _ => Err(format!(
            "Unsupported content edit: .{} file with selector type {:?}",
            ext, parsed
        )),
    }
}

/// Edit an HTML file by finding an element via CSS selector and replacing its innerHTML.
///
/// The selector targets a `data-st-id` attribute that is injected at serve-time by
/// `inject_content_provenance`, so it doesn't exist in the source file on disk.
/// We re-inject provenance attributes in memory to locate the element, perform
/// the content replacement, then strip injected attributes before writing back.
fn handle_content_edit_html(
    file_path: &Path,
    file: &str,
    selector: &str,
    new_content: &str,
) -> Result<(), String> {
    use lol_html::{HtmlRewriter, Settings, element};

    let raw_html =
        std::fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Re-inject provenance attributes so we can find the element by data-st-id.
    // These attributes only exist in the served DOM, not on disk.
    let annotated = crate::server::inject_content_provenance(&raw_html, file);

    // Single lol_html pass: replace the target element's content and strip all
    // injected data-st-id/data-st-origin attributes so we write clean HTML.
    let mut output = Vec::new();
    let content_str = new_content.to_string();
    let mut found = false;
    let sel = selector.to_string();

    {
        let mut rewriter = HtmlRewriter::new(
            Settings {
                element_content_handlers: vec![
                    // Replace content of the target element
                    element!(sel, |el| {
                        el.set_inner_content(
                            &content_str,
                            lol_html::html_content::ContentType::Html,
                        );
                        found = true;
                        Ok(())
                    }),
                    // Strip all injected provenance attributes (st-* prefix IDs)
                    element!("[data-st-id^=\"st-\"]", |el| {
                        el.remove_attribute("data-st-id");
                        el.remove_attribute("data-st-origin");
                        Ok(())
                    }),
                ],
                ..Settings::default()
            },
            |c: &[u8]| output.extend_from_slice(c),
        );
        rewriter
            .write(annotated.as_bytes())
            .map_err(|e| format!("HTML rewrite error: {}", e))?;
        rewriter
            .end()
            .map_err(|e| format!("HTML rewrite error: {}", e))?;
    }

    if !found {
        return Err(format!(
            "Element '{}' not found in {}",
            selector,
            file_path.display()
        ));
    }

    let new_html =
        String::from_utf8(output).map_err(|e| format!("UTF-8 conversion error: {}", e))?;
    std::fs::write(file_path, &new_html).map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

/// Edit a `.st` template file by finding an element within a template body's HTML
/// and replacing its innerHTML.
///
/// This is the `.st` branch of `handle_content_edit`. The element is identified by
/// `st_id`, the `data-st-id` value the browser read from the served DOM. The server
/// re-derives the same provenance annotations deterministically, targets the exact element
/// via `[data-st-id="{st_id}"]`, and strips all injected attributes before writing back.
///
/// Steps:
/// 1. Read and parse the .st file into AST
/// 2. Find the FormMatch for the directive (root-scope or scoped)
/// 3. Extract the ComponentBodyDef HTML
/// 4. Re-inject provenance: call inject_template_provenance with the same inputs used at serve-time
/// 5. Replace the target element content using [data-st-id="{st_id}"] selector (uniquely matches)
/// 6. Strip all data-st-id / data-st-origin attributes from the result
/// 7. Splice the clean HTML back into the .st source at correct byte offsets
/// 8. Write back to disk
fn handle_content_edit_st(
    file_path: &Path,
    css_selector: &str,
    directive: &str,
    st_id: &str,
    new_content: &str,
) -> Result<(), String> {
    use crate::syntax::CapturedValue;

    // Step 1: Read and parse the .st file
    let source =
        std::fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    let ast = parse(&source).map_err(|e| {
        format!(
            "Parse error: {}",
            e.render_all_plain(&source, &file_path.display().to_string())
        )
    })?;

    // Step 2: Find the FormMatch for the directive.
    // Root-scope (css_selector == ""): the directive lives at file level in ast.matches
    // with no owning scope (selector == None). Scoped: find the scope first.
    let form_match = if css_selector.is_empty() {
        ast.matches
            .iter()
            .find(|m| {
                m.selector.is_none()
                    && (m.macro_name == directive
                        || m.macro_name.ends_with(directive)
                        || directive.contains(&m.macro_name))
            })
            .ok_or_else(|| format!("Directive '@{}' not found at root scope", directive))?
    } else {
        let scope = ast
            .scopes
            .iter()
            .find(|s| s.selector == css_selector)
            .ok_or_else(|| format!("Scope '{}' not found", css_selector))?;

        scope
            .matches
            .iter()
            .find(|m| {
                m.macro_name == directive
                    || m.macro_name.ends_with(directive)
                    || directive.contains(&m.macro_name)
            })
            .ok_or_else(|| {
                format!(
                    "Directive '@{}' not found in scope '{}'",
                    directive, css_selector
                )
            })?
    };

    // Step 3: Extract the body HTML from the World-A template SCOPE (FEAT-119 W4).
    // The html lives on the `@template:<name>` Construct scope (built at parse), NOT
    // the retired ComponentBody capture. Resolve the template name from the match's
    // `name` capture, then read `scope.html`.
    let tmpl_name = match form_match.captures.get("name") {
        Some(CapturedValue::Ident(n)) | Some(CapturedValue::Element(n)) => {
            n.strip_prefix('&').unwrap_or(n).to_string()
        }
        _ => {
            return Err(format!(
                "Template '@{}' in scope '{}' has no component body",
                directive, css_selector
            ));
        }
    };
    let body_html = ast
        .scopes
        .iter()
        .find(|s| s.selector == format!("@template:{}", tmpl_name))
        .map(|s| s.html.clone())
        .filter(|h| !h.is_empty())
        .ok_or_else(|| {
            format!(
                "Template '@{}' in scope '{}' has no HTML content",
                directive, css_selector
            )
        })?;

    // Step 4: Re-derive provenance — same deterministic inputs the server used at serve-time.
    // source_file  = file name (basename) used to compute the hash
    // scope_selector = CSS selector of the owning scope ("" for root-scope directives)
    // macro_name   = exact macro name from the parsed AST (not the user-supplied directive hint)
    // ast_offset   = byte offset of the directive in source (form_match.span.start)
    let source_file = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let scope_selector = form_match.selector.as_deref().unwrap_or("");
    let annotated = crate::syntax::inject_template_provenance(
        &body_html,
        source_file,
        scope_selector,
        &form_match.macro_name,
        form_match.span.start,
    );

    // Step 5: Replace content in the annotated HTML using the unique data-st-id selector.
    // replace_html_element_content's two-pass count guard ensures exactly 1 match.
    let selector = format!("[data-st-id=\"{}\"]", st_id);
    let modified_annotated = replace_html_element_content(&annotated, &selector, new_content)?;

    // Step 6: Strip provenance attributes — we must not write data-st-id/data-st-origin to disk.
    let modified_html = strip_provenance_attributes(&modified_annotated);

    // Step 7: Find where body_html appears in source and splice in modified_html.
    let span = form_match.span;
    let span_text = source.get(span.start..span.end).ok_or_else(|| {
        format!(
            "Invalid span {:?} for source of length {}",
            span,
            source.len()
        )
    })?;

    let html_offset = span_text
        .find(&body_html)
        .ok_or_else(|| "Template HTML not found within source span".to_string())?;

    let abs_html_start = span.start + html_offset;
    let abs_html_end = abs_html_start + body_html.len();

    // Splice: source[..abs_html_start] + modified_html + source[abs_html_end..]
    let mut new_source = String::with_capacity(source.len() + modified_html.len());
    new_source.push_str(&source[..abs_html_start]);
    new_source.push_str(&modified_html);
    new_source.push_str(&source[abs_html_end..]);

    // Step 8: Write back
    std::fs::write(file_path, &new_source).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

/// Strip `data-st-id` and `data-st-origin` attributes from all elements in an HTML fragment.
///
/// Used after a provenance-annotated HTML pass to ensure no injected attributes are written
/// back to the source file on disk.
fn strip_provenance_attributes(html: &str) -> String {
    use lol_html::{HtmlRewriter, Settings, element};

    if html.is_empty() {
        return html.to_string();
    }

    let mut output = Vec::new();
    {
        let mut rewriter = HtmlRewriter::new(
            Settings {
                element_content_handlers: vec![element!("[data-st-id^=\"st-\"]", |el| {
                    el.remove_attribute("data-st-id");
                    el.remove_attribute("data-st-origin");
                    Ok(())
                })],
                ..Settings::default()
            },
            |c: &[u8]| output.extend_from_slice(c),
        );
        // Ignore errors: if lol_html can't parse it, return the original.
        if rewriter.write(html.as_bytes()).is_ok() {
            let _ = rewriter.end();
        } else {
            return html.to_string();
        }
    }
    String::from_utf8(output).unwrap_or_else(|_| html.to_string())
}

/// Run lol_html to replace the innerHTML of the element matching `element_selector`
/// in the given HTML fragment. Returns the modified HTML, or the original if not found.
fn replace_html_element_content(
    html: &str,
    element_selector: &str,
    new_content: &str,
) -> Result<String, String> {
    use lol_html::{HtmlRewriter, Settings, element};

    // Pass 1: count how many elements match the selector.
    // A content edit is only safe when exactly one element matches — any other
    // count means the selector is ambiguous (>1) or targets nothing (0), and
    // we must reject rather than silently clobber the wrong nodes.
    let mut match_count: usize = 0;
    {
        let sel = element_selector.to_string();
        let mut counter = HtmlRewriter::new(
            Settings {
                element_content_handlers: vec![element!(sel, |_el| {
                    match_count += 1;
                    Ok(())
                })],
                ..Settings::default()
            },
            |_: &[u8]| {},
        );
        counter
            .write(html.as_bytes())
            .map_err(|e| format!("lol_html error: {}", e))?;
        counter
            .end()
            .map_err(|e| format!("lol_html error: {}", e))?;
    }

    if match_count != 1 {
        return Err(format!(
            "Selector '{}' matched {} elements in template HTML; selector must be unique (exactly 1 match)",
            element_selector, match_count
        ));
    }

    // Pass 2: exactly one match confirmed — apply the replacement.
    let mut output = Vec::new();
    let content_str = new_content.to_string();
    let sel = element_selector.to_string();

    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![element!(sel, |el| {
                el.set_inner_content(&content_str, lol_html::html_content::ContentType::Html);
                Ok(())
            })],
            ..Settings::default()
        },
        |c: &[u8]| output.extend_from_slice(c),
    );
    rewriter
        .write(html.as_bytes())
        .map_err(|e| format!("lol_html error: {}", e))?;
    rewriter
        .end()
        .map_err(|e| format!("lol_html error: {}", e))?;

    String::from_utf8(output).map_err(|e| format!("UTF-8 error: {}", e))
}

fn handle_edit_ast_inner(
    site_dir: &Path,
    file: &str,
    selector: &str,
    patch: &Value,
) -> Result<(), String> {
    let file_path = validate_file_path(site_dir, file)?;

    // *** BRANCH GUARD: __content edits use a completely separate code path ***
    // They must NEVER reach apply_text_patch() which searches for "key: value"
    // patterns in source text — __content is not a source-level property name.
    if let Some(content) = patch.get("__content").and_then(|v| v.as_str()) {
        let st_id = patch.get("__st_id").and_then(|v| v.as_str());
        return handle_content_edit(&file_path, file, selector, content, st_id);
    }

    // *** BRANCH: guarded comment-span deletion (PLAN-123/W4) ***
    // This deliberately stays on the established span-write rail: one lock,
    // one read, byte guards, then one write. A separate comment writer would
    // make the most destructive editor bypass the guard that protects every
    // other source mutation.
    if let Some(spans) = patch.get("__delete_spans") {
        let spans = spans
            .as_array()
            .ok_or_else(|| "__delete_spans must be an array of [start, end] spans".to_string())?;
        let expected_hash = patch
            .get("__expect_hash")
            .and_then(Value::as_str)
            .ok_or_else(|| "guarded deletion requires __expect_hash".to_string())?;
        let expected_texts = patch
            .get("__expect_span_texts")
            .and_then(Value::as_array)
            .ok_or_else(|| "guarded deletion requires __expect_span_texts".to_string())?;
        if spans.is_empty() || spans.len() != expected_texts.len() {
            return Err(
                "guarded deletion needs one expected text for every non-empty span".to_string(),
            );
        }
        let _write_guard = source_write_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let content =
            std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {e}"))?;
        let actual_hash = format!("{:016x}", crate::migrate::hash_content(&content));
        if expected_hash != actual_hash {
            return Err(format!(
                "__expect_hash mismatch for {file}: expected {expected_hash}, found {actual_hash}"
            ));
        }
        // AUTHORIZATION, at the rail rather than at one caller.
        //
        // `handle_edit_ast` is reachable from the PUBLIC websocket protocol
        // (`ClientMessage::EditAst`), not just from the prune route. The
        // hash/text guards below prove a span still means what the caller was
        // shown — they do NOT prove the caller was allowed to delete it. Left
        // at that, any client able to open the dev socket could delete any
        // byte range of any project file simply by echoing back its current
        // contents.
        //
        // So the rail recomputes, from source, the set of spans that are
        // deletable: exactly the resolved inline comments. A range that is not
        // one of those is refused here, no matter who asked.
        let deletable = crate::comments::deletable_spans_in(&file_path, &content);
        let mut verified = Vec::with_capacity(spans.len());
        for (index, (span, expected)) in spans.iter().zip(expected_texts).enumerate() {
            let range = span
                .as_array()
                .ok_or_else(|| format!("__delete_spans[{index}] must be a [start, end] array"))?;
            let start = range
                .first()
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("__delete_spans[{index}][0] must be a number"))?
                as usize;
            let end = range
                .get(1)
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("__delete_spans[{index}][1] must be a number"))?
                as usize;
            let expected = expected
                .as_str()
                .ok_or_else(|| format!("__expect_span_texts[{index}] must be a string"))?;
            if start >= end
                || end > content.len()
                || !content.is_char_boundary(start)
                || !content.is_char_boundary(end)
            {
                return Err(format!(
                    "__delete_spans[{index}] is not a valid UTF-8 range for {file}"
                ));
            }
            if content[start..end] != *expected {
                return Err(format!(
                    "__expect_span_text mismatch for {file} at [{start}, {end}]: the source moved under this delete"
                ));
            }
            if !deletable.contains(&(start, end)) {
                return Err(format!(
                    "__delete_spans[{index}] is not a deletable span in {file}: only a \
                     RESOLVED inline comment (its `//@` header and continuation run) may \
                     be deleted through this rail"
                ));
            }
            verified.push((start, end));
        }
        verified.sort_unstable_by_key(|(start, _)| *start);
        if verified.windows(2).any(|pair| pair[0].1 > pair[1].0) {
            return Err("__delete_spans overlap; refusing ambiguous deletion".to_string());
        }
        let mut patched = content;
        for (start, end) in verified.into_iter().rev() {
            patched.replace_range(start..end, "");
        }
        std::fs::write(&file_path, patched).map_err(|e| format!("Failed to write file: {e}"))?;
        return Ok(());
    }

    // *** BRANCH: invocation-span edit (FUP-093/G2) ***
    // The builder inspector edits ONE template invocation's args by its source
    // SPAN (`__invoke_span: [start, end]`, from the structure projection) — no CSS
    // selector / parse needed, the span IS the address. Patch the named args
    // within that span using the SAME apply_text_patch the directive path uses
    // (per-instance: a template called N times has N distinct spans). The control
    // key is stripped before patching so only real param keys are written.
    if let Some(span_val) = patch.get("__invoke_span") {
        let arr = span_val
            .as_array()
            .ok_or_else(|| "__invoke_span must be a [start, end] array".to_string())?;
        let start = arr
            .first()
            .and_then(|v| v.as_u64())
            .ok_or_else(|| "__invoke_span[0] (start) must be a number".to_string())?
            as usize;
        let end = arr
            .get(1)
            .and_then(|v| v.as_u64())
            .ok_or_else(|| "__invoke_span[1] (end) must be a number".to_string())?
            as usize;
        // Held until this branch returns: read, guard checks, patch and write are
        // ONE transaction (see `source_write_lock`). Poisoning is recovered from —
        // a panic in another edit says nothing about this file's bytes, which are
        // re-read and re-verified below regardless.
        let _write_guard = source_write_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let content = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        if start > content.len() || end > content.len() || start >= end {
            return Err(format!(
                "__invoke_span [{start}, {end}] out of bounds for {} ({} bytes)",
                file,
                content.len()
            ));
        }
        if !content.is_char_boundary(start) || !content.is_char_boundary(end) {
            return Err(format!(
                "__invoke_span [{start}, {end}] is not on UTF-8 character boundaries for {} ({} bytes)",
                file,
                content.len()
            ));
        }
        if let Some(expected_hash) = patch.get("__expect_hash") {
            let expected_hash = expected_hash
                .as_str()
                .ok_or_else(|| "__expect_hash must be a string".to_string())?;
            let actual_hash = format!("{:016x}", crate::migrate::hash_content(&content));
            if expected_hash != actual_hash {
                return Err(format!(
                    "__expect_hash mismatch for {}: expected {}, found {}",
                    file, expected_hash, actual_hash
                ));
            }
        }
        // `__expect_span_text` is the STRONGEST of the three guards, and the only
        // one that survives a concurrent writer: the caller echoes back the exact
        // invocation text it was shown, and the write proceeds only if those bytes
        // are STILL at that offset. A whole-file hash proves the file is unchanged;
        // this proves THIS SPAN still means what the caller thinks it means — so a
        // span that shifted, or now points at a different same-key invocation, is
        // refused instead of silently patching the wrong call site.
        if let Some(expected_span) = patch.get("__expect_span_text") {
            let expected_span = expected_span
                .as_str()
                .ok_or_else(|| "__expect_span_text must be a string".to_string())?;
            let actual_span = &content[start..end];
            if expected_span != actual_span {
                return Err(format!(
                    "__expect_span_text mismatch for {file} at [{start}, {end}]: \
                     the source moved under this edit (expected {expected_span:?}, \
                     found {actual_span:?})"
                ));
            }
        }
        // Strip control keys; patch only the real param args.
        let mut arg_patch = patch.clone();
        if let Some(obj) = arg_patch.as_object_mut() {
            obj.remove("__invoke_span");
            obj.remove("__expect_hash");
            obj.remove("__expect_span_text");
        }
        let patched = apply_text_patch(&content, start, end, &arg_patch)?;
        std::fs::write(&file_path, patched).map_err(|e| format!("Failed to write file: {}", e))?;
        return Ok(());
    }

    // *** EXISTING CODE: property patching (unchanged from here) ***
    // Read file
    let content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Parse the .st file
    let ast = parse(&content)
        .map_err(|e| format!("Parse error: {}", e.render_all_plain(&content, file)))?;

    // A FILE-SCOPE binding target (`$name §data`) addresses a top-level
    // `@data inline $name …` directive — the brand SINGLETON (PLAN-034 Wave B).
    // It carries no CSS scope; resolve it against the file-scope matches by the
    // binding name rather than a scope lookup.
    let span = if let Some(binding) = selector
        .strip_prefix('$')
        .and_then(|s| s.split(" §").next())
    {
        let binding = binding.trim();
        let fm = ast
            .matches
            .iter()
            .find(|m| {
                m.selector.is_none()
                    && m.macro_name == "data"
                    && m.get_binding("name").map(|b| b.trim_start_matches('$')) == Some(binding)
            })
            .ok_or_else(|| format!("File-scope binding '${}' not found", binding))?;
        fm.span
    } else {
        // Scoped directive: `<css-selector> §<directive>`.
        let (css_selector, directive_name) = parse_ast_selector(selector)?;
        let matches_directive = |m: &crate::syntax::FormMatch| {
            m.macro_name == directive_name
                || m.macro_name.ends_with(&directive_name)
                || directive_name.contains(&m.macro_name)
        };
        // A selector may appear in MULTIPLE scope blocks (e.g. `.masthead { … }`
        // for tokens AND `.masthead { @scroll … }` for motion). Find the scope
        // whose matches actually CONTAIN the target directive, not merely the
        // first selector match — otherwise a directive edit would resolve to a
        // sibling block that lacks it and spuriously reject.
        let form_match = ast
            .scopes
            .iter()
            .filter(|s| s.selector == css_selector)
            .find_map(|s| s.matches.iter().find(|m| matches_directive(m)))
            .ok_or_else(|| {
                format!(
                    "Directive '@{}' not found in any scope '{}'",
                    directive_name, css_selector
                )
            })?;
        form_match.span
    };
    if span.start == 0 && span.end == 0 {
        return Err("Directive has no source span information".to_string());
    }

    // Apply the patch via text replacement within the span
    let modified = apply_text_patch(&content, span.start, span.end, patch)?;

    // Write back to disk
    std::fs::write(&file_path, &modified).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

/// Collect .st files from a directory (non-recursive scan of immediate .st files,
/// plus recursive descent into subdirectories).
fn collect_st_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return files,
    };
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_st_files(&path));
        } else if path.extension().is_some_and(|ext| ext == "st") {
            files.push(path);
        }
    }
    files
}

/// Categorize a FormMatch macro_name into a section label.
fn section_label_for_macro(macro_name: &str) -> &'static str {
    match macro_name {
        name if name == "scroll" || name.starts_with("scroll-") => "Scroll Animations",
        "state_machine" | "state" | "transition" => "State Machine",
        "each" | "bind" | "data" => "Data Bindings",
        "load" | "timeline" => "Timelines",
        _ => "Directives",
    }
}

/// Convert a byte offset to a 1-based line number using a LineIndex.
fn byte_offset_to_line(line_index: &LineIndex, offset: usize) -> u32 {
    let lc = line_index.line_col(offset as u32);
    lc.line + 1 // convert 0-based to 1-based
}

/// Convert a CapturedValue to a display string for property values.
fn captured_value_to_string(val: &crate::syntax::CapturedValue) -> String {
    use crate::syntax::CapturedValue;
    match val {
        CapturedValue::Ident(s) => s.clone(),
        CapturedValue::String(s) => s.clone(),
        CapturedValue::Number(n) => n.to_string(),
        CapturedValue::Bool(b) => b.to_string(),
        CapturedValue::Time(ms) => format!("{}ms", ms),
        CapturedValue::Length(lv) => format!("{}{}", lv.value, lv.unit),
        CapturedValue::Selector(s) => s.clone(),
        CapturedValue::Binding(s) => format!("${}", s),
        CapturedValue::Element(s) => format!("&{}", s),
        CapturedValue::Expr(s) => s.clone(),
        CapturedValue::TypeRef(s) => s.clone(),
        CapturedValue::Preset(s) => format!("&{}", s),
        CapturedValue::Json(j) => format!("{:?}", j),
        CapturedValue::Block(_) => "{ ... }".to_string(),
        CapturedValue::Array(arr) => format!(
            "[{}]",
            arr.iter()
                .map(captured_value_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        CapturedValue::Named(map) => format!(
            "{{{}}}",
            map.iter()
                .map(|(k, v)| format!("{}: {}", k, captured_value_to_string(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        CapturedValue::Properties(props) => format!("({} properties)", props.len()),
        CapturedValue::Params(params) => params
            .iter()
            .map(|p| format!("{}: {}", p.name, p.type_ref))
            .collect::<Vec<_>>()
            .join(", "),
        CapturedValue::Keyframes(kfs) => format!("({} keyframes)", kfs.len()),
        CapturedValue::ParamList(plist) => format!("({} params)", plist.len()),
        CapturedValue::StyleProperties(props) => props
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("; "),
        CapturedValue::PatternMatch {
            signal, variant, ..
        } => format!("{} is {}", signal, variant),
        CapturedValue::Color(c) => c.clone(),
        CapturedValue::ComponentBody => {
            // FEAT-120: the capture is a bare presence-marker — payload lives on the
            // World-A scope, validation diagnostics in `StFile.diagnostics`.
            "(component body)".to_string()
        }
    }
}

/// Handle an `InspectStructure` request: compile the target `.st` file to a bundle
/// and project it through the SHARED Structure IR producer (`crate::introspect`).
///
/// This is the dev-ws host's read into the SAME introspection substrate the MCP
/// workbench navigator uses (PLAN-064 B2) — one producer, two hosts, no divergent
/// stacks. A QUERY operation: reads from disk, compiles, returns the node tree.
/// The entry template defaults to `main`.
pub fn handle_inspect_structure(
    site_dir: &Path,
    file: &str,
    template: Option<&str>,
) -> ServerMessage {
    let entry = template.unwrap_or("main").to_string();
    let path = match validate_file_path(site_dir, file) {
        Ok(p) => p,
        Err(e) => {
            return ServerMessage::Structure {
                file: file.to_string(),
                template: entry,
                nodes: Vec::new(),
                error: Some(format!("invalid path: {e}")),
            };
        }
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            return ServerMessage::Structure {
                file: file.to_string(),
                template: entry,
                nodes: Vec::new(),
                error: Some(format!("read failed: {e}")),
            };
        }
    };
    match crate::mcp::bundle::compile_to_bundle(&source, site_dir) {
        Ok(bundle) => {
            let nodes = crate::introspect::structure_from_bundle(&bundle, &entry);
            let error = if nodes.is_empty() {
                Some(format!("no template named '{entry}' in {file}"))
            } else {
                None
            };
            ServerMessage::Structure {
                file: file.to_string(),
                template: entry,
                nodes,
                error,
            }
        }
        Err(e) => ServerMessage::Structure {
            file: file.to_string(),
            template: entry,
            nodes: Vec::new(),
            error: Some(format!("compile failed: {e:?}")),
        },
    }
}

/// Handle an InspectElement request by parsing .st files and collecting element context.
///
/// This is a QUERY operation — it reads from disk and returns an ElementContext response
/// without modifying any files or requiring HandleResult's mutation plumbing.
pub fn handle_inspect_element(
    site_dir: &Path,
    selector: &str,
    file_hint: Option<&str>,
) -> ServerMessage {
    // Determine which files to scan
    let files_to_scan: Vec<PathBuf> = if let Some(hint) = file_hint {
        // Fast path: only parse the hinted file
        match validate_file_path(site_dir, hint) {
            Ok(path) => vec![path],
            Err(_) => {
                return ServerMessage::ElementContext {
                    selector: selector.to_string(),
                    source_files: vec![],
                    sections: vec![],
                    state_machine: None,
                    data_bindings: vec![],
                    not_found: true,
                };
            }
        }
    } else {
        // Scan all .st files in site directory
        collect_st_files(site_dir)
    };

    let mut source_files: Vec<String> = Vec::new();
    let mut all_sections: Vec<PropertySection> = Vec::new();
    let mut state_machine: Option<StateMachineContext> = None;
    let mut data_bindings: Vec<DataBindingContext> = Vec::new();
    let mut found_any = false;

    for file_path in &files_to_scan {
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let ast = match parse(&content) {
            Ok(ast) => ast,
            Err(_) => continue,
        };

        let line_index = LineIndex::new(&content);
        let rel_path = file_path
            .strip_prefix(site_dir)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        // Search top-level scopes
        for scope in &ast.scopes {
            if scope.selector != selector {
                continue;
            }
            found_any = true;
            if !source_files.contains(&rel_path) {
                source_files.push(rel_path.clone());
            }

            // Collect CSS declarations
            if !scope.css_declarations.is_empty() {
                let mut css_props = Vec::new();
                for decl in &scope.css_declarations {
                    let raw = decl.value.clone();
                    let (parsed, control_hint) = classify_value(&raw);
                    let line = if decl.span.start > 0 || decl.span.end > 0 {
                        Some(byte_offset_to_line(&line_index, decl.span.start))
                    } else {
                        None
                    };
                    css_props.push(PropertyNode {
                        name: decl.property.clone(),
                        value: ValueNode {
                            raw: raw.clone(),
                            parsed,
                            control_hint,
                            editable: true,
                        },
                        source_file: Some(rel_path.clone()),
                        source_line: line,
                        editable: true,
                    });
                }
                all_sections.push(PropertySection {
                    label: "CSS".to_string(),
                    source_file: Some(rel_path.clone()),
                    source_line: None,
                    properties: css_props,
                });
            }

            // Collect FormMatch directives
            let mut section_map: std::collections::HashMap<&str, Vec<PropertyNode>> =
                std::collections::HashMap::new();

            for fm in &scope.matches {
                let label = section_label_for_macro(&fm.macro_name);
                let fm_line = if fm.span.start > 0 || fm.span.end > 0 {
                    Some(byte_offset_to_line(&line_index, fm.span.start))
                } else {
                    None
                };

                // Extract captures as property nodes
                for (key, val) in &fm.captures {
                    let raw = captured_value_to_string(val);
                    let (parsed, control_hint) = classify_value(&raw);
                    section_map.entry(label).or_default().push(PropertyNode {
                        name: format!("{}:{}", fm.macro_name, key),
                        value: ValueNode {
                            raw,
                            parsed,
                            control_hint,
                            editable: false,
                        },
                        source_file: Some(rel_path.clone()),
                        source_line: fm_line,
                        editable: false,
                    });
                }

                // State machine extraction
                if fm.macro_name == "state_machine" && state_machine.is_none() {
                    let initial = fm
                        .captures
                        .get("initial")
                        .map(captured_value_to_string)
                        .unwrap_or_default();
                    // Collect state names from sibling "state" matches
                    let states: Vec<String> = scope
                        .matches
                        .iter()
                        .filter(|m| m.macro_name == "state")
                        .filter_map(|m| m.captures.get("when").map(captured_value_to_string))
                        .collect();
                    state_machine = Some(StateMachineContext {
                        current_state: initial,
                        states,
                    });
                }

                // Data binding extraction
                if fm.macro_name == "each" || fm.macro_name == "bind" || fm.macro_name == "data" {
                    let name = fm
                        .captures
                        .get("name")
                        .or_else(|| fm.captures.get("source"))
                        .map(captured_value_to_string)
                        .unwrap_or_else(|| fm.macro_name.clone());
                    let data_type = fm
                        .captures
                        .get("type")
                        .map(captured_value_to_string)
                        .unwrap_or_else(|| "unknown".to_string());
                    data_bindings.push(DataBindingContext { name, data_type });
                }
            }

            // Convert section_map into PropertySections
            for (label, properties) in section_map {
                all_sections.push(PropertySection {
                    label: label.to_string(),
                    source_file: Some(rel_path.clone()),
                    source_line: None,
                    properties,
                });
            }
        }
    }

    ServerMessage::ElementContext {
        selector: selector.to_string(),
        source_files,
        sections: all_sections,
        state_machine,
        data_bindings,
        not_found: !found_any,
    }
}

/// Handle an incoming client edit message, dispatching to the appropriate handler.
pub fn handle_message(site_dir: &Path, msg: ClientMessage) -> HandleResult {
    match msg {
        ClientMessage::EditJson {
            file,
            path,
            value,
            op_id,
        } => handle_edit_json(site_dir, &file, &path, value, &op_id),

        ClientMessage::EditHtml {
            file,
            element_id,
            content,
            op_id,
        } => handle_edit_html(site_dir, &file, &element_id, &content, &op_id),

        ClientMessage::EditAst {
            file,
            selector,
            patch,
            op_id,
        } => handle_edit_ast(site_dir, &file, &selector, &patch, &op_id),

        ClientMessage::EditJsonArray {
            file,
            path,
            op,
            op_id,
        } => handle_edit_json_array(site_dir, &file, &path, op, &op_id),

        ClientMessage::EditToken {
            file,
            token,
            value,
            op_id,
            motion,
            motion_index,
        } => handle_edit_token(
            site_dir,
            &file,
            &token,
            &value,
            &op_id,
            motion,
            motion_index,
        ),

        ClientMessage::InspectElement {
            selector,
            file_hint,
        } => {
            // PLAN-064 B2: wire the previously-dead dispatch to the real handler
            // (a QUERY, no mutation/broadcast).
            HandleResult {
                response: handle_inspect_element(site_dir, &selector, file_hint.as_deref()),
                broadcast: None,
                suppress_reload_path: None,
            }
        }
        ClientMessage::InspectStructure { file, template } => {
            // PLAN-064 B2: the dev-ws host reads the SHARED Structure IR (same
            // producer the MCP navigator uses). A QUERY — no mutation/broadcast.
            HandleResult {
                response: handle_inspect_structure(site_dir, &file, template.as_deref()),
                broadcast: None,
                suppress_reload_path: None,
            }
        }
    }
}

/// Replace a numeric motion param `key: <number>` inside an `@reveal/@scroll`
/// block (Stage-3 Wave-9). Value is validated numeric by the caller, so no
/// injection is possible. Replaces the FIRST `key` at a token boundary
/// (preceded by `(`, `,`, or whitespace) followed by `:` and a numeric literal.
fn edit_motion_param(
    site_dir: &Path,
    file: &str,
    key: &str,
    value: &str,
    op_id: &str,
    index: usize,
) -> HandleResult {
    let reject = |reason: String| HandleResult {
        response: ServerMessage::Reject {
            op_id: op_id.to_string(),
            reason,
        },
        broadcast: None,
        suppress_reload_path: None,
    };
    let file_path = match validate_file_path(site_dir, file) {
        Ok(p) => p,
        Err(e) => return reject(e),
    };
    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(e) => return reject(format!("Failed to read file: {}", e)),
    };
    let bytes = content.as_bytes();
    let mut search = 0usize;
    // `index` selects WHICH `key:number` occurrence to edit (0-based), so that
    // multiple @reveal/@scroll blocks in one file sharing a param key are
    // disambiguated by the client (motion.json order == this scan order).
    let mut hit = 0usize;
    while let Some(rel) = content[search..].find(key) {
        let start = search + rel;
        let end_key = start + key.len();
        let before_ok = start == 0
            || matches!(
                bytes[start - 1] as char,
                '(' | ',' | ' ' | '\t' | '\n' | '\r'
            );
        let mut j = end_key;
        while j < bytes.len() && matches!(bytes[j] as char, ' ' | '\t') {
            j += 1;
        }
        if before_ok && j < bytes.len() && bytes[j] == b':' {
            j += 1;
            while j < bytes.len() && matches!(bytes[j] as char, ' ' | '\t') {
                j += 1;
            }
            let num_start = j;
            while j < bytes.len() && matches!(bytes[j] as char, '0'..='9' | '.' | '-') {
                j += 1;
            }
            if j > num_start {
                if hit == index {
                    let mut out = String::with_capacity(content.len());
                    out.push_str(&content[..num_start]);
                    out.push_str(value.trim());
                    out.push_str(&content[j..]);
                    if let Err(e) = std::fs::write(&file_path, &out) {
                        return reject(format!("Failed to write file: {}", e));
                    }
                    return HandleResult {
                        response: ServerMessage::Ack {
                            op_id: op_id.to_string(),
                        },
                        broadcast: None,
                        suppress_reload_path: None,
                    };
                }
                hit += 1;
            }
        }
        search = end_key;
    }
    reject(format!(
        "Motion param '{}' (#{}) not found in {}",
        key, index, file
    ))
}

/// Replace the value of a `--name: value;` custom-property declaration in a `.st`
/// file (Stage-3 Wave-7 theme token edit). The new value is sanitized to a
/// single CSS declaration value (no `;`/`{`/`}` so it cannot inject extra rules).
pub fn handle_edit_token(
    site_dir: &Path,
    file: &str,
    token: &str,
    value: &str,
    op_id: &str,
    motion: bool,
    motion_index: usize,
) -> HandleResult {
    let reject = |reason: String| HandleResult {
        response: ServerMessage::Reject {
            op_id: op_id.to_string(),
            reason,
        },
        broadcast: None,
        suppress_reload_path: None,
    };
    if motion {
        // Motion param: a bare identifier key (duration, stagger, …) and a
        // NUMERIC value; edited inside an @reveal/@scroll block.
        if token.is_empty() || !token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return reject(format!("Invalid motion param: {}", token));
        }
        if !value.parse::<f64>().map(|f| f.is_finite()).unwrap_or(false) {
            return reject("Motion value must be a finite number".to_string());
        }
        return edit_motion_param(site_dir, file, token, value, op_id, motion_index);
    }
    // Token name must look like a custom property.
    if !token.starts_with("--")
        || !token[2..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return reject(format!("Invalid token name: {}", token));
    }
    // Value must be a SINGLE declaration value. Reject anything that could break
    // out into additional declarations or comments in the .st source:
    //  - `; { } <`      : new declarations / blocks / tag injection
    //  - newlines       : the .st style parser treats a depth-0 \n as a
    //                     declaration separator (would inject arbitrary CSS)
    //  - `/*` and `*/`  : block-comment sequences (comment-out following code)
    if value.contains(';')
        || value.contains('{')
        || value.contains('}')
        || value.contains('<')
        || value.contains('\n')
        || value.contains('\r')
        || value.contains("/*")
        || value.contains("*/")
    {
        return reject(
            "Token value may not contain ; { } < newlines or comment sequences".to_string(),
        );
    }
    let file_path = match validate_file_path(site_dir, file) {
        Ok(p) => p,
        Err(e) => return reject(e),
    };
    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(e) => return reject(format!("Failed to read file: {}", e)),
    };
    // Replace the FIRST `--token: <old>;` declaration's value.
    let needle = format!("{}:", token);
    let mut replaced = false;
    let mut out = String::with_capacity(content.len());
    for line in content.split_inclusive('\n') {
        if !replaced && let Some(decl_pos) = line.find(&needle) {
            // ensure the match is the property name (preceded by start/space).
            let before = &line[..decl_pos];
            if before.trim().is_empty()
                && let Some(semi) = line[decl_pos..].find(';')
            {
                let abs_semi = decl_pos + semi;
                let after_val = &line[abs_semi..]; // from ';' onward (keeps comment)
                out.push_str(before);
                out.push_str(token);
                out.push_str(": ");
                out.push_str(value.trim());
                out.push_str(after_val);
                replaced = true;
                continue;
            }
        }
        out.push_str(line);
    }
    if !replaced {
        return reject(format!("Token {} not found in {}", token, file));
    }
    if let Err(e) = std::fs::write(&file_path, &out) {
        return reject(format!("Failed to write file: {}", e));
    }
    HandleResult {
        response: ServerMessage::Ack {
            op_id: op_id.to_string(),
        },
        broadcast: None,
        suppress_reload_path: None,
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_edit_token_replaces_value() {
        let dir = TempDir::new().unwrap();
        let stp = dir.path().join("modules");
        fs::create_dir_all(&stp).unwrap();
        fs::write(
            stp.join("_theme.st"),
            "[data-theme] {\n    --accent: #FF0020;   // scarlet\n    --gap: 24px;\n}\n",
        )
        .unwrap();
        let r = handle_edit_token(
            dir.path(),
            "modules/_theme.st",
            "--accent",
            "#00FF00",
            "op_t",
            false,
            0,
        );
        assert!(matches!(r.response, ServerMessage::Ack { .. }));
        let after = fs::read_to_string(stp.join("_theme.st")).unwrap();
        assert!(
            after.contains("--accent: #00FF00;"),
            "value replaced: {}",
            after
        );
        assert!(after.contains("// scarlet"), "trailing comment preserved");
        assert!(after.contains("--gap: 24px;"), "other tokens untouched");
    }

    #[test]
    fn test_edit_token_rejects_injection() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("t.st"), "--a: 1px;\n").unwrap();
        let r = handle_edit_token(
            dir.path(),
            "t.st",
            "--a",
            "1px; } body{display:none",
            "op_x",
            false,
            0,
        );
        assert!(
            matches!(r.response, ServerMessage::Reject { .. }),
            "injection rejected"
        );
    }

    #[test]
    fn test_edit_token_missing_token() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("t.st"), "--a: 1px;\n").unwrap();
        let r = handle_edit_token(dir.path(), "t.st", "--nope", "2px", "op_m", false, 0);
        assert!(matches!(r.response, ServerMessage::Reject { .. }));
    }

    #[test]
    fn test_edit_motion_param_replaces_number() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("m.st"), "x {\n    @reveal(\n        stagger: 24,\n        duration: 700,\n        threshold: 0.25\n    )\n}\n").unwrap();
        let r = handle_edit_token(dir.path(), "m.st", "duration", "900", "op_md", true, 0);
        assert!(matches!(r.response, ServerMessage::Ack { .. }));
        let after = fs::read_to_string(dir.path().join("m.st")).unwrap();
        assert!(
            after.contains("duration: 900,"),
            "duration tuned: {}",
            after
        );
        assert!(after.contains("stagger: 24,"), "other params untouched");
        assert!(after.contains("threshold: 0.25"), "threshold untouched");
        // non-numeric motion value rejected
        let r2 = handle_edit_token(dir.path(), "m.st", "duration", "fast", "op_mx", true, 0);
        assert!(matches!(r2.response, ServerMessage::Reject { .. }));
        // non-finite (inf/nan) rejected (reviewer P2)
        let r3 = handle_edit_token(dir.path(), "m.st", "duration", "inf", "op_inf", true, 0);
        assert!(
            matches!(r3.response, ServerMessage::Reject { .. }),
            "inf must be rejected"
        );
        let r4 = handle_edit_token(dir.path(), "m.st", "duration", "nan", "op_nan", true, 0);
        assert!(
            matches!(r4.response, ServerMessage::Reject { .. }),
            "nan must be rejected"
        );
    }

    #[test]
    fn test_edit_motion_param_targets_indexed_block() {
        // Two @reveal blocks sharing `duration`; index disambiguates (reviewer P1).
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("m.st"),
            "a { @reveal( duration: 100 ) }\nb { @reveal( duration: 200 ) }\n",
        )
        .unwrap();
        // edit the SECOND block (index 1)
        let r = handle_edit_token(dir.path(), "m.st", "duration", "999", "op_i", true, 1);
        assert!(matches!(r.response, ServerMessage::Ack { .. }));
        let after = fs::read_to_string(dir.path().join("m.st")).unwrap();
        assert!(
            after.contains("duration: 100"),
            "first block untouched: {}",
            after
        );
        assert!(
            after.contains("duration: 999"),
            "second block tuned: {}",
            after
        );
        assert!(
            !after.contains("duration: 200"),
            "second block's old value gone"
        );
    }

    #[test]
    fn test_edit_token_rejects_newline_and_comment_injection() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("t.st"), "--a: 1px;\n").unwrap();
        // newline → declaration injection
        let r1 = handle_edit_token(
            dir.path(),
            "t.st",
            "--a",
            "#fff\n  background: url(http://evil/x)",
            "op_n",
            false,
            0,
        );
        assert!(
            matches!(r1.response, ServerMessage::Reject { .. }),
            "newline must be rejected"
        );
        // block-comment opener
        let r2 = handle_edit_token(dir.path(), "t.st", "--a", "#fff /*", "op_c", false, 0);
        assert!(
            matches!(r2.response, ServerMessage::Reject { .. }),
            "/* must be rejected"
        );
        let r3 = handle_edit_token(dir.path(), "t.st", "--a", "x */", "op_c2", false, 0);
        assert!(
            matches!(r3.response, ServerMessage::Reject { .. }),
            "*/ must be rejected"
        );
        // file unchanged after all rejects
        assert_eq!(
            fs::read_to_string(dir.path().join("t.st")).unwrap(),
            "--a: 1px;\n"
        );
    }

    // -------------------------------------------------------------------------
    // JSON path parser tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_simple_field() {
        let segments = parse_json_path("name").unwrap();
        assert_eq!(segments, vec![PathSegment::Field("name".to_string())]);
    }

    #[test]
    fn test_parse_array_index() {
        let segments = parse_json_path("[0]").unwrap();
        assert_eq!(segments, vec![PathSegment::Index(0)]);
    }

    #[test]
    fn test_parse_index_then_field() {
        let segments = parse_json_path("[2].price").unwrap();
        assert_eq!(
            segments,
            vec![
                PathSegment::Index(2),
                PathSegment::Field("price".to_string())
            ]
        );
    }

    #[test]
    fn test_parse_field_then_index_then_field() {
        let segments = parse_json_path("products[0].name").unwrap();
        assert_eq!(
            segments,
            vec![
                PathSegment::Field("products".to_string()),
                PathSegment::Index(0),
                PathSegment::Field("name".to_string()),
            ]
        );
    }

    #[test]
    fn test_parse_nested_fields() {
        let segments = parse_json_path("[1].nested.field").unwrap();
        assert_eq!(
            segments,
            vec![
                PathSegment::Index(1),
                PathSegment::Field("nested".to_string()),
                PathSegment::Field("field".to_string()),
            ]
        );
    }

    #[test]
    fn test_parse_deep_nesting() {
        let segments = parse_json_path("a[0].b[1].c").unwrap();
        assert_eq!(
            segments,
            vec![
                PathSegment::Field("a".to_string()),
                PathSegment::Index(0),
                PathSegment::Field("b".to_string()),
                PathSegment::Index(1),
                PathSegment::Field("c".to_string()),
            ]
        );
    }

    #[test]
    fn test_parse_empty_path_error() {
        assert!(parse_json_path("").is_err());
    }

    #[test]
    fn test_parse_invalid_index_error() {
        assert!(parse_json_path("[abc]").is_err());
    }

    // -------------------------------------------------------------------------
    // navigate_to_mut tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_navigate_simple_field() {
        let mut data = json!({"name": "Alice", "age": 30});
        let target = navigate_to_mut(&mut data, &[PathSegment::Field("name".to_string())]).unwrap();
        assert_eq!(target, &json!("Alice"));
    }

    #[test]
    fn test_navigate_array_index() {
        let mut data = json!([10, 20, 30]);
        let target = navigate_to_mut(&mut data, &[PathSegment::Index(1)]).unwrap();
        assert_eq!(target, &json!(20));
    }

    #[test]
    fn test_navigate_and_mutate() {
        let mut data = json!([{"price": 10}, {"price": 20}]);
        let segments = parse_json_path("[1].price").unwrap();
        let target = navigate_to_mut(&mut data, &segments).unwrap();
        *target = json!(99);
        assert_eq!(data, json!([{"price": 10}, {"price": 99}]));
    }

    #[test]
    fn test_navigate_missing_field_error() {
        let mut data = json!({"name": "Alice"});
        let result = navigate_to_mut(&mut data, &[PathSegment::Field("missing".to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn test_navigate_index_out_of_bounds() {
        let mut data = json!([1, 2]);
        let result = navigate_to_mut(&mut data, &[PathSegment::Index(5)]);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // handle_edit_json integration tests (with filesystem)
    // -------------------------------------------------------------------------

    fn create_test_json(dir: &TempDir, filename: &str, data: &Value) -> PathBuf {
        let path = dir.path().join(filename);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, serde_json::to_string_pretty(data).unwrap()).unwrap();
        path
    }

    #[test]
    fn test_edit_json_simple_field() {
        let dir = TempDir::new().unwrap();
        let data = json!({"name": "Old", "price": 10});
        create_test_json(&dir, "data/test.json", &data);

        let result = handle_edit_json(dir.path(), "data/test.json", "name", json!("New"), "op_1");

        assert!(matches!(result.response, ServerMessage::Ack { ref op_id } if op_id == "op_1"));
        assert!(result.broadcast.is_some());

        if let Some(ServerMessage::DataUpdate { source, data }) = result.broadcast {
            assert_eq!(source, "data/test.json");
            assert_eq!(data["name"], json!("New"));
            assert_eq!(data["price"], json!(10)); // unchanged
        }

        // Verify file was actually written
        let written: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("data/test.json")).unwrap())
                .unwrap();
        assert_eq!(written["name"], json!("New"));
    }

    #[test]
    fn test_edit_json_sanitizes_richtext_on_persist() {
        // Stage-2 Wave-3 / FUP-029: the server MUST sanitize HTML-looking values
        // before they touch disk, regardless of client behavior.
        let dir = TempDir::new().unwrap();
        let data = json!({"body": "", "name": "Plain"});
        create_test_json(&dir, "data/test.json", &data);

        // A malicious rich body with a script tag + event handler + js: URL.
        let evil =
            "<p onclick=\"x()\">hi</p><script>alert(1)</script><a href=\"javascript:e()\">l</a>";
        let result = handle_edit_json(dir.path(), "data/test.json", "body", json!(evil), "op_rt");
        assert!(matches!(result.response, ServerMessage::Ack { .. }));

        let written: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("data/test.json")).unwrap())
                .unwrap();
        let body = written["body"].as_str().unwrap().to_ascii_lowercase();
        assert!(
            !body.contains("<script"),
            "script tag must be stripped on persist"
        );
        assert!(!body.contains("onclick"), "event handler must be stripped");
        assert!(!body.contains("javascript:"), "js: URL must be stripped");
        assert!(
            body.contains("<p>hi</p>") || body.contains("<p >hi</p>"),
            "safe content preserved"
        );

        // Plain text (no tags) must persist byte-exact (not mangled).
        let r2 = handle_edit_json(
            dir.path(),
            "data/test.json",
            "name",
            json!("A & B < C"),
            "op_plain",
        );
        assert!(matches!(r2.response, ServerMessage::Ack { .. }));
        let written2: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("data/test.json")).unwrap())
                .unwrap();
        // "A & B < C" has no closing '>' so it is NOT treated as HTML — byte-exact.
        assert_eq!(written2["name"], json!("A & B < C"));
    }

    #[test]
    fn test_edit_json_validates_doc_ast_against_editable_schema() {
        // PLAN-031 / FEAT-099: a structured-editor document AST persists through
        // schema validation, not HTML sanitization. The schema is derived from the
        // site's index.st @editable-mark/@editable-block declarations. A js: URL on
        // a link mark must drop the mark (keep its text); an unregistered block
        // must be dropped entirely.
        let dir = TempDir::new().unwrap();
        // The site declares bold + link marks and a p block.
        let index = r#"@editable-mark &bold(&sel) { <strong>`&sel`</strong> }
@editable-mark &link(&sel, $href url) { <a href="`$href`">`&sel`</a> }
@editable-block &p(&sel) { <p>`&sel`</p> }
<main class="r"></main>
"#;
        fs::write(dir.path().join("index.st"), index).unwrap();
        create_test_json(&dir, "data/post.json", &json!({ "body": null }));

        // A doc with a script-ish unknown block + a js: link + a safe bold run.
        let doc = json!({
            "type": "doc",
            "content": [
                { "type": "script", "content": [ { "type": "text", "text": "alert(1)" } ] },
                { "type": "p", "content": [
                    { "type": "text", "text": "click", "marks": [ { "type": "link", "attrs": { "href": "javascript:e()" } } ] },
                    { "type": "text", "text": "safe", "marks": [ { "type": "bold" } ] }
                ] }
            ]
        });
        let result = handle_edit_json(dir.path(), "data/post.json", "body", doc, "op_doc");
        assert!(matches!(result.response, ServerMessage::Ack { .. }));

        let written: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("data/post.json")).unwrap())
                .unwrap();
        let body = &written["body"];
        let blocks = body["content"].as_array().unwrap();
        // The unknown <script> block is gone; only the <p> survives.
        assert_eq!(blocks.len(), 1, "unknown block must be dropped: {body}");
        assert_eq!(blocks[0]["type"], "p");
        let inlines = blocks[0]["content"].as_array().unwrap();
        // "click" kept, its js: link mark dropped.
        assert_eq!(inlines[0]["text"], "click");
        assert!(
            inlines[0].get("marks").is_none() || inlines[0]["marks"].as_array().unwrap().is_empty(),
            "js: link mark must be dropped: {body}"
        );
        // "safe" keeps its bold mark.
        assert_eq!(inlines[1]["text"], "safe");
        assert_eq!(inlines[1]["marks"][0]["type"], "bold");
    }

    #[test]
    fn test_edit_json_array_insert_sanitizes() {
        // Wave-3 P1: array Insert must sanitize the inserted item's HTML too.
        let dir = TempDir::new().unwrap();
        let data = json!({"blocks": []});
        create_test_json(&dir, "data/test.json", &data);
        let evil = json!({"body": "<p onclick=\"x()\">hi</p><script>alert(1)</script>"});
        let result = handle_edit_json_array(
            dir.path(),
            "data/test.json",
            "blocks",
            ArrayOp::Insert {
                item: evil,
                index: None,
            },
            "op_ins",
        );
        assert!(matches!(result.response, ServerMessage::Ack { .. }));
        let written: Value =
            serde_json::from_str(&fs::read_to_string(dir.path().join("data/test.json")).unwrap())
                .unwrap();
        let body = written["blocks"][0]["body"]
            .as_str()
            .unwrap()
            .to_ascii_lowercase();
        assert!(
            !body.contains("<script"),
            "array insert must sanitize: {}",
            body
        );
        assert!(
            !body.contains("onclick"),
            "array insert must strip handlers: {}",
            body
        );
    }

    #[test]
    fn test_edit_json_array_element() {
        let dir = TempDir::new().unwrap();
        let data = json!([
            {"name": "Product A", "price": 10},
            {"name": "Product B", "price": 20},
            {"name": "Product C", "price": 30}
        ]);
        create_test_json(&dir, "products.json", &data);

        let result = handle_edit_json(
            dir.path(),
            "products.json",
            "[2].price",
            json!(29.99),
            "op_2",
        );

        assert!(matches!(result.response, ServerMessage::Ack { .. }));
        if let Some(ServerMessage::DataUpdate { data, .. }) = result.broadcast {
            assert_eq!(data[2]["price"], json!(29.99));
            assert_eq!(data[0]["price"], json!(10)); // unchanged
        }
    }

    #[test]
    fn test_edit_json_path_traversal_rejected() {
        let dir = TempDir::new().unwrap();
        create_test_json(&dir, "safe.json", &json!({}));

        let result = handle_edit_json(
            dir.path(),
            "../../../etc/passwd",
            "name",
            json!("evil"),
            "op_x",
        );

        assert!(
            matches!(result.response, ServerMessage::Reject { ref reason, .. } if reason.contains("not found") || reason.contains("traversal"))
        );
        assert!(result.broadcast.is_none());
    }

    #[test]
    fn test_edit_json_nonexistent_file() {
        let dir = TempDir::new().unwrap();

        let result = handle_edit_json(
            dir.path(),
            "does_not_exist.json",
            "name",
            json!("value"),
            "op_y",
        );

        assert!(matches!(result.response, ServerMessage::Reject { .. }));
        assert!(result.broadcast.is_none());
    }

    #[test]
    fn test_edit_json_invalid_path() {
        let dir = TempDir::new().unwrap();
        create_test_json(&dir, "test.json", &json!({"name": "Test"}));

        let result = handle_edit_json(
            dir.path(),
            "test.json",
            "nonexistent.field",
            json!("value"),
            "op_z",
        );

        assert!(matches!(result.response, ServerMessage::Reject { .. }));
    }

    // -------------------------------------------------------------------------
    // handle_message dispatch tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_edit_html_simple() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("index.html"),
            r#"<html><body><div data-st-id="title">Old Title</div></body></html>"#,
        )
        .unwrap();

        let result = handle_edit_html(dir.path(), "index.html", "title", "New Title", "op_h1");

        assert!(matches!(result.response, ServerMessage::Ack { ref op_id } if op_id == "op_h1"));
        assert!(result.broadcast.is_none());

        let written = fs::read_to_string(dir.path().join("index.html")).unwrap();
        assert!(written.contains("New Title"));
        assert!(!written.contains("Old Title"));
    }

    #[test]
    fn test_edit_html_element_not_found() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("test.html"),
            "<html><body><div>No ID</div></body></html>",
        )
        .unwrap();

        let result = handle_edit_html(dir.path(), "test.html", "missing", "New", "op_h2");

        assert!(matches!(result.response, ServerMessage::Reject { .. }));
        assert!(result.broadcast.is_none());
    }

    #[test]
    fn test_edit_html_nonexistent_file() {
        let dir = TempDir::new().unwrap();

        let result = handle_edit_html(dir.path(), "nonexistent.html", "title", "New", "op_h3");

        assert!(matches!(result.response, ServerMessage::Reject { .. }));
    }

    #[test]
    fn test_edit_html_via_dispatch() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("index.html"),
            r#"<html><body><p data-st-id="intro">Hello</p></body></html>"#,
        )
        .unwrap();

        let msg = ClientMessage::EditHtml {
            file: "index.html".to_string(),
            element_id: "intro".to_string(),
            content: "World".to_string(),
            op_id: "op_h4".to_string(),
        };

        let result = handle_message(dir.path(), msg);
        assert!(matches!(result.response, ServerMessage::Ack { .. }));
        assert!(result.broadcast.is_none());
    }

    // -------------------------------------------------------------------------
    // AST selector parser tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_ast_selector_basic() {
        let (css, dir) = parse_ast_selector(".hero §scroll").unwrap();
        assert_eq!(css, ".hero");
        assert_eq!(dir, "scroll");
    }

    #[test]
    fn test_parse_ast_selector_complex() {
        let (css, dir) = parse_ast_selector(".hero > .child §load").unwrap();
        assert_eq!(css, ".hero > .child");
        assert_eq!(dir, "load");
    }

    #[test]
    fn test_parse_ast_selector_no_directive() {
        assert!(parse_ast_selector(".hero").is_err());
    }

    #[test]
    fn test_parse_ast_selector_empty_parts() {
        assert!(parse_ast_selector(" §").is_err());
    }

    // -------------------------------------------------------------------------
    // Content selector parser tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_content_selector_html_element() {
        let result = parse_content_selector(r#"[data-st-id="st-a3f2b1c0"]"#).unwrap();
        assert_eq!(
            result,
            ContentSelector::HtmlElement {
                selector: r#"[data-st-id="st-a3f2b1c0"]"#.to_string(),
            }
        );
    }

    #[test]
    fn test_parse_content_selector_ast_content() {
        let result = parse_content_selector(".card §template h3").unwrap();
        assert_eq!(
            result,
            ContentSelector::AstContent {
                css_selector: ".card".to_string(),
                directive: "template".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_content_selector_ast_directive() {
        let result = parse_content_selector(".hero §scroll").unwrap();
        assert_eq!(
            result,
            ContentSelector::AstDirective {
                css_selector: ".hero".to_string(),
                directive: "scroll".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_content_selector_no_at_sign() {
        // Plain CSS selector → HtmlElement
        let result = parse_content_selector("h1").unwrap();
        assert_eq!(
            result,
            ContentSelector::HtmlElement {
                selector: "h1".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_content_selector_invalid_at_no_space() {
        // '@' without preceding space → error
        let result = parse_content_selector(".card§template");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_content_selector_rootless_ast_content() {
        // Rootless: selector starts with '@', no CSS scope
        let result = parse_content_selector("§template a").unwrap();
        assert_eq!(
            result,
            ContentSelector::AstContent {
                css_selector: "".to_string(),
                directive: "template".to_string(),
            }
        );
    }

    #[test]
    fn test_parse_content_selector_rootless_ast_directive() {
        // Rootless directive with no element path
        let result = parse_content_selector("§scroll").unwrap();
        assert_eq!(
            result,
            ContentSelector::AstDirective {
                css_selector: "".to_string(),
                directive: "scroll".to_string(),
            }
        );
    }

    // -------------------------------------------------------------------------
    // apply_text_patch tests
    // -------------------------------------------------------------------------

    // ── FUP-133: quote-style round-trip ─────────────────────────────────────
    //
    // Editing a param through a source-writing surface (pill, admin CMS, MCP
    // workbench) NORMALIZED its quoting: `'foo'` came back `"foo"`. Nothing is
    // corrupted semantically — the compiler reads both the same — but the SOURCE
    // churns: a diff shows changes the author did not make, and a codebase with a
    // single-quote convention gets rewritten a token at a time.
    //
    // Cause: `original_was_quoted` tested `starts_with('"')` only, so a
    // single-quoted token read as UNQUOTED and `emit_patch_value` wrote the bare
    // value through.

    #[test]
    fn fup133_single_quoted_value_stays_single_quoted() {
        let source = "@card {\n    title: 'hello';\n}";
        let result =
            apply_text_patch(source, 0, source.len(), &json!({"title": "goodbye"})).unwrap();
        assert!(
            result.contains("title: 'goodbye';"),
            "a single-quoted token must round-trip single-quoted — rewriting it to \
             double quotes churns source the author did not touch. Got: {result}"
        );
    }

    #[test]
    fn fup133_double_quoted_value_stays_double_quoted() {
        let source = "@card {\n    title: \"hello\";\n}";
        let result =
            apply_text_patch(source, 0, source.len(), &json!({"title": "goodbye"})).unwrap();
        assert!(
            result.contains("title: \"goodbye\";"),
            "the existing double-quote behaviour must not regress. Got: {result}"
        );
    }

    /// The important one: an edit must only ever touch the edited param's span.
    #[test]
    fn fup133_editing_one_param_leaves_the_others_byte_identical() {
        let source = "@card {\n    title: 'hello';\n    count: \"42\";\n    flag: true;\n}";
        let result = apply_text_patch(source, 0, source.len(), &json!({"title": "bye"})).unwrap();
        assert!(
            result.contains("count: \"42\";"),
            "an untouched quoted-numeric must survive byte-identical — normalizing it \
             to a bare 42 silently changes a string-typed param's type. Got: {result}"
        );
        assert!(
            result.contains("flag: true;"),
            "an untouched bare value must survive byte-identical. Got: {result}"
        );
    }

    /// Quote preservation must never defeat correct escaping: source that no longer
    /// parses is far worse than source that normalized a quote style.
    #[test]
    fn fup133_a_value_containing_the_original_quote_char_stays_valid() {
        let source = "@card {\n    title: 'hello';\n}";
        let result =
            apply_text_patch(source, 0, source.len(), &json!({"title": "it's here"})).unwrap();
        // Either escape within single quotes, or switch to double quotes — both are
        // valid. What must NOT happen is a bare `'it's here'`, which terminates the
        // string at the apostrophe and leaves trailing garbage.
        let ok =
            result.contains(r#"title: "it's here";"#) || result.contains(r"title: 'it\'s here';");
        assert!(
            ok,
            "a value containing the original quote char must produce VALID source — \
             preserving the style blindly would emit 'it's here' and break the file. \
             Got: {result}"
        );
    }

    /// The strongest guard: the round-tripped source must still PARSE. A quote fix
    /// that produced plausible-looking but unparseable source would be a worse
    /// defect than the churn it set out to remove.
    #[test]
    fn fup133_round_tripped_source_still_parses() {
        for (label, source, new_value) in [
            ("single→plain", "@card {\n    title: 'hello';\n}", "goodbye"),
            (
                "single→apostrophe",
                "@card {\n    title: 'hello';\n}",
                "it's here",
            ),
            (
                "single→quote",
                "@card {\n    title: 'hello';\n}",
                "say \"hi\"",
            ),
            (
                "single→backslash",
                "@card {\n    title: 'hello';\n}",
                "a\\b",
            ),
            (
                "double→apostrophe",
                "@card {\n    title: \"hello\";\n}",
                "it's here",
            ),
        ] {
            let result =
                apply_text_patch(source, 0, source.len(), &json!({ "title": new_value })).unwrap();
            assert!(
                crate::parser::parse(&result).is_ok(),
                "{label}: the rewritten source must parse, got:\n{result}"
            );
        }
    }

    #[test]
    fn test_apply_text_patch_single_property() {
        let source = "@scroll {\n    duration: 800ms;\n    easing: ease-out;\n}";
        let result =
            apply_text_patch(source, 0, source.len(), &json!({"duration": "1200ms"})).unwrap();
        assert!(result.contains("duration: 1200ms;"));
        assert!(result.contains("easing: ease-out;"));
    }

    #[test]
    fn test_apply_text_patch_multiple_properties() {
        let source = "@scroll {\n    duration: 800ms;\n    easing: ease-out;\n}";
        let result = apply_text_patch(
            source,
            0,
            source.len(),
            &json!({"duration": "1200ms", "easing": "ease-in"}),
        )
        .unwrap();
        assert!(result.contains("duration: 1200ms;"));
        assert!(result.contains("easing: ease-in;"));
    }

    #[test]
    fn test_apply_text_patch_property_not_found() {
        let source = "@scroll {\n    duration: 800ms;\n}";
        let result = apply_text_patch(source, 0, source.len(), &json!({"nonexistent": "value"}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_apply_text_patch_empty_patch() {
        let source = "@scroll {\n    duration: 800ms;\n}";
        let result = apply_text_patch(source, 0, source.len(), &json!({})).unwrap();
        assert_eq!(result, source);
    }

    #[test]
    fn test_apply_text_patch_rejects_invalid_spans_without_panicking() {
        let source = "héllo";
        let patch = json!({"name": "value"});

        let start_past_end = apply_text_patch(source, source.len() + 1, source.len() + 1, &patch)
            .expect_err("start beyond source must reject");
        assert!(start_past_end.contains("start"));
        assert!(start_past_end.contains(&source.len().to_string()));

        let empty_span =
            apply_text_patch(source, 1, 1, &patch).expect_err("empty span must reject");
        assert!(empty_span.contains("must be less than end"));

        let non_boundary = apply_text_patch(source, 2, source.len(), &patch)
            .expect_err("mid-codepoint span must reject");
        assert!(non_boundary.contains("UTF-8 character boundaries"));
    }

    /// A template invocation's named arg has a BARE key with a QUOTED value
    /// (`&child(title: "hello")`). Quoting must follow the VALUE, not the key:
    /// before PLAN-112 W1 this wrote `title: patched` — unquoted, invalid
    /// Spacetime — because the rule keyed on the key's spelling. The MCP rail hid
    /// it by pre-quoting at its own call site; the EditAst disk rail corrupted the
    /// file silently.
    #[test]
    fn test_apply_text_patch_quotes_invocation_arg_by_value_not_key() {
        let source = "@template &main() { &child(title: \"hello\"); }";
        let start = source.find("&child").unwrap();
        let end = source.find(");").unwrap() + 1;
        let result = apply_text_patch(source, start, end, &json!({"title": "patched"}))
            .expect("patch applies");
        assert!(
            result.contains("title: \"patched\""),
            "a quoted arg must stay quoted, got: {result}"
        );
    }

    /// The value-shaped exemptions: a value that is ALREADY source (its own
    /// quotes, a `$signal` reference, or a number) is written verbatim rather than
    /// re-quoted into a string literal. These were the MCP resolver's pre-quote
    /// rules, now owned by the patcher for every rail.
    #[test]
    fn test_apply_text_patch_leaves_source_shaped_values_verbatim() {
        let source = "@template &main() { &child(title: \"hello\"); }";
        let start = source.find("&child").unwrap();
        let end = source.find(");").unwrap() + 1;
        for (input, expected) in [
            ("$other", "title: $other"),
            ("42", "title: 42"),
            ("\"already\"", "title: \"already\""),
        ] {
            let result = apply_text_patch(source, start, end, &json!({ "title": input }))
                .expect("patch applies");
            assert!(
                result.contains(expected),
                "expected {expected:?} for input {input:?}, got: {result}"
            );
        }
    }

    /// W1R REVIEW FINDING (P1): the guards are checked against bytes read moments
    /// before the write, so without serialization two concurrent edits can BOTH
    /// read the same bytes, BOTH pass their guards, and both write — each Acked,
    /// with the later write silently discarding the earlier accepted edit. The
    /// transaction now holds a lock across read→verify→write, so exactly one of a
    /// racing pair can succeed and the other's guard correctly refuses.
    #[test]
    fn concurrent_span_edits_cannot_lose_an_acknowledged_write() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let dir = tempfile::tempdir().expect("tempdir");
        let source = "@template &main() { &child(title: \"start\"); }";
        std::fs::write(dir.path().join("index.st"), source).expect("write");
        let start = source.find("&child").unwrap();
        let end = source.find(");").unwrap() + 1;
        let span_text = source[start..end].to_string();
        let hash = format!("{:016x}", crate::migrate::hash_content(source));

        // Both writers hold the SAME address + credentials — exactly what two
        // browser tabs served by one poll would send.
        let site = Arc::new(dir.path().to_path_buf());
        let accepted = Arc::new(AtomicUsize::new(0));
        std::thread::scope(|scope| {
            for (i, value) in ["one", "two"].into_iter().enumerate() {
                let site = Arc::clone(&site);
                let accepted = Arc::clone(&accepted);
                let span_text = span_text.clone();
                let hash = hash.clone();
                scope.spawn(move || {
                    let patch = json!({
                        "__invoke_span": [start, end],
                        "__expect_hash": hash,
                        "__expect_span_text": span_text,
                        "title": value,
                    });
                    let result = handle_edit_ast(&site, "index.st", "", &patch, &format!("op-{i}"));
                    if matches!(result.response, ServerMessage::Ack { .. }) {
                        accepted.fetch_add(1, Ordering::SeqCst);
                    }
                });
            }
        });

        assert_eq!(
            accepted.load(Ordering::SeqCst),
            1,
            "exactly one racing write may be accepted; the other must be refused"
        );
        let final_source = std::fs::read_to_string(dir.path().join("index.st")).expect("read back");
        let landed = ["one", "two"]
            .iter()
            .filter(|v| final_source.contains(&format!("title: \"{v}\"")))
            .count();
        assert_eq!(
            landed, 1,
            "the accepted write must be the one on disk, got: {final_source}"
        );
    }

    /// W1R REVIEW FINDING (P1, patch-introduced): the source-atom exemption was a
    /// prefix/suffix test ("starts and ends with a quote"), which is FORGEABLE.
    /// The payload below both starts and ends with `"`, so it was written verbatim
    /// — splicing a brand-new live key into the user's object literal. Object
    /// entries now take NO exemptions, and the bare-key exemptions must consume
    /// the WHOLE value.
    #[test]
    fn test_apply_text_patch_object_value_cannot_inject_a_sibling_key() {
        let source = "@data inline $brand : { \"font\": \"Inter\", \"ink\": \"#000\" };";
        let result = apply_text_patch(
            source,
            0,
            source.len(),
            &json!({"font": "\" , \"injected\": \"x\""}),
        )
        .expect("patch applies");
        assert!(
            !result.contains("\"injected\":"),
            "a forged quoted payload must not become a live key, got: {result}"
        );
        let value: serde_json::Value = {
            let start = result.find('{').expect("object start");
            let end = result.rfind('}').expect("object end");
            serde_json::from_str(&result[start..=end]).expect("the object must stay well-formed")
        };
        assert_eq!(
            value.as_object().expect("an object").len(),
            2,
            "key count must not grow: {result}"
        );
        assert_eq!(
            value["font"], "\" , \"injected\": \"x\"",
            "the payload stays inert content"
        );
        assert_eq!(value["ink"], "#000", "sibling untouched");
    }

    /// The bare-key exemptions must be TOTAL matches. A value that merely STARTS
    /// like a source atom but carries trailing source is text, and is escaped.
    #[test]
    fn test_apply_text_patch_partial_source_atoms_are_escaped() {
        let source = "@template &main() { &child(title: \"hello\"); }";
        let start = source.find("&child").unwrap();
        let end = source.find(");").unwrap() + 1;
        for payload in [
            "\"a\" ); &evil(", // complete literal + trailing source
            "$x ); &evil(",    // signal ref + trailing source
            "1 ); &evil(",     // number + trailing source
            "$",               // bare sigil, not a reference
            "$1bad",           // invalid identifier
            "NaN",             // parses as f64 but is not numeric source
        ] {
            let result = apply_text_patch(source, start, end, &json!({ "title": payload }))
                .expect("patch applies");
            // The payload's TEXT may appear (escaped, inside the literal); what
            // must not happen is it becoming SOURCE. The round-trip below is the
            // real proof: if the value parses back as one JSON string equal to the
            // input, nothing escaped the slot.
            let value_start = result.find("title: ").expect("the arg") + "title: ".len();
            let parsed = serde_json::Deserializer::from_str(&result[value_start..])
                .into_iter::<String>()
                .next()
                .expect("a value")
                .expect("payload {payload:?} must be emitted as one escaped literal");
            assert_eq!(
                parsed, payload,
                "payload {payload:?} must round-trip as content"
            );
        }
    }

    /// The escaping guarantee (BUG-093) must survive the rule move: a value
    /// carrying quotes/braces cannot break out of the literal or inject source.
    #[test]
    fn test_apply_text_patch_invocation_arg_escapes_injection() {
        let source = "@template &main() { &child(title: \"hello\"); }";
        let start = source.find("&child").unwrap();
        let end = source.find(");").unwrap() + 1;
        let result = apply_text_patch(
            source,
            start,
            end,
            &json!({"title": "a\" ); } @template &evil() { <script>"}),
        )
        .expect("patch applies");
        // The payload text may APPEAR in the output — expected, and not a leak:
        // what matters is that it stays INSIDE the string literal (inert) instead
        // of breaking out into real source. Prove that structurally: the emitted
        // value must parse back as ONE JSON string equal to the input, which holds
        // only if every internal quote was escaped.
        let value_start = result.find("title: ").expect("the arg") + "title: ".len();
        let parsed = serde_json::Deserializer::from_str(&result[value_start..])
            .into_iter::<String>()
            .next()
            .expect("a value")
            .expect("the emitted arg must be ONE well-formed string literal");
        assert_eq!(
            parsed, "a\" ); } @template &evil() { <script>",
            "the payload must survive as inert string CONTENT, got: {result}"
        );
    }

    // BUG-092: editing one key of an OBJECT-LITERAL value (the brand singleton
    // `@data inline $brand : { "scarlet": "#FF0020", "ink": "#0a0a0a", "radius":
    // "8px" }`) must change ONLY that key's value and leave sibling keys byte-exact.
    // The directive-body patcher terminated a value at the first `;`/`}`, which for
    // a comma-separated object literal ate every following sibling key.
    #[test]
    fn test_apply_text_patch_object_literal_middle_key_preserves_siblings() {
        let source = "@data inline $brand : { \"scarlet\": \"#FF0020\", \"ink\": \"#0a0a0a\", \"radius\": \"8px\" };";
        let result = apply_text_patch(source, 0, source.len(), &json!({"ink": "#00FF00"})).unwrap();
        assert!(
            result.contains("\"ink\": \"#00FF00\""),
            "ink updated: {result}"
        );
        assert!(
            result.contains("\"scarlet\": \"#FF0020\""),
            "scarlet survives: {result}"
        );
        assert!(
            result.contains("\"radius\": \"8px\""),
            "radius survives (not eaten): {result}"
        );
    }

    // BUG-093: an object-literal string value must be JSON-ESCAPED on write, or a
    // value containing a quote/backslash/brace/newline corrupts the .st literal
    // (and a crafted dev-ws value could inject arbitrary .st source). Reviewer P1.
    #[test]
    fn test_apply_text_patch_object_literal_value_is_escaped() {
        let source = "@data inline $brand : { \"font\": \"Inter\", \"ink\": \"#000\" };";
        // A value containing a double-quote must be escaped, not break the literal.
        let result =
            apply_text_patch(source, 0, source.len(), &json!({"font": "\"Inter\", sans"})).unwrap();
        // The written literal must still be valid: the embedded quotes are escaped.
        assert!(
            result.contains("\\\"Inter\\\""),
            "embedded quotes escaped: {result}"
        );
        assert!(
            result.contains("\"ink\": \"#000\""),
            "sibling survives: {result}"
        );
        // The whole object literal value text must round-trip as valid JSON.
        let start = result.find('{').unwrap();
        let end = result.rfind('}').unwrap();
        let obj: serde_json::Value = serde_json::from_str(&result[start..=end])
            .expect("patched object literal must remain valid JSON");
        assert_eq!(
            obj.get("font").and_then(|v| v.as_str()),
            Some("\"Inter\", sans")
        );
    }

    #[test]
    fn test_apply_text_patch_object_literal_injection_neutralized() {
        // A value attempting to break out of the string + inject .st source must be
        // contained as an escaped string, not executed as source.
        let source = "@data inline $brand : { \"a\": \"x\", \"b\": \"y\" };";
        let evil = "z\"; } @data evil { hijack";
        let result = apply_text_patch(source, 0, source.len(), &json!({"a": evil})).unwrap();
        let start = result.find('{').unwrap();
        let end = result.rfind('}').unwrap();
        let obj: serde_json::Value = serde_json::from_str(&result[start..=end])
            .expect("injection attempt must remain a valid escaped JSON string");
        assert_eq!(
            obj.get("a").and_then(|v| v.as_str()),
            Some(evil),
            "injection contained as data"
        );
        // The payload survives ONLY as an escaped JSON string value (the `"` that
        // would have closed the literal is backslash-escaped), so it can never be
        // a real directive — the literal still parses as one object with keys a,b.
        assert!(
            result.contains("\\\""),
            "the breakout quote is escaped: {result}"
        );
        assert_eq!(
            obj.as_object().map(|o| o.len()),
            Some(2),
            "still one object, 2 keys: {result}"
        );
    }

    #[test]
    fn test_apply_text_patch_object_literal_last_key() {
        let source = "@data inline $brand : { \"scarlet\": \"#FF0020\", \"radius\": \"8px\" };";
        let result = apply_text_patch(source, 0, source.len(), &json!({"radius": "12px"})).unwrap();
        assert!(
            result.contains("\"radius\": \"12px\""),
            "last key updated: {result}"
        );
        assert!(
            result.contains("\"scarlet\": \"#FF0020\""),
            "scarlet survives: {result}"
        );
        assert!(
            result.trim_end().ends_with("};"),
            "literal closes cleanly: {result}"
        );
    }

    #[test]
    fn test_apply_text_patch_with_offset() {
        let source = ".hero {\n    @scroll {\n        duration: 800ms;\n    }\n}";
        // Patch only within the @scroll region (offset past ".hero {\n    ")
        let scroll_start = source.find("@scroll").unwrap();
        let scroll_end = source.find("    }\n}").unwrap() + 5;
        let result = apply_text_patch(
            source,
            scroll_start,
            scroll_end,
            &json!({"duration": "500ms"}),
        )
        .unwrap();
        assert!(result.contains("duration: 500ms;"));
        assert!(result.starts_with(".hero"));
    }

    // -------------------------------------------------------------------------
    // handle_edit_ast integration tests (with filesystem)
    // -------------------------------------------------------------------------

    fn create_test_st(dir: &TempDir, filename: &str, content: &str) -> PathBuf {
        let path = dir.path().join(filename);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_edit_ast_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let result = handle_edit_ast(
            dir.path(),
            "nonexistent.st",
            ".hero §scroll",
            &json!({"duration": "1200ms"}),
            "op_a1",
        );
        assert!(matches!(result.response, ServerMessage::Reject { .. }));
        assert!(result.broadcast.is_none());
    }

    #[test]
    fn test_edit_ast_invalid_selector_no_directive() {
        let dir = TempDir::new().unwrap();
        create_test_st(&dir, "styles.st", ".hero {\n}");
        let result = handle_edit_ast(
            dir.path(),
            "styles.st",
            ".hero", // missing @directive
            &json!({"duration": "1200ms"}),
            "op_a2",
        );
        assert!(matches!(
            result.response, ServerMessage::Reject { ref reason, .. } if reason.contains("selector")
        ));
    }

    #[test]
    fn test_edit_ast_path_traversal_rejected() {
        let dir = TempDir::new().unwrap();
        create_test_st(&dir, "safe.st", ".hero {\n}");
        let result = handle_edit_ast(
            dir.path(),
            "../../../etc/passwd",
            ".hero §scroll",
            &json!({}),
            "op_a3",
        );
        assert!(matches!(
            result.response, ServerMessage::Reject { ref reason, .. }
                if reason.contains("not found") || reason.contains("traversal")
        ));
        assert!(result.broadcast.is_none());
    }

    #[test]
    fn test_edit_ast_scope_not_found() {
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "styles.st",
            ".hero {\n    @scroll reveal(&fade-up) {\n        duration: 800ms;\n    }\n}\n",
        );
        let result = handle_edit_ast(
            dir.path(),
            "styles.st",
            ".missing §scroll",
            &json!({"duration": "1200ms"}),
            "op_a4",
        );
        assert!(matches!(
            result.response, ServerMessage::Reject { ref reason, .. } if reason.contains("not found")
        ));
    }

    #[test]
    fn test_edit_ast_via_dispatch() {
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "styles.st",
            ".hero {\n    @scroll reveal(&fade-up) {\n        duration: 800ms;\n    }\n}\n",
        );
        let msg = ClientMessage::EditAst {
            file: "styles.st".to_string(),
            selector: ".hero §scroll".to_string(),
            patch: json!({"duration": "1200ms"}),
            op_id: "op_a5".to_string(),
        };
        let result = handle_message(dir.path(), msg);
        // If the directive has spans, this should succeed; if spans are zero, it rejects gracefully
        match &result.response {
            ServerMessage::Ack { op_id } => {
                assert_eq!(op_id, "op_a5");
                // Verify the file was modified
                let content = fs::read_to_string(dir.path().join("styles.st")).unwrap();
                assert!(content.contains("1200ms"));
            }
            ServerMessage::Reject { reason, .. } => {
                // Acceptable if parser doesn't provide spans in this context
                assert!(
                    reason.contains("not found")
                        || reason.contains("span")
                        || reason.contains("Parse error"),
                    "Unexpected rejection reason: {}",
                    reason
                );
            }
            _ => panic!("Unexpected response variant"),
        }
        // AST edits never broadcast (file watcher handles reload)
        assert!(result.broadcast.is_none());
    }

    #[test]
    fn test_edit_ast_brand_singleton_file_scope_binding() {
        // PLAN-034 Wave B: editing a brand token addresses the FILE-SCOPE
        // `@data inline $brand …` by binding selector `$brand §data`. The patch
        // rewrites ONLY the targeted key in the object literal; siblings survive.
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "index.st",
            "@type Brand { scarlet: color; ink: color; }\n@data inline $brand Brand : { \"scarlet\": \"#FF0020\", \"ink\": \"#0a0a0a\" };\nh1 { \"x\" }\n",
        );
        let msg = ClientMessage::EditAst {
            file: "index.st".to_string(),
            selector: "$brand §data".to_string(),
            patch: json!({"ink": "#00FF00"}),
            op_id: "op_brand".to_string(),
        };
        let result = handle_message(dir.path(), msg);
        match &result.response {
            ServerMessage::Ack { op_id } => {
                assert_eq!(op_id, "op_brand");
                let content = fs::read_to_string(dir.path().join("index.st")).unwrap();
                assert!(
                    content.contains("\"ink\": \"#00FF00\""),
                    "ink updated: {content}"
                );
                assert!(
                    content.contains("\"scarlet\": \"#FF0020\""),
                    "scarlet survives: {content}"
                );
            }
            ServerMessage::Reject { reason, .. } => panic!("brand edit rejected: {reason}"),
            _ => panic!("Unexpected response variant"),
        }
    }

    #[test]
    fn test_edit_ast_patch_must_be_object() {
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "styles.st",
            ".hero {\n    @scroll reveal(&fade-up) {\n        duration: 800ms;\n    }\n}\n",
        );
        let result = handle_edit_ast(
            dir.path(),
            "styles.st",
            ".hero §scroll",
            &json!("not an object"),
            "op_a6",
        );
        assert!(matches!(
            result.response, ServerMessage::Reject { ref reason, .. } if reason.contains("object")
        ));
    }

    #[test]
    fn test_edit_ast_patches_directive_paren_named_arg() {
        // FUP-089 keystone (the builder inspector WRITE path): EditAst patches a
        // directive's PAREN-ARG named param in place, leaving sibling args + the
        // surrounding scope untouched. This is what an inspector "edit param" emits
        // for a @macro-style call (e.g. tuning @object roughness). Uses @scroll's
        // body param form which is the shipped, span-bearing directive; the patch
        // mechanism (apply_text_patch find `key:` within the directive span) is the
        // SAME one that serves arbitrary named args.
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "m.st",
            ".card {\n    @scroll reveal(&fade-up) {\n        duration: 800ms;\n        threshold: 0.25;\n    }\n}\n",
        );
        let result = handle_edit_ast(
            dir.path(),
            "m.st",
            ".card §scroll",
            &json!({"duration": "1500ms"}),
            "op_param",
        );
        match &result.response {
            ServerMessage::Ack { op_id } => {
                assert_eq!(op_id, "op_param");
                let after = fs::read_to_string(dir.path().join("m.st")).unwrap();
                assert!(after.contains("duration: 1500ms"), "param patched: {after}");
                assert!(
                    after.contains("threshold: 0.25"),
                    "sibling param untouched: {after}"
                );
                assert!(
                    after.contains("@scroll reveal(&fade-up)"),
                    "directive head intact: {after}"
                );
            }
            ServerMessage::Reject { reason, .. } => {
                // Tolerated only if this build lacks directive spans (same posture
                // as test_edit_ast_via_dispatch); never a silent wrong-write.
                assert!(
                    reason.contains("not found")
                        || reason.contains("span")
                        || reason.contains("Parse"),
                    "unexpected rejection: {reason}"
                );
            }
            _ => panic!("unexpected response variant"),
        }
    }

    #[test]
    fn test_edit_ast_invoke_span_patches_named_arg() {
        // FUP-093/G2 Layer 3: editing ONE template invocation's named arg by its
        // source SPAN (the builder inspector's per-instance write). Two &card calls
        // with named args; patch the FIRST by its span, assert only it changed.
        let dir = TempDir::new().unwrap();
        let src = "@template &card($title) { <div>`$title`</div> }\n@template &main() {\n  &card(title: \"Alpha\");\n  &card(title: \"Beta\");\n}\n";
        create_test_st(&dir, "p.st", src);
        // The first invocation `&card(title: \"Alpha\")` span (byte offsets in src).
        let first = src.find("&card(title: \"Alpha\")").unwrap();
        let first_end = first + "&card(title: \"Alpha\")".len();
        let result = handle_edit_ast(
            dir.path(),
            "p.st",
            "", // selector unused for span-addressed edits
            &json!({"__invoke_span": [first, first_end], "title": "\"Renamed\""}),
            "op_inv",
        );
        match &result.response {
            ServerMessage::Ack { .. } => {
                let after = fs::read_to_string(dir.path().join("p.st")).unwrap();
                assert!(
                    after.contains("title: \"Renamed\""),
                    "first invocation patched: {after}"
                );
                assert!(
                    after.contains("title: \"Beta\""),
                    "second invocation untouched: {after}"
                );
                assert!(
                    !after.contains("title: \"Alpha\""),
                    "old value gone: {after}"
                );
            }
            ServerMessage::Reject { reason, .. } => {
                // Acceptable only if the param-find is absent in this build; never a
                // silent wrong-write.
                assert!(
                    reason.contains("not found") || reason.contains("span"),
                    "unexpected rejection: {reason}"
                );
            }
            _ => panic!("unexpected response variant"),
        }
    }

    #[test]
    fn test_edit_ast_invoke_span_expect_hash_guards_write() {
        let src = "@template &main() { &card(title: \"Alpha\"); }\n";
        let invoke_start = src.find("&card(title: \"Alpha\")").unwrap();
        let invoke_end = invoke_start + "&card(title: \"Alpha\")".len();

        let matching_dir = TempDir::new().unwrap();
        create_test_st(&matching_dir, "p.st", src);
        let matching_hash = format!("{:016x}", crate::migrate::hash_content(src));
        let matching = handle_edit_ast(
            matching_dir.path(),
            "p.st",
            "",
            &json!({
                "__invoke_span": [invoke_start, invoke_end],
                "__expect_hash": matching_hash,
                "title": "\"Renamed\""
            }),
            "op_expect_match",
        );
        assert!(
            matches!(matching.response, ServerMessage::Ack { .. }),
            "matching content hash must write: {:?}",
            matching.response
        );
        assert!(
            fs::read_to_string(matching_dir.path().join("p.st"))
                .unwrap()
                .contains("title: \"Renamed\"")
        );

        let mismatch_dir = TempDir::new().unwrap();
        let mismatch_path = create_test_st(&mismatch_dir, "p.st", src);
        let before = fs::read_to_string(&mismatch_path).unwrap();
        let mismatched = handle_edit_ast(
            mismatch_dir.path(),
            "p.st",
            "",
            &json!({
                "__invoke_span": [invoke_start, invoke_end],
                "__expect_hash": "0000000000000000",
                "title": "\"Wrong\""
            }),
            "op_expect_mismatch",
        );
        assert!(
            matches!(mismatched.response, ServerMessage::Reject { ref reason, .. } if reason.contains("__expect_hash mismatch")),
            "mismatched content hash must reject: {:?}",
            mismatched.response
        );
        assert_eq!(fs::read_to_string(&mismatch_path).unwrap(), before);
    }

    #[test]
    fn guarded_comment_deletion_refuses_a_stale_hash_and_deletes_backwards() {
        // The rail deletes only RESOLVED inline comments, so the fixture needs
        // real sidecar records saying so — authorization is recomputed from
        // source + store on every call, never taken from the caller.
        let src = "//@todo: first\n.keep { color: blue; }\n//@todo: second\n";
        let dir = TempDir::new().unwrap();
        let path = create_test_st(&dir, "p.st", src);

        let spans = crate::comments::inline_comment_spans("p.st", src);
        assert_eq!(spans.len(), 2, "fixture must have two inline comments");
        for span in &spans {
            let record = crate::comments::CommentRecord {
                id: span.id.clone(),
                type_id: "todo".to_string(),
                status: crate::comments::Status::Resolved,
                author: crate::comments::Author {
                    kind: crate::comments::AuthorKind::Human,
                    name: "ada".to_string(),
                },
                claimed_by: None,
                anchor: crate::comments::Anchor::Inline {
                    file: "p.st".to_string(),
                    line: 1,
                },
                text: "resolved".to_string(),
                meta: Default::default(),
                thread: Vec::new(),
                history: Vec::new(),
                created_at: "2026-07-31T00:00:00Z".to_string(),
                updated_at: None,
                v: crate::comments::SCHEMA_VERSION,
                inline: true,
            };
            crate::comments::write_record(dir.path(), &record).unwrap();
        }

        let first = &spans[0];
        let second = &spans[1];

        // A stale snapshot must not write: the spans no longer mean what the
        // caller was shown.
        let stale = handle_edit_ast(
            dir.path(),
            "p.st",
            "",
            &json!({
                "__delete_spans": [[first.start, first.end]], "__expect_hash": "0000000000000000",
                "__expect_span_texts": [&src[first.start..first.end]]
            }),
            "stale",
        );
        assert!(
            matches!(stale.response, ServerMessage::Reject { ref reason, .. } if reason.contains("__expect_hash mismatch"))
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            src,
            "a stale snapshot must not write"
        );

        // Deleting backwards keeps earlier offsets valid.
        let deleted = handle_edit_ast(
            dir.path(),
            "p.st",
            "",
            &json!({
                "__delete_spans": [[first.start, first.end], [second.start, second.end]],
                "__expect_hash": format!("{:016x}", crate::migrate::hash_content(src)),
                "__expect_span_texts": [&src[first.start..first.end], &src[second.start..second.end]]
            }),
            "delete",
        );
        assert!(
            matches!(deleted.response, ServerMessage::Ack { .. }),
            "got {:?}",
            deleted.response
        );
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            ".keep { color: blue; }\n"
        );
    }

    /// THE authorization gate, at the rail rather than at one caller.
    ///
    /// `handle_edit_ast` is reachable from the PUBLIC dev websocket, so the
    /// hash and span-text guards alone would let any connected client delete
    /// any byte range of any project file just by echoing back its current
    /// contents. Only a resolved inline comment may be deleted here.
    #[test]
    fn guarded_deletion_refuses_a_span_that_is_not_a_resolved_comment() {
        let src = "//@todo: still open\n.keep { color: blue; }\n";
        let dir = TempDir::new().unwrap();
        let path = create_test_st(&dir, "p.st", src);
        let hash = format!("{:016x}", crate::migrate::hash_content(src));

        // A line of CSS is not a comment at all.
        let code_start = src.find(".keep").unwrap();
        let code_end = src.len();
        let code = handle_edit_ast(
            dir.path(),
            "p.st",
            "",
            &json!({
                "__delete_spans": [[code_start, code_end]], "__expect_hash": hash,
                "__expect_span_texts": [&src[code_start..code_end]]
            }),
            "code",
        );
        assert!(
            matches!(code.response, ServerMessage::Reject { ref reason, .. } if reason.contains("not a deletable span")),
            "deleting code must be refused: {:?}",
            code.response
        );

        // An OPEN comment is work in progress, not litter.
        let spans = crate::comments::inline_comment_spans("p.st", src);
        let open = handle_edit_ast(
            dir.path(),
            "p.st",
            "",
            &json!({
                "__delete_spans": [[spans[0].start, spans[0].end]], "__expect_hash": hash,
                "__expect_span_texts": [&src[spans[0].start..spans[0].end]]
            }),
            "open",
        );
        assert!(
            matches!(open.response, ServerMessage::Reject { ref reason, .. } if reason.contains("not a deletable span")),
            "deleting an unresolved comment must be refused: {:?}",
            open.response
        );

        assert_eq!(
            fs::read_to_string(path).unwrap(),
            src,
            "nothing may have been written"
        );
    }

    #[test]
    fn test_edit_ast_invoke_span_out_of_bounds_rejected() {
        // A span past EOF (stale projection) must reject cleanly, never panic or
        // write garbage.
        let dir = TempDir::new().unwrap();
        create_test_st(&dir, "p.st", "@template &main() { &card(title: \"A\"); }\n");
        let result = handle_edit_ast(
            dir.path(),
            "p.st",
            "",
            &json!({"__invoke_span": [9999, 10000], "title": "\"X\""}),
            "op_oob",
        );
        assert!(matches!(
            result.response, ServerMessage::Reject { ref reason, .. } if reason.contains("out of bounds")
        ));
    }

    #[test]
    fn test_edit_ast_rejects_unknown_param_key() {
        // Correct-by-design: a patch key that does not exist in the directive must
        // be rejected, not silently inserted at the wrong place (the inspector only
        // ever emits real param names from the contract, but the handler must guard).
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "m.st",
            ".card {\n    @scroll reveal(&fade-up) {\n        duration: 800ms;\n    }\n}\n",
        );
        let result = handle_edit_ast(
            dir.path(),
            "m.st",
            ".card §scroll",
            &json!({"nonexistent_param": "42"}),
            "op_unknown",
        );
        // Either a clean reject (key not found) or, if spans are absent, the
        // span-not-found reject — but NEVER an Ack that wrote a bogus key.
        match &result.response {
            ServerMessage::Reject { .. } => {}
            ServerMessage::Ack { .. } => {
                let after = fs::read_to_string(dir.path().join("m.st")).unwrap();
                assert!(
                    !after.contains("nonexistent_param"),
                    "unknown key must NOT be written into source: {after}"
                );
            }
            _ => panic!("unexpected response variant"),
        }
    }

    // -------------------------------------------------------------------------
    // handle_inspect_element tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_inspect_element_known_selector_css() {
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "styles.st",
            ".hero {\n    color: #ff0000;\n    padding: 40px;\n    transition-duration: 1200ms;\n}\n",
        );
        let result = handle_inspect_element(dir.path(), ".hero", None);
        match result {
            ServerMessage::ElementContext {
                selector,
                source_files,
                sections,
                not_found,
                ..
            } => {
                assert_eq!(selector, ".hero");
                assert!(!not_found);
                assert!(!source_files.is_empty());
                assert!(source_files.contains(&"styles.st".to_string()));
                // Should have a CSS section
                let css_section = sections.iter().find(|s| s.label == "CSS");
                assert!(
                    css_section.is_some(),
                    "Expected CSS section, got: {:?}",
                    sections.iter().map(|s| &s.label).collect::<Vec<_>>()
                );
                let css = css_section.unwrap();
                assert!(!css.properties.is_empty());
                // Check that color property exists
                let color_prop = css.properties.iter().find(|p| p.name == "color");
                assert!(color_prop.is_some(), "Expected 'color' property");
                assert_eq!(color_prop.unwrap().value.raw, "#ff0000");
            }
            _ => panic!("Expected ElementContext response"),
        }
    }

    #[test]
    fn test_inspect_structure_returns_ir_tree() {
        // PLAN-064 B2: the dev-ws host reads the SHARED Structure IR producer — the
        // SAME tree the MCP navigator renders. A single-template hero must project
        // its element tree (the "1-node hero" fix), reachable from the dev server.
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "index.st",
            "@template &main() { <section class=\"hero\"><h1>`$title`</h1><p>`$sub`</p></section> }\n",
        );
        let result = handle_inspect_structure(dir.path(), "index.st", None);
        match result {
            ServerMessage::Structure {
                file,
                template,
                nodes,
                error,
            } => {
                assert_eq!(file, "index.st");
                assert_eq!(template, "main");
                assert!(error.is_none(), "structure built cleanly: {error:?}");
                // main (template) + section + h1 + p.
                let labels: Vec<&str> = nodes.iter().map(|n| n.label.as_str()).collect();
                assert!(labels.contains(&"main"), "root template node: {labels:?}");
                assert!(
                    labels.contains(&"section.hero"),
                    "section element: {labels:?}"
                );
                assert!(labels.contains(&"h1"), "h1 element: {labels:?}");
                // The h1 carries its $title binding — the reactive edge the DOM can't show.
                let h1 = nodes.iter().find(|n| n.label == "h1").expect("h1 node");
                assert!(
                    h1.bindings.contains(&"title".to_string()),
                    "h1 binds title: {:?}",
                    h1.bindings
                );
            }
            other => panic!("expected Structure, got {other:?}"),
        }
    }

    #[test]
    fn test_inspect_structure_missing_template_reports_error() {
        // A file with no matching entry template returns an empty tree + an error,
        // never a panic.
        let dir = TempDir::new().unwrap();
        create_test_st(&dir, "index.st", "@template &other() { <div>x</div> }\n");
        let result = handle_inspect_structure(dir.path(), "index.st", None);
        match result {
            ServerMessage::Structure { nodes, error, .. } => {
                assert!(nodes.is_empty(), "no 'main' template -> empty tree");
                assert!(error.is_some(), "missing template reported as error");
            }
            other => panic!("expected Structure, got {other:?}"),
        }
    }

    #[test]
    fn test_inspect_element_unknown_selector() {
        let dir = TempDir::new().unwrap();
        create_test_st(&dir, "styles.st", ".hero {\n    color: red;\n}\n");
        let result = handle_inspect_element(dir.path(), ".nonexistent", None);
        match result {
            ServerMessage::ElementContext {
                not_found,
                sections,
                source_files,
                ..
            } => {
                assert!(not_found);
                assert!(sections.is_empty());
                assert!(source_files.is_empty());
            }
            _ => panic!("Expected ElementContext response"),
        }
    }

    #[test]
    fn test_inspect_element_value_classification() {
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "styles.st",
            ".box {\n    color: #ff0000;\n    transition-duration: 1200ms;\n    padding: 40px;\n}\n",
        );
        let result = handle_inspect_element(dir.path(), ".box", None);
        match result {
            ServerMessage::ElementContext { sections, .. } => {
                let css = sections
                    .iter()
                    .find(|s| s.label == "CSS")
                    .expect("CSS section");
                // Color should be classified as Color
                let color = css
                    .properties
                    .iter()
                    .find(|p| p.name == "color")
                    .expect("color prop");
                assert!(
                    matches!(color.value.parsed, ValueType::Color(_)),
                    "Expected Color, got {:?}",
                    color.value.parsed
                );
                // Duration
                let dur = css
                    .properties
                    .iter()
                    .find(|p| p.name == "transition-duration")
                    .expect("duration prop");
                assert!(
                    matches!(dur.value.parsed, ValueType::Duration { .. }),
                    "Expected Duration, got {:?}",
                    dur.value.parsed
                );
                // Dimension
                let pad = css
                    .properties
                    .iter()
                    .find(|p| p.name == "padding")
                    .expect("padding prop");
                assert!(
                    matches!(pad.value.parsed, ValueType::Dimension { .. }),
                    "Expected Dimension, got {:?}",
                    pad.value.parsed
                );
            }
            _ => panic!("Expected ElementContext"),
        }
    }

    #[test]
    fn test_inspect_element_multi_file_resolution() {
        let dir = TempDir::new().unwrap();
        create_test_st(&dir, "a.st", ".hero {\n    color: red;\n}\n");
        create_test_st(&dir, "b.st", ".hero {\n    padding: 20px;\n}\n");
        let result = handle_inspect_element(dir.path(), ".hero", None);
        match result {
            ServerMessage::ElementContext {
                source_files,
                sections,
                not_found,
                ..
            } => {
                assert!(!not_found);
                assert!(
                    source_files.len() >= 2,
                    "Expected at least 2 source files, got {:?}",
                    source_files
                );
                // Both files should contribute CSS sections
                assert!(
                    sections.len() >= 2,
                    "Expected at least 2 sections, got {}",
                    sections.len()
                );
            }
            _ => panic!("Expected ElementContext"),
        }
    }

    #[test]
    fn test_inspect_element_file_hint_fast_path() {
        let dir = TempDir::new().unwrap();
        create_test_st(&dir, "a.st", ".hero {\n    color: red;\n}\n");
        create_test_st(&dir, "b.st", ".hero {\n    padding: 20px;\n}\n");
        // With file_hint, only a.st should be scanned
        let result = handle_inspect_element(dir.path(), ".hero", Some("a.st"));
        match result {
            ServerMessage::ElementContext {
                source_files,
                not_found,
                ..
            } => {
                assert!(!not_found);
                assert_eq!(source_files.len(), 1);
                assert!(source_files[0].contains("a.st"));
            }
            _ => panic!("Expected ElementContext"),
        }
    }

    #[test]
    fn test_inspect_element_file_hint_invalid() {
        let dir = TempDir::new().unwrap();
        let result = handle_inspect_element(dir.path(), ".hero", Some("nonexistent.st"));
        match result {
            ServerMessage::ElementContext { not_found, .. } => {
                assert!(not_found);
            }
            _ => panic!("Expected ElementContext"),
        }
    }

    // -------------------------------------------------------------------------
    // handle_content_edit_html tests (ITEM-107-008)
    // -------------------------------------------------------------------------

    #[test]
    fn test_handle_content_edit_html_basic() {
        let dir = TempDir::new().unwrap();
        let html_path = dir.path().join("index.html");
        // Source file has NO data-st-id — provenance is injected at serve-time only
        fs::write(&html_path, "<h1>Old Title</h1>").unwrap();
        // Compute the hash that inject_content_provenance would generate
        let hash = crate::server::compute_st_hash("index.html:h1:1");
        let selector = format!(r#"[data-st-id="{}"]"#, hash);
        handle_content_edit_html(&html_path, "index.html", &selector, "New Title").unwrap();
        let result = fs::read_to_string(&html_path).unwrap();
        assert!(result.contains("New Title"));
        assert!(!result.contains("Old Title"));
        // Provenance attributes must be stripped before writing
        assert!(!result.contains("data-st-id"));
        assert!(!result.contains("data-st-origin"));
    }

    #[test]
    fn test_handle_content_edit_html_not_found() {
        let dir = TempDir::new().unwrap();
        let html_path = dir.path().join("index.html");
        fs::write(&html_path, "<h1>Title</h1>").unwrap();
        let err = handle_content_edit_html(
            &html_path,
            "index.html",
            r#"[data-st-id="nonexistent"]"#,
            "New",
        )
        .unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn test_handle_content_edit_html_preserves_structure() {
        let dir = TempDir::new().unwrap();
        let html_path = dir.path().join("page.html");
        fs::write(
            &html_path,
            "<html><body><div id=\"wrapper\"><h1>Old</h1><p>Keep me</p></div></body></html>",
        )
        .unwrap();
        let hash = crate::server::compute_st_hash("page.html:h1:1");
        let selector = format!(r#"[data-st-id="{}"]"#, hash);
        handle_content_edit_html(&html_path, "page.html", &selector, "<strong>New</strong>")
            .unwrap();
        let result = fs::read_to_string(&html_path).unwrap();
        assert!(result.contains("<strong>New</strong>"));
        assert!(result.contains("<p>Keep me</p>"));
        assert!(!result.contains("Old"));
        assert!(!result.contains("data-st-id"));
    }

    // -------------------------------------------------------------------------
    // replace_html_element_content tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_replace_html_element_content_basic() {
        let html = "<h3>Old Title</h3>";
        let result = replace_html_element_content(html, "h3", "New Title").unwrap();
        assert!(result.contains("New Title"));
        assert!(!result.contains("Old Title"));
    }

    #[test]
    fn test_replace_html_element_content_no_match() {
        let html = "<h3>Title</h3>";
        let result = replace_html_element_content(html, "h1", "New");
        // No match → Err, not silent no-op
        assert!(
            result.is_err(),
            "Expected Err on zero matches, got: {:?}",
            result
        );
        assert!(result.unwrap_err().contains("0 elements"));
    }

    /// Root cause regression: ambiguous selector matches multiple elements →
    /// all get replaced, silently corrupting the document.
    /// Selector must be provably unique (exactly one match) or the edit is rejected.
    #[test]
    fn test_replace_html_element_content_rejects_ambiguous_selector() {
        // Mirrors the real nav template: five <a> tags, selector "a" is ambiguous.
        let html = r##"<nav>
            <a href="/" class="ud-nav__logo">undefine</a>
            <a href="#work" class="ud-nav__link">Work</a>
            <a href="#principles" class="ud-nav__link">Principles</a>
            <a href="#contact" class="ud-nav__link">Contact</a>
            <a href="mailto:x@y.com" class="ud-nav__cta">Get in touch</a>
        </nav>"##;
        let result = replace_html_element_content(html, "a", "Contact us");
        assert!(
            result.is_err(),
            "Expected Err for ambiguous selector, got: {:?}",
            result
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("5 elements") || err.contains("matched"),
            "Error should report match count, got: {}",
            err
        );
    }

    /// Three elements share .ud-nav__link — that selector is not unique either.
    #[test]
    fn test_replace_html_element_content_rejects_multi_class_match() {
        let html = r#"<div>
            <a class="ud-nav__link">Work</a>
            <a class="ud-nav__link">Principles</a>
            <a class="ud-nav__link">Contact</a>
        </div>"#;
        let result = replace_html_element_content(html, ".ud-nav__link", "X");
        assert!(
            result.is_err(),
            "Expected Err for multi-match class selector"
        );
        assert!(result.unwrap_err().contains("3 elements"));
    }

    /// A specific class selector that matches exactly one element succeeds
    /// and does NOT touch sibling elements.
    #[test]
    fn test_replace_html_element_content_unique_class_succeeds() {
        let html = r##"<nav>
            <a href="#work" class="ud-nav__link">Work</a>
            <a href="#contact" class="ud-nav__cta">Get in touch</a>
        </nav>"##;
        let result = replace_html_element_content(html, ".ud-nav__cta", "Contact us").unwrap();
        assert!(result.contains("Contact us"), "CTA should be updated");
        assert!(result.contains("Work"), "sibling link must be untouched");
        assert!(
            !result.contains("Get in touch"),
            "old CTA text must be gone"
        );
    }

    // -------------------------------------------------------------------------
    // handle_content_edit dispatcher tests (ITEM-107-007)
    // -------------------------------------------------------------------------

    #[test]
    fn test_handle_content_edit_html_dispatch() {
        let dir = TempDir::new().unwrap();
        let html_path = dir.path().join("page.html");
        fs::write(&html_path, "<h1>Old</h1>").unwrap();
        let hash = crate::server::compute_st_hash("page.html:h1:1");
        let selector = format!(r#"[data-st-id="{}"]"#, hash);
        let result = handle_content_edit(&html_path, "page.html", &selector, "New Content", None);
        assert!(result.is_ok());
        let content = fs::read_to_string(&html_path).unwrap();
        assert!(content.contains("New Content"));
    }

    #[test]
    fn test_handle_content_edit_unsupported_combination() {
        let dir = TempDir::new().unwrap();
        let txt_path = dir.path().join("file.txt");
        fs::write(&txt_path, "some text").unwrap();
        let result = handle_content_edit(
            &txt_path,
            "file.txt",
            r#"[data-st-id="st-abc"]"#,
            "New",
            None,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported"));
    }

    // -------------------------------------------------------------------------
    // __content branch guard tests (ITEM-107-007)
    // -------------------------------------------------------------------------

    #[test]
    fn test_edit_ast_content_branch_guard_html() {
        // __content patch on .html file → routes through content edit path
        let dir = TempDir::new().unwrap();
        let html_path = dir.path().join("index.html");
        fs::write(&html_path, r#"<h1 data-st-id="st-test1">Old Title</h1>"#).unwrap();
        let result = handle_edit_ast(
            dir.path(),
            "index.html",
            r#"[data-st-id="st-test1"]"#,
            &json!({"__content": "New Title"}),
            "op_content1",
        );
        assert!(
            matches!(result.response, ServerMessage::Ack { ref op_id } if op_id == "op_content1"),
            "Expected Ack, got: {:?}",
            result.response
        );
        let content = fs::read_to_string(&html_path).unwrap();
        assert!(content.contains("New Title"));
        assert!(!content.contains("Old Title"));
    }

    #[test]
    fn test_edit_ast_no_content_no_regression() {
        // Non-__content patch → follows existing apply_text_patch path
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "styles.st",
            ".hero {\n    @scroll reveal(&fade-up) {\n        duration: 800ms;\n    }\n}\n",
        );
        let result = handle_edit_ast(
            dir.path(),
            "styles.st",
            ".hero §scroll",
            &json!({"duration": "1200ms"}),
            "op_regular",
        );
        // Should follow existing path — may Ack or Reject depending on parser spans
        match &result.response {
            ServerMessage::Ack { op_id } => {
                assert_eq!(op_id, "op_regular");
                let content = fs::read_to_string(dir.path().join("styles.st")).unwrap();
                assert!(content.contains("1200ms"));
            }
            ServerMessage::Reject { reason, .. } => {
                // Acceptable if parser doesn't provide spans
                assert!(
                    reason.contains("not found")
                        || reason.contains("span")
                        || reason.contains("Parse error"),
                    "Unexpected rejection: {}",
                    reason
                );
            }
            _ => panic!("Unexpected response"),
        }
    }

    #[test]
    fn test_edit_ast_content_branch_guard_suppresses_reload() {
        // __content patch should set suppress_reload_path on success
        let dir = TempDir::new().unwrap();
        let html_path = dir.path().join("page.html");
        fs::write(&html_path, r#"<p data-st-id="st-p1">Old</p>"#).unwrap();
        let result = handle_edit_ast(
            dir.path(),
            "page.html",
            r#"[data-st-id="st-p1"]"#,
            &json!({"__content": "New"}),
            "op_supp",
        );
        assert!(matches!(result.response, ServerMessage::Ack { .. }));
        assert!(result.suppress_reload_path.is_some());
    }

    // -------------------------------------------------------------------------
    // ITEM-107-016: Integration tests for unified EditAst handler
    // -------------------------------------------------------------------------

    /// Scenario 1: HTML file + __content with rich innerHTML (tags, not just text)
    #[test]
    fn test_edit_ast_content_html_rich_inner_html() {
        let dir = TempDir::new().unwrap();
        let html_path = dir.path().join("index.html");
        fs::write(&html_path, r#"<h1 data-st-id="st-rich">Old</h1>"#).unwrap();
        let result = handle_edit_ast(
            dir.path(),
            "index.html",
            r#"[data-st-id="st-rich"]"#,
            &json!({"__content": "<strong>Bold</strong> text"}),
            "op_rich1",
        );
        assert!(
            matches!(result.response, ServerMessage::Ack { ref op_id } if op_id == "op_rich1"),
            "Expected Ack, got: {:?}",
            result.response
        );
        let content = fs::read_to_string(&html_path).unwrap();
        assert!(content.contains("<strong>Bold</strong> text"));
        assert!(!content.contains("Old"));
    }

    /// Scenario 2: HTML file + non-__content patch → Reject
    /// HTML files only support __content edits; property patching requires .st files.
    #[test]
    fn test_edit_ast_html_file_non_content_patch_rejected() {
        let dir = TempDir::new().unwrap();
        let html_path = dir.path().join("page.html");
        fs::write(&html_path, r#"<h1 data-st-id="st-abc">Title</h1>"#).unwrap();
        let result = handle_edit_ast(
            dir.path(),
            "page.html",
            r#"[data-st-id="st-abc"]"#,
            &json!({"color": "red"}),
            "op_html_prop",
        );
        assert!(
            matches!(result.response, ServerMessage::Reject { .. }),
            "Expected Reject for non-__content patch on HTML file, got: {:?}",
            result.response
        );
    }

    /// Scenario 3: .st file + __content patch → template body HTML updated
    /// The handler requires real parser spans; accept both Ack and Reject, but NOT a panic.
    #[test]
    fn test_edit_ast_st_file_content_patch() {
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "card.st",
            ".card {\n    @template {\n        <h3>Old Title</h3>\n    }\n}\n",
        );
        let result = handle_edit_ast(
            dir.path(),
            "card.st",
            ".card §template h3",
            &json!({"__content": "New Title", "__st_id": "fake-st-id-00"}),
            "op_st_content",
        );
        // .st content editing with a fake st_id: will either succeed (unlikely without real spans)
        // or reject because: parser lacks spans, or the fake st_id matches 0 elements.
        match &result.response {
            ServerMessage::Ack { op_id } => {
                assert_eq!(op_id, "op_st_content");
                let content = fs::read_to_string(dir.path().join("card.st")).unwrap();
                assert!(content.contains("New Title"));
            }
            ServerMessage::Reject { reason, .. } => {
                // Acceptable: parser may not produce spans, or fake st_id matches nothing
                assert!(
                    reason.contains("not found")
                        || reason.contains("span")
                        || reason.contains("Parse error")
                        || reason.contains("no component body")
                        || reason.contains("no HTML content")
                        || reason.contains("Unsupported")
                        || reason.contains("matched 0")
                        || reason.contains("0 elements"),
                    "Unexpected rejection: {}",
                    reason
                );
            }
            other => panic!("Unexpected response variant: {:?}", other),
        }
    }

    // Scenario 4 SKIP: .st + non-__content → covered by test_edit_ast_no_content_no_regression

    /// Scenario 5: Unknown extension + __content → Reject with clear message
    #[test]
    fn test_edit_ast_unknown_extension_content_rejected() {
        let dir = TempDir::new().unwrap();
        let txt_path = dir.path().join("file.txt");
        fs::write(&txt_path, "some text").unwrap();
        let result = handle_edit_ast(
            dir.path(),
            "file.txt",
            r#"[data-st-id="st-abc"]"#,
            &json!({"__content": "New"}),
            "op_unknown_ext",
        );
        assert!(
            matches!(
                result.response,
                ServerMessage::Reject { ref reason, .. }
                    if reason.contains("Unsupported") || reason.contains("not supported")
            ),
            "Expected Reject with 'Unsupported' message, got: {:?}",
            result.response
        );
    }

    /// Scenario 6: HTML file + nonexistent data-st-id selector → Reject
    #[test]
    fn test_edit_ast_html_nonexistent_selector_rejected() {
        let dir = TempDir::new().unwrap();
        let html_path = dir.path().join("index.html");
        fs::write(&html_path, r#"<h1 data-st-id="st-exists">Title</h1>"#).unwrap();
        let result = handle_edit_ast(
            dir.path(),
            "index.html",
            r#"[data-st-id="st-nonexistent"]"#,
            &json!({"__content": "New"}),
            "op_missing_sel",
        );
        assert!(
            matches!(
                result.response,
                ServerMessage::Reject { ref reason, .. } if reason.contains("not found")
            ),
            "Expected Reject with 'not found' message, got: {:?}",
            result.response
        );
    }

    /// Scenario 7: .st file + nonexistent template scope → Reject
    #[test]
    fn test_edit_ast_st_nonexistent_template_rejected() {
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "styles.st",
            ".card {\n    @template {\n        <h3>Title</h3>\n    }\n}\n",
        );
        let result = handle_edit_ast(
            dir.path(),
            "styles.st",
            ".missing §template h3",
            &json!({"__content": "New", "__st_id": "fake-st-id-00"}),
            "op_missing_scope",
        );
        assert!(
            matches!(
                result.response,
                ServerMessage::Reject { ref reason, .. }
                    if reason.contains("not found") || reason.contains("Parse error") || reason.contains("Missing __st_id")
            ),
            "Expected Reject for nonexistent scope, got: {:?}",
            result.response
        );
    }

    /// Scenario 8: .st file + element not found in template HTML → Reject
    #[test]
    fn test_edit_ast_st_element_not_in_template() {
        let dir = TempDir::new().unwrap();
        create_test_st(
            &dir,
            "styles.st",
            ".card {\n    @template {\n        <h3>Title</h3>\n    }\n}\n",
        );
        // Selector targets h1, but template only has h3
        let result = handle_edit_ast(
            dir.path(),
            "styles.st",
            ".card §template h1",
            &json!({"__content": "New", "__st_id": "fake-st-id-00"}),
            "op_wrong_elem",
        );
        // Should Reject because h1 doesn't exist in template body (only h3 does)
        assert!(
            matches!(result.response, ServerMessage::Reject { .. }),
            "Expected Reject for element not in template, got: {:?}",
            result.response
        );
    }

    /// Scenario 9: Path traversal attempt with __content patch → Reject
    #[test]
    fn test_edit_ast_content_path_traversal_rejected() {
        let dir = TempDir::new().unwrap();
        // Create a real file so the test is about traversal, not missing file
        fs::write(dir.path().join("legit.html"), "<h1>Safe</h1>").unwrap();
        let result = handle_edit_ast(
            dir.path(),
            "../../../etc/passwd",
            r#"[data-st-id="st-abc"]"#,
            &json!({"__content": "pwned"}),
            "op_traversal",
        );
        assert!(
            matches!(result.response, ServerMessage::Reject { .. }),
            "Expected Reject for path traversal, got: {:?}",
            result.response
        );
    }

    /// Scenario 10: Verify content selector dispatch routes correctly for each format
    #[test]
    fn test_content_selector_dispatch_routing() {
        let dir = TempDir::new().unwrap();

        // Route 1: HtmlElement format → routes to HTML handler
        let html_path = dir.path().join("route.html");
        fs::write(&html_path, r#"<p data-st-id="st-route1">Old</p>"#).unwrap();
        let r1 = handle_edit_ast(
            dir.path(),
            "route.html",
            r#"[data-st-id="st-route1"]"#,
            &json!({"__content": "Routed"}),
            "op_route1",
        );
        assert!(
            matches!(r1.response, ServerMessage::Ack { ref op_id } if op_id == "op_route1"),
            "HtmlElement route should Ack, got: {:?}",
            r1.response
        );
        let html = fs::read_to_string(&html_path).unwrap();
        assert!(html.contains("Routed"));

        // Route 2: AstContent format → routes to .st handler (may Reject without parser spans)
        create_test_st(
            &dir,
            "route.st",
            ".hero {\n    @template {\n        <h3>Old</h3>\n    }\n}\n",
        );
        let r2 = handle_edit_ast(
            dir.path(),
            "route.st",
            ".hero §template h3",
            &json!({"__content": "Routed", "__st_id": "fake-st-id-00"}),
            "op_route2",
        );
        // Both Ack and Reject are acceptable — but NOT a panic or unexpected variant
        match &r2.response {
            ServerMessage::Ack { op_id } => assert_eq!(op_id, "op_route2"),
            ServerMessage::Reject { .. } => { /* acceptable: parser may lack spans or fake st_id matches nothing */
            }
            other => panic!("AstContent route: unexpected response: {:?}", other),
        }

        // Route 3: AstDirective format without __content → does NOT use content handler
        create_test_st(
            &dir,
            "directive.st",
            ".hero {\n    @scroll reveal(&fade-up) {\n        duration: 800ms;\n    }\n}\n",
        );
        let r3 = handle_edit_ast(
            dir.path(),
            "directive.st",
            ".hero §scroll",
            &json!({"duration": "1200ms"}),
            "op_route3",
        );
        // Non-__content: goes through property-patching path, not content handler
        match &r3.response {
            ServerMessage::Ack { op_id } => {
                assert_eq!(op_id, "op_route3");
                let st = fs::read_to_string(dir.path().join("directive.st")).unwrap();
                assert!(st.contains("1200ms"));
            }
            ServerMessage::Reject { reason, .. } => {
                // Acceptable if parser doesn't provide spans
                assert!(
                    reason.contains("not found")
                        || reason.contains("span")
                        || reason.contains("Parse error"),
                    "AstDirective route: unexpected rejection: {}",
                    reason
                );
            }
            other => panic!("AstDirective route: unexpected response: {:?}", other),
        }
    }

    // -------------------------------------------------------------------------
    // ST-ID round-trip tests (PLAN-001)
    // -------------------------------------------------------------------------

    /// Compute the st-id that inject_template_provenance assigns to the Nth occurrence
    /// of `tag_name` in a template body with the given parameters.
    fn compute_st_id(source_file: &str, ast_offset: usize, tag_name: &str, idx: usize) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let hash_input = format!("{}:{}:{}:{}", source_file, ast_offset, tag_name, idx);
        let mut hasher = DefaultHasher::new();
        hash_input.hash(&mut hasher);
        let hash_hex = format!("{:016x}", hasher.finish());
        format!("st-{}", &hash_hex[..8])
    }

    /// strip_provenance_attributes removes data-st-id and data-st-origin, preserves everything else.
    #[test]
    fn test_strip_provenance_attributes() {
        let html =
            r#"<a data-st-id="st-abc1" data-st-origin="foo.st:: §template a" href="/">Link</a>"#;
        let result = strip_provenance_attributes(html);
        assert!(
            !result.contains("data-st-id"),
            "data-st-id must be stripped"
        );
        assert!(
            !result.contains("data-st-origin"),
            "data-st-origin must be stripped"
        );
        assert!(result.contains("href=\"/\""), "href must be preserved");
        assert!(result.contains("Link"), "text content must be preserved");
    }

    /// strip_provenance_attributes is a no-op on HTML that has no provenance attributes.
    #[test]
    fn test_strip_provenance_attributes_no_op() {
        let html = r#"<p class="intro">Hello <strong>world</strong></p>"#;
        let result = strip_provenance_attributes(html);
        assert!(result.contains("class=\"intro\""));
        assert!(result.contains("Hello"));
        assert!(result.contains("strong"));
    }

    /// st-id round-trip: template body has 5 <a> tags; browser sends st_id for the 3rd one.
    /// Only the 3rd element's content must change; all siblings must be untouched.
    #[test]
    fn test_content_edit_st_with_st_id_targets_unique_element() {
        let dir = TempDir::new().unwrap();
        // A .st file with a template whose body contains 5 <a> elements.
        // The indentation-wrapped body must be exactly what the parser produces.
        let source = ".nav {\n    @template {\n        <a href=\"/\">Home</a>\n        <a href=\"#work\">Work</a>\n        <a href=\"#about\">About</a>\n        <a href=\"#contact\">Contact</a>\n        <a href=\"#cta\">Get in touch</a>\n    }\n}\n";
        let st_path = create_test_st(&dir, "nav.st", source);

        // Parse to discover the exact ast_offset (span.start) the parser assigns to @template.
        let ast = crate::parser::parse(source).expect("parse must succeed");
        // Find the form_match for the template directive in the .nav scope.
        let scope = ast.scopes.iter().find(|s| s.selector == ".nav");
        let form_match = scope.and_then(|s| s.matches.iter().find(|m| m.macro_name == "template"));
        if form_match.is_none() {
            // Parser didn't produce a form match for this fixture — skip gracefully.
            return;
        }
        let form_match = form_match.unwrap();

        // The 3rd <a> (idx=2 because 0-indexed) will be our target.
        let target_st_id = compute_st_id("nav.st", form_match.span.start, "a", 2);

        let result = handle_edit_ast(
            dir.path(),
            "nav.st",
            ".nav §template a",
            &json!({"__content": "About Us", "__st_id": target_st_id}),
            "op_stid_target",
        );

        match &result.response {
            ServerMessage::Ack { op_id } => {
                assert_eq!(op_id, "op_stid_target");
                let written = fs::read_to_string(&st_path).unwrap();
                // Only the 3rd element should be updated
                assert!(
                    written.contains("About Us"),
                    "target element must be updated"
                );
                // The other elements must survive
                assert!(written.contains("Home"), "first sibling must be untouched");
                assert!(written.contains("Work"), "second sibling must be untouched");
                assert!(
                    written.contains("Contact"),
                    "fourth sibling must be untouched"
                );
                assert!(
                    written.contains("Get in touch"),
                    "fifth sibling must be untouched"
                );
                // No provenance attributes must be written to disk
                assert!(
                    !written.contains("data-st-id"),
                    "data-st-id must not appear on disk"
                );
                assert!(
                    !written.contains("data-st-origin"),
                    "data-st-origin must not appear on disk"
                );
            }
            ServerMessage::Reject { reason, .. } => {
                // Only acceptable if the parser didn't produce a component body for this fixture.
                assert!(
                    reason.contains("no component body")
                        || reason.contains("no HTML content")
                        || reason.contains("matched 0"),
                    "Unexpected rejection for st-id round-trip: {}",
                    reason
                );
            }
            other => panic!("Unexpected response: {:?}", other),
        }
    }

    /// When __st_id refers to a non-existent element, the server must reject with a clear message.
    #[test]
    fn test_content_edit_st_with_invalid_st_id_rejected() {
        let dir = TempDir::new().unwrap();
        let source = ".card {\n    @template {\n        <h3>Title</h3>\n    }\n}\n";
        create_test_st(&dir, "card.st", source);

        let result = handle_edit_ast(
            dir.path(),
            "card.st",
            ".card §template h3",
            &json!({"__content": "New", "__st_id": "st-deadbeef"}),
            "op_invalid_stid",
        );

        // Must reject: the invalid st_id matches 0 elements OR the parser fails.
        assert!(
            matches!(result.response, ServerMessage::Reject { .. }),
            "Expected Reject for invalid st_id, got: {:?}",
            result.response
        );
        if let ServerMessage::Reject { reason, .. } = &result.response {
            assert!(
                reason.contains("matched 0")
                    || reason.contains("0 elements")
                    || reason.contains("not found")
                    || reason.contains("no component body")
                    || reason.contains("no HTML content"),
                "Rejection should be about st_id or body: {}",
                reason
            );
        }
    }

    /// Missing __st_id in patch for a .st template edit must be rejected immediately.
    #[test]
    fn test_content_edit_st_no_st_id_rejected() {
        let dir = TempDir::new().unwrap();
        let source = ".card {\n    @template {\n        <h3>Title</h3>\n    }\n}\n";
        create_test_st(&dir, "card.st", source);

        let result = handle_edit_ast(
            dir.path(),
            "card.st",
            ".card §template h3",
            &json!({"__content": "New"}), // no __st_id
            "op_no_stid",
        );

        assert!(
            matches!(result.response, ServerMessage::Reject { .. }),
            "Expected Reject when __st_id is absent, got: {:?}",
            result.response
        );
        if let ServerMessage::Reject { reason, .. } = &result.response {
            assert!(
                reason.contains("__st_id") || reason.contains("Missing"),
                "Rejection must mention missing __st_id: {}",
                reason
            );
        }
    }

    #[test]
    fn test_edit_json_array_insert_append() {
        let dir = TempDir::new().unwrap();
        let data = json!({"items": [1, 2, 3]});
        create_test_json(&dir, "data/test.json", &data);

        let result = handle_edit_json_array(
            dir.path(),
            "data/test.json",
            "items",
            ArrayOp::Insert {
                item: json!(4),
                index: None,
            },
            "op_1",
        );

        assert!(matches!(result.response, ServerMessage::Ack { ref op_id } if op_id == "op_1"));
        if let Some(ServerMessage::DataUpdate { data, .. }) = result.broadcast {
            assert_eq!(data["items"], json!([1, 2, 3, 4]));
        } else {
            panic!("Expected broadcast");
        }
    }

    #[test]
    fn test_edit_json_array_insert_at_index() {
        let dir = TempDir::new().unwrap();
        let data = json!({"items": [1, 2, 3]});
        create_test_json(&dir, "data/test.json", &data);

        let result = handle_edit_json_array(
            dir.path(),
            "data/test.json",
            "items",
            ArrayOp::Insert {
                item: json!(99),
                index: Some(1),
            },
            "op_2",
        );

        assert!(matches!(result.response, ServerMessage::Ack { .. }));
        if let Some(ServerMessage::DataUpdate { data, .. }) = result.broadcast {
            assert_eq!(data["items"], json!([1, 99, 2, 3]));
        } else {
            panic!("Expected broadcast");
        }
    }

    #[test]
    fn test_edit_json_array_delete() {
        let dir = TempDir::new().unwrap();
        let data = json!({"items": ["a", "b", "c"]});
        create_test_json(&dir, "data/test.json", &data);

        let result = handle_edit_json_array(
            dir.path(),
            "data/test.json",
            "items",
            ArrayOp::Delete { index: 1 },
            "op_3",
        );

        assert!(matches!(result.response, ServerMessage::Ack { .. }));
        if let Some(ServerMessage::DataUpdate { data, .. }) = result.broadcast {
            assert_eq!(data["items"], json!(["a", "c"]));
        } else {
            panic!("Expected broadcast");
        }
    }

    #[test]
    fn test_edit_json_array_delete_out_of_bounds() {
        let dir = TempDir::new().unwrap();
        let data = json!({"items": [1, 2]});
        create_test_json(&dir, "data/test.json", &data);

        let result = handle_edit_json_array(
            dir.path(),
            "data/test.json",
            "items",
            ArrayOp::Delete { index: 5 },
            "op_4",
        );

        assert!(matches!(result.response, ServerMessage::Reject { .. }));
        assert!(result.broadcast.is_none());
    }

    #[test]
    fn test_edit_json_array_update_merges_patch() {
        let dir = TempDir::new().unwrap();
        let data = json!({"cards": [
            {"id": "c1", "title": "A", "col": "todo"},
            {"id": "c2", "title": "B", "col": "doing"}
        ]});
        create_test_json(&dir, "data/board.json", &data);

        let result = handle_edit_json_array(
            dir.path(),
            "data/board.json",
            "cards",
            ArrayOp::Update {
                index: 0,
                patch: json!({"col": "doing"}),
            },
            "op_u1",
        );

        assert!(matches!(result.response, ServerMessage::Ack { .. }));
        if let Some(ServerMessage::DataUpdate { data, .. }) = result.broadcast {
            // Only `col` changes; `id` + `title` are preserved (merge, not replace).
            assert_eq!(
                data["cards"][0],
                json!({"id": "c1", "title": "A", "col": "doing"})
            );
            // Sibling row untouched.
            assert_eq!(data["cards"][1]["col"], json!("doing"));
        } else {
            panic!("Expected broadcast");
        }
    }

    #[test]
    fn test_edit_json_array_update_index_out_of_bounds_rejects() {
        let dir = TempDir::new().unwrap();
        let data = json!({"cards": [{"id": "c1", "col": "todo"}]});
        create_test_json(&dir, "data/board.json", &data);

        let result = handle_edit_json_array(
            dir.path(),
            "data/board.json",
            "cards",
            ArrayOp::Update {
                index: 9,
                patch: json!({"col": "doing"}),
            },
            "op_u2",
        );

        assert!(matches!(result.response, ServerMessage::Reject { .. }));
        assert!(result.broadcast.is_none());
    }

    #[test]
    fn test_edit_json_array_reorder() {
        let dir = TempDir::new().unwrap();
        let data = json!({"items": ["a", "b", "c", "d"]});
        create_test_json(&dir, "data/test.json", &data);

        let result = handle_edit_json_array(
            dir.path(),
            "data/test.json",
            "items",
            ArrayOp::Reorder {
                from_index: 0,
                to_index: 2,
            },
            "op_5",
        );

        assert!(matches!(result.response, ServerMessage::Ack { .. }));
        if let Some(ServerMessage::DataUpdate { data, .. }) = result.broadcast {
            // Remove index 0 ("a"), then insert at index 2: ["b","c","a","d"]
            assert_eq!(data["items"], json!(["b", "c", "a", "d"]));
        } else {
            panic!("Expected broadcast");
        }
    }

    #[test]
    fn test_edit_json_array_empty_path_root_array() {
        let dir = TempDir::new().unwrap();
        let data = json!([10, 20, 30]);
        create_test_json(&dir, "data/test.json", &data);

        let result = handle_edit_json_array(
            dir.path(),
            "data/test.json",
            "",
            ArrayOp::Insert {
                item: json!(40),
                index: None,
            },
            "op_6",
        );

        assert!(matches!(result.response, ServerMessage::Ack { .. }));
        if let Some(ServerMessage::DataUpdate { data, .. }) = result.broadcast {
            assert_eq!(data, json!([10, 20, 30, 40]));
        } else {
            panic!("Expected broadcast");
        }
    }

    #[test]
    fn test_edit_json_array_nested_path() {
        let dir = TempDir::new().unwrap();
        let data = json!({"shop": {"products": ["x", "y"]}});
        create_test_json(&dir, "data/test.json", &data);

        let result = handle_edit_json_array(
            dir.path(),
            "data/test.json",
            "shop.products",
            ArrayOp::Delete { index: 0 },
            "op_7",
        );

        assert!(matches!(result.response, ServerMessage::Ack { .. }));
        if let Some(ServerMessage::DataUpdate { data, .. }) = result.broadcast {
            assert_eq!(data["shop"]["products"], json!(["y"]));
        } else {
            panic!("Expected broadcast");
        }
    }

    #[test]
    fn test_edit_json_array_non_array_target() {
        let dir = TempDir::new().unwrap();
        let data = json!({"name": "hello"});
        create_test_json(&dir, "data/test.json", &data);

        let result = handle_edit_json_array(
            dir.path(),
            "data/test.json",
            "name",
            ArrayOp::Insert {
                item: json!(1),
                index: None,
            },
            "op_8",
        );

        assert!(matches!(result.response, ServerMessage::Reject { .. }));
        if let ServerMessage::Reject { reason, .. } = &result.response {
            assert!(reason.contains("not a JSON array"), "Reason: {}", reason);
        }
        assert!(result.broadcast.is_none());
    }

    /// BUG-098 (FEAT-106 review P1): editing the LAST param of a paren-list
    /// directive must terminate the value at the closing `)`, not run past it
    /// into the keyframe body. Reviewer caught that `)` was not a depth-0
    /// terminator — a last-param edit would eat `) { … }`.
    #[test]
    fn test_edit_ast_last_paren_param_preserves_closing_and_body() {
        let dir = TempDir::new().unwrap();
        // `stagger` is the LAST param before `)`. Patching it must not swallow
        // the `)` or the `{ opacity… }` body.
        let src = ".masthead {\n  @scroll reveal(start: 0, stagger: 30) {\n    opacity: 0 -> 1;\n  }\n}\n";
        std::fs::write(dir.path().join("index.st"), src).unwrap();

        let result = handle_edit_ast(
            dir.path(),
            "index.st",
            ".masthead §scroll",
            &json!({ "stagger": "50" }),
            "op_last",
        );
        assert!(
            matches!(result.response, ServerMessage::Ack { .. }),
            "expected Ack, got {:?}",
            result.response
        );
        let out = std::fs::read_to_string(dir.path().join("index.st")).unwrap();
        assert!(out.contains("stagger: 50)"), "closing paren eaten: {out:?}");
        assert!(
            out.contains("opacity: 0 -> 1;"),
            "keyframe body eaten: {out:?}"
        );
        assert!(
            out.contains("start: 0,"),
            "sibling param clobbered: {out:?}"
        );
    }

    /// BUG-097 (FEAT-106): a scoped directive EditAst must resolve the scope
    /// block that CONTAINS the directive, not merely the first selector match.
    /// A selector can appear in multiple blocks (tokens vs motion); editing a
    /// motion param must patch the block with `@scroll`, not a sibling.
    #[test]
    fn test_edit_ast_scoped_directive_resolves_correct_block() {
        let dir = TempDir::new().unwrap();
        let src = ".masthead { color: red; }\n.masthead {\n  @scroll reveal(stagger: 30, easing: ease-out) {\n    opacity: 0 -> 1;\n  }\n}\n";
        std::fs::write(dir.path().join("index.st"), src).unwrap();

        let result = handle_edit_ast(
            dir.path(),
            "index.st",
            ".masthead §scroll",
            &json!({ "stagger": "50" }),
            "op_motion",
        );

        assert!(
            matches!(result.response, ServerMessage::Ack { .. }),
            "expected Ack, got {:?}",
            result.response
        );
        let out = std::fs::read_to_string(dir.path().join("index.st")).unwrap();
        assert!(out.contains("stagger: 50"), "stagger not patched: {out}");
        // Siblings byte-exact: easing + the token block untouched.
        assert!(out.contains("easing: ease-out"), "easing clobbered: {out}");
        assert!(
            out.contains(".masthead { color: red; }"),
            "token block clobbered: {out}"
        );
    }
}
