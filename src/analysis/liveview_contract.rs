//! Wave B — the self-aware LiveView contract comparator (FEAT-135).
//!
//! The keystone of the Spacetime⇄Phoenix LiveView bridge: make the integration
//! VALIDATE ITSELF at compile time. A page that binds `@host $b : live("Mod")`
//! and fires `@data signal … { send emit "move_card" … }` is checked against the
//! server module's declared contract — a `<page>.contract.json` sidecar today
//! (the exact shape the Elixir `__spacetime_contract__/0` will emit in Wave C).
//!
//! If the page fires an event the server does not handle, that is a COMPILE error
//! (E0928), not a runtime surprise — footguns #2/#3 from the design's catalog made
//! impossible by construction. The comparator is pure data-in/data-out so its core
//! is unit-tested independently of the parse pipeline.

use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::syntax::FormMatch;
use std::collections::HashMap;
use std::path::Path;

/// One server-declared event the page may fire (`events do handle "move_card" …`).
#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
pub struct ContractEvent {
    pub name: String,
    /// The reply tags this event's handler can answer with (FUP-143 hole #2).
    ///
    /// `None` = the key was ABSENT: not declared, so not checked. Every sidecar
    /// written before this field existed is in that state, and failing them all
    /// would make the feature unshippable.
    ///
    /// `Some([])` = declared EMPTY: an explicit claim that the handler never
    /// replies, which IS checked — a page decoding something for it is decoding
    /// a message that will never arrive.
    ///
    /// `Option` rather than `#[serde(default)] Vec`: a plain Vec deserialises
    /// both cases to `vec![]`, so the distinction the docs promised could not
    /// actually be represented (found by review).
    #[serde(default)]
    pub replies: Option<Vec<String>>,
}

/// One server-declared assign the page may subscribe to (`assigns do assign :count`).
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Default)]
pub struct ContractAssign {
    pub name: String,
}

/// One server-declared push the page may receive (`pushes do push "flash" …`).
#[derive(Debug, Clone, serde::Deserialize, PartialEq, Default)]
pub struct ContractPush {
    pub name: String,
}

/// The server LiveView's introspected contract — the `__spacetime_contract__/0`
/// projection. Loaded from a `<page>.contract.json` sidecar (Wave B) or, later,
/// emitted by the Elixir Spark transformer (Wave C). Mechanically identical shape.
#[derive(Debug, Clone, serde::Deserialize, PartialEq)]
pub struct ServerContract {
    pub module: String,
    #[serde(default)]
    pub events: Vec<ContractEvent>,
    #[serde(default)]
    pub pushes: Vec<ContractPush>,
    #[serde(default)]
    pub assigns: Vec<ContractAssign>,
    #[serde(default)]
    pub fingerprint: String,
}

impl ServerContract {
    /// Event names the server handles (the set a page's `send emit "X"` must be ⊆).
    pub fn event_names(&self) -> Vec<&str> {
        self.events.iter().map(|e| e.name.as_str()).collect()
    }

    /// Assign names the page may mirror through `@data subscribe`.
    pub fn assign_names(&self) -> Vec<&str> {
        self.assigns.iter().map(|a| a.name.as_str()).collect()
    }

    /// Transient push names a page may receive through `@data stream`.
    pub fn push_names(&self) -> Vec<&str> {
        self.pushes.iter().map(|push| push.name.as_str()).collect()
    }
}

/// A single page-side outbound intent extracted from a `@data signal/stream`:
/// "this page fires event `event` at the live host bound to module `module`."
#[derive(Debug, Clone, PartialEq)]
pub struct PageIntent {
    /// The `@host` binding name the signal targets, e.g. `$board`.
    pub host_binding: String,
    /// The event name the page emits (`send emit "move_card"` → `move_card`).
    pub event: String,
    /// Source span of the signal directive, for diagnostic placement.
    pub span: crate::parser::SourceSpan,
    /// The `receive` arms this page decodes, with their PATTERN CATEGORY kept.
    /// Flattening them to strings made `other => …` read as a catch-all and
    /// `error => …` read as an exact tag, both wrong (see `DecodedArm`).
    pub decoded_arms: Vec<DecodedArm>,
}

/// A `live(...)` host binding: `@host $board : live("MyAppWeb.BoardLive")`.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveHost {
    /// The binding name, e.g. `$board` (kept with the leading `$`).
    pub binding: String,
    /// The server module string, e.g. `MyAppWeb.BoardLive`.
    pub module: String,
}

/// Collect every `live(...)` host binding declared in the file.
///
/// `@host` lowers to a `host` FormMatch carrying `name` (binding) + `module`; the
/// `live` transport is recognised by the macro variant `host-live`. We accept any
/// `@host` whose `module` capture is present and whose matched macro is the live
/// one (the http/ws variants have no `module`).
pub fn collect_live_hosts(matches: &[FormMatch]) -> Vec<LiveHost> {
    let mut hosts = Vec::new();
    for fm in matches {
        if fm.macro_name != "host" {
            continue;
        }
        // The live variant is the only @host form with a `module` capture.
        let (Some(binding), Some(module)) = (fm.get_binding("name"), live_module(fm)) else {
            continue;
        };
        hosts.push(LiveHost {
            binding: binding.to_string(),
            module: module.to_string(),
        });
    }
    hosts
}

