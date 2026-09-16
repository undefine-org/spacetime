//! Dev Edit Protocol Message Types
//!
//! Defines the WebSocket protocol for real-time content editing via dev tools.
//! These messages are exchanged between the Spacetime dev server and browser clients
//! to support live editing of JSON data files, HTML content, and AST patches.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// =============================================================================
// Client → Server Messages
// =============================================================================

/// Messages sent from client to server for content editing
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Edit a JSON data file at a specific path
    ///
    /// Example: Edit `data/products.json` at path `[2].price` to set value `29.99`
    EditJson {
        /// Relative path to the JSON file (from site root)
        file: String,
        /// JSON path expression (e.g., `[2].price`, `products[0].name`)
        path: String,
        /// New value to set at the path
        value: Value,
        /// Client-generated operation ID for ack/reject tracking
        op_id: String,
    },

    /// Edit HTML element content (stub — not yet implemented)
    ///
    /// @deprecated Use `EditAst` with `__content` sentinel in the patch object instead.
    /// `EditHtml` is kept for backwards compatibility but will be removed in a future version.
    EditHtml {
        /// Relative path to the HTML file
        file: String,
        /// Element ID or selector to target
        element_id: String,
        /// New content for the element
        content: String,
        /// Client-generated operation ID
        op_id: String,
    },

    /// Patch the Spacetime AST (stub — not yet implemented)
    EditAst {
        /// Relative path to the .st file
        file: String,
        /// AST selector for the target node
        selector: String,
        /// Patch to apply to the AST node
        patch: Value,
        /// Client-generated operation ID
        op_id: String,
    },

    /// Edit a JSON array (insert, delete, reorder items)
    EditJsonArray {
        /// Relative path to the JSON file
        file: String,
        /// JSON path to the array (empty string = root array, "products" = nested)
        path: String,
        /// Array operation to perform
        op: ArrayOp,
        /// Client-generated operation ID
        op_id: String,
    },

    /// Inspect an element to get its context (selector, properties, state, bindings)
    InspectElement {
        /// CSS selector for the element to inspect
        selector: String,
        /// Optional file hint for faster lookup
        file_hint: Option<String>,
    },

    /// Inspect the STRUCTURE of a `.st` file's template(s): the whole Structure IR
    /// tree (PLAN-064 B2). Unlike `InspectElement` (one CSS-selected element), this
    /// returns the full element/hole/template node graph the shared
    /// `crate::introspect` producer builds — the SAME tree the MCP workbench
    /// navigator renders, so both live-coding hosts read one introspection
    /// substrate. `template` selects the entry template (default `"main"`).
    InspectStructure {
        /// Relative path to the `.st` file (from site root).
        file: String,
        /// Entry template name (defaults to `main` when absent).
        #[serde(default)]
        template: Option<String>,
    },

    /// Edit a theme token: replace the value of a `--name: value;` custom
    /// property declaration in a `.st` file (Stage-3 Wave-7). The token name is
    /// matched literally; brand integrity is structural (named tokens, not free
    /// CSS). The server validates the file path and that the token exists.
    EditToken {
        /// Relative path to the `.st` file containing the token
        file: String,
        /// Token name: a CSS custom property (`--unkn-accent`) by default, or a
        /// motion param key (`duration`) when `motion` is true.
        token: String,
        /// New value (without trailing `;`)
        value: String,
        /// Client-generated operation ID
        op_id: String,
        /// When true, edit a motion param `key: value` inside an `@reveal/@scroll`
        /// block (terminated by `,` or `)`) instead of a `--token: value;` decl.
        #[serde(default)]
        motion: bool,
        /// For motion edits: the 0-based occurrence index (within the file) of
        /// the `key:number` to replace, disambiguating multiple motion blocks
        /// that share a param key. Defaults to 0 (first).
        #[serde(default)]
        motion_index: usize,
    },
}

/// Operations on JSON arrays
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum ArrayOp {
    /// Insert an item (None index = append)
    Insert { item: Value, index: Option<usize> },
    /// Delete item by index
    Delete { index: usize },
    /// Move item from one index to another
    Reorder { from_index: usize, to_index: usize },
    /// Merge a partial patch into the object at `index` (in-place field update).
    /// `patch`'s keys overwrite the target object's keys; other fields are kept.
    /// Backs the collection `.update(idOrPredicate, patch)` mutator: a card
    /// changing column persists as `Update { index, patch: { col: "doing" } }`.
    Update { index: usize, patch: Value },
}

