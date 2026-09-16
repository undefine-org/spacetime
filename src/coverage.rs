//! Macro-expansion coverage (PLAN-027 W6) — the introspection moat.
//!
//! Records which macros (and their specific matched forms) were actually
//! EXERCISED during compilation, so a test suite can be measured for *directive*
//! coverage: "you never tested `@when … drag`", "claim op `matches` never
//! exercised", "primitive X never mounted". No other web framework can offer
//! this because it requires owning the macro layer.
//!
//! Mechanism: a thread-local sink that the resolve step pokes with each resolved
//! `(directive, matched_macro)`. The CLI enables it, compiles the test corpus,
//! then compares the exercised set against the registry's full macro list
//! (`MetaRegistry::iter_macros`) to compute coverage + the gap list. Read-only:
//! emitted code is unchanged.

use std::cell::RefCell;
use std::collections::BTreeSet;

thread_local! {
    /// When `Some`, expansion records exercised macro identities here. `None`
    /// disables recording (zero overhead on the normal compile path).
    static SINK: RefCell<Option<CoverageSink>> = const { RefCell::new(None) };
}

/// The set of exercised macro identities collected during a coverage run.
#[derive(Debug, Default, Clone)]
pub struct CoverageSink {
    /// Macro names that resolved at least once (e.g. `then-claims`, `when`).
    pub macros: BTreeSet<String>,
    /// Directive names that matched at least once (e.g. `then`, `when`).
    pub directives: BTreeSet<String>,
}

/// Begin recording expansion coverage on this thread. Idempotent (resets).
pub fn begin() {
    SINK.with(|s| *s.borrow_mut() = Some(CoverageSink::default()));
}

/// Stop recording and return the collected sink, or `None` if not recording.
pub fn end() -> Option<CoverageSink> {
    SINK.with(|s| s.borrow_mut().take())
}

/// Is coverage recording active on this thread?
pub fn is_active() -> bool {
    SINK.with(|s| s.borrow().is_some())
}

/// Record that `directive` matched, resolving to macro `matched` (which may be a
/// specific multi-form macro like `then-claims` for directive `then`). Cheap
/// no-op when recording is inactive.
pub fn record(directive: &str, matched: Option<&str>) {
    SINK.with(|s| {
        if let Some(sink) = s.borrow_mut().as_mut() {
            sink.directives.insert(directive.to_string());
            sink.macros.insert(matched.unwrap_or(directive).to_string());
        }
    });
}

/// A computed coverage report: exercised vs total, with the uncovered gap.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoverageReport {
    pub total_macros: usize,
    pub exercised_macros: usize,
    /// Macros known to the registry but never exercised by the suite.
    pub uncovered: Vec<String>,
    /// Percentage of registry macros exercised (0–100).
    pub directive_percent: f64,
}

impl CoverageReport {
    /// Build a report from an exercised set + the full list of known macros.
    pub fn compute(exercised: &BTreeSet<String>, all_macros: &[String]) -> CoverageReport {
        let total = all_macros.len();
        let uncovered: Vec<String> = all_macros
            .iter()
            .filter(|m| !exercised.contains(*m))
            .cloned()
            .collect();
        let exercised_count = total.saturating_sub(uncovered.len());
        let pct = if total == 0 {
            100.0
        } else {
            (exercised_count as f64 / total as f64) * 100.0
        };
        CoverageReport {
            total_macros: total,
            exercised_macros: exercised_count,
            uncovered,
            directive_percent: pct,
        }
    }

    /// Human-readable one-screen summary.
    pub fn render_text(&self) -> String {
        let mut out = format!(
            "Directive coverage: {}/{} macros exercised ({:.1}%)\n",
            self.exercised_macros, self.total_macros, self.directive_percent
        );
        if self.uncovered.is_empty() {
            out.push_str("  ✓ every registered macro form was exercised\n");
        } else {
            out.push_str(&format!("  {} never exercised:\n", self.uncovered.len()));
            for m in &self.uncovered {
                out.push_str(&format!("    · @{m}\n"));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_only_when_active() {
        let _ = end(); // ensure clean
        record("when", Some("when"));
        assert!(end().is_none(), "recording must be inactive by default");

        begin();
        record("then", Some("then-claims"));
        record("when", None);
        let sink = end().expect("active");
        assert!(sink.macros.contains("then-claims"));
        assert!(sink.macros.contains("when"));
        assert!(sink.directives.contains("then"));
        assert!(sink.directives.contains("when"));
    }

    #[test]
    fn report_computes_gap() {
        let mut ex = BTreeSet::new();
        ex.insert("when".to_string());
        ex.insert("then-claims".to_string());
        let all = vec![
            "when".to_string(),
            "then-claims".to_string(),
            "fixture".to_string(),
            "mount".to_string(),
        ];
        let r = CoverageReport::compute(&ex, &all);
        assert_eq!(r.total_macros, 4);
        assert_eq!(r.exercised_macros, 2);
        assert_eq!(
            r.uncovered,
            vec!["fixture".to_string(), "mount".to_string()]
        );
        assert_eq!(r.directive_percent, 50.0);
    }
}
