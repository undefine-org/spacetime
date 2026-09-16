//! Value-level type inference (FEAT-109 W0).
//!
//! # What this is
//!
//! Given a literal as it appears in source — `#FF0020`, `8px`, `600ms` — say
//! what type it is, with NO annotation anywhere. This is the thing that makes
//! Spacetime a type-inferred language at the value level rather than a
//! type-annotated one wearing the costume of a type-inferred one.
//!
//! # Why it is a separate function from `capture_type_accepts`
//!
//! It runs the SAME grammars — the `%capture_type` productions named by the
//! `%scalar_type` rows in `stdlib/scalars/types.st`. It differs in POLARITY, and
//! the polarity is the entire design:
//!
//! ```text
//!   refusal      (capture_type_accepts)   unknown -> ACCEPT   never block a build
//!   recognition  (this module)            unknown -> ABSTAIN  never invent a type
//! ```
//!
//! `capture_type_accepts` answers "should I refuse this value?", and every one
//! of its terminal arms defaults to accept: an unknown production name is "not
//! ours to refuse", a failed match on a missing definition is accept, and a
//! token reference is accepted by every scalar deliberately (a scalar accepts a
//! reference to itself). Those defaults are correct there — a false refusal
//! blocks a build the author cannot fix, while a missed diagnostic costs five
//! minutes.
//!
//! They are exactly inverted here. Measured, before this module existed
//! (`tests/infer_feasibility_probe.rs`): naively reversing the
//! `%scalar_type` -> `%capture` join reports `date` — a row that names no
//! production at all — as the UNIQUE type of `"garbage ~~~ nonsense"`, and
//! reports 17 of 19 sample literals as ambiguous across up to 13 scalars.
//!
//! So this is a second function over a shared substrate, not a refactor of the
//! first. The grammars are what keep the two aligned by construction, which is
//! the "no second parser" rule FEAT-109 set as its risk mitigation.
//!
//! # Why the answer is not `Option<Scalar>`
//!
//! Some ambiguity is real and must stay visible:
//!
//! - `100%` is a length AND a percentage. The length grammar includes
//!   percentage by design, so this is not a defect to be resolved — it is a true
//!   plural answer, and callers that need one type must say which they want.
//! - `600` is a number. It is NOT a duration. The unit belongs to the
//!   DECLARATION SITE, not the spelling (PLAN-122 C7): a blanket
//!   "a number can be a time" once made every staggered animation 1000x too slow
//!   across twenty files with a green build. An inferrer that breaks that tie by
//!   picking is that incident with a new face.
//!
//! Hence [`Inferred::Ambiguous`], which is a RESULT and not an error: it carries
//! every candidate so a caller with context can narrow it, and a caller without
//! context can decline to guess.
//!
//! # Why a reference is an answer rather than a failure
//!
//! `--ink` and `$brand.ink` are not untypable — they are [`Inferred::Reference`],
//! and what they refer to is a question for RESOLUTION (a name -> declaration
//! lookup that is global and may not terminate), not for recognition (text ->
//! production, total and local). Collapsing the two is what made value typing
//! feel paradoxical for most of this arc.

use std::collections::HashSet;
use std::sync::OnceLock;

/// What a literal was inferred to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inferred {
    /// Exactly one scalar's production matched: the literal IS that scalar.
    Scalar(String),
    /// More than one scalar matched and none is more specific than the others.
    /// Sorted, so the answer is stable to compare and to print. Callers with
    /// context (a declared param type, a property slot) narrow it; callers
    /// without context MUST NOT pick.
    Ambiguous(Vec<String>),
    /// A name standing for a value supplied elsewhere: `--ink`, `var(--ink)`,
    /// `$brand.ink`. Not a failure — a handoff to resolution.
    Reference,
    /// Nothing matched. The correct answer for a compound value (`1px solid
    /// red`), for nonsense, and for the empty string.
    Unknown,
}

/// The scalars this module can infer: every `%scalar_type` row whose `%capture`
/// names a production that ACTUALLY EXISTS.
///
/// A row naming a missing production (today: `date`, `url`, `string` — none has
/// a `%capture_type`) is excluded, because `capture_type_accepts` answers `true`
/// for such a name on every input. Including them is precisely how the naive
/// reverse join concluded that `"garbage ~~~ nonsense"` is a `date`.
pub fn inferrable_scalars() -> &'static HashSet<String> {
    static CACHE: OnceLock<HashSet<String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        scalar_productions()
            .iter()
            .map(|(id, _)| id.clone())
            .collect()
    })
}

