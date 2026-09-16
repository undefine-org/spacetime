//! Build-time JavaScript execution via rustyscript (V8) + LinkeDOM.
//!
//! Provides a sandboxed JS environment for `%emit build-js` blocks.
//! Host functions (readFile, writeFile, parseHTML, etc.) enable build scripts
//! to read source data, manipulate DOM, and write static output files.

#[cfg(feature = "headless")]
mod inner {
    use rustyscript::{Runtime, RuntimeOptions};
    use serde_json::Value;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::fmt;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    /// LinkeDOM bundle — same bundle used by test runtime
    const LINKEDOM_JS: &str = include_str!("../tests/v8_runtime/linkedom.bundle.js");

    /// Shared state accessible from host functions
    struct BuildState {
        /// Site source directory (for readFile)
        site_dir: PathBuf,
        /// Output directory (for writeFile)
        output_dir: PathBuf,
        /// HTML content of the paired .html file
        html_content: String,
        /// Build log messages
        log_messages: Vec<BuildLogMessage>,
        /// Files written during build
        written_files: HashMap<String, String>,
    }

    /// A log message emitted during build
    #[derive(Debug, Clone)]
    pub struct BuildLogMessage {
        pub level: LogLevel,
        pub message: String,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LogLevel {
        Info,
        Warn,
    }

    /// Error during build script execution
    #[derive(Debug)]
    pub struct BuildError {
        pub message: String,
    }

    impl fmt::Display for BuildError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    /// Result of executing build scripts
    #[derive(Debug)]
    pub struct BuildResult {
        /// Files written during build: relative_path -> content
        pub written_files: HashMap<String, String>,
        /// Log messages emitted during build
        pub log_messages: Vec<BuildLogMessage>,
    }

    /// Validate that a path doesn't escape its sandbox directory.
    /// Returns the canonical resolved path if valid.
    fn validate_path(base: &Path, relative: &str) -> Result<PathBuf, String> {
        // Reject obvious traversal attempts
        if relative.contains("..") {
            return Err(format!("Path traversal not allowed: {}", relative));
        }
        let resolved = base.join(relative);
        // Verify the resolved path is under the base
        let canon_base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
        let canon_resolved = resolved.canonicalize().unwrap_or_else(|_| resolved.clone());
        if !canon_resolved.starts_with(&canon_base) {
            return Err(format!(
                "Path escapes sandbox: {} (resolved to {})",
                relative,
                canon_resolved.display()
            ));
        }
        Ok(resolved)
    }

    /// Validate a path for writing — the file doesn't exist yet so we validate the parent.
    fn validate_write_path(base: &Path, relative: &str) -> Result<PathBuf, String> {
        if relative.contains("..") {
            return Err(format!("Path traversal not allowed: {}", relative));
        }
        let resolved = base.join(relative);
        // For write paths, check that the parent directory is under base
        if let Some(parent) = resolved.parent() {
            let canon_base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
            // Parent might not exist yet (we'll create it), so use the base check on the path
            let mut check = parent.to_path_buf();
            loop {
                if check.exists() {
                    let canon_check = check.canonicalize().unwrap_or(check.clone());
                    if !canon_check.starts_with(&canon_base) {
                        return Err(format!("Write path escapes sandbox: {}", relative));
                    }
                    break;
                }
                if !check.pop() {
                    break;
                }
            }
        }
        Ok(resolved)
    }

    /// Helper to create a rustyscript error from a string message
    fn js_error(msg: impl Into<String>) -> rustyscript::Error {
        rustyscript::Error::Runtime(msg.into())
    }

