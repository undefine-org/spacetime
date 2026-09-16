//! `CapturedValue` ⟷ EDN — the scalar/structural codec (PLAN-148 W1).
//!
//! Normative mapping: `docs/edn-spacetime/SPEC.md` §4.
//!
//! # Encoding rules, and why they are what they are
//!
//! Each rule below exists because the naive alternative was MEASURED to fail in
//! one of the two runtimes this format must serve (`clojure-reader` in Rust,
//! `BeamLisp.Reader` on the BEAM).
//!
//! * **Units are vectors, never tagged literals.** `[:st/time 500]`, not
//!   `#st/time 500`. `BeamLisp.Reader` has NO EDN tagged literals: `#st/time 500`
//!   raises "expected one form, got 2" (it has `#Name{...}` *records*, a
//!   different construct). Vectors need zero reader support anywhere.
//! * **Typerefs are strings.** `"Message[]"`, not the symbol `Message[]` — the
//!   latter parses WITHOUT ERROR as `Symbol("Message")` + `Vector([])`, silently
//!   destroying the type. A genuine silent-corruption trap.
//! * **Selectors are strings.** `.kit-btn` reads as a symbol in both runtimes,
//!   but `#hero` is an EOF error in `clojure-reader`. Strings are total.
//! * **`$binding` / `&element` pass through verbatim** as symbols — both readers
//!   treat these sigils as ordinary symbol constituents. They are the most
//!   frequent tokens in `.st`, so this costs nothing and reads naturally.
//! * **No form begins with `@`.** Directives are keyed by macro name, which frees
//!   `@` to mean `deref` on the BEAM exactly as a Lisp programmer expects.
//!
//! Every non-scalar is a vector led by a `:st/…` head keyword, so decoding is a
//! single unambiguous dispatch that can never collide with user data.

use clojure_reader::edn::Edn;

use crate::parser::ast::JsonValue;
use crate::syntax::{
    CapturedValue, KeyframeDef, LengthValue, ParamDef, PropertyDef, TemplateParamDef,
    TemplateParamKind,
};

/// A codec failure. Always names what was expected and what was found — an EDN
/// author must be able to fix the source from the message alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodecError {
    pub message: String,
}

impl CodecError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CodecError {}

type Result<T> = std::result::Result<T, CodecError>;

// ── head keywords ────────────────────────────────────────────────────────────
//
// One constant per non-scalar variant. Kept as consts (not inline literals) so
// the encoder and decoder can never disagree about a spelling.

const K_TIME: &str = "st/time";
const K_LEN: &str = "st/len";
const K_SEL: &str = "st/sel";
const K_EXPR: &str = "st/expr";
const K_TYPE: &str = "st/type";
const K_PRESET: &str = "st/preset";
const K_COLOR: &str = "st/color";
const K_JSON: &str = "st/json";
const K_BLOCK: &str = "st/block";
const K_ARRAY: &str = "st/array";
const K_NAMED: &str = "st/named";
const K_PROPS: &str = "st/props";
const K_PARAMS: &str = "st/params";
const K_KEYFRAMES: &str = "st/keyframes";
const K_PARAMLIST: &str = "st/paramlist";
const K_STYLES: &str = "st/styles";
const K_PMATCH: &str = "st/pmatch";
const K_COMPONENT_BODY: &str = "st/component-body";

fn key(k: &str) -> Edn<'static> {
    Edn::Key(Box::leak(k.to_string().into_boxed_str()))
}

fn sym(s: &str) -> Edn<'static> {
    Edn::Symbol(Box::leak(s.to_string().into_boxed_str()))
}

fn string(s: &str) -> Edn<'static> {
    Edn::Str(Box::leak(s.to_string().into_boxed_str()))
}

/// A `[:st/head …]` vector.
fn tagged(head: &str, mut rest: Vec<Edn<'static>>) -> Edn<'static> {
    let mut v = Vec::with_capacity(rest.len() + 1);
    v.push(key(head));
    v.append(&mut rest);
    Edn::Vector(v)
}

fn number(n: f64) -> Edn<'static> {
    // EDN distinguishes integers from floats; preserve the distinction so a
    // round-tripped value prints the way the author wrote it.
    if n.fract() == 0.0 && n.is_finite() && n.abs() < 9e15 {
        Edn::Int(n as i64)
    } else {
        Edn::Double(n.into())
    }
}

