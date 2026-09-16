//! JavaScript syntax validation using SWC parser.
//!
//! Validates that generated JavaScript is syntactically correct, including:
//! - Valid ES6+ syntax
//! - Proper function declarations
//! - Valid arrow functions
//! - No unclosed braces/quotes/parens

use std::fmt;
use swc_common::Spanned;

/// Error returned when JavaScript validation fails.
#[derive(Debug, Clone)]
pub struct JsSyntaxError {
    /// Human-readable error message
    pub message: String,
    /// Line number where error occurred (1-indexed)
    pub line: Option<u32>,
    /// Column number where error occurred (1-indexed)
    pub column: Option<u32>,
}

impl JsSyntaxError {
    /// Create a new JS syntax error with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
        }
    }

    /// Create a new JS syntax error with location information.
    pub fn with_location(message: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            message: message.into(),
            line: Some(line),
            column: Some(column),
        }
    }
}

impl fmt::Display for JsSyntaxError {
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

impl std::error::Error for JsSyntaxError {}

/// Validates that the given JavaScript string is syntactically correct.

/// Uses SWC's ECMAScript parser for syntax-only validation, avoiding
/// the need for a V8 isolate.
///
/// # Arguments
/// * `js` - The JavaScript string to validate
///
/// # Returns
/// * `Ok(())` if the JavaScript is valid
/// * `Err(Vec<JsSyntaxError>)` if there are syntax errors
///
/// # Example
/// ```rust,ignore
/// use spacetime::validation::validate_js;
///
/// let valid_js = "const x = 42;";
/// assert!(validate_js(valid_js).is_ok());
///
/// let invalid_js = "const x = ;";  // Missing value
/// assert!(validate_js(invalid_js).is_err());
/// ```
pub fn validate_js(js: &str) -> Result<(), Vec<JsSyntaxError>> {
    // Skip validation for empty JS
    if js.trim().is_empty() {
        return Ok(());
    }

    use swc_common::{FileName, SourceMap, input::SourceFileInput};
    use swc_ecma_parser::{EsSyntax, Parser, Syntax};

    // Wrap in a function to allow return statements (matching previous V8 behavior)
    let wrapped = format!("(function() {{\n{}\n}})", js);

    let cm = SourceMap::default();
    let fm = cm.new_source_file(FileName::Anon.into(), wrapped);
    let mut parser = Parser::new(
        Syntax::Es(EsSyntax::default()),
        SourceFileInput::from(&*fm),
        None,
    );

    match parser.parse_script() {
        Ok(_) => {
            let errors = parser.take_errors();
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors
                    .into_iter()
                    .map(|e| {
                        let span = e.span();
                        let loc = cm.lookup_char_pos(span.lo);
                        // Adjust line for the wrapper: line 1 = "(function() {", so subtract 1
                        let line = if loc.line > 1 {
                            (loc.line - 1) as u32
                        } else {
                            loc.line as u32
                        };
                        let col = (loc.col_display + 1) as u32;
                        JsSyntaxError::with_location(format!("{:?}", e.into_kind()), line, col)
                    })
                    .collect())
            }
        }
        Err(e) => {
            let span = e.span();
            let loc = cm.lookup_char_pos(span.lo);
            let line = if loc.line > 1 {
                (loc.line - 1) as u32
            } else {
                loc.line as u32
            };
            let col = (loc.col_display + 1) as u32;
            Err(vec![JsSyntaxError::with_location(
                format!("{:?}", e.into_kind()),
                line,
                col,
            )])
        }
    }
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
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(!errors[0].message.is_empty());
    }

    #[test]
    fn error_display() {
        let err = JsSyntaxError::new("test error");
        assert!(err.to_string().contains("test error"));

        let err_with_loc = JsSyntaxError::with_location("located error", 5, 10);
        let display = err_with_loc.to_string();
        assert!(display.contains("5:10"));
        assert!(display.contains("located error"));
    }
}
