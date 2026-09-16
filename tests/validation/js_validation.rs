//! JavaScript syntax validation using V8 engine via rustyscript.
//!
//! Validates that generated JavaScript is syntactically correct, including:
//! - Valid ES6+ syntax
//! - Proper function declarations
//! - Valid arrow functions
//! - No unclosed braces/quotes/parens

use rustyscript::{Runtime, RuntimeOptions};
use serde_json::Value;
use std::fmt;
use std::sync::Once;

static V8_INIT: Once = Once::new();

fn ensure_v8_initialized() {
    V8_INIT.call_once(|| {
        rustyscript::init_platform(4, true);
    });
}

/// Error returned when JavaScript validation fails.
#[derive(Debug, Clone)]
pub struct JsValidationError {
    /// Human-readable error message
    pub message: String,
    /// Line number where error occurred (1-indexed)
    pub line: Option<u32>,
    /// Column number where error occurred (1-indexed)
    pub column: Option<u32>,
}

impl fmt::Display for JsValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(col)) => {
                write!(f, "JS error at {}:{}: {}", line, col, self.message)
            }
            (Some(line), None) => {
                write!(f, "JS error at line {}: {}", line, self.message)
            }
            _ => write!(f, "JS error: {}", self.message),
        }
    }
}

impl std::error::Error for JsValidationError {}

/// Validates that the given JavaScript string is syntactically correct.
///
/// Uses V8's parser which will fail at parse time for syntax errors.
/// We wrap the code in a function to parse without executing side effects.
///
/// # Arguments
/// * `js` - The JavaScript string to validate
///
/// # Returns
/// * `Ok(())` if the JavaScript is valid
/// * `Err(JsValidationError)` if there are syntax errors
pub fn validate_js(js: &str) -> Result<(), JsValidationError> {
    // Skip validation for empty JS
    if js.trim().is_empty() {
        return Ok(());
    }

    // Create a V8 runtime and try to parse the JavaScript
    ensure_v8_initialized();
    let mut runtime = match Runtime::new(RuntimeOptions::default()) {
        Ok(r) => r,
        Err(e) => {
            return Err(JsValidationError {
                message: format!("Failed to create JS runtime: {}", e),
                line: None,
                column: None,
            });
        }
    };

    // Wrap in a function to parse without executing side effects
    let wrapped = format!("(function() {{\n{}\n}})", js);

    match runtime.eval::<Value>(&wrapped) {
        Ok(_) => Ok(()),
        Err(err) => {
            let (line, column) = extract_error_location(&err);

            Err(JsValidationError {
                message: format!("{}", err),
                line,
                column,
            })
        }
    }
}

/// Extract line and column from a V8 error.
fn extract_error_location(err: &rustyscript::Error) -> (Option<u32>, Option<u32>) {
    let err_str = err.to_string();

    // V8 error format: "at line X, col Y" or "X:Y" patterns
    if let Some(loc_start) = err_str.find("line ") {
        let rest = &err_str[loc_start + 5..];

        let line_end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let line_str = &rest[..line_end];

        if let Ok(line) = line_str.parse::<u32>() {
            let col = rest
                .find("col")
                .or_else(|| rest.find("column"))
                .and_then(|col_start| {
                    let after_col = &rest[col_start..];
                    let num_start = after_col.find(|c: char| c.is_ascii_digit())?;
                    let num_rest = &after_col[num_start..];
                    let num_end = num_rest
                        .find(|c: char| !c.is_ascii_digit())
                        .unwrap_or(num_rest.len());
                    num_rest[..num_end].parse::<u32>().ok()
                });

            // Adjust line number to account for wrapper function
            let adjusted_line = if line > 1 { line - 1 } else { line };
            return (Some(adjusted_line), col);
        }
    }

    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_simple_js() {
        let js = "const x = 42;";
        assert!(validate_js(js).is_ok());
    }

    #[test]
    fn valid_function_declaration() {
        let js = r#"
            function greet(name) {
                return "Hello, " + name;
            }
        "#;
        assert!(validate_js(js).is_ok());
    }

    #[test]
    fn valid_arrow_function() {
        let js = r#"
            const greet = (name) => {
                return "Hello, " + name;
            };
        "#;
        assert!(validate_js(js).is_ok());
    }

    #[test]
    fn valid_class_declaration() {
        let js = r#"
            class Widget {
                constructor(id) {
                    this.id = id;
                }

                render() {
                    return document.getElementById(this.id);
                }
            }
        "#;
        assert!(validate_js(js).is_ok());
    }

    #[test]
    fn valid_async_function() {
        let js = r#"
            async function fetchData(url) {
                const response = await fetch(url);
                return response.json();
            }
        "#;
        assert!(validate_js(js).is_ok());
    }

    #[test]
    fn valid_iife() {
        let js = r#"
            (function() {
                const state = {};
                window.myApp = { state };
            })();
        "#;
        assert!(validate_js(js).is_ok());
    }

    #[test]
    fn valid_template_literal() {
        let js = r#"
            const name = "World";
            const greeting = `Hello, ${name}!`;
        "#;
        assert!(validate_js(js).is_ok());
    }

    #[test]
    fn valid_destructuring() {
        let js = r#"
            const { a, b } = obj;
            const [x, y, ...rest] = arr;
        "#;
        assert!(validate_js(js).is_ok());
    }

    #[test]
    fn valid_object_shorthand() {
        let js = r#"
            const x = 1;
            const y = 2;
            const obj = { x, y, method() { return this.x; } };
        "#;
        assert!(validate_js(js).is_ok());
    }

    #[test]
    fn valid_web_animations_api() {
        // This is the kind of JS Spacetime generates
        let js = r#"
            const element = document.querySelector('.card');
            element.animate([
                { opacity: 0, transform: 'translateY(20px)' },
                { opacity: 1, transform: 'translateY(0)' }
            ], {
                duration: 300,
                easing: 'ease-out',
                fill: 'forwards'
            });
        "#;
        assert!(validate_js(js).is_ok());
    }

    #[test]
    fn valid_intersection_observer() {
        // Another pattern Spacetime uses
        let js = r#"
            const observer = new IntersectionObserver((entries) => {
                entries.forEach(entry => {
                    if (entry.isIntersecting) {
                        entry.target.classList.add('visible');
                    }
                });
            }, { threshold: 0.1 });

            document.querySelectorAll('.fade-in').forEach(el => {
                observer.observe(el);
            });
        "#;
        assert!(validate_js(js).is_ok());
    }

    #[test]
    fn empty_js_is_valid() {
        assert!(validate_js("").is_ok());
        assert!(validate_js("   ").is_ok());
    }

    #[test]
    fn invalid_unclosed_brace() {
        let js = "function test() {";
        let result = validate_js(js);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_missing_expression() {
        let js = "const x = ;";
        let result = validate_js(js);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_syntax() {
        let js = "const = 42;";
        let result = validate_js(js);
        assert!(result.is_err());
    }

    #[test]
    fn error_has_message() {
        let js = "function test(";
        let result = validate_js(js);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.message.is_empty());
    }
}