fn opt_string(o: &Option<String>) -> Edn<'static> {
    match o {
        Some(s) => string(s),
        None => Edn::Nil,
    }
}

// ── encode ───────────────────────────────────────────────────────────────────

/// `CapturedValue` → EDN. Total: every variant has an encoding (SPEC §4).
pub fn to_edn_value(v: &CapturedValue) -> Edn<'static> {
    match v {
        // Scalars that are natural EDN values.
        CapturedValue::Ident(s) => sym(s),
        CapturedValue::String(s) => string(s),
        CapturedValue::Number(n) => number(*n),
        CapturedValue::Bool(b) => Edn::Bool(*b),

        // `$x` / `&x` survive verbatim as symbols in BOTH readers (measured).
        CapturedValue::Binding(s) => sym(s),
        CapturedValue::Element(s) => sym(s),

        // Units: vectors, never tagged literals (see module docs).
        CapturedValue::Time(ms) => tagged(K_TIME, vec![Edn::Int(*ms as i64)]),
        CapturedValue::Length(LengthValue { value, unit }) => {
            tagged(K_LEN, vec![number(*value), key(unit)])
        }

        // Strings-with-a-role: wrapped so the decoder recovers the exact variant.
        CapturedValue::Selector(s) => tagged(K_SEL, vec![string(s)]),
        CapturedValue::Expr(s) => tagged(K_EXPR, vec![string(s)]),
        CapturedValue::TypeRef(s) => tagged(K_TYPE, vec![string(s)]),
        CapturedValue::Color(s) => tagged(K_COLOR, vec![string(s)]),
        // A preset name is an identifier, so a keyword reads most naturally —
        // and `~ease-out` is impossible here (`~` is unquote on the BEAM).
        CapturedValue::Preset(s) => tagged(K_PRESET, vec![key(s)]),

        CapturedValue::Json(j) => tagged(K_JSON, vec![json_to_edn(j)]),

        // Nested structures.
        CapturedValue::Array(items) => {
            tagged(K_ARRAY, items.iter().map(to_edn_value).collect())
        }
        CapturedValue::Named(map) => {
            // BTreeMap ordering: EDN maps are unordered, but a DETERMINISTIC
            // encoding is required for byte-comparison in the corpus tests.
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let pairs = keys
                .into_iter()
                .map(|k| (string(k), to_edn_value(&map[k])))
                .collect();
            tagged(K_NAMED, vec![Edn::Map(pairs)])
        }

        CapturedValue::Properties(props) => tagged(
            K_PROPS,
            props
                .iter()
                .map(|p| {
                    Edn::Vector(vec![
                        string(&p.name),
                        string(&p.type_ref),
                        Edn::Bool(p.optional),
                    ])
                })
                .collect(),
        ),

        CapturedValue::Params(params) => tagged(
            K_PARAMS,
            params
                .iter()
                .map(|p| {
                    Edn::Vector(vec![
                        string(&p.name),
                        string(&p.type_ref),
                        opt_string(&p.default),
                    ])
                })
                .collect(),
        ),

        CapturedValue::Keyframes(kfs) => tagged(
            K_KEYFRAMES,
            kfs.iter()
                .map(|k| {
                    Edn::Vector(vec![
                        string(&k.property),
                        Edn::Vector(k.values.iter().map(|v| string(v)).collect()),
                        opt_string(&k.selector),
                    ])
                })
                .collect(),
        ),

        CapturedValue::ParamList(ps) => tagged(
            K_PARAMLIST,
            ps.iter()
                .map(|p| {
                    Edn::Vector(vec![
                        string(&p.name),
                        key(match p.kind {
                            TemplateParamKind::Binding => "binding",
                            TemplateParamKind::Element => "element",
                        }),
                        Edn::Bool(p.optional),
                        opt_string(&p.type_ref),
                        opt_string(&p.default),
                        Edn::Bool(p.collection),
                    ])
                })
                .collect(),
        ),

        CapturedValue::StyleProperties(props) => tagged(
            K_STYLES,
            props
                .iter()
                .map(|(k, v)| Edn::Vector(vec![string(k), string(v)]))
                .collect(),
        ),

        CapturedValue::PatternMatch {
            signal,
            variant,
            bindings,
        } => tagged(
            K_PMATCH,
            vec![
                string(signal),
                string(variant),
                Edn::Vector(bindings.iter().map(|b| string(b)).collect()),
            ],
        ),

        // A bare presence-marker: it records that the match HAS a component body,
        // but carries no data (the payload lives in the `@template:<name>` scope
        // and body diagnostics land in `StFile.diagnostics`). Encode the marker.
        CapturedValue::ComponentBody => tagged(K_COMPONENT_BODY, vec![]),

        // `Block(Vec<FormMatch>)` needs the FormMatch codec, which lives in W3
        // (`read.rs`/`print.rs`). Encoding it here would duplicate that logic and
        // invite the two copies to drift, so this variant is routed through the
        // form codec instead — and says so loudly rather than silently emitting a
        // half-value.
        CapturedValue::Block(_) => tagged(
            K_BLOCK,
            vec![string(
                "unsupported at codec layer: Block is encoded by the form codec (W3)",
            )],
        ),
    }
}