/// Is `name` a `%capture_type` that actually exists in the registry?
///
/// Public because the acceptance gate needs to assert the *reason* a scalar was
/// excluded, rather than accepting any exclusion.
pub fn production_exists(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    if crate::syntax::STDLIB_REGISTRY
        .capture_types()
        .any(|c| c.name == name)
    {
        return true;
    }
    // A `%capture` may also name a BUILTIN the parser maps to a `CaptureType`
    // variant rather than a stdlib grammar — `number` and `bool` both do
    // (stdlib/scalars/types.st). Those have real extractors and are perfectly
    // inferrable; only a name that is NEITHER is the accept-everything hazard.
    // `Custom` is precisely "matched no builtin", which is why it is the test.
    !matches!(
        crate::parser::parse_capture_type_public(name),
        crate::parser::meta_ast::CaptureType::Custom(_)
    )
}


/// `(scalar id, production name)` for every row we can actually run, ordered so
/// that a more specific scalar is tried before a more general one.
fn scalar_productions() -> &'static Vec<(String, String)> {
    static CACHE: OnceLock<Vec<(String, String)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let (registry, _errors) = crate::compiler::cached_stdlib_registry();
        let mut rows: Vec<(String, String)> = registry
            .scalar_types()
            .filter(|r| production_exists(&r.capture))
            .map(|r| (r.id.clone(), r.capture.clone()))
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows
    })
}


/// The production to ask when the question is "IS this a T?" rather than "is
/// this legal in a T position?".
///
/// stdlib already splits these where it matters: `color` admits `inherit` and
/// `$brand.ink` because both are legal in a colour slot, while `color_core`
/// admits neither because neither IS a colour
/// (stdlib/capture-types/css-values.st documents the split, and
/// `background_route` was the consumer that forced it). Inference is a routing
/// consumer by definition, so it must ask the second question wherever the
/// grammar offers it.
///
/// Discovered by convention, checked by existence: a `<name>_core` production is
/// used when one exists, so a future scalar that splits the same way inherits
/// this for free and one that does not is unaffected. No list to keep in sync —
/// which is the same rule the rest of this arc has followed eight times.
///
/// Returns an owned name: it is derived, at most once per row, behind a cached
/// row list, and the allocation is not worth a lifetime dance to avoid.
fn recognition_production(production: &str) -> String {
    static CORES: OnceLock<HashSet<String>> = OnceLock::new();
    let cores = CORES.get_or_init(|| {
        crate::syntax::STDLIB_REGISTRY
            .capture_types()
            .filter(|c| c.name.ends_with("_core"))
            .map(|c| c.name.clone())
            .collect()
    });
    let core = format!("{production}_core");
    if cores.contains(&core) {
        core
    } else {
        production.to_string()
    }
}

/// Does `loser` lose to `winner`?
///
/// Read from the `%loses_to` clause on each `%scalar_type` row
/// (stdlib/scalars/types.st). This WAS a `const LOSES_TO` table in Rust; moving
/// it to the registry is FEAT-109 W5, and it is the same move this arc has made
/// eight times — a fact about the language belongs where the language is
/// described, not in a match arm a stdlib author cannot see or edit.
///
/// Adding a `%scalar_type` therefore needs no Rust change to participate in
/// precedence, which is the acceptance test for the whole `%scalar_type` design.
fn subsumes(loser: &str, winner: &str) -> bool {
    static CACHE: OnceLock<HashSet<(String, String)>> = OnceLock::new();
    let pairs = CACHE.get_or_init(|| {
        let (registry, _) = crate::compiler::cached_stdlib_registry();
        registry
            .scalar_types()
            .flat_map(|row| {
                row.loses_to
                    .iter()
                    .map(|w| (row.id.clone(), w.clone()))
                    .collect::<Vec<_>>()
            })
            .collect()
    });
    pairs.contains(&(loser.to_string(), winner.to_string()))
}

