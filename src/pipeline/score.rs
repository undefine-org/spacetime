//! `@score` — arrangement over one playhead (SIP-001 §Proposal/4, PLAN-128 W4).
//!
//! # R1: the driver owns the time domain
//!
//! A score never declares a duration of its own. It inherits the domain of
//! whatever drives it, read from the `domain:` column of the driver registry:
//!
//! | driver              | `domain:`  | placements read |
//! |---------------------|------------|-----------------|
//! | `time`, `loop`      | declared   | `at 2s for 3s`  |
//! | `scroll`, `visible` | normalized | `for 30%`       |
//! | `clip`              | inherited  | from the parent |
//! | `steps`             | derived    | from the content |
//!
//! So `@score &.scroll(cover) { &x at 2s; }` is E0951: scroll progress is
//! 0..1, and "2 seconds into a scroll" names nothing. The identical placement
//! under `&.time(60s)` is correct.
//!
//! This is ONE check reading a registry column, not a rule per driver. A new
//! score-capable driver declares its domain and is covered — no Rust changes.
//! That is the same discipline the `primitive_args` column restored after
//! BUG-253, where five rows shared one implementation and silently read the
//! first row's data.
//!
//! # Why an error rather than a coercion
//!
//! `at 2s` under a normalized driver COULD be read as "2000, clamped to 1" —
//! that is precisely the silent-nonsense class. An author who writes seconds
//! under a scroll driver has a wrong mental model, and the compiler is the
//! only thing positioned to say so.

use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::metasystem::MetaRegistry;
use crate::parser::meta_ast::{BindArg, BindDecl, BindOutput, BindValue, RegisterValue};
use crate::syntax::CapturedValue;

use super::evaluate::EvaluatedMatch;

/// What a driver's time domain admits as a placement measure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Domain {
    /// The driver's own parameter IS the domain: `&.time(60s)` ⇒ `at 2s`.
    Declared,
    /// Progress is 0..1, so placements are percentages: `&.scroll` ⇒ `for 30%`.
    Normalized,
    /// The span the parent assigned (`&.clip`). The parent decides, and it is
    /// not knowable here.
    Inherited,
    /// Computed from the score's own content (`.steps` ⇒ one step per line).
    Derived,
}

impl Domain {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "declared" => Some(Self::Declared),
            "normalized" => Some(Self::Normalized),
            "inherited" => Some(Self::Inherited),
            "derived" => Some(Self::Derived),
            _ => None,
        }
    }

    /// Does this domain admit a measure of the given kind?
    ///
    /// `Inherited` and `Derived` admit both: the former because the parent
    /// supplies the scale and this pass cannot see it, the latter because a
    /// step index is neither a time nor a ratio. Being permissive here is
    /// deliberate — a false rejection teaches authors to distrust the check.
    fn admits(self, measure: &Measure) -> bool {
        match (self, measure) {
            // A length that is not a ratio names no position in ANY domain.
            (_, Measure::Spatial(_)) => false,
            (Self::Declared, Measure::Time) => true,
            (Self::Declared, Measure::Ratio) => false,
            (Self::Normalized, Measure::Ratio) => true,
            (Self::Normalized, Measure::Time) => false,
            (Self::Inherited | Self::Derived, _) => true,
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Self::Declared => "declared in time",
            Self::Normalized => "normalized (0..1)",
            Self::Inherited => "inherited from its parent",
            Self::Derived => "derived from the score's content",
        }
    }

    /// The spelling this domain wants, for the did-you-mean. Derived from the
    /// column rather than hand-written per driver.
    fn wants(self) -> &'static str {
        match self {
            Self::Declared => "a time (`2s`, `400ms`)",
            Self::Normalized => "a percentage (`30%`)",
            Self::Inherited => "a time or a percentage",
            Self::Derived => "a step index",
        }
    }
}

/// The kind of measure an author actually wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Measure {
    /// `2s`, `400ms` — captured as `CapturedValue::Time`.
    Time,
    /// `30%` — captured as `CapturedValue::Length` with unit `%`.
    Ratio,
    /// `100px`, `4em`, `20vh` — a CSS length that is not a ratio.
    ///
    /// No time domain admits one: a score places clips in TIME or in
    /// PROGRESS, never in pixels. `score_measure` accepts `length` because
    /// that is the capture type carrying `%`, so a stray `100px` reaches here
    /// and must be rejected rather than ignored. Review found it compiling
    /// clean under every driver — a placement the compiler accepted and then
    /// discarded, which is the banned silent class.
    Spatial(String),
}

/// Validate every `@score` placement against its driver's domain.
///
/// Runs over evaluated matches carrying a `score` bind. Emits E0951 per
/// offending placement — per placement rather than per score, because an author
/// fixing one wants to see them all.
pub fn check_score_domains(
    evaluated: &[EvaluatedMatch],
    registry: &MetaRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for ev in evaluated {
        if !ev
            .bind_decls
            .iter()
            .any(|b| b.primitive.as_str() == "score")
        {
            continue;
        }

        let span = ev.form_match.span;
        let captures = &ev.form_match.captures;

        // The driver member names the registry row that owns the domain.
        let Some(member) = captures
            .get("driver")
            .and_then(named_field(&["member"]))
            .and_then(text_of)
        else {
            continue;
        };

        if let Some(planned) = planned_driver_primitive(registry, &member) {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::E0948,
                    format!(
                        "driver `.{member}` is registered but its primitive `{planned}` is PLANNED, not yet bound"
                    ),
                )
                .with_span(crate::diagnostics::SourceSpan::new(span.start, span.end))
                .with_hint(
                    "the word is known so the diagnostic can be precise; the primitive lands with its wave",
                ),
            );
            continue;
        }

        let Some(domain) = domain_of(registry, &member) else {
            // A driver with no `domain:` column cannot drive a score — and
            // saying so is the WHOLE point. Skipping silently (the first cut
            // of this code) meant `@score &.click { … }` compiled clean while
            // every placement in it was unchecked and unreachable: the exact
            // silent-default class this diagnostic exists to abolish. Found
            // in review.
            //
            // NB the failure is stated in terms of the REGISTRY, so adding
            // `domain:` to a row is the fix and the message says so. No Rust
            // change is ever needed to make a new driver score-capable.
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::E0951,
                    format!(
                        "`.{member}` cannot drive a `@score` — it declares no time \
                         domain, so there is nothing to place clips in"
                    ),
                )
                .with_span(crate::diagnostics::SourceSpan::new(span.start, span.end))
                .with_hint(format!(
                    "score-capable drivers are those with a `domain:` column in \
                     stdlib/macros/drivers.st ({}) · an event driver like `.click` \
                     marks a MOMENT and has no timeline of its own; use `@on` for \
                     it, or drive the score with `.time`/`.scroll`/`.clip` and let \
                     the event start it",
                    score_capable_drivers(registry).join(", ")
                )),
            );
            continue;
        };

        for measure in placement_measures(captures.get("entries")) {
            if domain.admits(&measure) {
                continue;
            }
            if let Measure::Spatial(unit) = &measure {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::E0951,
                        format!(
                            "a `@score` placement cannot be a spatial length — `{unit}` \
                             measures distance, and a score places clips in time or in \
                             progress"
                        ),
                    )
                    .with_span(crate::diagnostics::SourceSpan::new(span.start, span.end))
                    .with_hint(format!(
                        "this score is driven by `.{member}`, whose domain is {} — use {}",
                        domain.describe(),
                        domain.wants()
                    )),
                );
                continue;
            }
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::E0951,
                    format!(
                        "this `@score` is driven by `.{member}`, whose time domain is \
                         {} — {} does not name a position in it",
                        domain.describe(),
                        match measure {
                            Measure::Time => "a time",
                            Measure::Ratio => "a percentage",
                            // Handled by the Spatial branch above, which
                            // carries a better message (it names the unit).
                            Measure::Spatial(_) => "a length",
                        }
                    ),
                )
                .with_span(crate::diagnostics::SourceSpan::new(span.start, span.end))
                .with_hint(format!(
                    "use {} instead · a score never declares its own duration: it \
                     inherits the domain of whatever drives it (SIP-001 W4/R1), and \
                     `.{member}` declares `domain:` in stdlib/macros/drivers.st",
                    domain.wants()
                )),
            );
        }
    }

    diagnostics
}

/// The element name a selector addresses: `.shot` -> `shot`.
///
/// Only a simple class selector yields a name. A compound or descendant
/// selector has no single element to inherit a window for, and guessing one
/// would silently join the wrong clip — so it returns None and the score falls
/// back to a synthetic name (which then has no publisher, and the missing-join
/// diagnostic reports it).
fn selector_element(selector: Option<&str>) -> Option<String> {
    let sel = selector?.trim();
    // The naming rule addresses a window BY ELEMENT, and the element is the
    // LAST compound of the selector. Combinators are address, not identity:
    // a consumer may scope its lookup (`.g-kara .w1`) without losing its
    // name — otherwise it falls back to a synthetic `__drive_clip_N` nothing
    // publishes, which compiles clean and never moves.
    let last = sel
        .split(|c: char| c.is_whitespace() || c == '>' || c == '+' || c == '~')
        .filter(|part| !part.is_empty())
        .last()?;
    let name = last.strip_prefix('.')?;
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some(name.replace('-', "_"))
}