// =============================================================================
// Server → Client Messages
// =============================================================================

/// Messages sent from server to client in response to edits
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Broadcast updated data after a successful edit
    ///
    /// Sent to ALL connected clients so they can refresh their views.
    DataUpdate {
        /// Source file that was modified (relative path)
        source: String,
        /// The full updated data from the file
        data: Value,
    },

    /// Acknowledge a successful edit operation
    Ack {
        /// The op_id from the original edit request
        op_id: String,
    },

    /// Reject a failed edit operation
    Reject {
        /// The op_id from the original edit request
        op_id: String,
        /// Human-readable reason for rejection
        reason: String,
    },

    /// Return element context (properties, state machine, data bindings)
    ElementContext {
        /// CSS selector of the inspected element
        selector: String,
        /// Source files that define this element's properties
        source_files: Vec<String>,
        /// Grouped properties by section (animations, styles, etc.)
        sections: Vec<PropertySection>,
        /// Current state machine context if applicable
        state_machine: Option<StateMachineContext>,
        /// Data bindings applied to this element
        data_bindings: Vec<DataBindingContext>,
        /// True if element was not found
        not_found: bool,
    },

    /// The Structure IR tree for an inspected `.st` file (PLAN-064 B2). The flat,
    /// depth-tagged node list the shared `crate::introspect` producer builds — the
    /// SAME shape the MCP workbench navigator consumes. `nodes` is empty when the
    /// file has no matching entry template or fails to compile (with `error` set).
    Structure {
        /// Relative path of the inspected file.
        file: String,
        /// Entry template that was walked (the resolved name, e.g. `main`).
        template: String,
        /// The Structure IR nodes (serialized `crate::introspect::StructureNode`).
        nodes: Vec<crate::introspect::StructureNode>,
        /// A compile/lookup error, when the tree could not be built.
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
}

// =============================================================================
// Inspector Protocol Types
// =============================================================================

/// A section of properties grouped by source or category
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertySection {
    /// Label for this section (e.g., "Animations", "Styles", "Scroll Triggers")
    pub label: String,
    /// Source file where these properties are defined
    pub source_file: Option<String>,
    /// Line number in source file
    pub source_line: Option<u32>,
    /// Properties in this section
    pub properties: Vec<PropertyNode>,
}

/// A single property with its value and metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PropertyNode {
    /// Property name (e.g., "duration", "opacity", "easing")
    pub name: String,
    /// The value with classification and control hint
    pub value: ValueNode,
    /// Source file where this property is defined
    pub source_file: Option<String>,
    /// Line number in source file
    pub source_line: Option<u32>,
    /// Whether this property can be edited
    pub editable: bool,
}

/// A value with its raw form, parsed type, and UI control hint
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValueNode {
    /// Raw string value as written in source
    pub raw: String,
    /// Parsed value type for classification
    pub parsed: ValueType,
    /// UI control hint for editing
    pub control_hint: ControlHint,
    /// Whether this value can be edited
    pub editable: bool,
}

/// Parsed CSS value type for classification
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ValueType {
    /// Color value: #rrggbb, rgb(...), hsl(...), etc.
    Color(String),
    /// Duration value: Nms, Ns
    Duration { ms: f64 },
    /// Dimension value: Npx, Nem, N%, Nrem, Nvw, Nvh
    Dimension { value: f64, unit: String },
    /// Easing function: ease-*, cubic-bezier(...), linear
    Easing(String),
    /// Pure number
    Number(f64),
    /// String value
    Str(String),
    /// Identifier (unquoted string)
    Identifier(String),
    /// Unknown or unparseable value
    Unknown(String),
}

/// UI control hint for editing a value
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ControlHint {
    /// Use a color picker
    ColorPicker,
    /// Use a slider with min/max/step
    Slider { min: f64, max: f64, step: f64 },
    /// Use a duration input
    DurationInput,
    /// Use an easing curve picker
    EasingCurve,
    /// Use a text input
    TextInput,
    /// Use a select dropdown
    Select { options: Vec<String> },
    /// No specific control hint
    None,
}