fn json_to_edn(j: &JsonValue) -> Edn<'static> {
    match j {
        JsonValue::Null => Edn::Nil,
        JsonValue::Bool(b) => Edn::Bool(*b),
        // JSON numbers are carried as SOURCE TEXT in the AST; keep them as text
        // so no precision or formatting is invented on the way through.
        JsonValue::Number(n) => string(n),
        JsonValue::String(s) => Edn::Vector(vec![key("str"), string(s)]),
        JsonValue::Array(items) => Edn::Vector(items.iter().map(json_to_edn).collect()),
        // String keys (not keywords) preserve exact key bytes.
        JsonValue::Object(pairs) => Edn::Map(
            pairs
                .iter()
                .map(|(k, v)| (string(k), json_to_edn(v)))
                .collect(),
        ),
    }
}

// ── decode ───────────────────────────────────────────────────────────────────

/// EDN → `CapturedValue`. The exact inverse of [`to_edn_value`] (SPEC §4).
pub fn from_edn_value(e: &Edn<'_>) -> Result<CapturedValue> {
    match e {
        Edn::Str(s) => Ok(CapturedValue::String(unescape_edn(s))),
        Edn::Bool(b) => Ok(CapturedValue::Bool(*b)),
        Edn::Int(i) => Ok(CapturedValue::Number(*i as f64)),
        Edn::Double(d) => Ok(CapturedValue::Number(f64::from(*d))),

        // A bare symbol is an Ident, a Binding (`$…`) or an Element (`&…`) —
        // the sigil discriminates, exactly as it does in `.st`.
        Edn::Symbol(s) => Ok(match s.chars().next() {
            Some('$') => CapturedValue::Binding((*s).to_string()),
            Some('&') => CapturedValue::Element((*s).to_string()),
            _ => CapturedValue::Ident((*s).to_string()),
        }),

        Edn::Vector(items) => from_tagged_vector(items),

        other => Err(CodecError::new(format!(
            "cannot decode {other:?} as a captured value: expected a scalar \
             (string/number/bool/symbol) or a [:st/… …] vector"
        ))),
    }
}

fn head_of(items: &[Edn<'_>]) -> Result<String> {
    match items.first() {
        Some(Edn::Key(k)) => Ok((*k).to_string()),
        Some(other) => Err(CodecError::new(format!(
            "expected a :st/… head keyword, found {other:?}"
        ))),
        None => Err(CodecError::new(
            "empty vector: expected a [:st/… …] tagged form",
        )),
    }
}

/// Unescape an EDN string literal's body.
///
/// BUG-377: `clojure_reader` hands back the RAW SLICE between the quotes, with
/// its escapes intact — `"\"\""` in EDN source arrives as the four characters
/// `\"\"`, not as the two characters `""`. Everything downstream treats that
/// slice as decoded text, so an expr carrying a string literal was written back
/// into `.st` source still escaped:
///
/// ```text
/// source     $draft <- "";        // valid
/// edn        [:st/expr "\"\""]     // correct
/// printed    $draft <- \"\";      // WRONG — and it still parses
/// ```
///
/// The printed file passed `check` (the parser reads `\"` as a two-character
/// string), so the defect reached the browser as a signal assigned literal
/// backslash-quote text instead of being cleared. A chat composer never
/// emptied, and every following message arrived concatenated onto the last.
///
/// Decoded HERE, one escape at a time, rather than by a chain of `replace`
/// passes: sequential replacement decodes its own output, so a literal
/// backslash-n (written `\\n`) has its second backslash paired with the `n` and
/// becomes a NEWLINE — silently corrupting any text holding a Windows path or a
/// regex.
pub(crate) fn unescape_edn(raw: &str) -> String {
    if !raw.contains('\\') {
        return raw.to_string();
    }

    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();

    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        // An escape consumes the NEXT character whatever it is, so `\\"` inside
        // the string does not terminate anything.
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            // An unknown escape is carried through UNCHANGED, both characters.
            // Dropping the backslash would silently rewrite text this function
            // does not understand — a regex `\d` must survive as `\d`.
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }

    out
}

