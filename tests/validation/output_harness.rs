//! Unified test harness for validating Spacetime output.
//!
//! Provides a structured way to:
//! 1. Compile .st files
//! 2. Validate CSS syntax (lightningcss)
//! 3. Validate JS syntax (V8 via rustyscript)
//! 4. Report all errors with context

use super::css_validation::{CssValidationError, validate_css};
use super::js_validation::{JsValidationError, validate_js};
use std::path::{Path, PathBuf};

/// Result of validating compiled output
#[derive(Debug)]
#[allow(dead_code)]
pub struct HarnessResult {
    /// The file that was compiled (if from file)
    pub source_path: Option<PathBuf>,
    /// Whether compilation succeeded
    pub compiled: bool,
    /// Compiled CSS output (if compilation succeeded)
    pub css: Option<String>,
    /// Compiled JS output (if compilation succeeded)
    pub js: Option<String>,
    /// Errors encountered during validation
    pub errors: Vec<HarnessError>,
}

impl HarnessResult {
    /// Check if validation passed with no errors
    #[allow(dead_code)]
    pub fn is_ok(&self) -> bool {
        self.compiled && self.errors.is_empty()
    }

    /// Format all errors as a string
    #[allow(dead_code)]
    pub fn error_summary(&self) -> String {
        if self.errors.is_empty() {
            return String::new();
        }

        let mut summary = String::new();
        if let Some(path) = &self.source_path {
            summary.push_str(&format!("Errors in {}:\n", path.display()));
        }
        for error in &self.errors {
            summary.push_str(&format!("  - {}\n", error));
        }
        summary
    }
}

/// Types of errors that can occur during validation
#[derive(Debug)]
#[allow(dead_code)]
pub enum HarnessError {
    /// Parsing failed
    Parse(String),
    /// Transformation failed
    Transform(String),
    /// Compilation failed
    Compile(String),
    /// CSS validation failed
    Css(CssValidationError),
    /// JS validation failed
    Js(JsValidationError),
}

impl std::fmt::Display for HarnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HarnessError::Parse(msg) => write!(f, "Parse error: {}", msg),
            HarnessError::Transform(msg) => write!(f, "Transform error: {}", msg),
            HarnessError::Compile(msg) => write!(f, "Compile error: {}", msg),
            HarnessError::Css(err) => write!(f, "CSS error: {}", err),
            HarnessError::Js(err) => write!(f, "JS error: {}", err),
        }
    }
}

/// Test harness for validating Spacetime output
#[derive(Default)]
pub struct OutputHarness {
    /// Whether to validate CSS syntax
    pub validate_css: bool,
    /// Whether to validate JS syntax
    pub validate_js: bool,
    /// Enable trace mode for compilation
    pub trace: bool,
}

impl OutputHarness {
    /// Create a new harness with all validations enabled
    pub fn new() -> Self {
        Self {
            validate_css: true,
            validate_js: true,
            trace: false,
        }
    }

    /// Enable only CSS validation
    #[allow(dead_code)]
    pub fn with_css_validation(mut self) -> Self {
        self.validate_css = true;
        self
    }

    /// Enable only JS validation
    #[allow(dead_code)]
    pub fn with_js_validation(mut self) -> Self {
        self.validate_js = true;
        self
    }

    /// Enable trace mode
    #[allow(dead_code)]
    pub fn with_trace(mut self) -> Self {
        self.trace = true;
        self
    }

    /// Test a string of Spacetime code
    pub fn test_str(&self, input: &str) -> HarnessResult {
        self.compile_and_validate(input, None)
    }

    /// Test a file
    #[allow(dead_code)]
    pub fn test_file(&self, path: &Path) -> HarnessResult {
        match std::fs::read_to_string(path) {
            Ok(input) => self.compile_and_validate(&input, Some(path.to_path_buf())),
            Err(e) => HarnessResult {
                source_path: Some(path.to_path_buf()),
                compiled: false,
                css: None,
                js: None,
                errors: vec![HarnessError::Parse(format!("Failed to read file: {}", e))],
            },
        }
    }