    /// Execute build scripts in a V8 context with LinkeDOM and host functions.
    ///
    /// # Arguments
    /// - `scripts`: resolved `%emit build-js` code blocks
    /// - `site_dir`: root of the site source (for readFile sandboxing)
    /// - `html_content`: content of the paired .html file
    /// - `output_dir`: where writeFile writes to
    ///
    /// # Returns
    /// `BuildResult` with written files and log messages, or errors.
    pub fn execute_build_scripts(
        scripts: &[String],
        site_dir: &Path,
        html_content: &str,
        output_dir: &Path,
    ) -> Result<BuildResult, Vec<BuildError>> {
        crate::ensure_v8_initialized();

        // Shared state captured by closures
        let state = Rc::new(RefCell::new(BuildState {
            site_dir: site_dir.to_path_buf(),
            output_dir: output_dir.to_path_buf(),
            html_content: html_content.to_string(),
            log_messages: Vec::new(),
            written_files: HashMap::new(),
        }));

        // Create V8 runtime
        let mut runtime = Runtime::new(RuntimeOptions::default()).map_err(|e| {
            vec![BuildError {
                message: format!("Failed to create V8 runtime: {}", e),
            }]
        })?;

        // Register host functions as closures capturing shared state

        // readFile(path) → string
        let s = state.clone();
        runtime
            .register_function("readFile", move |args: &[Value]| {
                let path_arg = args
                    .first()
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| js_error("readFile: expected string argument"))?;
                let state = s.borrow();
                let resolved = validate_path(&state.site_dir, path_arg).map_err(js_error)?;
                let content = std::fs::read_to_string(&resolved)
                    .map_err(|e| js_error(format!("readFile failed: {}", e)))?;
                Ok(Value::String(content))
            })
            .map_err(|e| {
                vec![BuildError {
                    message: format!("Failed to register readFile: {}", e),
                }]
            })?;

        // readJson(path) → object
        let s = state.clone();
        runtime
            .register_function("readJson", move |args: &[Value]| {
                let path_arg = args
                    .first()
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| js_error("readJson: expected string argument"))?;
                let state = s.borrow();
                let resolved = validate_path(&state.site_dir, path_arg).map_err(js_error)?;
                let content = std::fs::read_to_string(&resolved)
                    .map_err(|e| js_error(format!("readJson failed: {}", e)))?;
                let value: Value = serde_json::from_str(&content)
                    .map_err(|e| js_error(format!("JSON parse failed: {}", e)))?;
                Ok(value)
            })
            .map_err(|e| {
                vec![BuildError {
                    message: format!("Failed to register readJson: {}", e),
                }]
            })?;

        // writeFile(path, content) → void
        let s = state.clone();
        runtime
            .register_function("writeFile", move |args: &[Value]| {
                let path_arg = args
                    .first()
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| js_error("writeFile: expected path string"))?
                    .to_string();
                let content_arg = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| js_error("writeFile: expected content string"))?
                    .to_string();

                let mut state = s.borrow_mut();
                let resolved =
                    validate_write_path(&state.output_dir, &path_arg).map_err(js_error)?;