/// THE INHERITED-NAMING RULE (W5a/D1) — the one place this is decided.
///
/// A `domain: inherited` driver does not own a clock: its progress is a window
/// some enclosing score already assigned it. So it must address that progress
/// BY ELEMENT, never by a counter, because the two passes that need to meet
/// there never talk to each other:
///
/// - `expand_score_binds` (this file) publishes `__clip_title` for the clip it
///   places, and
/// - `expand_drive_binds` (drivers.rs) lowers `@on &.clip` on `.title` and must
///   watch THAT NAME.
///
/// The name IS the address. That is what lets a score place an element in one
/// rule and the element consume its window in another, with no cross-pass
/// channel and no registry of scores.
///
/// Getting this wrong is silent by construction, and was: relaxing `.clip` to
/// `on_kinds: elem` WITHOUT this rule compiled clean and produced two names
/// that never meet — apply-animations watching `__drive_clip_1` while the score
/// published `__clip_title`. A green build where nothing moves.
///
/// Both call sites route through here so the rule cannot drift between them.
pub(crate) fn inherited_progress_name(selector: Option<&str>) -> Option<String> {
    selector_element(selector).map(|elem| format!("__clip_{elem}"))
}

/// True when this driver INHERITS its progress rather than producing it — the
/// registry's `domain: inherited` column, read as data (R1). A driver becomes
/// score-capable by declaring a domain, not by being named in Rust.
pub(crate) fn driver_inherits_progress(registry: &MetaRegistry, member: &str) -> bool {
    domain_of(registry, member) == Some(Domain::Inherited)
}

/// The primitive a score's driver publishes through, or None when the driver
/// INHERITS its progress (`.clip`, `.playback`) and therefore has nothing of
/// its own to start.
fn planned_driver_primitive(registry: &MetaRegistry, member: &str) -> Option<String> {
    driver_clause(registry, member)
        .and_then(|clause| registry.entry_field(clause, "primitive"))
        .and_then(|value| match value {
            RegisterValue::Ident(primitive) => {
                primitive.strip_prefix("planned:").map(str::to_string)
            }
            _ => None,
        })
}

fn score_primitive_has_name_param(registry: &MetaRegistry, primitive: &str) -> bool {
    score_primitive_has_param(registry, primitive, "name")
}

/// Does the named primitive declare a param of this name? Data-driven: the
/// score lowering hands a driver the params it DECLARES (a step count, a media
/// subject), never a hardcoded per-driver list.
fn score_primitive_has_param(registry: &MetaRegistry, primitive: &str, param: &str) -> bool {
    registry
        .get_primitive(primitive)
        .map(|p| {
            p.params.iter().any(|decl| match decl {
                crate::parser::meta_ast::PrimitiveParam::Typed { name, .. }
                | crate::parser::meta_ast::PrimitiveParam::TypedData { name, .. } => {
                    name == param
                }
                _ => false,
            })
        })
        .unwrap_or(false)
}

/// Resolve a `.steps` `advance:` event-expr (`&.click | &.key(" ")`) to the
/// DOM event names the referenced drivers trigger — the SAME `trigger:` values
/// the `@on` rail reads, so a deck advances on the same events a
/// `@on &.<event>` would bind. A bare event name (`click`) passes through.
/// This is the "reuse @on" discipline: the advance is data, resolved through
/// the registry, never a second event mechanism invented in a primitive.
fn resolve_advance_events(raw: &str, registry: &MetaRegistry) -> String {
    raw.split('|')
        .map(|part| {
            // A driver ref is `&.<member>(…)`; a bare name is already the event.
            let member = part
                .trim()
                .trim_start_matches('&')
                .trim_start_matches('.')
                .split(|c: char| c == '(' || c == ')' || c == ' ')
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if member.is_empty() {
                return String::new();
            }
            // The member's own row's `trigger:` — the event the @on rail binds.
            let trigger = registry
                .entries_of("driver")
                .into_iter()
                .find(|(_, c)| registry.entry_key(c) == Some(member.as_str()))
                .map(|(_, c)| crate::pipeline::drivers::registry_pairs(registry, c, "primitive_args"))
                .unwrap_or_default()
                .into_iter()
                .find(|(k, _)| k == "trigger")
                .map(|(_, v)| v);
            match trigger {
                Some(t) => t,
                // Not a driver ref — keep the author's own event name.
                None => member,
            }
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}
fn score_driver_primitive(registry: &MetaRegistry, member: &str) -> Option<String> {
    let clause = driver_clause(registry, member)?;
    match registry.entry_field(clause, "primitive") {
        Some(RegisterValue::Ident(p)) => {
            // `none` is a POSITIVE declaration, not an absence: this driver
            // produces no clock because its progress is inherited. Distinct
            // from `planned:` (reserved but unbuilt, E0948) and from a missing
            // field (a malformed row). Emitting a bind for a primitive
            // literally named "none" would be a silent no-op — the exact shape
            // this project treats as banned.
            if p == "none" || p.starts_with("planned:") {
                None
            } else {
                Some(p.clone())
            }
        }
        _ => None,
    }
}

/// The registry row for a driver member.
fn driver_clause<'a>(
    registry: &'a MetaRegistry,
    member: &str,
) -> Option<&'a crate::parser::meta_ast::RegistersClause> {
    registry
        .entries_of("driver")
        .into_iter()
        .find(|(_, c)| registry.entry_key(c) == Some(member))
        .map(|(_, c)| c)
}

/// The drivers that CAN drive a score, read from the registry so the
/// did-you-mean is never a stale hand-written list. Sorted for a stable
/// message.
fn score_capable_drivers(registry: &MetaRegistry) -> Vec<String> {
    let entries = registry.entries_of("driver");
    let mut names: Vec<String> = entries
        .iter()
        .filter(|(_, c)| {
            matches!(
                registry.entry_field(c, "domain"),
                Some(RegisterValue::Ident(_)) | Some(RegisterValue::String(_))
            )
        })
        .filter_map(|(_, c)| registry.entry_key(c).map(|k| format!(".{k}")))
        .collect();
    names.sort();
    names
}

/// Read a driver row's `domain:` column.
fn domain_of(registry: &MetaRegistry, member: &str) -> Option<Domain> {
    let entries = registry.entries_of("driver");
    let (_, clause) = entries
        .iter()
        .find(|(_, c)| registry.entry_key(c) == Some(member))?;
    match registry.entry_field(clause, "domain") {
        Some(RegisterValue::Ident(s)) | Some(RegisterValue::String(s)) => Domain::parse(&s),
        _ => None,
    }
}

/// Walk the score body and yield the measure kind of every placement that
/// carries one (`at`, `for`, `in`, `out`). `after`/`during`/`as` reference
/// other clips rather than measuring, so they have nothing to check.
///
/// The shape, as `inspect --layer expansion` reports it:
/// `entries: [{line: {head: {places: [{at: {at: {time: 2000ms}}}], …}}}]`
fn placement_measures(entries: Option<&CapturedValue>) -> Vec<Measure> {
    let mut found = Vec::new();
    if let Some(v) = entries {
        collect(v, &mut found);
    }
    found
}

/// Recursive walk. Deliberately shape-agnostic: it looks for `Time` and
/// percentage `Length` leaves anywhere beneath the entries capture rather than
/// hardcoding the nesting depth of `entries → line → head → places → at → at`.
/// The grammar's nesting is an implementation detail of the capture types and
/// will change as the body grammar grows; what stays true is that a measure
/// leaf is a measure leaf.
fn collect(value: &CapturedValue, out: &mut Vec<Measure>) {
    match value {
        CapturedValue::Time(_) => out.push(Measure::Time),
        CapturedValue::Length(l) if l.unit == "%" => out.push(Measure::Ratio),
        CapturedValue::Length(l) => out.push(Measure::Spatial(l.unit.clone())),
        // PLAN-122 flattened scalar captures to source text (`String("50%")`),
        // so the typed arms above no longer see real measures — parse the
        // text or the domain check is blind to every placement in the score.
        CapturedValue::String(s) => {
            if let Some((v, unit)) = parse_scalar_measure(s) {
                if unit == "%" {
                    out.push(Measure::Ratio);
                } else if duration_ms(v, &unit).is_some() {
                    out.push(Measure::Time);
                } else {
                    out.push(Measure::Spatial(unit));
                }
            }
        }
        CapturedValue::Array(items) => {
            for i in items {
                collect(i, out);
            }
        }
        CapturedValue::Named(map) => {
            // Sorted for deterministic diagnostic ORDER — an author fixing the
            // first of several errors must see a stable list between runs.
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in keys {
                collect(&map[k], out);
            }
        }
        _ => {}
    }
}