fn as_str(e: &Edn<'_>, ctx: &str) -> Result<String> {
    match e {
        Edn::Str(s) => Ok(unescape_edn(s)),
        other => Err(CodecError::new(format!(
            "{ctx}: expected a string, found {other:?}"
        ))),
    }
}

fn as_opt_str(e: &Edn<'_>, ctx: &str) -> Result<Option<String>> {
    match e {
        Edn::Nil => Ok(None),
        other => as_str(other, ctx).map(Some),
    }
}

fn as_bool(e: &Edn<'_>, ctx: &str) -> Result<bool> {
    match e {
        Edn::Bool(b) => Ok(*b),
        other => Err(CodecError::new(format!(
            "{ctx}: expected a boolean, found {other:?}"
        ))),
    }
}

fn as_f64(e: &Edn<'_>, ctx: &str) -> Result<f64> {
    match e {
        Edn::Int(i) => Ok(*i as f64),
        Edn::Double(d) => Ok(f64::from(*d)),
        other => Err(CodecError::new(format!(
            "{ctx}: expected a number, found {other:?}"
        ))),
    }
}

fn as_key(e: &Edn<'_>, ctx: &str) -> Result<String> {
    match e {
        Edn::Key(k) => Ok((*k).to_string()),
        other => Err(CodecError::new(format!(
            "{ctx}: expected a keyword, found {other:?}"
        ))),
    }
}

fn as_vec<'a>(e: &'a Edn<'a>, ctx: &str) -> Result<&'a Vec<Edn<'a>>> {
    match e {
        Edn::Vector(v) => Ok(v),
        other => Err(CodecError::new(format!(
            "{ctx}: expected a vector, found {other:?}"
        ))),
    }
}