                if let Some(parent) = resolved.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| js_error(format!("mkdir failed: {}", e)))?;
                }

                std::fs::write(&resolved, &content_arg)
                    .map_err(|e| js_error(format!("writeFile failed: {}", e)))?;

                state.written_files.insert(path_arg, content_arg);
                Ok(Value::Null)
            })
            .map_err(|e| {
                vec![BuildError {
                    message: format!("Failed to register writeFile: {}", e),
                }]
            })?;

        // copyFile(src, dest) → void
        let s = state.clone();
        runtime
            .register_function("copyFile", move |args: &[Value]| {
                let src_arg = args
                    .first()
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| js_error("copyFile: expected source path string"))?;
                let dest_arg = args
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| js_error("copyFile: expected dest path string"))?;

                let state = s.borrow();
                let src_resolved = validate_path(&state.site_dir, src_arg).map_err(js_error)?;
                let dest_resolved =
                    validate_write_path(&state.output_dir, dest_arg).map_err(js_error)?;

                if let Some(parent) = dest_resolved.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| js_error(format!("mkdir failed: {}", e)))?;
                }

                std::fs::copy(&src_resolved, &dest_resolved)
                    .map_err(|e| js_error(format!("copyFile failed: {}", e)))?;

                Ok(Value::Null)
            })
            .map_err(|e| {
                vec![BuildError {
                    message: format!("Failed to register copyFile: {}", e),
                }]
            })?;

        // fileExists(path) → boolean
        let s = state.clone();
        runtime
            .register_function("fileExists", move |args: &[Value]| {
                let path_arg = args
                    .first()
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| js_error("fileExists: expected string argument"))?;
                let state = s.borrow();
                match validate_path(&state.site_dir, path_arg) {
                    Ok(resolved) => Ok(Value::Bool(resolved.exists())),
                    Err(_) => Ok(Value::Bool(false)),
                }
            })
            .map_err(|e| {
                vec![BuildError {
                    message: format!("Failed to register fileExists: {}", e),
                }]
            })?;

        // listFiles(dir_path) → string[]
        let s = state.clone();
        runtime
            .register_function("listFiles", move |args: &[Value]| {
                let dir_arg = args
                    .first()
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| js_error("listFiles: expected string argument"))?;
                let state = s.borrow();
                let resolved = validate_path(&state.site_dir, dir_arg).map_err(js_error)?;

                let mut files = Vec::new();
                if resolved.is_dir()
                    && let Ok(entries) = std::fs::read_dir(&resolved)
                {
                    for entry in entries.flatten() {
                        if let Ok(rel) = entry.path().strip_prefix(&state.site_dir) {
                            files.push(rel.to_string_lossy().to_string());
                        }
                    }
                }

                serde_json::to_value(files)
                    .map_err(|e| js_error(format!("listFiles failed: {}", e)))
            })
            .map_err(|e| {
                vec![BuildError {
                    message: format!("Failed to register listFiles: {}", e),
                }]
            })?;

        // getHtml() → string
        let s = state.clone();
        runtime
            .register_function("getHtml", move |_args: &[Value]| {
                let state = s.borrow();
                Ok(Value::String(state.html_content.clone()))
            })
            .map_err(|e| {
                vec![BuildError {
                    message: format!("Failed to register getHtml: {}", e),
                }]
            })?;

        // log(msg) → void
        let s = state.clone();
        runtime
            .register_function("log", move |args: &[Value]| {
                let msg = args
                    .first()
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default();

                s.borrow_mut().log_messages.push(BuildLogMessage {
                    level: LogLevel::Info,
                    message: msg.clone(),
                });

                log::info!("[build] {}", msg);
                Ok(Value::Null)
            })
            .map_err(|e| {
                vec![BuildError {
                    message: format!("Failed to register log: {}", e),
                }]
            })?;

        // warn(msg) → void
        let s = state.clone();
        runtime
            .register_function("warn", move |args: &[Value]| {
                let msg = args
                    .first()
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default();

                s.borrow_mut().log_messages.push(BuildLogMessage {
                    level: LogLevel::Warn,
                    message: msg.clone(),
                });

                log::warn!("[build] {}", msg);
                Ok(Value::Null)
            })
            .map_err(|e| {
                vec![BuildError {
                    message: format!("Failed to register warn: {}", e),
                }]
            })?;

        // Create global aliases for registered functions
        let alias_js = r#"
            globalThis.readFile = (...args) => rustyscript.functions.readFile(...args);
            globalThis.readJson = (...args) => rustyscript.functions.readJson(...args);
            globalThis.writeFile = (...args) => rustyscript.functions.writeFile(...args);
            globalThis.copyFile = (...args) => rustyscript.functions.copyFile(...args);
            globalThis.fileExists = (...args) => rustyscript.functions.fileExists(...args);
            globalThis.listFiles = (...args) => rustyscript.functions.listFiles(...args);
            globalThis.getHtml = (...args) => rustyscript.functions.getHtml(...args);
            globalThis.log = (...args) => rustyscript.functions.log(...args);
            globalThis.warn = (...args) => rustyscript.functions.warn(...args);

            globalThis.__spacetime_headless__ = true;
            globalThis.__v8__ = true;
        "#;
        runtime.eval::<Value>(alias_js).map_err(|e| {
            vec![BuildError {
                message: format!("Failed to set up global aliases: {}", e),
            }]
        })?;

        // Initialize LinkeDOM
        runtime.eval::<Value>(LINKEDOM_JS).map_err(|e| {
            vec![BuildError {
                message: format!("Failed to initialize LinkeDOM: {}", e),
            }]
        })?;

        // Set up parseHTML global
        runtime
            .eval::<Value>(
                r#"
if (typeof globalThis.parseHTML === 'undefined') {
    if (typeof linkedom !== 'undefined' && typeof linkedom.parseHTML === 'function') {
        globalThis.parseHTML = linkedom.parseHTML;
    } else {
        globalThis.parseHTML = function(html) {
            var doc = new DOMParser().parseFromString(html, 'text/html');
            return { document: doc, window: globalThis };
        };
    }
}
"#,
            )
            .map_err(|e| {
                vec![BuildError {
                    message: format!("Failed to set up parseHTML: {}", e),
                }]
            })?;

        // Fix LinkeDOM SVG serialization — restore camelCase for SVG elements/attributes
        runtime
            .eval::<Value>(
                r#"
(function() {
    var SVG_ELEMENTS = {
        altglyph: 'altGlyph', altglyphdef: 'altGlyphDef', altglyphitem: 'altGlyphItem',
        animatecolor: 'animateColor', animatemotion: 'animateMotion',
        animatetransform: 'animateTransform', clippath: 'clipPath',
        feblend: 'feBlend', fecolormatrix: 'feColorMatrix',
        fecomponenttransfer: 'feComponentTransfer', fecomposite: 'feComposite',
        feconvolvematrix: 'feConvolveMatrix', fediffuselighting: 'feDiffuseLighting',
        fedisplacementmap: 'feDisplacementMap', fedistantlight: 'feDistantLight',
        fedropshadow: 'feDropShadow', feflood: 'feFlood',
        fefunca: 'feFuncA', fefuncb: 'feFuncB', fefuncg: 'feFuncG', fefuncr: 'feFuncR',
        fegaussianblur: 'feGaussianBlur', feimage: 'feImage', femerge: 'feMerge',
        femergenode: 'feMergeNode', femorphology: 'feMorphology', feoffset: 'feOffset',
        fepointlight: 'fePointLight', fespecularlighting: 'feSpecularLighting',
        fespotlight: 'feSpotLight', fetile: 'feTile', feturbulence: 'feTurbulence',
        foreignobject: 'foreignObject', glyphref: 'glyphRef',
        lineargradient: 'linearGradient', radialgradient: 'radialGradient',
        textpath: 'textPath'
    };

    var SVG_ATTRS = {
        viewbox: 'viewBox', preserveaspectratio: 'preserveAspectRatio',
        attributename: 'attributeName', attributetype: 'attributeType',
        basefrequency: 'baseFrequency', baseprofile: 'baseProfile',
        calcmode: 'calcMode', clippathunits: 'clipPathUnits',
        contentscripttype: 'contentScriptType', contentstyletype: 'contentStyleType',
        diffuseconstant: 'diffuseConstant', edgemode: 'edgeMode',
        filterunits: 'filterUnits', glyphref: 'glyphRef',
        gradienttransform: 'gradientTransform', gradientunits: 'gradientUnits',
        kernelmatrix: 'kernelMatrix', kernelunitlength: 'kernelUnitLength',
        keypoints: 'keyPoints', keysplines: 'keySplines', keytimes: 'keyTimes',
        lengthadjust: 'lengthAdjust', limitingconeangle: 'limitingConeAngle',
        markerheight: 'markerHeight', markerunits: 'markerUnits',
        markerwidth: 'markerWidth', maskcontentunits: 'maskContentUnits',
        maskunits: 'maskUnits', numoctaves: 'numOctaves', pathlength: 'pathLength',
        patterncontentunits: 'patternContentUnits', patterntransform: 'patternTransform',
        patternunits: 'patternUnits', pointsatx: 'pointsAtX', pointsaty: 'pointsAtY',
        pointsatz: 'pointsAtZ', preservealpha: 'preserveAlpha',
        primitiveunits: 'primitiveUnits', refx: 'refX', refy: 'refY',
        repeatcount: 'repeatCount', repeatdur: 'repeatDur',
        requiredextensions: 'requiredExtensions', requiredfeatures: 'requiredFeatures',
        specularconstant: 'specularConstant', specularexponent: 'specularExponent',
        spreadmethod: 'spreadMethod', startoffset: 'startOffset',
        stddeviation: 'stdDeviation', stitchtiles: 'stitchTiles',
        surfacescale: 'surfaceScale', systemlanguage: 'systemLanguage',
        tablevalues: 'tableValues', targetx: 'targetX', targety: 'targetY',
        textlength: 'textLength', viewtarget: 'viewTarget',
        xchannelselector: 'xChannelSelector', ychannelselector: 'yChannelSelector',
        zoomandpan: 'zoomAndPan'
    };

    // Build regex patterns for efficient replacement
    var elemLower = Object.keys(SVG_ELEMENTS);
    var elemPattern = new RegExp('<(/?)(' + elemLower.join('|') + ')([\\s>/])', 'gi');
    var attrLower = Object.keys(SVG_ATTRS);
    var attrPattern = new RegExp('\\b(' + attrLower.join('|') + ')=', 'gi');

    function fixSvgCasing(html) {
        html = html.replace(elemPattern, function(match, slash, tag, after) {
            var fixed = SVG_ELEMENTS[tag.toLowerCase()];
            return '<' + slash + (fixed || tag) + after;
        });
        html = html.replace(attrPattern, function(match, attr) {
            var fixed = SVG_ATTRS[attr.toLowerCase()];
            return (fixed || attr) + '=';
        });
        return html;
    }

    // Patch the outerHTML getter on LinkeDOM's Element prototype
    // LinkeDOM uses doc.documentElement.outerHTML for serialization
    var origParseHTML = globalThis.parseHTML;
    globalThis.parseHTML = function(html) {
        var result = origParseHTML(html);
        var doc = result.document;
        if (!doc || !doc.documentElement) return result;

        // Find the prototype that owns outerHTML
        var el = doc.documentElement;
        var proto = Object.getPrototypeOf(el);
        var patched = false;
        while (proto && !patched) {
            var desc = Object.getOwnPropertyDescriptor(proto, 'outerHTML');
            if (desc && desc.get) {
                var origGetter = desc.get;
                Object.defineProperty(proto, 'outerHTML', {
                    get: function() {
                        return fixSvgCasing(origGetter.call(this));
                    },
                    configurable: true
                });
                patched = true;
            }
            proto = Object.getPrototypeOf(proto);
        }
        // Fallback: if no prototype getter found, patch the doc.toString
        if (!patched) {
            var origToString = doc.toString;
            doc.toString = function() {
                return fixSvgCasing(origToString.call(this));
            };
        }
        return result;
    };
})();
"#,
            )
            .map_err(|e| {
                vec![BuildError {
                    message: format!("Failed to patch LinkeDOM SVG serialization: {}", e),
                }]
            })?;

        // Register site context object
        let site_dir_str = site_dir.to_string_lossy();
        let output_dir_str = output_dir.to_string_lossy();
        let site_setup = format!(
            r#"globalThis.site = {{ dir: "{}", outputDir: "{}" }};"#,
            site_dir_str.replace('\\', "\\\\").replace('"', "\\\""),
            output_dir_str.replace('\\', "\\\\").replace('"', "\\\""),
        );
        let _ = runtime.eval::<Value>(&site_setup);

        // Execute build scripts sequentially
        let mut errors = Vec::new();
        for (i, script) in scripts.iter().enumerate() {
            if let Err(e) = runtime.eval::<Value>(script) {
                errors.push(BuildError {
                    message: format!("Build script {} failed: {}", i + 1, e),
                });
            }
        }

        // Extract results from shared state
        let state = state.borrow();
        let result = BuildResult {
            written_files: state.written_files.clone(),
            log_messages: state.log_messages.clone(),
        };

        if errors.is_empty() {
            Ok(result)
        } else {
            Err(errors)
        }
    }
}