/// Read a nested field out of a `Named` capture by path.
fn named_field<'a>(path: &'a [&'a str]) -> impl Fn(&CapturedValue) -> Option<CapturedValue> + 'a {
    move |value: &CapturedValue| {
        let mut cur = value.clone();
        for key in path {
            let CapturedValue::Named(map) = cur else {
                return None;
            };
            cur = map.get(*key)?.clone();
        }
        Some(cur)
    }
}

fn text_of(value: CapturedValue) -> Option<String> {
    match value {
        CapturedValue::Ident(s) | CapturedValue::String(s) | CapturedValue::Binding(s) => Some(s),
        _ => None,
    }
}

// --- Lowering: the window map (W4/R3) ---------------------------------------

/// One clip's placement, resolved into the window its progress derives from.
///
/// Both numbers are in the SCORE'S OWN DOMAIN (milliseconds under `.time`, a
/// 0..1 ratio under `.scroll`) — which is why the domain check runs first and
/// why `score-window` takes a `total` to scale by. The primitive never learns
/// which driver is upstream, and that is precisely what lets one score body
/// run under `&.time` on a page and `&.clip` under render.
#[derive(Debug, Clone, PartialEq)]
struct Window {
    /// The element this clip animates.
    subject: String,
    /// When it starts, in the score's domain.
    offset: f64,
    /// How long it lasts. `0` means "to the end of the score" — the honest
    /// reading of a clip with an `at` but no `for`.
    span: f64,
    /// True when `offset` is a FRACTION of the score's total rather than an
    /// absolute position in the domain. Under a runtime-total driver
    /// (`.playback`, whose total is the media duration resolved per sample) a
    /// percentage cannot be folded to a constant at compile time, so the
    /// fraction is carried into the emitted map and scaled by the RESOLVED
    /// total at sample time (BUG-270). Under a compile-time total this is
    /// always false — the D2/bare-splice scaling has already folded the
    /// fraction into the constant by the time a window is emitted.
    offset_relative: bool,
    /// True when `span` is a fraction of the score's total rather than an
    /// absolute extent — the same runtime-total rule as `offset_relative`.
    span_relative: bool,
    /// `rate 1 -> 2` — the DERIVATIVE of the window map (W4/R3).
    ///
    /// `(from, to)`. A constant rate (`rate 0`, `rate -1`) has from == to.
    /// `None` is the affine default, rate 1 throughout — stored as None rather
    /// than `Some((1.0, 1.0))` so the common case emits no rate machinery at
    /// all and the generated code stays readable.
    rate: Option<(f64, f64)>,
}

/// Rate diagnostics derived from the same integral as `score-window`.
///
/// A window map has one origin. Endpoint signs that differ would require the
/// origin to change within one map, so they are refused rather than given an
/// accidental advance-then-reverse meaning. A same-signed integral that does
/// not consume exactly one clip-length is valid, but suspicious enough to say
/// how much of the authored window/progress is unused.
fn rate_diagnostic(window: &Window) -> Option<Diagnostic> {
    let (from, to) = window.rate?;
    if from * to < 0.0 {
        return Some(
            Diagnostic::error(
                DiagnosticCode::E0954,
                format!(
                    "`{}` has a rate ramp from {from} to {to} that crosses zero",
                    window.subject
                ),
            )
            .with_hint(
                "split this into two clips: one ending at rate 0, then a separate \
                 reversed clip beginning at rate 0. A negative rate starts at the \
                 clip's end, so one window cannot choose both origins.",
            ),
        );
    }

    // `rate 0` is an intentional hold, not an accidental dead window.
    if from == 0.0 && to == 0.0 {
        return None;
    }

    let covered = ((from + to) / 2.0).abs();
    if (covered - 1.0).abs() < f64::EPSILON {
        return None;
    }

    let (unused, suggested_span, description) = if covered > 1.0 {
        // Same-signed rate integral is monotone, so bisection finds exactly
        // where the clip reaches its end without a separate rate-case branch.
        let mut lo = 0.0;
        let mut hi = 1.0;
        for _ in 0..64 {
            let local = (lo + hi) / 2.0;
            let area = (from * local + (to - from) * local * local / 2.0).abs();
            if area < 1.0 {
                lo = local;
            } else {
                hi = local;
            }
        }
        let active = (lo + hi) / 2.0;
        (1.0 - active, window.span * active, "after it finishes")
    } else {
        (
            1.0 - covered,
            window.span / covered,
            "because its progress never reaches the end",
        )
    };
    let percentage = (unused * 100.0).round();
    Some(
        Diagnostic::warning(
            DiagnosticCode::W0718,
            format!(
                "`{}` leaves {percentage:.0}% of its window unused {description}",
                window.subject
            ),
        )
        .with_hint(format!(
            "if that is not deliberate, write `for {:.0}ms` so this rate uses the whole window",
            suggested_span
        )),
    )
}

/// Expand every `score(…)` bind into one `score-window` per clip.
///
/// The score itself emits NO scheduler: it reuses the driver the `@on` rail
/// already builds, and each clip becomes a pure derivation of that driver's
/// progress signal. Sequencing (`->`) is not a runtime concept either — a step
/// with no explicit `at` starts where the previous one ended, which is
/// arithmetic done HERE, at compile time, and baked into the offsets.
pub fn expand_score_binds(
    evaluated: &mut [EvaluatedMatch],
    matches: &[crate::syntax::FormMatch],
    registry: &MetaRegistry,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut counter: usize = 0;

    for ev in evaluated.iter_mut() {
        if !ev.bind_decls.iter().any(|b| b.primitive == "score") {
            continue;
        }
        // This pass CONSUMES the score bind below. When the score is VALID it
        // is lowered into score-window binds (which must still resolve); when
        // the driver cannot drive a score the bind is DROPPED, and the
        // empty-decls fallback in resolve_evaluated would resurrect it from
        // the macro definition (E0956 masking the E0951 that explains the
        // page). The marker is set at the bottom of this iteration, where the
        // outcome is known.
        let span = ev.form_match.span;
        let captures = ev.form_match.captures.clone();
        let ev_selector = ev.form_match.selector.clone();

        let mut expanded: Vec<BindDecl> = Vec::new();
        for decl in std::mem::take(&mut ev.bind_decls) {
            if decl.primitive != "score" {
                expanded.push(decl);
                continue;
            }
            counter += 1;

            // The driver member decides the domain, and the domain decides how
            // a measure converts to the score's scale.
            let Some(member) = captures
                .get("driver")
                .and_then(named_field(&["member"]))
                .and_then(text_of)
            else {
                continue;
            };
            if planned_driver_primitive(registry, &member).is_some() {
                // Already reported by `check_score_domains`. Do not lower an
                // honestly-unbound driver into orphan score windows.
                continue;
            }
            let Some(domain) = domain_of(registry, &member) else {
                // Already reported by `check_score_domains` — a driver that
                // cannot drive a score has nothing to lower. Staying silent
                // here avoids reporting one mistake twice.
                continue;
            };

            // The score's full extent in its own domain: the driver's declared
            // duration where it has one, else the normalized 0..1 unit.
            let total = match domain {
                Domain::Declared => driver_duration(&captures).unwrap_or(1.0),
                _ => 1.0,
            };

            // THE SCORE'S OWN DRIVER. Without this the windows watch a parent
            // signal that nothing publishes — every clip sits at 0 forever,
            // silently. A score's driver is the SAME registry row an `@on`
            // head would use, read through the same helper, so a driver that
            // works for `@on` works here by construction.
            let driver_primitive = score_driver_primitive(registry, &member);

            // BUG-270: whether this score's total is a RUNTIME signal rather
            // than a compile-time constant — true only for a driver that owns
            // its clock yet inherits its scale (`.playback`: the media
            // duration). Computed once here so the window-map lowering AND the
            // emission agree about whether a relative measure stays a fraction
            // (BUG-270) or is folded to a constant (every compile-time total).
            let runtime_total = driver_primitive.is_some() && domain == Domain::Inherited;

            let windows = resolve_windows(
                captures.get("entries"),
                total,
                runtime_total,
                matches,
                registry,
            );
            let mut has_invalid_rate = false;
            for window in &windows {
                if let Some(diagnostic) = rate_diagnostic(window) {
                    has_invalid_rate |= diagnostic.code == DiagnosticCode::E0954;
                    diagnostics.push(
                        diagnostic
                            .with_span(crate::diagnostics::SourceSpan::new(span.start, span.end)),
                    );
                }
            }
            // A sign-crossing map has no semantics. Do not emit a plausible
            // window alongside the error: author input must be refused, never
            // accepted with an arbitrary fallback.
            if has_invalid_rate {
                continue;
            }
            if windows.is_empty() {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::E0951,
                        "this `@score` places no clips — its body has no steps that \
                         resolve to a window"
                            .to_string(),
                    )
                    .with_span(crate::diagnostics::SourceSpan::new(span.start, span.end))
                    .with_hint(
                        "a score body is a sequence of `&element` steps carrying \
                         placements (`at 2s for 3s`) · an empty score compiles to \
                         nothing, which is indistinguishable from one never written",
                    ),
                );
                continue;
            }

            // The score's own progress publication. `as &film` names it; an
            // unnamed score gets a synthetic name, so the clips can reference
            // it either way.
            let score_name = captures
                .get("as")
                .and_then(named_field(&["name"]))
                .and_then(text_of)
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| {
                    // A `.clip`-driven score does NOT invent a name: it adopts
                    // the window its parent already computed for this element
                    // (`__clip_<element>`). That is `domain: inherited` made
                    // concrete — the clip receives its span rather than
                    // deriving one, so the inner body never mentions time and
                    // runs at whatever scale the parent hands it.
                    //
                    // Deriving the name from the SELECTOR means the two passes
                    // never have to talk: the parent writes `__clip_shot`, and
                    // `.shot`'s own score reads `__clip_shot`.
                    // `domain: inherited` has TWO readings, told apart by the
                    // `primitive` column (DATA, not a second rule): a driver
                    // that INHERITS A WINDOW and owns no clock of its own
                    // (`.clip`, `primitive: none`) adopts the window its parent
                    // already computed for this element (`__clip_<element>`);
                    // a driver that INHERITS A SCALE and owns its clock
                    // (`.playback`, `primitive: media-driver`) publishes under
                    // its own name — the inherited-naming rule is about "no
                    // clock of its own", so it keys on the primitive, not the
                    // domain column.
                    if domain == Domain::Inherited && driver_primitive.is_none() {
                        if let Some(name) = inherited_progress_name(ev_selector.as_deref()) {
                            return name;
                        }
                    }
                    format!("__score_{counter}")
                });

            // Windows must watch the signal the primitive ACTUALLY publishes.
            // Most driver primitives name it, but fixed-export primitives such
            // as intersection yield a registry-declared export (`ratio`) and
            // cannot manufacture `score_name`. `.clip` has no primitive, so
            // its adopted parent window remains the score's parent signal.
            let driver_signal_name = driver_primitive
                .as_ref()
                .map(|prim| {
                    if score_primitive_has_name_param(registry, prim) {
                        score_name.clone()
                    } else {
                        driver_clause(registry, &member)
                            .and_then(|clause| registry.entry_field(clause, "progress_export"))
                            .and_then(|value| match value {
                                RegisterValue::Ident(export) => Some(export.clone()),
                                _ => None,
                            })
                            .unwrap_or_else(|| "progress".to_string())
                    }
                })
                .unwrap_or_else(|| score_name.clone());

            // `.clip` is INHERITED (R1): its progress is the window a parent
            // score already assigned, so it has no primitive of its own and
            // publishes nothing here. Every other score-capable driver emits
            // its primitive.
            if let Some(prim) = driver_primitive {
                let driver_params = captures
                    .get("driver")
                    .and_then(named_field(&["driver_params"]));
                // A SUBJECT (`&voice.playback`) names the element the driver
                // OBSERVES — the media element, resolved through ST.ref the
                // same way `@on &voice.playback` resolves it. The score host
                // `&self` stays the SIGNAL scope (where windows resolve), so
                // the subject goes to the primitive's `media` param.
                let subject = captures
                    .get("driver")
                    .and_then(named_field(&["subject"]))
                    .and_then(text_of)
                    .filter(|s| !s.is_empty());
                let mut args = vec![BindArg::Element {
                    name: "self".to_string(),
                    child_selector: None,
                }];
                if let Some(clause) = driver_clause(registry, &member) {
                    // A driver's params ride the SAME registry rail `@on` uses
                    // (param_map, row constants, per-signature typing), so a
                    // driver that works there works here.
                    //
                    // One difference is real and belongs to the score, not the
                    // driver: `@score &.time(12s)` states the score's EXTENT,
                    // while a bare positional arg would land in the
                    // primitive's first slot — `name` — and set the timeline's
                    // name to "12000". A score head's unnamed measure is its
                    // duration, so it is passed as one; named params still
                    // flow through untouched.
                    let mut driver_args = crate::pipeline::drivers::driver_param_args_for(
                        driver_params.as_ref(),
                        registry,
                        &prim,
                        clause,
                    );
                    driver_args.retain(|a| !matches!(a, BindArg::Positional(_)));
                    if domain == Domain::Declared
                        && !driver_args
                            .iter()
                            .any(|a| matches!(a, BindArg::Named { name, .. } if name == "duration"))
                    {
                        driver_args.push(BindArg::Named {
                            name: "duration".to_string(),
                            value: BindValue::Number(total),
                        });
                    }
                    args.extend(driver_args);
                }
                // A DERIVED-domain driver (`.steps`) needs the NUMBER of steps,
                // computed from the score body itself — the one thing the
                // registry cannot supply, and the reason `derived` means what
                // it does. Passed only when the primitive declares a `count`
                // param, so this is a data-driven default, never a per-driver
                // branch.
                if domain == Domain::Derived
                    && score_primitive_has_param(registry, &prim, "count")
                    && !args
                        .iter()
                        .any(|a| matches!(a, BindArg::Named { name, .. } if name == "count"))
                {
                    args.push(BindArg::Named {
                        name: "count".to_string(),
                        value: BindValue::Number(windows.len().max(1) as f64),
                    });
                }
                // A subject names the OBSERVED media element; hand it to the
                // primitive's `media` param when it declares one.
                if let Some(subject) = &subject
                    && score_primitive_has_param(registry, &prim, "media")
                {
                    args.push(BindArg::Named {
                        name: "media".to_string(),
                        value: BindValue::String(subject.clone()),
                    });
                }
                // A score's loop RESTARTS. `loop-driver` defaults to
                // `mode: "pingpong"` — the old `@loop` pulse surface's choice —
                // but a score is an arrangement on a 0..1 playhead, and looping
                // the arrangement means playing it again, not playing it
                // backward: under pingpong every clip's window runs in reverse
                // on the second half-cycle (a title card that wiped IN wipes
                // back OUT), and the rendered film would palindrome instead of
                // cutting to its first frame. Inject `restart` through the same
                // data-driven rail as `count` — only when the driver declares a
                // `mode` param and the author did not supply one, so an explicit
                // `&.loop(12s, mode: "pingpong")` still says what it means.
                if score_primitive_has_param(registry, &prim, "mode")
                    && !args
                        .iter()
                        .any(|a| matches!(a, BindArg::Named { name, .. } if name == "mode"))
                {
                    args.push(BindArg::Named {
                        name: "mode".to_string(),
                        value: BindValue::String("restart".to_string()),
                    });
                }
                // A `.steps` `advance:` event-expr (`&.click | &.key(" ")`)
                // names DRIVERS, not DOM events. Resolve it to the referenced
                // rows' `trigger:` values through the SAME registry the `@on`
                // rail reads — so a deck advances on the same events a
                // `@on &.<event>` would bind, no second event mechanism. A
                // BARE union (`click | keydown`, no `&`) names DOM events
                // directly; normalize the `|` to `,` all the same, so the
                // steps-driver's `,` split binds TWO real listeners instead of
                // one dead `addEventListener("click | keydown")`.
                for arg in &mut args {
                    if let BindArg::Named { name, value } = arg
                        && name == "advance"
                        && let BindValue::String(raw) = value
                    {
                        *value = BindValue::String(resolve_advance_events(raw, registry));
                    }
                }
                // Named-publishing primitives receive the score name. Fixed
                // exports instead travel through the alias channel, exactly
                // as `@on` does: windows always watch `score_name`, never a
                // primitive-specific raw export such as `ratio`.
                let primitive_has_name = score_primitive_has_name_param(registry, &prim);
                if primitive_has_name {
                    args.push(BindArg::Named {
                        name: "name".to_string(),
                        value: BindValue::String(score_name.clone()),
                    });
                }
                let progress_export = driver_clause(registry, &member)
                    .and_then(|clause| registry.entry_field(clause, "progress_export"))
                    .and_then(|value| match value {
                        RegisterValue::Ident(export) => Some(export.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "progress".to_string());
                expanded.push(BindDecl {
                    primitive: prim,
                    args,
                    outputs: vec![BindOutput {
                        name: if primitive_has_name {
                            score_name.clone()
                        } else {
                            progress_export
                        },
                        alias: None,
                    }],
                    span: decl.span,
                });
            }

            // The window map's SCALE (total). A driver that inherits a scale
            // from its OWN clock (`.playback`: the media duration) cannot know
            // it at compile time — `score-window` resolves a SIGNAL-NAME total
            // at runtime instead of a constant. Every other domain stays a
            // compile-time constant (the declared ms, the normalized unit, or
            // the parent's unit for `.clip`, which has no clock).
            let total_bind = if runtime_total {
                BindValue::String(format!("{driver_signal_name}__dur"))
            } else {
                BindValue::Number(total)
            };

            for (i, w) in windows.iter().enumerate() {
                // The window's signal name is derived from the SUBJECT, not
                // from its index, so a nested score driven by `&.clip` can
                // find the window its parent assigned by looking up its own
                // element name. That is the whole join: no cross-pass
                // channel, no registry of scores — the name IS the address.
                // `_i` still disambiguates a subject placed twice in one
                // score.
                // The suffix disambiguates a subject placed TWICE, so it must
                // count PRIOR PLACEMENTS OF THIS SUBJECT — not the loop index.
                // Keyed on `i`, the second CLIP of a score got `_1` even with a
                // different subject, so `&b at 3s` published `__clip_b_1` while
                // `@on &.clip` on `.b` watched `__clip_b`. The first clip always
                // worked, which is precisely why a single-clip test could not
                // see it.
                let repeat = windows[..i]
                    .iter()
                    .filter(|p| p.subject == w.subject)
                    .count();
                let clip_signal = if repeat == 0 {
                    format!("__clip_{}", w.subject.replace('-', "_"))
                } else {
                    format!("__clip_{}_{repeat}", w.subject.replace('-', "_"))
                };
                let _ = &score_name;
                expanded.push(BindDecl {
                    primitive: "score-window".to_string(),
                    args: vec![
                        BindArg::Element {
                            name: "self".to_string(),
                            child_selector: None,
                        },
                        BindArg::Named {
                            name: "name".to_string(),
                            value: BindValue::String(clip_signal.clone()),
                        },
                        BindArg::Named {
                            name: "parent".to_string(),
                            value: BindValue::String(driver_signal_name.clone()),
                        },
                        BindArg::Named {
                            name: "offset".to_string(),
                            value: BindValue::Number(w.offset),
                        },
                        BindArg::Named {
                            name: "span".to_string(),
                            value: BindValue::Number(w.span),
                        },
                        BindArg::Named {
                            name: "total".to_string(),
                            value: total_bind.clone(),
                        },
                        // BUG-270: under a RUNTIME-total driver a percentage
                        // cannot be folded to a constant span at compile time,
                        // so the relative flag rides into the emitted map and
                        // `score-window` scales the fraction by the RESOLVED
                        // total at sample time. Compile-time totals keep today's
                        // constant folding (the flag stays off and the emitted
                        // numbers are already absolute), which is why the flag
                        // is gated on `runtime_total` rather than on the window's
                        // own relative fields.
                        BindArg::Named {
                            name: "offset_rel".to_string(),
                            value: BindValue::Number(if runtime_total && w.offset_relative {
                                1.0
                            } else {
                                0.0
                            }),
                        },
                        BindArg::Named {
                            name: "span_rel".to_string(),
                            value: BindValue::Number(if runtime_total && w.span_relative {
                                1.0
                            } else {
                                0.0
                            }),
                        },
                        BindArg::Named {
                            name: "rate_from".to_string(),
                            value: BindValue::Number(w.rate.map(|r| r.0).unwrap_or(1.0)),
                        },
                        BindArg::Named {
                            name: "rate_to".to_string(),
                            value: BindValue::Number(w.rate.map(|r| r.1).unwrap_or(1.0)),
                        },
                    ],
                    // The publication name, NOT the raw export. `score-window`
                    // declares a `name` param and yields under it, so the
                    // output IS that name — exactly as `drive` does for a
                    // driver primitive with a `name` param. Naming the raw
                    // `progress` export here instead made every clip in every
                    // score publish under one colliding name.
                    outputs: vec![BindOutput {
                        name: clip_signal,
                        alias: None,
                    }],
                    span: decl.span,
                });
            }

            // PLAN-150 W8: emit a transition-edge bind for each transition form
            // placed on a `->` between two clips. A transition rides the clips'
            // own `__clip_<name>` windows (already published above), so it is
            // choreography, not a scheduler: `&from` plays over the outgoing
            // clip's tail, `&to` over the incoming clip's head.
            for edge in transition_edges(captures.get("entries"), matches, registry) {
                let clip_sig = |subject: &str| format!("__clip_{}", subject.replace('-', "_"));
                counter += 1;
                expanded.push(BindDecl {
                    primitive: "transition-edge".to_string(),
                    args: vec![
                        BindArg::Element { name: "self".to_string(), child_selector: None },
                        BindArg::Named { name: "fromSel".to_string(), value: BindValue::String(format!(".{}", edge.from_subject)) },
                        BindArg::Named { name: "toSel".to_string(), value: BindValue::String(format!(".{}", edge.to_subject)) },
                        BindArg::Named { name: "fromSig".to_string(), value: BindValue::String(clip_sig(&edge.from_subject)) },
                        BindArg::Named { name: "toSig".to_string(), value: BindValue::String(clip_sig(&edge.to_subject)) },
                        BindArg::Named { name: "fromKf".to_string(), value: BindValue::String(edge.from_text.clone()) },
                        BindArg::Named { name: "toKf".to_string(), value: BindValue::String(edge.to_text.clone()) },
                        BindArg::Named { name: "overlap".to_string(), value: BindValue::Number(edge.overlap) },
                    ],
                    outputs: Vec::new(),
                    span: decl.span,
                });
            }

            // PLAN-150 W7: emit a score-audio bind for each `@audio(src:) as
            // &name at <t>;` entry. Sound is on the score's ONE transport: the
            // audio seeks/plays to follow the score's own progress signal.
            for a in audio_entries(captures.get("entries")) {
                expanded.push(BindDecl {
                    primitive: "score-audio".to_string(),
                    args: vec![
                        BindArg::Element { name: "self".to_string(), child_selector: None },
                        BindArg::Named { name: "src".to_string(), value: BindValue::String(a.src) },
                        BindArg::Named { name: "name".to_string(), value: BindValue::String(a.name) },
                        BindArg::Named { name: "at".to_string(), value: BindValue::Number(a.at_ms) },
                        BindArg::Named { name: "total".to_string(), value: BindValue::Number(total) },
                        BindArg::Named { name: "progress".to_string(), value: BindValue::String(driver_signal_name.clone()) },
                    ],
                    outputs: Vec::new(),
                    span: decl.span,
                });
            }
        }
        // Consumed WITHOUT replacement: nothing may resolve this match —
        // the empty-decls fallback must not bring the score bind back.
        // (A lowered score leaves score-window binds and clears the marker.)
        ev.binds_consumed = expanded.is_empty();
        ev.bind_decls = expanded;
    }

    diagnostics
}