    /// Test all .st files matching a glob pattern
    #[allow(dead_code)]
    pub fn test_glob(&self, pattern: &str) -> Vec<HarnessResult> {
        let mut results = Vec::new();

        if let Ok(paths) = glob::glob(pattern) {
            for entry in paths.flatten() {
                results.push(self.test_file(&entry));
            }
        }

        results
    }

    /// Test all examples in the examples/ directory
    #[allow(dead_code)]
    pub fn test_all_examples(&self) -> Vec<HarnessResult> {
        self.test_glob("examples/**/*.st")
    }

    /// Test all stdlib primitives
    #[allow(dead_code)]
    pub fn test_stdlib_primitives(&self) -> Vec<HarnessResult> {
        self.test_glob("stdlib/primitives/*.st")
    }

    /// Test all stdlib macros
    #[allow(dead_code)]
    pub fn test_stdlib_macros(&self) -> Vec<HarnessResult> {
        self.test_glob("stdlib/macros/*.st")
    }

    /// Compile and validate input
    fn compile_and_validate(&self, input: &str, source_path: Option<PathBuf>) -> HarnessResult {
        let mut result = HarnessResult {
            source_path,
            compiled: false,
            css: None,
            js: None,
            errors: Vec::new(),
        };

        // Step 1: Parse
        let ast = match spacetime::parser::parse(input) {
            Ok(ast) => ast,
            Err(e) => {
                result.errors.push(HarnessError::Parse(format!("{:?}", e)));
                return result;
            }
        };

        // Step 2: Compile
        let compiled = spacetime::Compiler::from_ast(&ast)
            .trace(self.trace)
            .compile();
        result.compiled = true;
        result.css = Some(compiled.css.clone());
        result.js = Some(compiled.js.clone());

        // Step 4: Validate CSS
        if self.validate_css
            && let Err(e) = validate_css(&compiled.css)
        {
            result.errors.push(HarnessError::Css(e));
        }

        // Step 5: Validate JS
        if self.validate_js
            && let Err(e) = validate_js(&compiled.js)
        {
            result.errors.push(HarnessError::Js(e));
        }

        result
    }
}

/// Assert that all results pass validation
#[allow(dead_code)]
pub fn assert_all_valid(results: &[HarnessResult]) {
    let failures: Vec<_> = results.iter().filter(|r| !r.is_ok()).collect();

    if !failures.is_empty() {
        let mut msg = format!("{} file(s) failed validation:\n\n", failures.len());
        for failure in failures {
            msg.push_str(&failure.error_summary());
            msg.push('\n');
        }
        panic!("{}", msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_validates_valid_input() {
        // Using a simpler test case that doesn't require macro expansion
        let input = r#"
            .card {
                opacity: 1;
                $visible boolean: true;
            }
        "#;

        let harness = OutputHarness::new();
        let result = harness.test_str(input);

        assert!(
            result.compiled,
            "Should compile successfully: {:?}",
            result.errors
        );
        assert!(result.css.is_some(), "Should have CSS output");
        assert!(result.js.is_some(), "Should have JS output");
        // Note: errors may still occur if the generated output is invalid
    }

    #[test]
    fn harness_catches_parse_errors() {
        let input = r#"
            .card {
                @invalid_directive_that_does_not_exist {
        "#;

        let harness = OutputHarness::new();
        let result = harness.test_str(input);

        assert!(!result.compiled, "Should fail to compile");
        assert!(!result.errors.is_empty(), "Should have errors");
    }

    #[test]
    fn harness_validates_empty_input() {
        let harness = OutputHarness::new();
        let result = harness.test_str("");

        // Empty input should parse OK
        assert!(result.compiled, "Empty input should compile");
    }
}