/// Infer the type of a literal as written in source.
///
/// Recognition polarity: this answers [`Inferred::Unknown`] whenever it cannot
/// positively match, and never guesses. See the module docs for why that is the
/// opposite of `capture_type_accepts` and why both are correct.
pub fn infer_value_type(text: &str) -> Inferred {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Inferred::Unknown;
    }

    // A reference is its own answer, and it must be decided BEFORE the scalar
    // productions run — every scalar accepts a reference to itself, so asking
    // the grammars first yields all thirteen at once (measured in the probe).
    if is_reference(trimmed) {
        return Inferred::Reference;
    }

    let mut hits: Vec<String> = Vec::new();
    for (id, production) in scalar_productions() {
        if crate::syntax::events::capture_type_accepts(&recognition_production(production), trimmed)
        {
            hits.push(id.clone());
        }
    }

    match hits.len() {
        0 => Inferred::Unknown,
        1 => Inferred::Scalar(hits.remove(0)),
        _ => {
            // Drop any candidate that a surviving candidate is more specific
            // than. What remains is either a single answer or a genuine plural.
            let surviving: Vec<String> = hits
                .iter()
                .filter(|cand| !hits.iter().any(|other| subsumes(cand, other)))
                .cloned()
                .collect();
            match surviving.len() {
                0 => Inferred::Unknown,
                1 => Inferred::Scalar(surviving.into_iter().next().unwrap()),
                _ => Inferred::Ambiguous(surviving),
            }
        }
    }
}
/// Infer the type of a literal that appears in a KNOWN SLOT.
///
/// `slot` is the type the surrounding declaration asks for — a form's
/// `$duration:duration`, a property's declared scalar. Where
/// [`infer_value_type`] must abstain because a literal's shape does not settle
/// the question, the slot often does settle it.
///
/// # Narrowing, never widening
///
/// The slot may only choose among answers recognition ALREADY reached, plus one
/// carefully bounded case below. It can never overrule an unambiguous literal:
///
/// ```text
///   "600"     in a `duration` slot  -> duration   the slot supplies the unit
///   "600"     with no slot          -> number     nothing said otherwise
///   "100%"    in a `length` slot    -> length     a tie, broken by context
///   "#FF0020" in a `duration` slot  -> COLOR      a type error to report,
///                                                 not a value to reinterpret
/// ```
///
/// That last line is the discipline. A blanket "a number can be used as a time"
/// once made every staggered animation in the corpus 1000× too slow — twenty
/// files, silently, with a fully green build, because nothing was wrong at any
/// single site. A slot that could reinterpret settled values would reintroduce
/// exactly that failure with better manners.
///
/// # The one widening, and why it is safe
///
/// A bare number in a dimensional slot (`duration`, `time`, `length`, `angle`,
/// `percentage`) is admitted AS that slot's type. This is the unit problem, and
/// it is the reason the wave exists: the unit belongs to the DECLARATION SITE,
/// not the spelling, so `stagger: 0.05` is seconds and `duration: 600` is
/// milliseconds and neither number can know that alone.
///
/// It is safe here and was catastrophic as a subtype edge because it is
/// TERNARY: the answer depends on the value, the slot, AND the fact that a slot
/// was stated at all. `number ≤ time` as a global relation applies everywhere,
/// including where no one asked. This applies only where an author wrote the
/// slot down.
pub fn infer_value_type_in_context(text: &str, slot: Option<&str>) -> Inferred {
    let base = infer_value_type(text);

    let Some(slot) = slot else {
        return base;
    };

    // A slot naming a type the language does not have licenses nothing.
    //
    // HONESTY NOTE: deleting this check does NOT fail any test, and I am keeping
    // it anyway with the reason written down rather than pretending it is
    // load-bearing. Both downstream paths already refuse an unknown name —
    // `slot_supplies_a_unit` goes through `production_for`, which returns `None`
    // for a name with no scalar row, and the narrowing branch can only choose a
    // candidate recognition already produced. So this is DEFENCE IN DEPTH, not a
    // mechanism, and it earns its place by making the precondition explicit at
    // the top of the function instead of implicit in two callees.
    //
    // If a future edit gives the widening branch another way to fire, this is
    // the line that stops it, and `an_unknown_slot_cannot_widen_a_bare_number`
    // is the gate that will notice.
    if !inferrable_scalars().contains(slot) {
        return base;
    }

    match base {
        // A tie the slot is a member of: the slot chooses. This is pure
        // narrowing — the answer was already in the set.
        Inferred::Ambiguous(candidates) => {
            if candidates.iter().any(|c| c == slot) {
                Inferred::Scalar(slot.to_string())
            } else {
                Inferred::Ambiguous(candidates)
            }
        }

        // The bounded widening: a bare number in a dimensional slot takes the
        // slot's type, because the unit lives at the declaration site. Any OTHER
        // settled scalar stays exactly what it is — a colour in a duration slot
        // is a colour, and a type error for the caller to report.
        Inferred::Scalar(found) => {
            if found == "number" && slot_supplies_a_unit(slot) {
                Inferred::Scalar(slot.to_string())
            } else {
                Inferred::Scalar(found)
            }
        }

        // A reference is a resolution question and a slot does not answer it.
        // Unknown means recognition found nothing; a slot cannot conjure a type
        // from a value that matched no grammar at all.
        other => other,
    }
}