/// Declared duration (ms) of every `@score` on the page — the renderer's
/// timeline length (PLAN-132 W5).
///
/// The thesis is that a website and a video differ only in which signal drives
/// the timeline, so a render's length is not a new concept to invent: it is the
/// longest playhead the page already declares. That total is computed HERE,
/// with the SAME `domain_of` + `driver_duration` helpers the window-map
/// lowering (`expand_score_binds`) uses, so render and score cannot disagree
/// about what the longest playhead is — and render does not re-parse the
/// source text or the emitted JS to rediscover it.
///
/// Only `declared`-domain scores (`.time`, `.loop`) contribute a duration;
/// `normalized` (`.scroll`) and `inherited` (`.clip`) scores have no
/// author-declared playhead and add nothing to the timeline.
///
/// `matches` must be the page's FULL match set (top-level AND every scope's
/// matches) — scores live inside CSS selectors, which is the common case.
pub fn declared_score_totals(
    matches: &[crate::syntax::FormMatch],
    registry: &MetaRegistry,
) -> Vec<f64> {
    let mut totals = Vec::new();
    for m in matches {
        collect_score_total(m, registry, &mut totals);
    }
    totals
}

/// Walk one match for a declared-domain score head.
fn collect_score_total(
    m: &crate::syntax::FormMatch,
    registry: &MetaRegistry,
    out: &mut Vec<f64>,
) {
    // A `@score` head is the `score-block` macro; its `driver` capture nests
    // as a Named map (`member` + optional `driver_params`), exactly as the
    // window-map lowering reads it. Reject anything that is not a declared-
    // domain score so an un-drivable projection never contributes a length.
    if m.matched_macro.as_deref() == Some("score-block")
        && let Some(member) = m
            .captures
            .get("driver")
            .and_then(named_field(&["member"]))
            .and_then(text_of)
        && let Some(domain) = domain_of(registry, &member)
        && domain == Domain::Declared
        && let Some(total) = driver_duration(&m.captures)
    {
        out.push(total);
    }
}