/// Extract the `module` string from a live `@host` match. `module` is captured as
/// an `expr` (the `live($module:expr)` form), surfacing as a string-or-expr value;
/// we read it as a string literal, trimming any surrounding quotes.
fn live_module(fm: &FormMatch) -> Option<String> {
    // `live($module:expr)` captures the module as an Expr carrying the quoted
    // literal (`"MyAppWeb.BoardLive"`); a bare string/ident form is also accepted.
    let raw = fm
        .get_expr("module")
        .map(str::to_string)
        .or_else(|| fm.get_string("module").map(str::to_string))
        .or_else(|| fm.get_ident("module").map(str::to_string))?;
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// A page-side LiveView assign subscription: `@data subscribe $count from $host : count ;`.
#[derive(Debug, Clone, PartialEq)]
pub struct PageSubscription {
    /// The live host binding whose server contract declares the assign.
    pub host_binding: String,
    /// Server assign name requested by the page.
    pub assign: String,
    /// Local target binding that receives the assign value.
    pub target: String,
    /// Source span of the subscribe directive.
    pub span: crate::parser::SourceSpan,
}

/// Collect every `@data subscribe` declaration. The grammar's captures are the
/// canonical decode vocabulary; this comparator deliberately adds no parser.
pub fn collect_page_subscriptions(matches: &[FormMatch]) -> Vec<PageSubscription> {
    matches
        .iter()
        .filter(|fm| fm.macro_name == "data")
        .filter_map(|fm| {
            let (Some(target), Some(host), Some(assign)) = (
                fm.get_binding("name"),
                fm.get_binding("host"),
                fm.get_ident("assign"),
            ) else {
                return None;
            };
            Some(PageSubscription {
                host_binding: host.to_string(),
                assign: assign.to_string(),
                target: target.to_string(),
                span: fm.span,
            })
        })
        .collect()
}

/// A page-side transient stream: each literal receive arm is a push name the
/// bound LiveView must declare. The arm itself remains the canonical receive
/// matcher; this projection only reads the matched grammar data.
#[derive(Debug, Clone, PartialEq)]
pub struct PageStream {
    pub host_binding: String,
    pub tags: Vec<String>,
    pub span: crate::parser::SourceSpan,
}

pub fn collect_page_streams(matches: &[FormMatch]) -> Vec<PageStream> {
    matches
        .iter()
        .filter(|fm| fm.macro_name == "data")
        // Signals also carry receive arms; only passive streams lack `send`/verb.
        .filter(|fm| fm.get_ident("verb").is_none())
        .filter_map(|fm| {
            let host = fm.get_binding("host")?;
            let arms = match fm.captures.get("arms")? {
                crate::syntax::CapturedValue::Array(arms) => arms,
                _ => return None,
            };
            let tags = arms
                .iter()
                .filter_map(|arm| match arm {
                    crate::syntax::CapturedValue::Named(arm) => match arm.get("pat") {
                        Some(crate::syntax::CapturedValue::Named(pattern)) => {
                            match pattern.get("lit") {
                                Some(crate::syntax::CapturedValue::String(tag))
                                | Some(crate::syntax::CapturedValue::Ident(tag)) => {
                                    Some(tag.clone())
                                }
                                _ => None,
                            }
                        }
                        _ => None,
                    },
                    _ => None,
                })
                .collect::<Vec<_>>();
            if tags.is_empty() {
                return None;
            }
            Some(PageStream {
                host_binding: host.to_string(),
                tags,
                span: fm.span,
            })
        })
        .collect()
}

/// Compare stream receive tags against the bound live host's declared pushes.
pub fn compare_streams(
    hosts: &[LiveHost],
    streams: &[PageStream],
    contracts: &HashMap<String, ServerContract>,
) -> Vec<Diagnostic> {
    let module_by_binding: HashMap<&str, &str> = hosts
        .iter()
        .map(|host| (host.binding.as_str(), host.module.as_str()))
        .collect();

    streams
        .iter()
        .flat_map(|stream| {
            let Some(module) = module_by_binding.get(stream.host_binding.as_str()) else {
                return Vec::new();
            };
            let Some(contract) = contracts.get(*module) else {
                return Vec::new();
            };
            stream
                .tags
                .iter()
                .filter(|tag| !contract.push_names().contains(&tag.as_str()))
                .map(|tag| {
                    Diagnostic::error(
                        DiagnosticCode::E0928,
                        format!(
                            "live host `{}` ({}) does not declare push `{}` — the page receives it via `@data stream`, but the server contract declares: [{}]",
                            stream.host_binding,
                            module,
                            tag,
                            contract.push_names().join(", ")
                        ),
                    )
                    .with_span(crate::diagnostics::SourceSpan::from(stream.span))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Collect every page intent: a `@data signal`/`@data stream` whose `verb` is the
/// ws `emit` (a live/ws send) carries the fired event in `target`.
///
/// HTTP signals (`verb` = POST/GET/…) are NOT live-host intents and are skipped —
/// only `emit` targets a phx event name.
pub fn collect_page_intents(matches: &[FormMatch]) -> Vec<PageIntent> {
    let mut intents = Vec::new();
    for fm in matches {
        if fm.macro_name != "data" {
            continue;
        }
        // A reactive-output signal carries verb+target+host (see data-kind.st).
        let (Some(verb), Some(target), Some(host)) = (
            fm.get_ident("verb"),
            fm.get_string("target"),
            fm.get_binding("host"),
        ) else {
            continue;
        };
        // Only the ws `emit` verb names a phx event; HTTP verbs target a URL.
        if !verb.eq_ignore_ascii_case("emit") {
            continue;
        }
        intents.push(PageIntent {
            host_binding: host.to_string(),
            event: target.to_string(),
            span: fm.span,
            decoded_arms: decoded_arms_of(fm),
        });
    }
    intents
}

/// What a signal's `receive` arms decode, preserving PATTERN CATEGORY.
///
/// The authority is `stdlib/primitives/signal.st::armMatches`, and it does NOT
/// treat all patterns alike:
///
/// ```text
/// pat.lit  → String(env.type ?? env.status) === lit     exact
/// pat.name → "_"          → true                         wildcard
///          → "ok"         → env.ok                       semantic
///          → "error"/"err"→ !env.ok                      semantic
///          → "2xx".."5xx" → status class
///          → other        → env.type === name            exact
/// ```
///
/// A first version flattened every `name` pattern to `"_"`. That produced a
/// FALSE PASS for `other => …` (an exact type match, recorded as a catch-all)
/// and for the literal `"_"` (which matches only the tag `_`), and a FALSE FAIL
/// for `error => …`, which matches every non-`ok` reply at runtime.
#[derive(Debug, Clone, PartialEq)]
pub enum DecodedArm {
    /// `"ok" => …` — matches exactly this tag.
    Literal(String),
    /// `other => …` — a bare identifier; matches `env.type === "other"`.
    TypeName(String),
    /// `_ => …` — matches anything.
    Wildcard,
    /// `ok => …` — matches when the reply is successful.
    SemanticOk,
    /// `error`/`err` — matches every NON-ok reply, so it covers any tag but `ok`.
    SemanticErr,
}

impl DecodedArm {
    /// Does this arm decode a reply carrying `tag`?
    ///
    /// Mirrors `armMatches` for the LiveView envelope, where `ok` is set iff the
    /// tag is exactly `"ok"` (see signal.st's liveview callback).
    fn decodes(&self, tag: &str) -> bool {
        match self {
            DecodedArm::Wildcard => true,
            DecodedArm::Literal(t) | DecodedArm::TypeName(t) => t == tag,
            DecodedArm::SemanticOk => tag == "ok",
            DecodedArm::SemanticErr => tag != "ok",
        }
    }
}

fn decoded_arms_of(fm: &FormMatch) -> Vec<DecodedArm> {
    use crate::syntax::CapturedValue as CV;

    let Some(CV::Array(arms)) = fm.captures.get("arms") else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for arm in arms {
        let CV::Named(fields) = arm else { continue };
        let Some(CV::Named(pat)) = fields.get("pat") else {
            continue;
        };

        if let Some(lit) = pat.get("lit") {
            match lit {
                CV::String(s) | CV::Ident(s) => out.push(DecodedArm::Literal(s.clone())),
                _ => {}
            }
        } else if let Some(name) = pat.get("name") {
            let n = match name {
                CV::Ident(s) | CV::String(s) => s.clone(),
                _ => continue,
            };
            out.push(match n.as_str() {
                "_" => DecodedArm::Wildcard,
                "ok" => DecodedArm::SemanticOk,
                "error" | "err" => DecodedArm::SemanticErr,
                // `2xx`..`5xx` classify by HTTP status, which a phx reply has
                // none of; treat as a type name so it never masks a real tag.
                _ => DecodedArm::TypeName(n),
            });
        }
        // `pat.num` matches on numeric status — never a reply TAG, so it
        // contributes nothing rather than being mistaken for one.
    }
    out
}

/// Compare each intent's decoded tags against the tags its server handler can
/// produce — FUP-143 hole #2, the direction Elixir CANNOT check.
///
/// The Elixir side cannot enumerate its own reply tags: they live in handler
/// bodies and can be runtime values, so `@on_definition` sees nothing. A
/// beam-lisp `defcontract` CAN, because the body is data — so when the sidecar
/// declares `replies`, this closes the loop from the page side.
///
/// Silent when a contract declares no `replies` at all: a sidecar written before
/// the field existed must not fail every page. Declaring an empty list is
/// different and IS checked — it asserts the handler never replies, so a page
/// with receive arms for it is decoding something that will never arrive.
pub fn compare_replies(
    hosts: &[LiveHost],
    intents: &[PageIntent],
    contracts: &HashMap<String, ServerContract>,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let module_by_binding: HashMap<&str, &str> = hosts
        .iter()
        .map(|h| (h.binding.as_str(), h.module.as_str()))
        .collect();

    for intent in intents {
        let Some(module) = module_by_binding.get(intent.host_binding.as_str()) else {
            continue;
        };
        let Some(contract) = contracts.get(*module) else {
            continue;
        };
        let Some(ev) = contract.events.iter().find(|e| e.name == intent.event) else {
            continue; // compare() already reports an undeclared event
        };

        // Per-EVENT, not per-contract: a contract may declare replies for some
        // events and not others, and a contract-wide guard would skip the
        // declared ones the moment any event lacked the key.
        let Some(replies) = &ev.replies else {
            continue;
        };

        for tag in replies {
            if !intent.decoded_arms.iter().any(|a| a.decodes(tag)) {
                let decoded = if intent.decoded_arms.is_empty() {
                    "nothing".to_string()
                } else {
                    intent
                        .decoded_arms
                        .iter()
                        .map(|a| match a {
                            DecodedArm::Literal(t) => format!("\"{t}\""),
                            DecodedArm::TypeName(t) => t.clone(),
                            DecodedArm::Wildcard => "_".to_string(),
                            DecodedArm::SemanticOk => "ok".to_string(),
                            DecodedArm::SemanticErr => "error".to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                diags.push(
                    Diagnostic::error(
                        DiagnosticCode::E0928,
                        format!(
                            "live host `{}` ({}) can reply to `{}` with tag `{tag}`, but this \
                             page decodes: [{decoded}] — the reply would be silently dropped",
                            intent.host_binding, module, intent.event
                        ),
                    )
                    .with_span(crate::diagnostics::SourceSpan::from(intent.span)),
                );
            }
        }
    }
    diags
}

/// The pure comparator core: given the live hosts, the page intents, and the
/// per-binding server contracts, return a diagnostic for every intent that fires
/// an event the bound server module does not declare.
///
/// Intents whose host binding has NO loaded contract are SKIPPED (the page may
/// target a ws/http host, or the contract sidecar may be absent — absence is not
/// a mismatch). Only a present-but-disagreeing contract produces E0928.
pub fn compare(
    hosts: &[LiveHost],
    intents: &[PageIntent],
    contracts: &HashMap<String, ServerContract>,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    // binding → module, for resolving an intent's host to its contract.
    let module_by_binding: HashMap<&str, &str> = hosts
        .iter()
        .map(|h| (h.binding.as_str(), h.module.as_str()))
        .collect();

    for intent in intents {
        let Some(module) = module_by_binding.get(intent.host_binding.as_str()) else {
            continue; // intent targets a non-live host (ws/http) — not our concern
        };
        let Some(contract) = contracts.get(*module) else {
            continue; // no server contract loaded for this module — cannot check
        };
        if !contract.event_names().contains(&intent.event.as_str()) {
            let handled = contract.event_names().join(", ");
            diags.push(
                Diagnostic::error(
                    DiagnosticCode::E0928,
                    format!(
                        "live host `{}` ({}) does not handle event `{}` — the page fires it via `send emit`, but the server contract declares: [{}]",
                        intent.host_binding, module, intent.event, handled
                    ),
                )
                // parser::SourceSpan → diagnostics::SourceSpan via the From impl.
                .with_span(crate::diagnostics::SourceSpan::from(intent.span)),
            );
        }
    }
    diags
}

/// Compare subscriptions against their bound live host's declared assigns.
/// A contractless host remains valid (sidecars are opt-in), and a declared assign
/// is valid whether or not the server has pushed it during this page session.
pub fn compare_subscriptions(
    hosts: &[LiveHost],
    subscriptions: &[PageSubscription],
    contracts: &HashMap<String, ServerContract>,
) -> Vec<Diagnostic> {
    let module_by_binding: HashMap<&str, &str> = hosts
        .iter()
        .map(|h| (h.binding.as_str(), h.module.as_str()))
        .collect();

    subscriptions
        .iter()
        .filter_map(|subscription| {
            let module = module_by_binding.get(subscription.host_binding.as_str())?;
            let contract = contracts.get(*module)?;
            if contract.assign_names().contains(&subscription.assign.as_str()) {
                return None;
            }
            let declared = contract.assign_names().join(", ");
            Some(
                Diagnostic::error(
                    DiagnosticCode::E0928,
                    format!(
                        "live host `{}` ({}) does not declare assign `{}` — the page subscribes via `@data subscribe`, but the server contract declares: [{}]",
                        subscription.host_binding, module, subscription.assign, declared
                    ),
                )
                .with_span(crate::diagnostics::SourceSpan::from(subscription.span)),
            )
        })
        .collect()
}

/// Load a server contract sidecar from `<page_stem>.contract.json` next to the
/// entry file, keyed by its declared module. Returns an empty map when no sidecar
/// exists (Wave B is opt-in: a page without a sidecar is simply not contract-checked).
pub fn load_contracts(site_dir: &Path) -> HashMap<String, ServerContract> {
    let mut out = HashMap::new();
    let Ok(entries) = std::fs::read_dir(site_dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".contract.json"))
            && let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(contract) = serde_json::from_str::<ServerContract>(&text)
        {
            out.insert(contract.module.clone(), contract);
        }
    }
    out
}

/// Top-level entry: load sidecar contracts for the site, extract the page's live
/// hosts + intents from the matches, and compare. Returns E0928 diagnostics.
pub fn check_liveview_contracts(site_dir: &Path, matches: &[FormMatch]) -> Vec<Diagnostic> {
    let contracts = load_contracts(site_dir);
    if contracts.is_empty() {
        return Vec::new(); // opt-in: nothing to check against
    }
    let hosts = collect_live_hosts(matches);
    let intents = collect_page_intents(matches);
    let subscriptions = collect_page_subscriptions(matches);
    let streams = collect_page_streams(matches);
    let mut diags = compare(&hosts, &intents, &contracts);
    diags.extend(compare_subscriptions(&hosts, &subscriptions, &contracts));
    diags.extend(compare_streams(&hosts, &streams, &contracts));
    // FUP-143 hole #2: the server→page direction. Silent unless the contract
    // declares `replies`, which only a beam-lisp `defcontract` can currently
    // produce — Elixir cannot enumerate tags out of its own handler bodies.
    diags.extend(compare_replies(&hosts, &intents, &contracts));
    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SourceSpan;
    use crate::syntax::CapturedValue;

    fn contract(module: &str, events: &[&str]) -> ServerContract {
        ServerContract {
            module: module.to_string(),
            events: events
                .iter()
                .map(|e| ContractEvent {
                    name: e.to_string(),
                    replies: None,
                })
                .collect(),
            pushes: vec![],
            assigns: vec![],
            fingerprint: "f".to_string(),
        }
    }

    /// A contract whose events declare the tags their handlers can answer with —
    /// what a beam-lisp `defcontract` emits, and what Elixir cannot produce.
    fn contract_with_replies(module: &str, events: &[(&str, &[&str])]) -> ServerContract {
        ServerContract {
            module: module.to_string(),
            events: events
                .iter()
                .map(|(n, tags)| ContractEvent {
                    name: (*n).to_string(),
                    replies: Some(tags.iter().map(|t| (*t).to_string()).collect()),
                })
                .collect(),
            pushes: vec![],
            assigns: vec![],
            fingerprint: "f".to_string(),
        }
    }

    fn intent_decoding(binding: &str, event: &str, tags: &[&str]) -> PageIntent {
        PageIntent {
            host_binding: binding.to_string(),
            event: event.to_string(),
            span: SourceSpan::default(),
            decoded_arms: tags
                .iter()
                .map(|t| match *t {
                    "_" => DecodedArm::Wildcard,
                    "ok" => DecodedArm::SemanticOk,
                    "error" | "err" => DecodedArm::SemanticErr,
                    other => DecodedArm::Literal(other.to_string()),
                })
                .collect(),
        }
    }

    // ── FUP-143 hole #2 ─────────────────────────────────────────────────────
    // "reply tags live in handler bodies and can be runtime values, so
    //  @on_definition cannot soundly enumerate them"
    // True of Elixir; false of a beam-lisp term, whose body is data.

    #[test]
    fn a_reply_tag_the_page_cannot_decode_is_an_error() {
        let hosts = vec![host("$counter", "M.CounterLive")];
        let contracts: HashMap<String, ServerContract> = [(
            "M.CounterLive".to_string(),
            contract_with_replies("M.CounterLive", &[("dec", &["ok", "err"])]),
        )]
        .into();

        // the page decodes only "ok" — an `err` reply is silently dropped in
        // the browser, which is exactly the runtime surprise this closes
        let intents = vec![intent_decoding("$counter", "dec", &["ok"])];
        let diags = compare_replies(&hosts, &intents, &contracts);

        assert_eq!(diags.len(), 1, "the undecoded tag must be reported");
        assert!(
            diags[0].message.contains("`err`"),
            "the message names the TAG, not just the event: {}",
            diags[0].message
        );
    }

    #[test]
    fn decoding_every_tag_is_silent() {
        let hosts = vec![host("$counter", "M.CounterLive")];
        let contracts: HashMap<String, ServerContract> = [(
            "M.CounterLive".to_string(),
            contract_with_replies("M.CounterLive", &[("dec", &["ok", "err"])]),
        )]
        .into();
        let intents = vec![intent_decoding("$counter", "dec", &["ok", "err"])];
        assert!(compare_replies(&hosts, &intents, &contracts).is_empty());
    }

    #[test]
    fn a_catch_all_arm_decodes_anything() {
        // `_ => Failed($.reply)` cannot drop a reply, whatever the server sends
        let hosts = vec![host("$counter", "M.CounterLive")];
        let contracts: HashMap<String, ServerContract> = [(
            "M.CounterLive".to_string(),
            contract_with_replies("M.CounterLive", &[("dec", &["ok", "err", "weird"])]),
        )]
        .into();
        let intents = vec![intent_decoding("$counter", "dec", &["ok", "_"])];
        assert!(compare_replies(&hosts, &intents, &contracts).is_empty());
    }

    #[test]
    fn a_contract_without_replies_is_not_checked() {
        // Back-compat that matters: every sidecar on disk predates this field.
        // Failing them all would make the feature un-shippable.
        let hosts = vec![host("$counter", "M.CounterLive")];
        let contracts: HashMap<String, ServerContract> =
            [("M.CounterLive".to_string(), contract("M.CounterLive", &["dec"]))].into();
        let intents = vec![intent_decoding("$counter", "dec", &[])];
        assert!(compare_replies(&hosts, &intents, &contracts).is_empty());
    }

    #[test]
    fn a_page_decoding_nothing_is_reported_readably() {
        let hosts = vec![host("$counter", "M.CounterLive")];
        let contracts: HashMap<String, ServerContract> = [(
            "M.CounterLive".to_string(),
            contract_with_replies("M.CounterLive", &[("go", &["ok"])]),
        )]
        .into();
        let intents = vec![intent_decoding("$counter", "go", &[])];
        let diags = compare_replies(&hosts, &intents, &contracts);
        assert_eq!(diags.len(), 1);
        assert!(
            diags[0].message.contains("decodes: [nothing]"),
            "an empty arm list reads as `nothing`, not `[]`: {}",
            diags[0].message
        );
    }

    // ── pattern-category semantics (milestone-1 review) ────────────────────
    // A reviewer proved that flattening every `name` pattern to `"_"` produced
    // false passes and a false fail. The authority is signal.st::armMatches.

    #[test]
    fn a_bare_identifier_arm_is_an_EXACT_type_match_not_a_catch_all() {
        // `other => …` matches env.type === "other" at runtime. Treating it as
        // a wildcard meant a page decoding only `other` passed while the server
        // could send `weird`, which the browser would drop.
        let hosts = vec![host("$c", "M")];
        let contracts: HashMap<String, ServerContract> =
            [("M".to_string(), contract_with_replies("M", &[("go", &["weird"])]))].into();

        let intent = PageIntent {
            host_binding: "$c".to_string(),
            event: "go".to_string(),
            span: SourceSpan::default(),
            decoded_arms: vec![DecodedArm::TypeName("other".to_string())],
        };
        assert_eq!(
            compare_replies(&hosts, &[intent], &contracts).len(),
            1,
            "`other` decodes only the tag `other`, so `weird` must be reported"
        );
    }

    #[test]
    fn an_error_arm_decodes_every_non_ok_tag() {
        // signal.st: `error`/`err` → !env.ok, and the LiveView envelope sets ok
        // only when the tag is exactly "ok". So `error => …` DOES decode
        // `weird` — reporting it was a false fail that would reject correct
        // pages.
        let hosts = vec![host("$c", "M")];
        let contracts: HashMap<String, ServerContract> =
            [("M".to_string(), contract_with_replies("M", &[("go", &["weird", "ok"])]))].into();

        let intent = PageIntent {
            host_binding: "$c".to_string(),
            event: "go".to_string(),
            span: SourceSpan::default(),
            decoded_arms: vec![DecodedArm::SemanticOk, DecodedArm::SemanticErr],
        };
        assert!(compare_replies(&hosts, &[intent], &contracts).is_empty());
    }

    #[test]
    fn the_literal_underscore_is_not_a_wildcard() {
        // `"_" => …` is a LITERAL: it matches only a tag spelled `_`. Recording
        // it as a catch-all silently accepted a page that decodes nothing real.
        let hosts = vec![host("$c", "M")];
        let contracts: HashMap<String, ServerContract> =
            [("M".to_string(), contract_with_replies("M", &[("go", &["ok"])]))].into();

        let intent = PageIntent {
            host_binding: "$c".to_string(),
            event: "go".to_string(),
            span: SourceSpan::default(),
            decoded_arms: vec![DecodedArm::Literal("_".to_string())],
        };
        assert_eq!(compare_replies(&hosts, &[intent], &contracts).len(), 1);
    }

    #[test]
    fn declared_empty_replies_are_CHECKED_absent_are_not() {
        // The distinction a plain `Vec` could not represent: absent means "not
        // declared" (skip), empty means "never replies" (a real claim).
        let hosts = vec![host("$c", "M")];

        let mut declared_empty = contract_with_replies("M", &[("go", &[])]);
        declared_empty.events[0].replies = Some(vec![]);
        let contracts: HashMap<String, ServerContract> =
            [("M".to_string(), declared_empty)].into();
        let intent = PageIntent {
            host_binding: "$c".to_string(),
            event: "go".to_string(),
            span: SourceSpan::default(),
            decoded_arms: vec![DecodedArm::Literal("ok".to_string())],
        };
        // nothing to check: the handler declares no tags, so no tag can be
        // undecoded — but the contract was CONSULTED rather than skipped
        assert!(compare_replies(&hosts, &[intent.clone()], &contracts).is_empty());

        // absent → skipped entirely
        let absent: HashMap<String, ServerContract> =
            [("M".to_string(), contract("M", &["go"]))].into();
        assert!(compare_replies(&hosts, &[intent], &absent).is_empty());
    }

    #[test]
    fn replies_are_checked_PER_EVENT_not_per_contract() {
        // A contract may declare replies for some events and not others. A
        // contract-wide guard skipped the declared ones the moment any event
        // lacked the key.
        let hosts = vec![host("$c", "M")];
        let mut c = contract_with_replies("M", &[("declared", &["ok", "err"])]);
        c.events.push(ContractEvent {
            name: "undeclared".to_string(),
            replies: None,
        });
        let contracts: HashMap<String, ServerContract> = [("M".to_string(), c)].into();

        let intent = PageIntent {
            host_binding: "$c".to_string(),
            event: "declared".to_string(),
            span: SourceSpan::default(),
            decoded_arms: vec![DecodedArm::Literal("ok".to_string())],
        };
        assert_eq!(
            compare_replies(&hosts, &[intent], &contracts).len(),
            1,
            "the declared event is still checked despite a sibling with no key"
        );
    }

    #[test]
    fn an_unbound_host_is_skipped() {
        // a page may target a ws/http host with no contract at all
        let hosts = vec![];
        let contracts: HashMap<String, ServerContract> = HashMap::new();
        let intents = vec![intent_decoding("$other", "go", &[])];
        assert!(compare_replies(&hosts, &intents, &contracts).is_empty());
    }

    fn host(binding: &str, module: &str) -> LiveHost {
        LiveHost {
            binding: binding.to_string(),
            module: module.to_string(),
        }
    }

    fn intent(binding: &str, event: &str) -> PageIntent {
        PageIntent {
            host_binding: binding.to_string(),
            event: event.to_string(),
            span: SourceSpan::default(),
            decoded_arms: vec![],
        }
    }

    fn subscription(binding: &str, assign: &str, target: &str) -> PageSubscription {
        PageSubscription {
            host_binding: binding.to_string(),
            assign: assign.to_string(),
            target: target.to_string(),
            span: SourceSpan::default(),
        }
    }

    fn with_assigns(mut contract: ServerContract, assigns: &[&str]) -> ServerContract {
        contract.assigns = assigns
            .iter()
            .map(|name| ContractAssign {
                name: (*name).to_string(),
            })
            .collect();
        contract
    }

    fn with_pushes(mut contract: ServerContract, pushes: &[&str]) -> ServerContract {
        contract.pushes = pushes
            .iter()
            .map(|name| ContractPush {
                name: (*name).to_string(),
            })
            .collect();
        contract
    }

    fn stream(binding: &str, tags: &[&str]) -> PageStream {
        PageStream {
            host_binding: binding.to_string(),
            tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
            span: SourceSpan::default(),
        }
    }

    fn contracts_map(cs: Vec<ServerContract>) -> HashMap<String, ServerContract> {
        cs.into_iter().map(|c| (c.module.clone(), c)).collect()
    }

    #[test]
    fn declared_stream_push_has_no_diagnostics() {
        let hosts = vec![host("$board", "MyAppWeb.BoardLive")];
        let streams = vec![stream("$board", &["flash"])];
        let contracts = contracts_map(vec![with_pushes(
            contract("MyAppWeb.BoardLive", &[]),
            &["flash"],
        )]);
        assert!(compare_streams(&hosts, &streams, &contracts).is_empty());
    }

    #[test]
    fn unknown_stream_push_is_e0928() {
        let hosts = vec![host("$board", "MyAppWeb.BoardLive")];
        let streams = vec![stream("$board", &["flahs"])];
        let contracts = contracts_map(vec![with_pushes(
            contract("MyAppWeb.BoardLive", &[]),
            &["flash"],
        )]);
        let diagnostics = compare_streams(&hosts, &streams, &contracts);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::E0928);
        assert!(diagnostics[0].message.contains("flahs"));
        assert!(diagnostics[0].message.contains("flash"));
    }

    /// GREEN: a page firing only events the server handles passes clean.
    #[test]
    fn aligned_page_has_no_diagnostics() {
        let hosts = vec![host("$board", "MyAppWeb.BoardLive")];
        let intents = vec![intent("$board", "move_card"), intent("$board", "archive")];
        let cs = contracts_map(vec![contract(
            "MyAppWeb.BoardLive",
            &["move_card", "archive"],
        )]);
        assert!(compare(&hosts, &intents, &cs).is_empty());
    }

    /// RED: firing an event the server does not handle is E0928.
    #[test]
    fn unknown_event_is_e0928() {
        let hosts = vec![host("$board", "MyAppWeb.BoardLive")];
        let intents = vec![intent("$board", "mov_card")]; // typo
        let cs = contracts_map(vec![contract("MyAppWeb.BoardLive", &["move_card"])]);
        let diags = compare(&hosts, &intents, &cs);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagnosticCode::E0928);
        assert!(diags[0].message.contains("mov_card"));
        assert!(diags[0].message.contains("move_card")); // lists what IS handled
    }

    /// An intent whose host has no loaded contract is skipped (not flagged).
    #[test]
    fn intent_without_contract_is_skipped() {
        let hosts = vec![host("$board", "MyAppWeb.BoardLive")];
        let intents = vec![intent("$board", "anything")];
        let cs: HashMap<String, ServerContract> = HashMap::new();
        assert!(compare(&hosts, &intents, &cs).is_empty());
    }

    /// An intent targeting a non-live host binding (ws/http) is not our concern.
    #[test]
    fn intent_to_non_live_host_is_skipped() {
        let hosts = vec![host("$board", "MyAppWeb.BoardLive")];
        let intents = vec![intent("$api", "post_thing")]; // $api is not a live host
        let cs = contracts_map(vec![contract("MyAppWeb.BoardLive", &["move_card"])]);
        assert!(compare(&hosts, &intents, &cs).is_empty());
    }

    /// GREEN: a contract-declared assign may be subscribed even before a server push.
    #[test]
    fn declared_assign_subscription_has_no_diagnostics() {
        let hosts = vec![host("$board", "MyAppWeb.BoardLive")];
        let subscriptions = vec![subscription("$board", "count", "$count")];
        let cs = contracts_map(vec![with_assigns(
            contract("MyAppWeb.BoardLive", &[]),
            &["count"],
        )]);
        assert!(compare_subscriptions(&hosts, &subscriptions, &cs).is_empty());
    }

    /// RED: an undeclared subscription is the same E0928 contract mismatch as an event typo.
    #[test]
    fn unknown_subscription_assign_is_e0928() {
        let hosts = vec![host("$board", "MyAppWeb.BoardLive")];
        let subscriptions = vec![subscription("$board", "coutn", "$count")];
        let cs = contracts_map(vec![with_assigns(
            contract("MyAppWeb.BoardLive", &[]),
            &["count"],
        )]);
        let diags = compare_subscriptions(&hosts, &subscriptions, &cs);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, DiagnosticCode::E0928);
        assert!(diags[0].message.contains("coutn"));
        assert!(diags[0].message.contains("count"));
    }

    /// Contract absence is opt-in, so a subscription with no sidecar stays valid.
    #[test]
    fn subscription_without_contract_is_skipped() {
        let hosts = vec![host("$board", "MyAppWeb.BoardLive")];
        let subscriptions = vec![subscription("$board", "count", "$count")];
        assert!(compare_subscriptions(&hosts, &subscriptions, &HashMap::new()).is_empty());
    }

    #[test]
    fn collect_page_subscriptions_reads_host_assign_and_target() {
        let fm = FormMatch::new("data")
            .capture("name", CapturedValue::Binding("$count".to_string()))
            .capture("host", CapturedValue::Binding("$board".to_string()))
            .capture("assign", CapturedValue::Ident("count".to_string()));
        assert_eq!(
            collect_page_subscriptions(&[fm]),
            vec![subscription("$board", "count", "$count")]
        );
    }

    /// Extraction: a live `@host` FormMatch yields its binding + module.
    #[test]
    fn collect_live_hosts_reads_binding_and_module() {
        let fm = FormMatch::new("host")
            .capture("name", CapturedValue::Binding("$board".to_string()))
            .capture(
                "module",
                CapturedValue::String("MyAppWeb.BoardLive".to_string()),
            );
        let hosts = collect_live_hosts(std::slice::from_ref(&fm));
        assert_eq!(hosts, vec![host("$board", "MyAppWeb.BoardLive")]);
    }

    /// Extraction: an `emit` signal yields a page intent; an HTTP signal does not.
    #[test]
    fn collect_page_intents_only_for_emit_verb() {
        let emit_sig = FormMatch::new("data")
            .capture("verb", CapturedValue::Ident("emit".to_string()))
            .capture("target", CapturedValue::String("move_card".to_string()))
            .capture("host", CapturedValue::Binding("$board".to_string()));
        let http_sig = FormMatch::new("data")
            .capture("verb", CapturedValue::Ident("POST".to_string()))
            .capture("target", CapturedValue::String("/api/x".to_string()))
            .capture("host", CapturedValue::Binding("$api".to_string()));
        let intents = collect_page_intents(&[emit_sig, http_sig]);
        assert_eq!(intents.len(), 1);
        assert_eq!(intents[0].event, "move_card");
        assert_eq!(intents[0].host_binding, "$board");
    }

    #[test]
    fn collect_page_intents_includes_ephemeral_emit() {
        // The ephemeral modifier is transport metadata only: contract validation
        // must see the same emitted event as an ordinary `send emit`.
        let ephemeral_emit = FormMatch::new("data")
            .capture("verb", CapturedValue::Ident("emit".to_string()))
            .capture("target", CapturedValue::String("cursor".to_string()))
            .capture("host", CapturedValue::Binding("$board".to_string()))
            .capture("ephemeral", CapturedValue::Named(HashMap::new()));
        let intents = collect_page_intents(&[ephemeral_emit]);
        assert_eq!(intents, vec![intent("$board", "cursor")]);
        let contracts = contracts_map(vec![contract("MyAppWeb.BoardLive", &["cursor"])]);
        assert!(
            compare(
                &[host("$board", "MyAppWeb.BoardLive")],
                &intents,
                &contracts
            )
            .is_empty()
        );
    }
}