// Re-export when headless feature is enabled
#[cfg(feature = "headless")]
pub use inner::*;

// Stub when headless is not enabled
#[cfg(not(feature = "headless"))]
pub mod stub {
    use std::collections::HashMap;
    use std::path::Path;

    #[derive(Debug)]
    pub struct BuildError {
        pub message: String,
    }

    impl std::fmt::Display for BuildError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    #[derive(Debug, Clone)]
    pub struct BuildLogMessage {
        pub level: LogLevel,
        pub message: String,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LogLevel {
        Info,
        Warn,
    }

    pub struct BuildResult {
        pub written_files: HashMap<String, String>,
        pub log_messages: Vec<BuildLogMessage>,
    }

    pub fn execute_build_scripts(
        _scripts: &[String],
        _site_dir: &Path,
        _html_content: &str,
        _output_dir: &Path,
    ) -> Result<BuildResult, Vec<BuildError>> {
        Err(vec![BuildError {
            message: "Build scripts require the 'headless' feature (rustyscript)".to_string(),
        }])
    }
}

#[cfg(not(feature = "headless"))]
pub use stub::*;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
#[cfg(feature = "headless")]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn test_site_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        // Create a test file
        std::fs::write(dir.path().join("test.txt"), "hello world").unwrap();
        std::fs::write(
            dir.path().join("data.json"),
            r#"{"greeting": "Hello", "farewell": "Goodbye"}"#,
        )
        .unwrap();
        dir
    }

    #[test]
    #[serial]
    fn test_read_file() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let result = execute_build_scripts(
            &["var content = readFile('test.txt'); log('Read: ' + content);".to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        )
        .unwrap();

        assert!(
            result
                .log_messages
                .iter()
                .any(|m| m.message.contains("Read: hello world"))
        );
    }

    #[test]
    #[serial]
    fn test_write_file() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let result = execute_build_scripts(
            &["writeFile('out.html', '<h1>Hello</h1>');".to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        )
        .unwrap();

        assert!(result.written_files.contains_key("out.html"));
        let written = std::fs::read_to_string(output.path().join("out.html")).unwrap();
        assert_eq!(written, "<h1>Hello</h1>");
    }

    #[test]
    #[serial]
    fn test_write_file_nested_dir() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let result = execute_build_scripts(
            &["writeFile('en/index.html', '<h1>English</h1>');".to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        )
        .unwrap();

        assert!(result.written_files.contains_key("en/index.html"));
        let written = std::fs::read_to_string(output.path().join("en/index.html")).unwrap();
        assert_eq!(written, "<h1>English</h1>");
    }

    #[test]
    #[serial]
    fn test_read_json() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let result = execute_build_scripts(
            &["var data = readJson('data.json'); log('greeting: ' + data.greeting);".to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        )
        .unwrap();

        assert!(
            result
                .log_messages
                .iter()
                .any(|m| m.message.contains("greeting: Hello"))
        );
    }

    #[test]
    #[serial]
    fn test_get_html() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let result = execute_build_scripts(
            &["var html = getHtml(); log('html length: ' + html.length);".to_string()],
            site.path(),
            "<html><body><h1>Test</h1></body></html>",
            output.path(),
        )
        .unwrap();

        assert!(
            result
                .log_messages
                .iter()
                .any(|m| m.message.contains("html length:"))
        );
    }

    #[test]
    #[serial]
    fn test_parse_html_and_manipulate() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let script = r#"
            var result = parseHTML('<html><body><h1 data-t="title">Original</h1></body></html>');
            var doc = result.document;
            doc.querySelector('[data-t="title"]').textContent = 'Translated';
            writeFile('out.html', doc.documentElement.outerHTML);
        "#;

        let result = execute_build_scripts(
            &[script.to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        )
        .unwrap();

        let written = std::fs::read_to_string(output.path().join("out.html")).unwrap();
        assert!(written.contains("Translated"));
        assert!(!written.contains("Original"));
        assert!(result.written_files.contains_key("out.html"));
    }

    #[test]
    #[serial]
    fn test_svg_camelcase_preserved() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let script = r#"
            var html = '<html><body><svg viewBox="0 0 100 100"><defs>' +
                '<radialGradient id="g1"><stop offset="0%"/></radialGradient>' +
                '<linearGradient id="g2"><stop offset="0%"/></linearGradient>' +
                '</defs><clipPath id="c1"><rect/></clipPath>' +
                '<feGaussianBlur stdDeviation="5"/>' +
                '</svg></body></html>';
            var result = parseHTML(html);
            var doc = result.document;
            writeFile('svg.html', doc.documentElement.outerHTML);
        "#;

        let _result = execute_build_scripts(
            &[script.to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        )
        .unwrap();

        let written = std::fs::read_to_string(output.path().join("svg.html")).unwrap();
        assert!(
            written.contains("<radialGradient"),
            "Expected <radialGradient> but got lowercase. Output: {}",
            written
        );
        assert!(
            written.contains("<linearGradient"),
            "Expected <linearGradient> but got lowercase. Output: {}",
            written
        );
        assert!(
            written.contains("<clipPath"),
            "Expected <clipPath> but got lowercase. Output: {}",
            written
        );
        assert!(
            written.contains("viewBox="),
            "Expected viewBox attribute but got lowercase. Output: {}",
            written
        );
        assert!(
            written.contains("stdDeviation="),
            "Expected stdDeviation attribute but got lowercase. Output: {}",
            written
        );
    }

    #[test]
    #[serial]
    fn test_file_exists() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let result = execute_build_scripts(
            &[
                "log('exists: ' + fileExists('test.txt'));".to_string(),
                "log('missing: ' + fileExists('nope.txt'));".to_string(),
            ],
            site.path(),
            "<html></html>",
            output.path(),
        )
        .unwrap();

        assert!(
            result
                .log_messages
                .iter()
                .any(|m| m.message == "exists: true")
        );
        assert!(
            result
                .log_messages
                .iter()
                .any(|m| m.message == "missing: false")
        );
    }

    #[test]
    #[serial]
    fn test_sandbox_read_traversal_rejected() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let result = execute_build_scripts(
            &["readFile('../../../etc/passwd');".to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        );

        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_sandbox_write_traversal_rejected() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let result = execute_build_scripts(
            &["writeFile('../escape.txt', 'evil');".to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        );

        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_log_and_warn() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let result = execute_build_scripts(
            &[
                "log('info message');".to_string(),
                "warn('warning message');".to_string(),
            ],
            site.path(),
            "<html></html>",
            output.path(),
        )
        .unwrap();

        assert!(
            result
                .log_messages
                .iter()
                .any(|m| m.level == LogLevel::Info && m.message == "info message")
        );
        assert!(
            result
                .log_messages
                .iter()
                .any(|m| m.level == LogLevel::Warn && m.message == "warning message")
        );
    }

    #[test]
    #[serial]
    fn test_copy_file() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        execute_build_scripts(
            &["copyFile('test.txt', 'copied.txt');".to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        )
        .unwrap();

        let content = std::fs::read_to_string(output.path().join("copied.txt")).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    #[serial]
    fn test_multiple_scripts_sequential() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let result = execute_build_scripts(
            &[
                "writeFile('a.txt', 'first');".to_string(),
                "writeFile('b.txt', 'second');".to_string(),
            ],
            site.path(),
            "<html></html>",
            output.path(),
        )
        .unwrap();

        assert_eq!(result.written_files.len(), 2);
        assert_eq!(
            std::fs::read_to_string(output.path().join("a.txt")).unwrap(),
            "first"
        );
        assert_eq!(
            std::fs::read_to_string(output.path().join("b.txt")).unwrap(),
            "second"
        );
    }

    // =========================================================================
    // Build-JS infrastructure and @locale integration tests
    // =========================================================================

    #[test]
    #[serial]
    fn test_parse_html() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let script = r#"
            const { document } = parseHTML('<html><body><h1>Test</h1></body></html>');
            writeFile('out.html', document.querySelector('h1').textContent);
        "#;

        execute_build_scripts(
            &[script.to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        )
        .unwrap();

        let written = std::fs::read_to_string(output.path().join("out.html")).unwrap();
        assert_eq!(written, "Test");
    }

    #[test]
    #[serial]
    fn test_get_html_returns_content() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();
        let html_input = "<html><body><h1>GetHtml Test</h1></body></html>";

        let script = r#"
            var html = getHtml();
            writeFile('captured.html', html);
        "#;

        execute_build_scripts(
            &[script.to_string()],
            site.path(),
            html_input,
            output.path(),
        )
        .unwrap();

        let written = std::fs::read_to_string(output.path().join("captured.html")).unwrap();
        assert_eq!(written, html_input);
    }

    #[test]
    #[serial]
    fn test_file_exists_true_and_false() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let script = r#"
            var existsResult = fileExists('test.txt');
            var missingResult = fileExists('nonexistent_file.xyz');
            writeFile('exists.txt', String(existsResult));
            writeFile('missing.txt', String(missingResult));
        "#;

        execute_build_scripts(
            &[script.to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        )
        .unwrap();

        let exists_val = std::fs::read_to_string(output.path().join("exists.txt")).unwrap();
        let missing_val = std::fs::read_to_string(output.path().join("missing.txt")).unwrap();
        assert_eq!(exists_val, "true");
        assert_eq!(missing_val, "false");
    }

    #[test]
    #[serial]
    fn test_sandboxing_read() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let result = execute_build_scripts(
            &["readFile('../../../etc/passwd');".to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        );

        assert!(result.is_err(), "readFile with path traversal should fail");
        let errors = result.unwrap_err();
        let combined: String = errors.iter().map(|e| e.message.clone()).collect();
        assert!(
            combined.contains("traversal")
                || combined.contains("sandbox")
                || combined.contains("Path"),
            "Error should mention path traversal: {}",
            combined
        );
    }

    #[test]
    #[serial]
    fn test_sandboxing_write() {
        let site = test_site_dir();
        let output = TempDir::new().unwrap();

        let result = execute_build_scripts(
            &["writeFile('../escape.txt', 'bad');".to_string()],
            site.path(),
            "<html></html>",
            output.path(),
        );

        assert!(result.is_err(), "writeFile with path traversal should fail");
        let errors = result.unwrap_err();
        let combined: String = errors.iter().map(|e| e.message.clone()).collect();
        assert!(
            combined.contains("traversal")
                || combined.contains("sandbox")
                || combined.contains("Path"),
            "Error should mention path traversal: {}",
            combined
        );
        // Verify the file was NOT created outside the sandbox
        assert!(!output.path().parent().unwrap().join("escape.txt").exists());
    }

    #[test]
    #[serial]
    fn test_locale_build_generates_per_locale_html() {
        let site_path = PathBuf::from("tests/fixtures/i18n-test-site");
        let output = TempDir::new().unwrap();

        let html_content = "<html><head><title>Test Site</title></head><body>\
             <h1 data-t=\"greeting\">Hello</h1>\
             <p data-t=\"farewell\">Goodbye</p>\
             <span data-t=\"missing_key\">This key only exists in HTML</span>\
             </body></html>";

        let build_script = r#"
            var locales = ["en", "fr"];
            var srcPattern = "locales/{locale}.json";
            var defaultLocale = "en";
            var rtlLocales = ["ar", "he", "fa", "ur"];

            for (var i = 0; i < locales.length; i++) {
                var locale = locales[i];
                var result = parseHTML(getHtml());
                var doc = result.document;

                var dictPath = srcPattern.replace("{locale}", locale);
                var dict = readJson(dictPath);

                var el = doc.querySelector("[data-t]");
                while (el) {
                    var key = el.getAttribute("data-t");
                    if (dict[key] !== undefined) {
                        el.textContent = dict[key];
                    } else {
                        warn('Missing translation key "' + key + '" for locale "' + locale + '"');
                    }
                    el.removeAttribute("data-t");
                    el = doc.querySelector("[data-t]");
                }

                doc.documentElement.setAttribute("lang", locale);
                if (rtlLocales.indexOf(locale) >= 0) {
                    doc.documentElement.setAttribute("dir", "rtl");
                }

                for (var k = 0; k < locales.length; k++) {
                    var l = locales[k];
                    var link = doc.createElement("link");
                    link.setAttribute("rel", "alternate");
                    link.setAttribute("hreflang", l);
                    link.setAttribute("href", "/" + l + "/");
                    doc.head.appendChild(link);
                }
                var xdefault = doc.createElement("link");
                xdefault.setAttribute("rel", "alternate");
                xdefault.setAttribute("hreflang", "x-default");
                xdefault.setAttribute("href", "/" + defaultLocale + "/");
                doc.head.appendChild(xdefault);

                writeFile(locale + "/index.html",
                    "<!DOCTYPE html>\n" + doc.documentElement.outerHTML);
                log("Generated /" + locale + "/index.html");
            }

            writeFile("index.html",
                '<!DOCTYPE html>\n<html>\n<head>\n' +
                '  <meta http-equiv="refresh" content="0; url=/' + defaultLocale + '/">\n' +
                '  <link rel="canonical" href="/' + defaultLocale + '/" />\n' +
                '</head>\n<body>\n' +
                '  <p>Redirecting to <a href="/' + defaultLocale + '/">' +
                defaultLocale + '</a>...</p>\n' +
                '</body>\n</html>');
        "#;

        let result = execute_build_scripts(
            &[build_script.to_string()],
            &site_path,
            html_content,
            output.path(),
        )
        .unwrap();

        assert!(
            result.written_files.contains_key("en/index.html"),
            "Should write en/index.html. Written: {:?}",
            result.written_files.keys().collect::<Vec<_>>()
        );
        assert!(
            result.written_files.contains_key("fr/index.html"),
            "Should write fr/index.html"
        );
        assert!(
            result.written_files.contains_key("index.html"),
            "Should write root index.html redirect"
        );

        let en_html = std::fs::read_to_string(output.path().join("en/index.html")).unwrap();
        assert!(en_html.contains("Hello"), "en page should contain 'Hello'");
        assert!(
            en_html.contains("Goodbye"),
            "en page should contain 'Goodbye'"
        );
        assert!(
            en_html.contains("lang=\"en\""),
            "en page should have lang=\"en\""
        );

        let fr_html = std::fs::read_to_string(output.path().join("fr/index.html")).unwrap();
        assert!(
            fr_html.contains("Bonjour"),
            "fr page should contain 'Bonjour'"
        );
        assert!(
            fr_html.contains("Au revoir"),
            "fr page should contain 'Au revoir'"
        );
        assert!(
            fr_html.contains("lang=\"fr\""),
            "fr page should have lang=\"fr\""
        );

        let root_html = std::fs::read_to_string(output.path().join("index.html")).unwrap();
        assert!(
            root_html.contains("url=/en/"),
            "root should redirect to /en/"
        );

        assert!(
            en_html.contains("hreflang=\"en\""),
            "en page should have hreflang en"
        );
        assert!(
            en_html.contains("hreflang=\"fr\""),
            "en page should have hreflang fr"
        );
        assert!(
            en_html.contains("hreflang=\"x-default\""),
            "en page should have x-default hreflang"
        );

        assert!(
            result
                .log_messages
                .iter()
                .any(|m| m.level == LogLevel::Warn && m.message.contains("missing_key")),
            "Should warn about missing translation key 'missing_key'"
        );
    }
}