/// The driver's declared duration, in its own units (`&.time(60s)` → 60000).
fn driver_duration(captures: &std::collections::HashMap<String, CapturedValue>) -> Option<f64> {
    let params = captures
        .get("driver")
        .and_then(named_field(&["driver_params"]))?;
    let mut found = Vec::new();
    collect_durations(&params, &mut found);
    found.first().copied()
}

/// A driver parameter arrives as an unparsed token (`60s`), not a typed Time —
/// `driver_expr` captures `call_arg*`, which keeps values opaque. So the unit
/// is read here rather than assumed.
fn collect_durations(value: &CapturedValue, out: &mut Vec<f64>) {
    match value {
        CapturedValue::Time(ms) => out.push(*ms as f64),
        CapturedValue::Ident(s) | CapturedValue::String(s) | CapturedValue::Expr(s) => {
            if let Some(ms) = parse_duration_text(s) {
                out.push(ms);
            }
        }
        CapturedValue::Array(items) => {
            for i in items {
                collect_durations(i, out);
            }
        }
        CapturedValue::Named(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in keys {
                collect_durations(&map[k], out);
            }
        }
        _ => {}
    }
}

/// `60s` → 60000, `400ms` → 400. Returns None for anything that is not a
/// duration, so a non-time driver param (`cover`) is simply not one.
fn parse_duration_text(s: &str) -> Option<f64> {
    let s = s.trim();
    if let Some(num) = s.strip_suffix("ms") {
        return num.trim().parse::<f64>().ok();
    }
    if let Some(num) = s.strip_suffix('s') {
        return num.trim().parse::<f64>().ok().map(|v| v * 1000.0);
    }
    None
}