fn arity(items: &[Edn<'_>], want: usize, head: &str) -> Result<()> {
    // `items` includes the head keyword, so the payload count is len - 1.
    if items.len() - 1 == want {
        Ok(())
    } else {
        Err(CodecError::new(format!(
            "[:{head} …]: expected {want} element(s), found {}",
            items.len() - 1
        )))
    }
}

fn from_tagged_vector(items: &[Edn<'_>]) -> Result<CapturedValue> {
    let head = head_of(items)?;
    let rest = &items[1..];

    match head.as_str() {
        K_TIME => {
            arity(items, 1, K_TIME)?;
            Ok(CapturedValue::Time(as_f64(&rest[0], "[:st/time n]")? as u32))
        }
        K_LEN => {
            arity(items, 2, K_LEN)?;
            Ok(CapturedValue::Length(LengthValue {
                value: as_f64(&rest[0], "[:st/len n :unit]")?,
                unit: as_key(&rest[1], "[:st/len n :unit]")?,
            }))
        }
        K_SEL => {
            arity(items, 1, K_SEL)?;
            Ok(CapturedValue::Selector(as_str(&rest[0], "[:st/sel s]")?))
        }
        K_EXPR => {
            arity(items, 1, K_EXPR)?;
            Ok(CapturedValue::Expr(as_str(&rest[0], "[:st/expr s]")?))
        }
        K_TYPE => {
            arity(items, 1, K_TYPE)?;
            Ok(CapturedValue::TypeRef(as_str(&rest[0], "[:st/type s]")?))
        }
        K_COLOR => {
            arity(items, 1, K_COLOR)?;
            Ok(CapturedValue::Color(as_str(&rest[0], "[:st/color s]")?))
        }
        K_PRESET => {
            arity(items, 1, K_PRESET)?;
            Ok(CapturedValue::Preset(as_key(&rest[0], "[:st/preset :k]")?))
        }
        K_JSON => {
            arity(items, 1, K_JSON)?;
            Ok(CapturedValue::Json(edn_to_json(&rest[0])?))
        }
        K_ARRAY => Ok(CapturedValue::Array(
            rest.iter().map(from_edn_value).collect::<Result<Vec<_>>>()?,
        )),
        K_NAMED => {
            arity(items, 1, K_NAMED)?;
            match &rest[0] {
                Edn::Map(pairs) => {
                    let mut map = std::collections::HashMap::new();
                    for (k, v) in pairs {
                        map.insert(as_str(k, "[:st/named {…}] key")?, from_edn_value(v)?);
                    }
                    Ok(CapturedValue::Named(map))
                }
                other => Err(CodecError::new(format!(
                    "[:st/named …]: expected a map, found {other:?}"
                ))),
            }
        }
        K_PROPS => {
            let mut out = Vec::with_capacity(rest.len());
            for it in rest {
                let v = as_vec(it, "[:st/props …] entry")?;
                if v.len() != 3 {
                    return Err(CodecError::new(format!(
                        "[:st/props …] entry: expected [name type optional], found {} element(s)",
                        v.len()
                    )));
                }
                out.push(PropertyDef {
                    name: as_str(&v[0], "property name")?,
                    type_ref: as_str(&v[1], "property type")?,
                    optional: as_bool(&v[2], "property optional")?,
                });
            }
            Ok(CapturedValue::Properties(out))
        }
        K_PARAMS => {
            let mut out = Vec::with_capacity(rest.len());
            for it in rest {
                let v = as_vec(it, "[:st/params …] entry")?;
                if v.len() != 3 {
                    return Err(CodecError::new(format!(
                        "[:st/params …] entry: expected [name type default], found {} element(s)",
                        v.len()
                    )));
                }
                out.push(ParamDef {
                    name: as_str(&v[0], "param name")?,
                    type_ref: as_str(&v[1], "param type")?,
                    default: as_opt_str(&v[2], "param default")?,
                });
            }
            Ok(CapturedValue::Params(out))
        }
        K_KEYFRAMES => {
            let mut out = Vec::with_capacity(rest.len());
            for it in rest {
                let v = as_vec(it, "[:st/keyframes …] entry")?;
                if v.len() != 3 {
                    return Err(CodecError::new(format!(
                        "[:st/keyframes …] entry: expected [property [values] selector], \
                         found {} element(s)",
                        v.len()
                    )));
                }
                let values = as_vec(&v[1], "keyframe values")?
                    .iter()
                    .map(|x| as_str(x, "keyframe value"))
                    .collect::<Result<Vec<_>>>()?;
                out.push(KeyframeDef {
                    property: as_str(&v[0], "keyframe property")?,
                    values,
                    selector: as_opt_str(&v[2], "keyframe selector")?,
                });
            }
            Ok(CapturedValue::Keyframes(out))
        }
        K_PARAMLIST => {
            let mut out = Vec::with_capacity(rest.len());
            for it in rest {
                let v = as_vec(it, "[:st/paramlist …] entry")?;
                if v.len() != 6 {
                    return Err(CodecError::new(format!(
                        "[:st/paramlist …] entry: expected \
                         [name kind optional type default collection], found {} element(s)",
                        v.len()
                    )));
                }
                let kind = match as_key(&v[1], "param kind")?.as_str() {
                    "binding" => TemplateParamKind::Binding,
                    "element" => TemplateParamKind::Element,
                    other => {
                        return Err(CodecError::new(format!(
                            "param kind: expected :binding or :element, found :{other}"
                        )));
                    }
                };
                out.push(TemplateParamDef {
                    name: as_str(&v[0], "param name")?,
                    kind,
                    optional: as_bool(&v[2], "param optional")?,
                    type_ref: as_opt_str(&v[3], "param type")?,
                    default: as_opt_str(&v[4], "param default")?,
                    collection: as_bool(&v[5], "param collection")?,
                });
            }
            Ok(CapturedValue::ParamList(out))
        }
        K_STYLES => {
            let mut out = Vec::with_capacity(rest.len());
            for it in rest {
                let v = as_vec(it, "[:st/styles …] entry")?;
                if v.len() != 2 {
                    return Err(CodecError::new(format!(
                        "[:st/styles …] entry: expected [property value], found {} element(s)",
                        v.len()
                    )));
                }
                out.push((
                    as_str(&v[0], "style property")?,
                    as_str(&v[1], "style value")?,
                ));
            }
            Ok(CapturedValue::StyleProperties(out))
        }
        K_PMATCH => {
            arity(items, 3, K_PMATCH)?;
            let bindings = as_vec(&rest[2], "[:st/pmatch …] bindings")?
                .iter()
                .map(|b| as_str(b, "pattern binding"))
                .collect::<Result<Vec<_>>>()?;
            Ok(CapturedValue::PatternMatch {
                signal: as_str(&rest[0], "pattern signal")?,
                variant: as_str(&rest[1], "pattern variant")?,
                bindings,
            })
        }
        K_COMPONENT_BODY => {
            arity(items, 0, K_COMPONENT_BODY)?;
            Ok(CapturedValue::ComponentBody)
        }
        K_BLOCK => Err(CodecError::new(
            "[:st/block …] is decoded by the form codec (W3), not the value codec",
        )),
        other => Err(CodecError::new(format!(
            "unknown tagged form :{other} — expected one of :st/time :st/len :st/sel \
             :st/expr :st/type :st/color :st/preset :st/json :st/array :st/named \
             :st/props :st/params :st/keyframes :st/paramlist :st/styles :st/pmatch \
             :st/component-body"
        ))),
    }
}

fn edn_to_json(e: &Edn<'_>) -> Result<JsonValue> {
    match e {
        Edn::Nil => Ok(JsonValue::Null),
        Edn::Bool(b) => Ok(JsonValue::Bool(*b)),
        // A bare string is a JSON NUMBER carried as source text; a JSON string is
        // `[:str "…"]`. Without this split the two would be indistinguishable on
        // the way back.
        Edn::Str(s) => Ok(JsonValue::Number((*s).to_string())),
        Edn::Vector(items) => {
            if let Some(Edn::Key(k)) = items.first()
                && *k == "str"
            {
                if items.len() != 2 {
                    return Err(CodecError::new(format!(
                        "[:str …]: expected 1 element, found {}",
                        items.len() - 1
                    )));
                }
                return Ok(JsonValue::String(as_str(&items[1], "[:str s]")?));
            }
            Ok(JsonValue::Array(
                items.iter().map(edn_to_json).collect::<Result<Vec<_>>>()?,
            ))
        }
        Edn::Map(pairs) => {
            let mut out = Vec::with_capacity(pairs.len());
            for (k, v) in pairs {
                out.push((as_str(k, "json object key")?, edn_to_json(v)?));
            }
            Ok(JsonValue::Object(out))
        }
        other => Err(CodecError::new(format!(
            "cannot decode {other:?} as a JSON value"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// The W1 proof obligation (SPEC §2, P1): every variant survives a round trip.
    fn round_trip(v: CapturedValue) {
        let encoded = to_edn_value(&v);
        let decoded = from_edn_value(&encoded)
            .unwrap_or_else(|e| panic!("decode failed for {v:?}\n  encoded: {encoded:?}\n  err: {e}"));
        assert_eq!(v, decoded, "round trip changed the value\n  encoded: {encoded:?}");
    }

    #[test]
    fn scalars_round_trip() {
        round_trip(CapturedValue::Ident("add".into()));
        round_trip(CapturedValue::String("hello".into()));
        round_trip(CapturedValue::Number(3.5));
        round_trip(CapturedValue::Number(42.0));
        round_trip(CapturedValue::Bool(true));
        round_trip(CapturedValue::Bool(false));
    }

    #[test]
    fn sigil_bearing_scalars_round_trip() {
        // `$x` / `&x` must survive verbatim — they are the most frequent tokens
        // in `.st` and both readers treat the sigils as symbol constituents.
        round_trip(CapturedValue::Binding("$mcpPrompt".into()));
        round_trip(CapturedValue::Element("&main".into()));
    }

    #[test]
    fn units_round_trip() {
        round_trip(CapturedValue::Time(500));
        round_trip(CapturedValue::Length(LengthValue {
            value: 20.0,
            unit: "px".into(),
        }));
    }

    #[test]
    fn wrapped_strings_round_trip() {
        round_trip(CapturedValue::Selector(".kit-btn".into()));
        // The id-selector case: `#hero` is an EOF error as a bare EDN symbol,
        // which is exactly why selectors are strings.
        round_trip(CapturedValue::Selector("#hero".into()));
        round_trip(CapturedValue::Expr("$a + $b".into()));
        round_trip(CapturedValue::TypeRef("Message[]".into()));
        round_trip(CapturedValue::Color("#f0e6d6".into()));
        round_trip(CapturedValue::Preset("ease-out".into()));
    }

    #[test]
    fn collections_round_trip() {
        round_trip(CapturedValue::Array(vec![
            CapturedValue::Number(1.0),
            CapturedValue::String("two".into()),
            CapturedValue::Bool(true),
        ]));

        let mut named = HashMap::new();
        named.insert("action".to_string(), CapturedValue::String("go".into()));
        named.insert("count".to_string(), CapturedValue::Number(2.0));
        round_trip(CapturedValue::Named(named));

        // Nesting must survive too — the recursive case is where a codec breaks.
        round_trip(CapturedValue::Array(vec![CapturedValue::Array(vec![
            CapturedValue::Time(250),
        ])]));
    }

    #[test]
    fn structured_defs_round_trip() {
        round_trip(CapturedValue::Properties(vec![
            PropertyDef {
                name: "name".into(),
                type_ref: "string".into(),
                optional: false,
            },
            PropertyDef {
                name: "price".into(),
                type_ref: "number".into(),
                optional: true,
            },
        ]));

        round_trip(CapturedValue::Params(vec![ParamDef {
            name: "duration".into(),
            type_ref: "time".into(),
            default: Some("600ms".into()),
        }]));

        round_trip(CapturedValue::Keyframes(vec![KeyframeDef {
            property: "opacity".into(),
            values: vec!["0".into(), "1".into()],
            selector: Some(".child".into()),
        }]));

        round_trip(CapturedValue::ParamList(vec![
            TemplateParamDef {
                name: "title".into(),
                kind: TemplateParamKind::Binding,
                optional: false,
                type_ref: None,
                default: None,
                collection: false,
            },
            TemplateParamDef {
                name: "items".into(),
                kind: TemplateParamKind::Element,
                optional: true,
                type_ref: Some("url".into()),
                default: Some("\"\"".into()),
                collection: true,
            },
        ]));

        round_trip(CapturedValue::StyleProperties(vec![
            ("color".into(), "red".into()),
            ("margin".into(), "0 auto".into()),
        ]));

        round_trip(CapturedValue::PatternMatch {
            signal: "$status".into(),
            variant: "Ok".into(),
            bindings: vec!["value".into()],
        });
    }

    #[test]
    fn component_body_marker_round_trips() {
        round_trip(CapturedValue::ComponentBody);
    }

    #[test]
    fn json_round_trips() {
        round_trip(CapturedValue::Json(JsonValue::Null));
        round_trip(CapturedValue::Json(JsonValue::Bool(true)));
        round_trip(CapturedValue::Json(JsonValue::Number("42".into())));
        round_trip(CapturedValue::Json(JsonValue::String("hi".into())));
        round_trip(CapturedValue::Json(JsonValue::Array(vec![
            JsonValue::Number("1".into()),
            JsonValue::String("two".into()),
        ])));
        round_trip(CapturedValue::Json(JsonValue::Object(vec![
            ("a".into(), JsonValue::Number("1".into())),
            ("b".into(), JsonValue::Null),
        ])));
    }

    #[test]
    fn json_number_and_string_stay_distinct() {
        // The trap this guards: JSON numbers are carried as SOURCE TEXT, so a
        // naive encoding makes `Number("42")` and `String("42")` identical.
        let n = CapturedValue::Json(JsonValue::Number("42".into()));
        let s = CapturedValue::Json(JsonValue::String("42".into()));
        assert_ne!(to_edn_value(&n), to_edn_value(&s));
        round_trip(n);
        round_trip(s);
    }

    #[test]
    fn named_encoding_is_deterministic() {
        // Corpus tests compare encodings, so HashMap iteration order must not
        // leak into the output.
        let mut a = HashMap::new();
        a.insert("z".to_string(), CapturedValue::Number(1.0));
        a.insert("a".to_string(), CapturedValue::Number(2.0));
        let mut b = HashMap::new();
        b.insert("a".to_string(), CapturedValue::Number(2.0));
        b.insert("z".to_string(), CapturedValue::Number(1.0));
        assert_eq!(
            to_edn_value(&CapturedValue::Named(a)),
            to_edn_value(&CapturedValue::Named(b))
        );
    }

    #[test]
    fn integers_do_not_become_floats() {
        // `500` must not round-trip as `500.0`: the printed `.st` would differ.
        assert_eq!(to_edn_value(&CapturedValue::Number(42.0)), Edn::Int(42));
    }

    #[test]
    fn errors_name_the_problem() {
        // Diagnostics are part of the contract: an author must be able to fix the
        // source from the message alone.
        let err = from_edn_value(&Edn::Vector(vec![key("st/nope")])).unwrap_err();
        assert!(err.message.contains("unknown tagged form"), "{}", err.message);

        let err = from_edn_value(&Edn::Vector(vec![key("st/time")])).unwrap_err();
        assert!(err.message.contains("expected 1 element"), "{}", err.message);
    }

    #[test]
    fn decoding_real_edn_text_works() {
        // End-to-end through the actual reader, not just constructed values —
        // this is the path a `.edn` file takes.
        let parsed = clojure_reader::edn::read_string("[:st/len 20 :px]").unwrap();
        assert_eq!(
            from_edn_value(&parsed).unwrap(),
            CapturedValue::Length(LengthValue {
                value: 20.0,
                unit: "px".into()
            })
        );

        let parsed = clojure_reader::edn::read_string("$mcpPrompt").unwrap();
        assert_eq!(
            from_edn_value(&parsed).unwrap(),
            CapturedValue::Binding("$mcpPrompt".into())
        );
    }
}


#[cfg(test)]
mod bug377_string_round_trip {
    use super::*;

    // BUG-377: `clojure_reader` returns the RAW slice between the quotes, so a
    // string literal arrived with its escapes intact and was written back into
    // `.st` source still escaped (`$draft <- \"\";`). The printed file still
    // PARSED — as a two-character string — so nothing failed until a browser
    // assigned the garbage.

    fn expr_of(edn: &str) -> String {
        let parsed = clojure_reader::edn::read_string(edn).unwrap();
        match from_edn_value(&parsed).unwrap() {
            CapturedValue::Expr(s) => s,
            other => panic!("expected an Expr, got {other:?}"),
        }
    }

    #[test]
    fn an_empty_string_literal_decodes_to_two_quotes() {
        assert_eq!(expr_of(r#"[:st/expr "\"\""]"#), "\"\"");
    }

    #[test]
    fn a_quoted_word_keeps_its_quotes_and_nothing_else() {
        assert_eq!(expr_of(r#"[:st/expr "\"hi\""]"#), "\"hi\"");
    }

    #[test]
    fn an_unescaped_expr_is_unchanged() {
        // The overwhelmingly common case: no backslash, no work, no risk.
        assert_eq!(expr_of(r#"[:st/expr "$.value"]"#), "$.value");
    }

    #[test]
    fn an_unknown_escape_survives_both_characters() {
        // A regex `\d` must NOT become `d`. Dropping the backslash would
        // silently rewrite text this decoder does not understand.
        assert_eq!(expr_of(r#"[:st/expr "\\d+"]"#), "\\d+");
    }

    #[test]
    fn a_literal_backslash_n_does_not_become_a_newline() {
        // The trap a chain of `replace` passes falls into: sequential
        // replacement decodes its own output, pairing the second backslash of
        // `\\n` with the `n`. A Windows path or a regex is corrupted in silence.
        assert_eq!(expr_of(r#"[:st/expr "C:\\new"]"#), "C:\\new");
        assert!(!expr_of(r#"[:st/expr "C:\\new"]"#).contains('\n'));
    }

    #[test]
    fn a_real_newline_escape_still_decodes() {
        assert_eq!(expr_of(r#"[:st/expr "a\nb"]"#), "a\nb");
    }
}