/// State machine context for an element
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateMachineContext {
    /// Current state name
    pub current_state: String,
    /// All available states
    pub states: Vec<String>,
}

/// Data binding context for an element
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataBindingContext {
    /// Name of the data binding
    pub name: String,
    /// Type of data being bound
    pub data_type: String,
}

/// Classify a raw CSS value string into a ValueType and ControlHint
///
/// Best-effort classification:
/// - `#rrggbb` / `rgb(...)` / `hsl(...)` → Color + ColorPicker
/// - `Nms` / `Ns` → Duration + DurationInput
/// - `Npx` / `Nem` / `N%` / `Nrem` / `Nvw` / `Nvh` → Dimension + Slider
/// - `ease-*` / `cubic-bezier(...)` / `linear` → Easing + EasingCurve
/// - Pure numeric → Number + Slider
/// - Fallback → Unknown + TextInput
pub fn classify_value(raw: &str) -> (ValueType, ControlHint) {
    use crate::types::value_infer::{Inferred, infer_value_type};

    let trimmed = raw.trim();

    // Empty is not a value. Kept ahead of inference because the contract here is
    // an EMPTY payload (`Unknown("")`), not the trimmed input.
    if trimmed.is_empty() {
        return (ValueType::Unknown(String::new()), ControlHint::TextInput);
    }

    // FEAT-109 Tier 0. This function used to be an 86-line recognizer: a hex
    // scanner, an `rgb`/`hsl` prefix test, a `ms`/`s` suffix test, a unit list,
    // an `ease`/`cubic-bezier` prefix test. It was the second of FOUR
    // independent answers to "what kind of value is this?" in this tree, and
    // like all four it was written because the type had been discarded at
    // capture and had to be guessed back from the string.
    //
    // It disagreed with the others, measurably:
    //
    //   `#e8eef7ff`  (8-digit hex)   sync: NOT a colour · grammar + LSP: colour
    //   `#abcd`      (4-digit hex)   sync: NOT a colour · grammar + LSP: colour
    //   `#zzz`       (not hex at all) sync: A COLOUR — it counted characters
    //                                 without ever looking at them
    //   `oklch(...)` `color-mix(...)` sync: invisible — the prefix test knew
    //                                 only `rgb` and `hsl`
    //
    // Now one grammar decides, so the subsystems cannot drift apart again. That
    // property — not any single answer — is what Tier 0 buys, and
    // `tests/tier0_one_classifier_test.rs::sync_agrees_with_the_inferrer_on_every_probe`
    // is the gate that keeps it.
    match infer_value_type(trimmed) {
        Inferred::Scalar(ty) => scalar_to_value_type(&ty, trimmed),

        // A reference names a value supplied elsewhere (`var(--ink)`,
        // `$brand.ink`). There is no literal here to edit, so the admin gets a
        // text input — the same answer the old code reached for `var()`, now for
        // a reason rather than by falling off the end.
        //
        // When the admin can edit THROUGH a reference (resolving it, editing the
        // declaration it names), this mapping is the thing that changes.
        Inferred::Reference => (
            ValueType::Unknown(trimmed.to_string()),
            ControlHint::TextInput,
        ),

        // A TIE. Recognition refuses to pick, and it is right to: `100%` really
        // is both a length and a percentage, because the length grammar includes
        // percentage by design.
        //
        // But a tie is only a problem for a consumer if the candidates DISAGREE
        // about what to do. Here they frequently do not — `length` and
        // `percentage` are one editing affordance (a magnitude with a unit), so
        // the ambiguity that must stay visible in the TYPE system dissolves
        // entirely at the presentation layer.
        //
        // So: map every candidate, and if they all land on the same answer, take
        // it. That is not breaking the tie — it is observing that the tie does
        // not matter HERE. Where the candidates genuinely differ (a bare
        // identifier tying across `string`/`richtext`/`url`, which carry
        // different editors), no consensus exists and a text input is correct.
        Inferred::Ambiguous(candidates) => {
            let mut mapped = candidates.iter().map(|c| scalar_to_value_type(c, trimmed));
            match mapped.next() {
                Some(first) => {
                    let unanimous = mapped.all(|m| same_control(&m.1, &first.1));
                    if unanimous {
                        first
                    } else {
                        no_editable_literal(trimmed)
                    }
                }
                None => no_editable_literal(trimmed),
            }
        }

        // Recognition abstained: a compound value (`1px solid red`), a function
        // the scalar grammars do not model (`calc(...)`), or nonsense.
        Inferred::Unknown => (
            ValueType::Unknown(trimmed.to_string()),
            ControlHint::TextInput,
        ),
    }
}