/// Walk the score body and resolve each step into a window.
///
/// SEQUENCING IS ARITHMETIC. A step with no explicit `at` starts where the
/// previous one ended, so `->` needs no runtime representation at all: the
/// playhead position is carried along here and baked into each offset. That is
/// what keeps the emitted output a set of pure maps rather than an ordered
/// structure something must walk.
fn resolve_windows(
    entries: Option<&CapturedValue>,
    total: f64,
    runtime_total: bool,
    matches: &[crate::syntax::FormMatch],
    registry: &MetaRegistry,
) -> Vec<Window> {
    let mut windows = Vec::new();
    let mut cursor = 0.0f64;
    // Whether the playhead currently sits in the RELATIVE (fraction-of-total)
    // domain rather than the absolute (ms) domain. 0 is scale-invariant, so
    // the cursor starts neutral; a step made entirely of relative measures
    // pushes it relative, an absolute step pushes it absolute, and a mix is
    // read on each measure's own account (BUG-270). This is what lets a
    // percentage-derived offset under a runtime-total driver scale by the
    // RESOLVED total at sample time instead of a compile-time constant.
    let mut cursor_relative = false;
    let Some(entries) = entries else {
        return windows;
    };
    for step in expand_fragments(steps_of(entries), matches, registry, total, runtime_total) {
        let Some(subject) = step.subject.clone() else {
            // A GAP: no subject, so nothing is placed — but the playhead still
            // moves, which is the entire point. This is what keeps a score
            // relative: `&a for 2s; gap 500ms; &b for 2s;` carries no absolute
            // offsets, so inserting a clip near the top does not renumber
            // everything below it.
            let gap = step.span.unwrap_or(0.0);
            cursor += gap;
            cursor_relative = (cursor_relative || cursor - gap == 0.0)
                && (step.span_relative || gap == 0.0);
            continue;
        };
        let offset = step.at.unwrap_or(cursor);
        // A step with no explicit `at` starts where the playhead is, so its
        // offset is relative exactly when the cursor it inherits is.
        let offset_rel = if step.at.is_some() {
            step.at_relative
        } else {
            cursor_relative
        };
        let span = step.span.unwrap_or(0.0);
        let span_rel = step.span_relative;
        // A span of 0 runs to the end of the score, so the cursor lands there
        // too — a following step would start at the end, which is the honest
        // reading of "this clip has no declared length".
        cursor = offset
            + if span > 0.0 {
                span
            } else {
                (total - offset).max(0.0)
            };
        // The cursor after a step is relative iff the step's offset and span
        // are both relative-or-zero (0 is scale-invariant either way). A step
        // that places an absolute position lands the cursor in the absolute
        // domain regardless of how it got there.
        cursor_relative = (offset_rel || offset == 0.0) && (span_rel || span == 0.0);
        windows.push(Window {
            subject,
            offset,
            span,
            offset_relative: offset_rel,
            span_relative: span_rel,
            rate: step.rate,
        });
    }
    windows
}

/// One step's resolved placements.
#[derive(Default, Debug)]
struct Step {
    subject: Option<String>,
    at: Option<f64>,
    span: Option<f64>,
    rate: Option<(f64, f64)>,
    /// A `@form score --name` splice (R2). Grouping is a FRAGMENT, not a
    /// keyword, so a step may name one instead of an element — and it expands
    /// into that fragment's own steps before any window is computed.
    fragment: Option<String>,
    /// True when `at` was written as a PERCENTAGE rather than a time
    /// (W5a/D2). A percentage is RELATIVE — it means "of the frame this step
    /// sits in" — so when a fragment is spliced with an explicit span, its
    /// relative measures scale to that span. An absolute time does not: it
    /// already names a position in the score's own domain. Under a
    /// runtime-total driver the flag survives into the emitted window, where
    /// the fraction scales by the resolved total at sample time (BUG-270).
    at_relative: bool,
    /// The `for` counterpart of `at_relative`: true when `for` was written as
    /// a percentage. Read independently because a step may mix an absolute
    /// `at` with a relative `for` (or vice versa), and each measure must
    /// scale (or not) on its own account.
    span_relative: bool,
}

/// Flatten the entries capture into steps, in source order.
///
/// Shape (as `inspect --layer expansion` reports it):
/// `entries: [{line: {head: {places: […], subject: {elem: title}}}}]`
fn steps_of(entries: &CapturedValue) -> Vec<Step> {
    let mut out = Vec::new();
    walk_steps(entries, &mut out);
    out
}

fn walk_steps(value: &CapturedValue, out: &mut Vec<Step>) {
    match value {
        CapturedValue::Array(items) => {
            for i in items {
                walk_steps(i, out);
            }
        }
        CapturedValue::Named(map) => {
            // A gap is a step with no subject: it advances the playhead and
            // places nothing. Representing it as a Step (rather than a special
            // case in the resolver) means the cursor arithmetic stays in ONE
            // place — a gap is just a clip you cannot see.
            if let Some(g) = map.get("gap") {
                let mut step = Step::default();
                read_gap(g, &mut step);
                if step.span.is_some() {
                    out.push(step);
                    return;
                }
            }
            // A step is recognizable by carrying a `subject`; anything else is
            // structure on the way down (a line, a head, a `next`).
            if let Some(subject) = map.get("subject") {
                let mut step = Step {
                    subject: element_name(subject),
                    fragment: fragment_name(subject),
                    ..Default::default()
                };
                if let Some(places) = map.get("places") {
                    read_places(places, &mut step);
                }
                out.push(step);
            }
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in keys {
                if k != "subject" && k != "places" {
                    walk_steps(&map[k], out);
                }
            }
        }
        _ => {}
    }
}

/// Splice every `@form score --name` step into that fragment's own steps (R2).
///
/// Grouping is a FRAGMENT, not a `@scene` keyword — so a fragment must actually
/// EXPAND, or `--story` is an opaque subject that animates a nonexistent
/// element. (It was: the launch demo emitted `__clip___story` and neither of
/// the fragment's two clips.)
///
/// A fragment's steps are resolved relative to the splice point: the splice's
/// own `at`/`for` frame the fragment, and its inner placements are read inside
/// that frame. Percentages therefore mean "of the fragment", which is what
/// makes one fragment reusable at different scales — the same property `&.clip`
/// gives a nested score.
fn expand_fragments(
    steps: Vec<Step>,
    matches: &[crate::syntax::FormMatch],
    registry: &MetaRegistry,
    total: f64,
    runtime_total: bool,
) -> Vec<Step> {
    expand_fragments_guarded(steps, matches, registry, total, runtime_total, &mut Vec::new(), None)
}

