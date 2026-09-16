//! FEAT-109 feasibility probe — is value-level inference already derivable from
//! the two tables that exist today, with no new mechanism?
//!
//! The claim under test: a `%scalar_type` row names its production via `%capture`
//! (stdlib/scalars/types.st), and `capture_type_accepts` can run any production
//! against any text. So inference is the REVERSE of the join the table already
//! declares: for a literal, find the scalar whose production accepts it.
//!
//! This probe asserts NOTHING about the design. It MEASURES what today's tables
//! answer, so the wave plan is written against observed behaviour rather than a
//! guess. Run with --nocapture and read the table.

use spacetime::syntax::events::capture_type_accepts;

/// Every scalar row, with the production it names.
fn scalar_rows() -> Vec<(String, String)> {
    let reg = spacetime::compiler::cached_stdlib_registry().0;
    reg.scalar_types()
        .map(|r| (r.id.clone(), r.capture.clone()))
        .collect()
}

#[test]
fn probe_what_the_scalar_table_says() {
    let rows = scalar_rows();
    println!("\n=== %scalar_type rows: {} ===", rows.len());
    for (id, capture) in &rows {
        println!(
            "  {:<12} -> %capture {:?}{}",
            id,
            capture,
            if capture.is_empty() {
                "   (base scalar, no production)"
            } else {
                ""
            }
        );
    }
    assert!(!rows.is_empty(), "no scalar rows loaded");
}

/// The literals FEAT-109 named as Tier-1 targets, plus the shapes that must NOT
/// infer (ambiguity is the whole risk).
const SAMPLES: &[&str] = &[
    // unambiguous colour
    "#FF0020",
    "#e8eef7",
    "oklch(70% 0.1 200)",
    "rgb(1 2 3)",
    "chartreuse",
    // unambiguous dimension
    "8px",
    "1.5rem",
    "100%",
    "45deg",
    // time
    "600ms",
    "2s",
    // easing
    "ease-out",
    "cubic-bezier(.4,0,.2,1)",
    // bare number — AMBIGUOUS by construction
    "600",
    "0",
    // not a scalar at all
    "--ink",
    "$brand.ink",
    "1px solid red",
    "garbage ~~~ nonsense",
];

#[test]
fn probe_which_scalars_accept_which_literals() {
    let rows: Vec<(String, String)> = scalar_rows()
        .into_iter()
        .filter(|(_, c)| !c.is_empty())
        .collect();

    println!("\n=== reverse join: literal -> accepting scalars ===");
    let mut unique = 0usize;
    let mut ambiguous = 0usize;
    let mut none = 0usize;

    for s in SAMPLES {
        let hits: Vec<&str> = rows
            .iter()
            .filter(|(_, capture)| capture_type_accepts(capture, s))
            .map(|(id, _)| id.as_str())
            .collect();

        let verdict = match hits.len() {
            0 => {
                none += 1;
                "NONE"
            }
            1 => {
                unique += 1;
                "UNIQUE"
            }
            _ => {
                ambiguous += 1;
                "AMBIGUOUS"
            }
        };
        println!("  {:<24} {:<10} {:?}", format!("{:?}", s), verdict, hits);
    }

    println!(
        "\n  unique={} ambiguous={} none={} (of {})",
        unique,
        ambiguous,
        none,
        SAMPLES.len()
    );
    println!("  NB: UNIQUE is what Tier 1 can infer with zero new mechanism.");
}