/// Does this slot carry a unit that a bare number can adopt?
///
/// Derived from the scalar's own grammar rather than listed — but the derivation
/// has to ask the RIGHT question, and my first attempt asked the wrong one.
///
/// The wrong question: "does this production refuse a bare number?" `color`
/// refuses `600`, so that test called `color` unit-bearing and
/// `infer_value_type_in_context("600", Some("color"))` answered `color`. A slot
/// was supplying a KIND, not a unit — precisely the widening this wave exists to
/// forbid. Mutation testing did not catch it (both mutations left the suite
/// green); `a_unitless_slot_does_not_absorb_a_bare_number`, written to close
/// that hole, failed on the first run and found it.
///
/// The right question: "is this production a NUMBER PLUS A UNIT?" — which is
/// answered positively, by showing the grammar accepts the same magnitude in
/// several unit spellings while refusing it bare. A scalar that accepts
/// `1px`/`1rem`/`1%` or `1s`/`1ms` is dimensional; `color` accepts none of them
/// and is not.
///
/// Derivation still beats a list: a `%scalar_type` added tomorrow gets the right
/// answer with no edit here, and what makes a slot unit-bearing stays a fact
/// about its language rather than about its name.
fn slot_supplies_a_unit(slot: &str) -> bool {
    static CACHE: OnceLock<std::sync::Mutex<std::collections::HashMap<String, bool>>> =
        OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    if let Some(hit) = cache.lock().unwrap().get(slot) {
        return *hit;
    }

    // One magnitude, many unit spellings. A dimensional scalar accepts at least
    // one of these; a non-dimensional one accepts none. Kept small and
    // representative rather than exhaustive — the question is whether the
    // production has a unit AXIS at all, not which units it permits.
    const DIMENSIONED: [&str; 8] = ["1px", "1rem", "1em", "1%", "1s", "1ms", "1deg", "1turn"];
    const BARE: [&str; 3] = ["600", "0", "1.5"];

    let answer = match production_for(slot) {
        Some(production) => {
            let accepts_dimensioned = DIMENSIONED
                .iter()
                .any(|d| crate::syntax::events::capture_type_accepts(&production, d));
            let accepts_bare = BARE
                .iter()
                .any(|n| crate::syntax::events::capture_type_accepts(&production, n));
            // Dimensional AND not already numeric: `number` accepts both, and a
            // slot that already takes bare numbers has no unit to supply.
            accepts_dimensioned && !accepts_bare
        }
        None => false,
    };

    cache.lock().unwrap().insert(slot.to_string(), answer);
    answer
}

/// The `%capture` production a scalar row names, if the row exists.
fn production_for(scalar: &str) -> Option<String> {
    scalar_productions()
        .iter()
        .find(|(id, _)| id == scalar)
        .map(|(_, production)| production.clone())
}
/// Is this a name standing for a value supplied elsewhere?
///
/// Deliberately delegates to the same predicate the refusal path uses, so the
/// two agree on what a reference IS even though they disagree on what to do
/// about one.
fn is_reference(text: &str) -> bool {
    crate::syntax::events::is_token_reference(text) || text.starts_with('$')
}