/// `active` is the splice stack: the fragments currently being expanded, in
/// order. A fragment that appears in its own stack is a CYCLE.
///
/// Without this, `@form score --loopy { &a for 25%; --loopy; }` overflows the
/// stack and ABORTS the compiler — not a diagnostic, not an error exit, a
/// crash. Found by review. A cycle is refused and the step is left in place
/// unexpanded, so the downstream diagnostics still see it rather than the
/// score silently losing an entry.
fn expand_fragments_guarded(
    steps: Vec<Step>,
    matches: &[crate::syntax::FormMatch],
    registry: &MetaRegistry,
    total: f64,
    runtime_total: bool,
    active: &mut Vec<String>,
    current_frame: Option<f64>,
) -> Vec<Step> {
    let mut out = Vec::new();
    for step in steps {
        let Some(name) = step.fragment.clone() else {
            out.push(step);
            continue;
        };
        if active.contains(&name) {
            // A fragment that splices itself, directly or through a chain.
            // Leave the step unexpanded and stop descending.
            out.push(step);
            continue;
        }
        let Some(body) = score_fragment_body(matches, registry, &name) else {
            // An unknown fragment is NOT silently dropped — it stays a step,
            // so the orphan-signal gate and the missing-element diagnostics
            // see it. (A dedicated E0951 for it belongs with BUG-255's name
            // resolution, which owns `during &ghost` and the same class.)
            out.push(step);
            continue;
        };
        let inner = steps_of(&body);
        if inner.is_empty() {
            out.push(step);
            continue;
        }
        // The splice's own placement frames the fragment; an inner step with
        // no explicit `at` follows the previous one, exactly as at top level.
        // The FRAME this splice sits in (W5b/BUG-267): its own `for` span,
        // else the enclosing frame a parent splice established, else None for
        // a bare top-level splice (the scaling falls back to the score's
        // total). Threaded into the recursion so a bare splice inside a framed
        // one inherits the frame rather than the whole score.
        let frame = step.span.or(current_frame);
        active.push(name.clone());
        let expanded_inner =
            expand_fragments_guarded(inner, matches, registry, total, runtime_total, active, frame);
        active.pop();
        // W5a/D2: a fragment's RELATIVE measures scale to the frame this
        // splice sits in. `&head for 50%` means "half of this fragment's
        // span", so `--story for 20s` makes it 10s and `--story for 4s` makes
        // it 2s — the SAME fragment, two scales. That is what makes a fragment
        // genuinely portable across domains (R2), rather than portable only by
        // accident.
        //
        // An ABSOLUTE measure does not scale: `&head at 2s` already names a
        // position in the score's own domain, and rescaling it would silently
        // move a clip the author placed exactly.
        //
        // THE FRAME IS WHATEVER THIS SPLICE SITS IN (W5b/BUG-267): its own
        // `for` span when it has one, else the enclosing frame a parent splice
        // established, else — for a bare top-level splice under an absolute
        // clock like `.time`/`.loop` — the SCORE'S TOTAL. A bare splice keeps
        // the parent's scale, so a relative measure is a fraction of the whole
        // score, the reading that makes `--story` alone behave like its body
        // written inline. Without this, `for 50%` under a `.time(6s)` score
        // stayed the raw 0.5 against `total = 6000` — a half-millisecond
        // window that made the clip jump to its end at parent progress 0.125.
        // Under a normalized domain (`.clip`, `.scroll`, `.steps`) the total
        // is 1.0, so a fraction stays a fraction — unchanged, as D2 promised.
        //
        // BUG-270: a BARE splice under a RUNTIME-TOTAL driver must NOT fold
        // here. `total` is the compile-time placeholder (1.0) the pass would
        // scale against, but the real total is the media duration, resolved
        // per sample — so the fraction is left UNSCALED and its relative flag
        // survives into the emitted window, where `score-window` scales it by
        // the RESOLVED total at sample time. An absolute frame (`for 20s`)
        // is still folded: it is a compile-time position in the domain
        // regardless of who owns the total.
        let fold: Option<f64> = match frame {
            Some(f) => Some(f),
            None if runtime_total => None,
            None => Some(total),
        };
        if let Some(width) = fold {
            for mut s in expanded_inner {
                if s.at_relative {
                    s.at = s.at.map(|v| v * width);
                    // Scaled into its frame's domain, it is no longer relative
                    // — otherwise a nested splice would scale it a second time.
                    s.at_relative = false;
                }
                if s.span_relative {
                    s.span = s.span.map(|v| v * width);
                    // As above: one scale, then it is absolute.
                    s.span_relative = false;
                }
                if let Some(base) = step.at {
                    s.at = Some(base + s.at.unwrap_or(0.0));
                }
                if s.rate.is_none() {
                    s.rate = step.rate;
                }
                out.push(s);
            }
        } else {
            // No fold (runtime-total bare splice): every inner step keeps its
            // fraction and its relative flag, so the window-map lowering below
            // can scale it by the resolved total. Sequencing still applies —
            // the splice's own `at` frames the fragment regardless of domain.
            for mut s in expanded_inner {
                if let Some(base) = step.at {
                    s.at = Some(base + s.at.unwrap_or(0.0));
                }
                if s.rate.is_none() {
                    s.rate = step.rate;
                }
                out.push(s);
            }
        }
    }
    out
}

/// The body of a `@form score --name` declaration.
fn score_fragment_body(
    matches: &[crate::syntax::FormMatch],
    registry: &MetaRegistry,
    name: &str,
) -> Option<CapturedValue> {
    let declaration = matches.iter().find(|m| {
        let is_form_macro = m
            .matched_macro
            .as_deref()
            .map(|mm| {
                registry
                    .get_macro(mm)
                    .and_then(|def| def.registers.as_ref())
                    .is_some_and(|r| r.name == "form")
            })
            .unwrap_or(false);
        is_form_macro
            && m.captures
                .get("name")
                .and_then(|v| text_of(v.clone()))
                // A `@form` declaration stores its name WITH the `--` sigil
                // (`--story`), while a splice's captured form name may arrive
                // either way. Normalize both sides rather than guessing which
                // spelling the capture used.
                .map(|n| n.trim_start_matches("--").to_string())
                .as_deref()
                == Some(name)
    })?;
    // Only a SCORE fragment splices into a score body. A motion or style form
    // named here is a different kind of thing, and silently accepting one
    // would place a keyframe list on a timeline.
    let kind = declaration
        .captures
        .get("kind")
        .and_then(|v| text_of(v.clone()))
        .unwrap_or_default();
    if kind != "score" {
        return None;
    }
    // The declaration captures its lines under `entries`, the same name the
    // `@score` body uses — one grammar, one capture name.
    declaration.captures.get("entries").cloned()
}

/// PLAN-150 W7: a `@audio(src:) as &name at <t>;` entry on a score.
struct AudioEntry {
    src: String,
    name: String,
    at_ms: f64,
}

/// Collect every audio entry in the score body. An audio entry is a `Named`
/// record under the `audio` key of a `score_entry` (the `score_audio` capture),
/// carrying `src`, `name`, and an `at` measure.
fn audio_entries(entries: Option<&CapturedValue>) -> Vec<AudioEntry> {
    let mut out = Vec::new();
    let Some(entries) = entries else { return out };
    fn walk(v: &CapturedValue, out: &mut Vec<AudioEntry>) {
        match v {
            CapturedValue::Array(items) => items.iter().for_each(|i| walk(i, out)),
            CapturedValue::Named(map) => {
                if let Some(CapturedValue::Named(audio)) = map.get("audio") {
                    let src = audio.get("src").and_then(|v| text_of(v.clone())).unwrap_or_default();
                    let name = audio
                        .get("name")
                        .and_then(|v| text_of(v.clone()))
                        .map(|n| n.trim_start_matches('&').to_string())
                        .unwrap_or_default();
                    // The `at` measure is a nested record; find its ms value.
                    let at_ms = audio
                        .get("at")
                        .and_then(|v| first_measure_ms(v))
                        .unwrap_or(0.0);
                    if !src.is_empty() {
                        out.push(AudioEntry { src, name, at_ms });
                    }
                }
                // Recurse into the rest of the structure (lines, entries).
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                for k in keys {
                    if k != "audio" {
                        walk(&map[k], out);
                    }
                }
            }
            _ => {}
        }
    }
    walk(entries, &mut out);
    out
}

/// The first time/ratio measure (ms) anywhere beneath a capture — an `at`
/// placement's value, resolved the same way `collect` reads measures.
fn first_measure_ms(v: &CapturedValue) -> Option<f64> {
    match v {
        CapturedValue::Time(_) => None, // handled via text below (PLAN-122 flattening)
        CapturedValue::String(s) => parse_scalar_measure(s).and_then(|(val, unit)| duration_ms(val, &unit)),
        CapturedValue::Named(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in keys {
                if let Some(ms) = first_measure_ms(&map[k]) {
                    return Some(ms);
                }
            }
            None
        }
        CapturedValue::Array(items) => items.iter().find_map(first_measure_ms),
        _ => None,
    }
}

/// PLAN-150 W8: a transition placed on a score `->` between two clips.
struct TransitionEdge {
    from_subject: String,
    to_subject: String,
    /// The verbatim keyframe body text of the `&from` / `&to` regions
    /// (`opacity: 1 -> 0; easing: …;`). The primitive parses each `prop: a -> b`
    /// line at runtime — no Rust-side reification, so the transition surface can
    /// grow keyframe features without a second parser here.
    from_text: String,
    to_text: String,
    overlap: f64,
}

/// The body (`&from`/`&to` keyframe text) + duration of a `@form transition`.
fn transition_form_body(
    matches: &[crate::syntax::FormMatch],
    registry: &MetaRegistry,
    name: &str,
) -> Option<(String, String)> {
    let declaration = matches.iter().find(|m| {
        let is_form_macro = m
            .matched_macro
            .as_deref()
            .map(|mm| {
                registry
                    .get_macro(mm)
                    .and_then(|def| def.registers.as_ref())
                    .is_some_and(|r| r.name == "form")
            })
            .unwrap_or(false);
        is_form_macro
            && m.captures
                .get("name")
                .and_then(|v| text_of(v.clone()))
                .map(|n| n.trim_start_matches("--").to_string())
                .as_deref()
                == Some(name)
    })?;
    let kind = declaration
        .captures
        .get("kind")
        .and_then(|v| text_of(v.clone()))
        .unwrap_or_default();
    if kind != "transition" {
        return None;
    }
    // The transition body captures `from`/`to` as verbatim keyframe text
    // (transition_body capture, stdlib/capture-types/transition-body.st).
    let body = declaration.captures.get("body")?;
    // The from/to region text arrives as an `Expr` (a balanced run), which
    // `text_of` does not cover — read it directly.
    let read = |field: &str| -> Option<String> {
        match body {
            CapturedValue::Named(m) => m.get(field).and_then(|v| match v {
                CapturedValue::Expr(s)
                | CapturedValue::String(s)
                | CapturedValue::Ident(s) => Some(s.clone()),
                other => text_of(other.clone()),
            }),
            _ => None,
        }
    };
    Some((read("from").unwrap_or_default(), read("to").unwrap_or_default()))
}