/// The answer for a value with no editable literal behind it.
///
/// `Unknown` rather than `Str` deliberately, and it is worth being precise about
/// why, because the two look interchangeable. `ValueType::Str` means "this IS a
/// string value" — a claim. `Unknown` means "the admin has nothing better than a
/// text box for this" — an abstention. A bare identifier that ties across the
/// shape-scalars has not been shown to be a string; it has been shown to be
/// nothing in particular, and `test_classify_value_unknown_string` in this file
/// has asserted exactly that since before inference existed.
///
/// Preserving it matters beyond the test passing: `Str` would tell a future
/// consumer that the type question was ANSWERED, and the whole discipline of
/// this arc is that an abstention must stay legible as one.
fn no_editable_literal(trimmed: &str) -> (ValueType, ControlHint) {
    (
        ValueType::Unknown(trimmed.to_string()),
        ControlHint::TextInput,
    )
}

/// Do two control hints call for the same editor?
///
/// Compares the KIND, not the payload: two sliders are the same affordance even
/// if a future caller gives them different bounds. Used to decide whether an
/// ambiguous inference matters at the presentation layer.
fn same_control(a: &ControlHint, b: &ControlHint) -> bool {
    use ControlHint::*;
    matches!(
        (a, b),
        (ColorPicker, ColorPicker)
            | (Slider { .. }, Slider { .. })
            | (DurationInput, DurationInput)
            | (EasingCurve, EasingCurve)
            | (TextInput, TextInput)
            | (Select { .. }, Select { .. })
            | (None, None)
    )
}

