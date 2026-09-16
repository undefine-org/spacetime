//! Browser test result + timing-parity data structures.
//!
//! The Playwright-based multi-browser runner was removed in PLAN-027 W2 (replaced
//! by the pure-Rust CDP backend in `src/cdp.rs`). These structs remain for the
//! timing-parity model and its unit tests.
//!
//! Provides cross-browser testing for Firefox/Chrome parity verification.
//! Uses real browsers to catch timing differences that headless V8 testing misses.
//!
//! # Usage
//!
//! ```bash
//! cargo run -- test tests/ --browser firefox,chromium
//! ```

use std::collections::HashMap;

/// Timing report from ST._timing.report()
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TimingReport {
    #[serde(rename = "avgDelta")]
    pub avg_delta: f64,
    #[serde(rename = "maxDelta")]
    pub max_delta: f64,
    pub jitter: f64,
    #[serde(rename = "sampleCount")]
    pub sample_count: usize,
}

/// Test results from a single browser
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BrowserTestResult {
    pub browser: String,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total: usize,
    pub duration_ms: f64,
    pub errors: Vec<String>,
    pub timing: Option<TimingReport>,
}

/// Parity issue detected between browsers
#[derive(Debug, Clone)]
pub struct ParityIssue {
    pub browser: String,
    pub issue_type: ParityIssueType,
    pub details: String,
}

/// Types of parity issues
#[derive(Debug, Clone)]
pub enum ParityIssueType {
    JitterTooHigh { jitter: f64, threshold: f64 },
    MaxDeltaTooHigh { max: f64, threshold: f64 },
    TestResultMismatch { test_name: String },
}

/// Combined results from all browsers
#[derive(Debug)]
pub struct BrowserTestResults {
    pub results: HashMap<String, BrowserTestResult>,
    pub parity_issues: Vec<ParityIssue>,
}

/// Thresholds for acceptable browser variance
pub const MAX_JITTER_MS: f64 = 5.0;
pub const MAX_DELTA_MS: f64 = 33.0; // 2 frames at 60fps

impl BrowserTestResults {
    /// Check timing parity between browsers
    pub fn check_timing_parity(&mut self) {
        for (browser, result) in &self.results {
            if let Some(timing) = &result.timing {
                // Check jitter threshold
                if timing.jitter > MAX_JITTER_MS {
                    self.parity_issues.push(ParityIssue {
                        browser: browser.clone(),
                        issue_type: ParityIssueType::JitterTooHigh {
                            jitter: timing.jitter,
                            threshold: MAX_JITTER_MS,
                        },
                        details: format!(
                            "{} jitter ({:.1}ms) exceeds threshold ({:.1}ms)",
                            browser, timing.jitter, MAX_JITTER_MS
                        ),
                    });
                }

                // Check max delta threshold
                if timing.max_delta > MAX_DELTA_MS {
                    self.parity_issues.push(ParityIssue {
                        browser: browser.clone(),
                        issue_type: ParityIssueType::MaxDeltaTooHigh {
                            max: timing.max_delta,
                            threshold: MAX_DELTA_MS,
                        },
                        details: format!(
                            "{} max RAF delta ({:.1}ms) exceeds threshold ({:.1}ms)",
                            browser, timing.max_delta, MAX_DELTA_MS
                        ),
                    });
                }
            }
        }
    }

    /// Check if all tests passed with acceptable parity
    pub fn is_success(&self) -> bool {
        self.parity_issues.is_empty() && self.results.values().all(|r| r.failed == 0)
    }

    /// Format results for console output
    pub fn format_text(&self) -> String {
        let mut output = String::new();

        for (browser, result) in &self.results {
            let timing_info = result.timing.as_ref().map_or(String::new(), |t| {
                format!(" (avg: {:.1}ms, jitter: {:.1}ms)", t.avg_delta, t.jitter)
            });

            output.push_str(&format!(
                "{}: {} passed, {} failed{}\n",
                browser, result.passed, result.failed, timing_info
            ));
        }

        if self.parity_issues.is_empty() {
            output.push_str("\n✓ Parity check passed\n");
        } else {
            output.push_str("\nParity issues:\n");
            for issue in &self.parity_issues {
                output.push_str(&format!("  ✗ {}\n", issue.details));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parity_check_jitter() {
        let mut results = BrowserTestResults {
            results: HashMap::from([(
                "firefox".to_string(),
                BrowserTestResult {
                    browser: "firefox".to_string(),
                    passed: 10,
                    failed: 0,
                    skipped: 0,
                    total: 10,
                    duration_ms: 1000.0,
                    errors: vec![],
                    timing: Some(TimingReport {
                        avg_delta: 16.7,
                        max_delta: 20.0,
                        jitter: 8.0, // Exceeds threshold
                        sample_count: 60,
                    }),
                },
            )]),
            parity_issues: vec![],
        };

        results.check_timing_parity();
        assert_eq!(results.parity_issues.len(), 1);
        assert!(!results.is_success());
    }

    #[test]
    fn test_parity_check_pass() {
        let mut results = BrowserTestResults {
            results: HashMap::from([(
                "firefox".to_string(),
                BrowserTestResult {
                    browser: "firefox".to_string(),
                    passed: 10,
                    failed: 0,
                    skipped: 0,
                    total: 10,
                    duration_ms: 1000.0,
                    errors: vec![],
                    timing: Some(TimingReport {
                        avg_delta: 16.7,
                        max_delta: 20.0,
                        jitter: 2.0, // Within threshold
                        sample_count: 60,
                    }),
                },
            )]),
            parity_issues: vec![],
        };

        results.check_timing_parity();
        assert!(results.parity_issues.is_empty());
        assert!(results.is_success());
    }
}