/// Walk the score's entries and collect every transition placed on a `->`
/// between two clips. A transition step is a fragment step whose form resolves
/// to a `kind: transition` declaration; its neighbours in source order are the
/// outgoing (`from`) and incoming (`to`) clips.
fn transition_edges(
    entries: Option<&CapturedValue>,
    matches: &[crate::syntax::FormMatch],
    registry: &MetaRegistry,
) -> Vec<TransitionEdge> {
    let mut edges = Vec::new();
    let Some(entries) = entries else { return edges };
    // The raw step sequence in source order (transitions included). `steps_of`
    // keeps a transition as a fragment step (its `score_fragment_body` is None,
    // so it is not spliced) — exactly what lets us see it here as an edge.
    let steps = steps_of(entries);
    for (i, step) in steps.iter().enumerate() {
        let Some(frag) = step.fragment.clone() else { continue };
        // Only a TRANSITION fragment is an edge; a score fragment splices, a
        // clip has a subject not a fragment.
        let Some((from_text, to_text)) = transition_form_body(matches, registry, &frag) else {
            continue;
        };
        // Neighbours: the nearest clip step before and after with a subject.
        let prev = steps[..i].iter().rev().find_map(|s| s.subject.clone());
        let next = steps[i + 1..].iter().find_map(|s| s.subject.clone());
        let (Some(from_subject), Some(to_subject)) = (prev, next) else { continue };
        edges.push(TransitionEdge {
            from_subject,
            to_subject,
            from_text,
            to_text,
            // A sensible default overlap; the transition's own `$d` refining
            // this against the window lengths is a follow-up (the golden's
            // "its duration overlaps the two windows").
            overlap: 0.3,
        });
    }
    edges
}

/// The form name a step's subject applies, when it is a fragment splice.
fn fragment_name(value: &CapturedValue) -> Option<String> {
    match value {
        CapturedValue::Named(map) => {
            let f = map.get("form")?;
            match f {
                CapturedValue::Named(inner) => inner
                    .get("form")
                    .and_then(|v| text_of(v.clone()))
                    .map(|s| s.trim_start_matches("--").to_string()),
                other => text_of(other.clone()).map(|s| s.trim_start_matches("--").to_string()),
            }
        }
        _ => None,
    }
}

/// A gap's own measure, read into the step's span.
fn read_gap(value: &CapturedValue, step: &mut Step) {
    match value {
        CapturedValue::Named(map) => {
            if let Some(v) = map.get("len") {
                if let Some((ms, rel)) = measure_value_kind(v) {
                    step.span = Some(ms);
                    step.span_relative |= rel;
                }
            }
            if step.span.is_none() {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                for k in keys {
                    if step.span.is_none() {
                        read_gap(&map[k], step);
                    }
                }
            }
        }
        other => {
            if let Some((ms, rel)) = measure_value_kind(other) {
                step.span = Some(ms);
                step.span_relative |= rel;
            }
        }
    }
}

/// Read `at`/`for` out of a step's placement list. `in`/`out`/`during`/`after`
/// are source trims and relative placements — they need name resolution over
/// the whole score, which is BUG-255's territory, not this pass's.
fn read_places(value: &CapturedValue, step: &mut Step) {
    match value {
        CapturedValue::Array(items) => {
            for i in items {
                read_places(i, step);
            }
        }
        CapturedValue::Named(map) => {
            if let Some(v) = map.get("at") {
                if let Some((ms, rel)) = measure_value_kind(v) {
                    step.at = Some(ms);
                    step.at_relative |= rel;
                }
            }
            if let Some(v) = map.get("for") {
                if let Some((ms, rel)) = measure_value_kind(v) {
                    step.span = Some(ms);
                    step.span_relative |= rel;
                }
            }
            if let Some(CapturedValue::Named(r)) = map.get("rate") {
                let from = number_value(r.get("from"));
                // `rate 0` is a constant rate, so a missing `to` means "the
                // same throughout" — NOT "ramp to 1". Defaulting to 1 here
                // would silently turn every freeze into a ramp.
                let to = number_value(r.get("to")).or(from);
                if let (Some(f), Some(t)) = (from, to) {
                    step.rate = Some((f, t));
                }
            }
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in keys {
                if k != "at" && k != "for" && k != "rate" {
                    read_places(&map[k], step);
                }
            }
        }
        _ => {}
    }
}

/// A placement's numeric value in the score's domain — milliseconds for a
/// time, a 0..1 fraction for a percentage — AND whether the measure was
/// RELATIVE (a percentage) rather than absolute (a time). W5a/D2 needs the
/// distinction (a relative measure scales when a fragment is spliced into a
/// frame) and BUG-270 needs it to survive into the emitted window under a
/// runtime-total driver.
fn measure_value_kind(value: &CapturedValue) -> Option<(f64, bool)> {
    match value {
        CapturedValue::Time(ms) => Some((*ms as f64, false)),
        CapturedValue::Length(l) if l.unit == "%" => Some((l.value / 100.0, true)),
        // PLAN-122 flattened every CSS scalar capture to its SOURCE TEXT, so a
        // `for 50%` arrives as `String("50%")`, not a typed `Length` — the
        // arms above are for captures that predate (or bypass) the flattening.
        // Parse the text here, at the consumer, or EVERY percentage placement
        // silently loses its span and the score collapses to "first clip owns
        // the whole window, every later clip is dead" (found live in the
        // browser: offsets emitted 0 and 1, spans 0 and 0).
        CapturedValue::String(s) => {
            parse_scalar_measure(s).and_then(|(v, unit)| match unit.as_str() {
                "%" => Some((v / 100.0, true)),
                time => duration_ms(v, time).map(|ms| (ms, false)),
            })
        }
        CapturedValue::Named(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            keys.iter().find_map(|k| measure_value_kind(&map[*k]))
        }
        _ => None,
    }
}

/// Split a scalar's source text into `(value, unit)`: `50%` → `(50.0, "%")`,
/// `2s` → `(2.0, "s")`. The number and unit are ADJACENT by the time a scalar
/// survives its grammar (the `length_unit`/`time_unit` adjacency rule), so a
/// space inside the text means it is not a scalar at all.
fn parse_scalar_measure(s: &str) -> Option<(f64, String)> {
    let s = s.trim();
    let split = s.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))?;
    if split == 0 {
        return None;
    }
    let value = s[..split].parse::<f64>().ok()?;
    let unit = &s[split..];
    if unit.is_empty() || unit.contains(char::is_whitespace) {
        return None;
    }
    Some((value, unit.to_string()))
}

/// A duration in milliseconds, from a value + time unit (`time_unit` in
/// stdlib: `ms | s | m | h | us`). `fps` is a rate, not a duration — None.
fn duration_ms(value: f64, unit: &str) -> Option<f64> {
    match unit {
        "ms" => Some(value),
        "s" => Some(value * 1_000.0),
        "m" => Some(value * 60_000.0),
        "h" => Some(value * 3_600_000.0),
        "us" => Some(value / 1_000.0),
        _ => None,
    }
}

/// A numeric literal from a capture. `rate` takes plain numbers, not measures:
/// a rate is a RATIO of progress to progress, so it is dimensionless.
fn number_value(value: Option<&CapturedValue>) -> Option<f64> {
    match value? {
        CapturedValue::Number(n) => Some(*n),
        CapturedValue::Ident(s) | CapturedValue::String(s) | CapturedValue::Expr(s) => {
            s.trim().parse::<f64>().ok()
        }
        _ => None,
    }
}

fn element_name(value: &CapturedValue) -> Option<String> {
    match value {
        CapturedValue::Element(s) | CapturedValue::Ident(s) => Some(s.clone()),
        CapturedValue::Named(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            keys.iter().find_map(|k| element_name(&map[*k]))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
        #[test]
        fn selector_element_reads_the_last_compound() {
            // The naming rule addresses a window BY ELEMENT. Combinators are
            // address, not identity: `.g-kara .w1` consumes the window owned by
            // the `.w1` element. A descendant consumer must not fall back to a
            // synthetic name nothing publishes (silent dead consumer).
            assert_eq!(super::selector_element(Some(".w1")).as_deref(), Some("w1"));
            assert_eq!(super::selector_element(Some(".g-kara .w1")).as_deref(), Some("w1"));
            assert_eq!(super::selector_element(Some(".stage > .sc1")).as_deref(), Some("sc1"));
            // A compound of two classes has no single element name: refuse,
            // never guess.
            assert_eq!(super::selector_element(Some(".cell.foo")), None);
            assert_eq!(super::selector_element(None), None);
        }
    use super::resolve_advance_events;
    use crate::metasystem::MetaRegistry;

    #[test]
    fn bare_union_normalizes_pipe_to_comma() {
        // Reviewer P2: `advance: click | keydown` (no `&`) used to skip the
        // `&`-gated resolver, so the steps-driver's `,` split saw one dead
        // `"click | keydown"` listener. With the `&` gate gone, a BARE union
        // of DOM events is normalized to a `,` list — two real listeners.
        let reg = MetaRegistry::new();
        assert_eq!(resolve_advance_events("click | keydown", &reg), "click,keydown");
        assert_eq!(resolve_advance_events("click", &reg), "click");
        assert_eq!(
            resolve_advance_events("click | keydown | pointerdown", &reg),
            "click,keydown,pointerdown"
        );
        // No `|` may survive the normalization (the dead-listener shape).
        let out = resolve_advance_events("click | keydown", &reg);
        assert!(!out.contains('|'), "a `|` in the resolved value makes a dead listener: {out}");
    }
}