/// Map an inferred scalar onto the sync layer's `ValueType` + editing control.
///
/// This is a PRESENTATION decision, which is why it lives here and not beside
/// the inferrer: the same `length` is a slider in the admin, a swatchless span
/// in the LSP, and a number-plus-unit in the protocol. What the value IS is one
/// question with one answer; what to DO about it is per-consumer.
///
/// A scalar with no arm falls to `Str`/`TextInput` rather than being invented
/// into an existing variant — an `angle` is not a `Dimension` just because both
/// are a number and a unit, and pretending otherwise is how a slider ends up
/// editing degrees as if they were pixels.
fn scalar_to_value_type(ty: &str, trimmed: &str) -> (ValueType, ControlHint) {
    match ty {
        "color" => (
            ValueType::Color(trimmed.to_string()),
            ControlHint::ColorPicker,
        ),

        "duration" | "time" => match crate::syntax::conversions::parse_duration_with_unit(trimmed) {
            Some((ms, _unit)) => (
                ValueType::Duration { ms: ms as f64 },
                ControlHint::DurationInput,
            ),
            // Recognised as a duration but not splittable into a number and a
            // unit: the grammar and the converter disagree, which is a bug in one
            // of them. Degrade rather than guess, and stay visible as Unknown.
            None => (
                ValueType::Unknown(trimmed.to_string()),
                ControlHint::TextInput,
            ),
        },

        // `length` and `percentage` are one editing affordance: a magnitude with
        // a unit. The grammar keeps them apart because `100%` is legal in places
        // `100px` is not; the admin does not care.
        "length" | "percentage" => {
            match crate::syntax::conversions::parse_length_with_unit(trimmed) {
                Some((value, unit)) => (
                    ValueType::Dimension {
                        value,
                        unit: unit.to_string(),
                    },
                    ControlHint::Slider {
                        min: 0.0,
                        max: 100.0,
                        step: 1.0,
                    },
                ),
                None => (
                    ValueType::Unknown(trimmed.to_string()),
                    ControlHint::TextInput,
                ),
            }
        }

        "easing" => (
            ValueType::Easing(trimmed.to_string()),
            ControlHint::EasingCurve,
        ),

        "number" => match trimmed.parse::<f64>() {
            Ok(val) => (
                ValueType::Number(val),
                ControlHint::Slider {
                    min: 0.0,
                    max: 100.0,
                    step: 1.0,
                },
            ),
            Err(_) => (
                ValueType::Unknown(trimmed.to_string()),
                ControlHint::TextInput,
            ),
        },

        // A scalar the sync layer has no variant for (`angle` today). Real
        // inference, no lossless home — the same gap `infer_captured_value`
        // documents, and it closes the same way: one general carrier instead of
        // a variant per scalar.
        _ => no_editable_literal(trimmed),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_client_message_edit_json() {
        let msg: ClientMessage = serde_json::from_str(
            r#"{"type":"EditJson","file":"data/products.json","path":"[2].price","value":29.99,"op_id":"op_1"}"#,
        )
        .unwrap();

        match msg {
            ClientMessage::EditJson {
                file,
                path,
                value,
                op_id,
            } => {
                assert_eq!(file, "data/products.json");
                assert_eq!(path, "[2].price");
                assert_eq!(value, json!(29.99));
                assert_eq!(op_id, "op_1");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_client_message_edit_html() {
        let msg: ClientMessage = serde_json::from_str(
            r#"{"type":"EditHtml","file":"index.html","element_id":"hero-title","content":"New Title","op_id":"op_2"}"#,
        )
        .unwrap();

        match msg {
            ClientMessage::EditHtml {
                file,
                element_id,
                content,
                op_id,
            } => {
                assert_eq!(file, "index.html");
                assert_eq!(element_id, "hero-title");
                assert_eq!(content, "New Title");
                assert_eq!(op_id, "op_2");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_client_message_edit_ast() {
        let msg: ClientMessage = serde_json::from_str(
            r#"{"type":"EditAst","file":"styles.st","selector":".hero @scroll","patch":{"duration":"1200ms"},"op_id":"op_3"}"#,
        )
        .unwrap();

        match msg {
            ClientMessage::EditAst {
                file,
                selector,
                patch,
                op_id,
            } => {
                assert_eq!(file, "styles.st");
                assert_eq!(selector, ".hero @scroll");
                assert_eq!(patch["duration"], "1200ms");
                assert_eq!(op_id, "op_3");
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_server_message_data_update() {
        let msg = ServerMessage::DataUpdate {
            source: "data/products.json".to_string(),
            data: json!([{"id": "1", "name": "Test", "price": 29.99}]),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"DataUpdate""#));
        assert!(json.contains(r#""source":"data/products.json""#));
    }

    #[test]
    fn test_server_message_ack() {
        let msg = ServerMessage::Ack {
            op_id: "op_1".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Ack""#));
        assert!(json.contains(r#""op_id":"op_1""#));
    }

    #[test]
    fn test_server_message_reject() {
        let msg = ServerMessage::Reject {
            op_id: "op_1".to_string(),
            reason: "File not found".to_string(),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Reject""#));
        assert!(json.contains(r#""reason":"File not found""#));
    }

    #[test]
    fn test_roundtrip_serialization() {
        let original = ClientMessage::EditJson {
            file: "data/test.json".to_string(),
            path: "[0].name".to_string(),
            value: json!("Updated"),
            op_id: "op_42".to_string(),
        };

        let json = serde_json::to_string(&original).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();

        match parsed {
            ClientMessage::EditJson {
                file,
                path,
                value,
                op_id,
            } => {
                assert_eq!(file, "data/test.json");
                assert_eq!(path, "[0].name");
                assert_eq!(value, json!("Updated"));
                assert_eq!(op_id, "op_42");
            }
            _ => panic!("Wrong variant after roundtrip"),
        }
    }

    #[test]
    fn test_classify_value_color_hex() {
        let (vtype, hint) = classify_value("#ff0000");
        match vtype {
            ValueType::Color(c) => assert_eq!(c, "#ff0000"),
            _ => panic!("Expected Color variant"),
        }
        match hint {
            ControlHint::ColorPicker => {}
            _ => panic!("Expected ColorPicker hint"),
        }
    }

    #[test]
    fn test_classify_value_color_rgb() {
        let (vtype, hint) = classify_value("rgb(255, 0, 0)");
        match vtype {
            ValueType::Color(_) => {}
            _ => panic!("Expected Color variant"),
        }
        match hint {
            ControlHint::ColorPicker => {}
            _ => panic!("Expected ColorPicker hint"),
        }
    }

    #[test]
    fn test_classify_value_duration_ms() {
        let (vtype, hint) = classify_value("1200ms");
        match vtype {
            ValueType::Duration { ms } => assert_eq!(ms, 1200.0),
            _ => panic!("Expected Duration variant"),
        }
        match hint {
            ControlHint::DurationInput => {}
            _ => panic!("Expected DurationInput hint"),
        }
    }

    #[test]
    fn test_classify_value_duration_s() {
        let (vtype, hint) = classify_value("1.5s");
        match vtype {
            ValueType::Duration { ms } => assert_eq!(ms, 1500.0),
            _ => panic!("Expected Duration variant"),
        }
        match hint {
            ControlHint::DurationInput => {}
            _ => panic!("Expected DurationInput hint"),
        }
    }

    #[test]
    fn test_classify_value_dimension_px() {
        let (vtype, hint) = classify_value("40px");
        match vtype {
            ValueType::Dimension { value, unit } => {
                assert_eq!(value, 40.0);
                assert_eq!(unit, "px");
            }
            _ => panic!("Expected Dimension variant"),
        }
        match hint {
            ControlHint::Slider { .. } => {}
            _ => panic!("Expected Slider hint"),
        }
    }

    #[test]
    fn test_classify_value_dimension_em() {
        let (vtype, hint) = classify_value("2.5em");
        match vtype {
            ValueType::Dimension { value, unit } => {
                assert_eq!(value, 2.5);
                assert_eq!(unit, "em");
            }
            _ => panic!("Expected Dimension variant"),
        }
        match hint {
            ControlHint::Slider { .. } => {}
            _ => panic!("Expected Slider hint"),
        }
    }

    #[test]
    fn test_classify_value_dimension_percent() {
        let (vtype, hint) = classify_value("50%");
        match vtype {
            ValueType::Dimension { value, unit } => {
                assert_eq!(value, 50.0);
                assert_eq!(unit, "%");
            }
            _ => panic!("Expected Dimension variant"),
        }
        match hint {
            ControlHint::Slider { .. } => {}
            _ => panic!("Expected Slider hint"),
        }
    }

    #[test]
    fn test_classify_value_easing_ease_out() {
        let (vtype, hint) = classify_value("ease-out");
        match vtype {
            ValueType::Easing(e) => assert_eq!(e, "ease-out"),
            _ => panic!("Expected Easing variant"),
        }
        match hint {
            ControlHint::EasingCurve => {}
            _ => panic!("Expected EasingCurve hint"),
        }
    }

    #[test]
    fn test_classify_value_easing_cubic_bezier() {
        let (vtype, hint) = classify_value("cubic-bezier(0.25, 0.1, 0.25, 1)");
        match vtype {
            ValueType::Easing(_) => {}
            _ => panic!("Expected Easing variant"),
        }
        match hint {
            ControlHint::EasingCurve => {}
            _ => panic!("Expected EasingCurve hint"),
        }
    }

    #[test]
    fn test_classify_value_easing_linear() {
        let (vtype, hint) = classify_value("linear");
        match vtype {
            ValueType::Easing(e) => assert_eq!(e, "linear"),
            _ => panic!("Expected Easing variant"),
        }
        match hint {
            ControlHint::EasingCurve => {}
            _ => panic!("Expected EasingCurve hint"),
        }
    }

    #[test]
    fn test_classify_value_number() {
        let (vtype, hint) = classify_value("42");
        match vtype {
            ValueType::Number(n) => assert_eq!(n, 42.0),
            _ => panic!("Expected Number variant"),
        }
        match hint {
            ControlHint::Slider { .. } => {}
            _ => panic!("Expected Slider hint"),
        }
    }

    // 3.14 below is an arbitrary decimal test fixture (verifying float
    // classification), not an intended pi approximation --
    // clippy::approx_constant is a false positive on this test.
    #[allow(clippy::approx_constant)]
    #[test]
    fn test_classify_value_number_float() {
        let (vtype, hint) = classify_value("3.14");
        match vtype {
            ValueType::Number(n) => assert_eq!(n, 3.14),
            _ => panic!("Expected Number variant"),
        }
        match hint {
            ControlHint::Slider { .. } => {}
            _ => panic!("Expected Slider hint"),
        }
    }

    #[test]
    fn test_classify_value_unknown_string() {
        let (vtype, hint) = classify_value("some-identifier");
        match vtype {
            ValueType::Unknown(_) => {}
            _ => panic!("Expected Unknown variant"),
        }
        match hint {
            ControlHint::TextInput => {}
            _ => panic!("Expected TextInput hint"),
        }
    }

    #[test]
    fn test_classify_value_empty_string() {
        let (vtype, hint) = classify_value("");
        match vtype {
            ValueType::Unknown(s) => assert_eq!(s, ""),
            _ => panic!("Expected Unknown variant"),
        }
        match hint {
            ControlHint::TextInput => {}
            _ => panic!("Expected TextInput hint"),
        }
    }

    #[test]
    fn test_classify_value_whitespace() {
        let (vtype, hint) = classify_value("   ");
        match vtype {
            ValueType::Unknown(s) => assert_eq!(s, ""),
            _ => panic!("Expected Unknown variant"),
        }
        match hint {
            ControlHint::TextInput => {}
            _ => panic!("Expected TextInput hint"),
        }
    }

    #[test]
    fn test_classify_value_calc_expression() {
        let (vtype, hint) = classify_value("calc(100% - 20px)");
        match vtype {
            ValueType::Unknown(_) => {}
            _ => panic!("Expected Unknown variant for calc()"),
        }
        match hint {
            ControlHint::TextInput => {}
            _ => panic!("Expected TextInput hint"),
        }
    }

    #[test]
    fn test_classify_value_var_expression() {
        let (vtype, hint) = classify_value("var(--my-color)");
        match vtype {
            ValueType::Unknown(_) => {}
            _ => panic!("Expected Unknown variant for var()"),
        }
        match hint {
            ControlHint::TextInput => {}
            _ => panic!("Expected TextInput hint"),
        }
    }

    #[test]
    fn test_inspect_element_message_serialization() {
        let msg = ClientMessage::InspectElement {
            selector: ".hero".to_string(),
            file_hint: Some("styles.st".to_string()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"InspectElement""#));
        assert!(json.contains(r#""selector":".hero""#));
        assert!(json.contains(r#""file_hint":"styles.st""#));
    }

    #[test]
    fn test_element_context_message_serialization() {
        let msg = ServerMessage::ElementContext {
            selector: ".hero".to_string(),
            source_files: vec!["styles.st".to_string()],
            sections: vec![],
            state_machine: None,
            data_bindings: vec![],
            not_found: false,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"ElementContext""#));
        assert!(json.contains(r#""selector":".hero""#));
        assert!(json.contains(r#""not_found":false"#));
    }

    #[test]
    fn test_property_section_roundtrip() {
        let section = PropertySection {
            label: "Animations".to_string(),
            source_file: Some("styles.st".to_string()),
            source_line: Some(42),
            properties: vec![],
        };

        let json = serde_json::to_string(&section).unwrap();
        let parsed: PropertySection = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.label, "Animations");
        assert_eq!(parsed.source_file, Some("styles.st".to_string()));
        assert_eq!(parsed.source_line, Some(42));
    }

    #[test]
    fn test_value_node_with_color() {
        let node = ValueNode {
            raw: "#ff0000".to_string(),
            parsed: ValueType::Color("#ff0000".to_string()),
            control_hint: ControlHint::ColorPicker,
            editable: true,
        };

        let json = serde_json::to_string(&node).unwrap();
        let parsed: ValueNode = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.raw, "#ff0000");
        assert_eq!(parsed.editable, true);
    }

    #[test]
    fn test_state_machine_context_roundtrip() {
        let ctx = StateMachineContext {
            current_state: "idle".to_string(),
            states: vec![
                "idle".to_string(),
                "hover".to_string(),
                "active".to_string(),
            ],
        };

        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: StateMachineContext = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.current_state, "idle");
        assert_eq!(parsed.states.len(), 3);
    }

    #[test]
    fn test_data_binding_context_roundtrip() {
        let ctx = DataBindingContext {
            name: "products".to_string(),
            data_type: "Product[]".to_string(),
        };

        let json = serde_json::to_string(&ctx).unwrap();
        let parsed: DataBindingContext = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, "products");
        assert_eq!(parsed.data_type, "Product[]");
    }
}
