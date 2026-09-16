//! Parse emit content into structured IR with Marker nodes.
//!
//! Uses the tokenizer to split content into JS chunks and markers,
//! then parses JS chunks with V8 and splices in Marker nodes.
//!
//! The parser handles common patterns directly and falls back to Raw
//! for complex cases that would require full AST transformation.

use std::collections::HashMap;

use crate::emit::emit_tokenizer::{EmitSegment, tokenize_emit};
use crate::ir::{BinOp, DeclKind, JsExpr, JsLit, JsPart, JsStmt, Marker};

/// Error during emit content parsing
#[derive(Debug, Clone)]
pub struct EmitParseError {
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl std::fmt::Display for EmitParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.column) {
            (Some(l), Some(c)) => write!(f, "emit parse error at {}:{}: {}", l, c, self.message),
            (Some(l), None) => write!(f, "emit parse error at line {}: {}", l, self.message),
            _ => write!(f, "emit parse error: {}", self.message),
        }
    }
}

impl std::error::Error for EmitParseError {}

impl EmitParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
        }
    }
}

/// Result of parsing emit content, containing main code and cleanup code
#[derive(Debug, Clone, Default)]
pub struct ParsedEmit {
    /// Main statements (setup code)
    pub main: Vec<JsStmt>,
    /// Cleanup statements (teardown code from %cleanup blocks)
    pub cleanup: Vec<JsStmt>,
}

/// Convert an EmitSegment to a Marker, if the segment type has a corresponding
/// Marker variant. Returns None for segments like JsChunk, ForLoop, Cleanup
/// that don't have Marker equivalents.
fn segment_to_marker(seg: &EmitSegment) -> Option<Marker<JsExpr>> {
    match seg {
        EmitSegment::Param {
            name,
            field,
            in_string,
        } => Some(Marker::Param {
            name: name.clone(),
            field: field.clone(),
            in_string: *in_string,
        }),
        EmitSegment::Element { name } => Some(Marker::Element(name.clone())),
        EmitSegment::Signal { name } => Some(Marker::Signal(name.clone())),
        EmitSegment::VarRef {
            var,
            field,
            in_string,
        } => Some(Marker::VarRef {
            var: var.clone(),
            field: field.clone(),
            in_string: *in_string,
        }),
        // Yield, ForLoop, Cleanup, and JsChunk need special handling
        _ => None,
    }
}

/// Parse emit content into structured IR with Marker nodes intact.
///
/// Returns a vector of JS statements where Spacetime markers are preserved
/// as `JsExpr::Marker` nodes that will be resolved during final code emission.
///
/// Note: This function ignores cleanup blocks. Use `parse_emit_js_with_cleanup`
/// to get both main and cleanup statements.
pub fn parse_emit_js(content: &str) -> Result<Vec<JsStmt>, EmitParseError> {
    let parsed = parse_emit_js_with_cleanup(content)?;
    Ok(parsed.main)
}

/// Parse emit content into structured IR, returning both main and cleanup statements.
///
/// This function handles `%cleanup { ... }` blocks by parsing them separately
/// and returning them in the `cleanup` field of the result.
pub fn parse_emit_js_with_cleanup(content: &str) -> Result<ParsedEmit, EmitParseError> {
    let all_segments = tokenize_emit(content);

    if all_segments.is_empty() {
        return Ok(ParsedEmit::default());
    }

    // Separate cleanup segments from main segments
    let mut main_segments = Vec::new();
    let mut cleanup_contents = Vec::new();
    for seg in &all_segments {
        match seg {
            EmitSegment::Cleanup { content } => cleanup_contents.push(content.clone()),
            other => main_segments.push(other.clone()),
        }
    }

    // Build JS string with marker identifiers for main code
    let (js_code, marker_map) = build_parseable_js(&main_segments);

    // Validate the main JS is syntactically correct
    validate_js(&js_code)?;

    // Convert main segments to IR statements
    let main_stmts = convert_segments_to_ir(&main_segments, &marker_map)?;

    // Process cleanup segments
    let mut cleanup_stmts = Vec::new();
    for cleanup_content in cleanup_contents {
        // Recursively parse cleanup content
        let parsed = parse_emit_js_with_cleanup(&cleanup_content)?;
        cleanup_stmts.extend(parsed.main);
        cleanup_stmts.extend(parsed.cleanup);
    }

    Ok(ParsedEmit {
        main: main_stmts,
        cleanup: cleanup_stmts,
    })
}

/// Build parseable JS by replacing markers with unique identifiers.
/// Returns the JS string and a map from identifier -> Marker.
fn build_parseable_js(segments: &[EmitSegment]) -> (String, HashMap<String, Marker<JsExpr>>) {
    let mut js = String::new();
    let mut map = HashMap::new();
    let mut counter = 0;

    for seg in segments {
        // Try to use the common helper for simple segment types
        if let Some(marker) = segment_to_marker(seg) {
            let key = format!("__PH{}__", counter);
            map.insert(key.clone(), marker);
            js.push_str(&key);
            counter += 1;
            continue;
        }

        // Handle special cases that need custom logic
        match seg {
            EmitSegment::JsChunk(s) => js.push_str(s),
            EmitSegment::Yield {
                expr,
                signal,
                signal_param,
            } => {
                // Yield is a statement-level construct, emit as function call for parsing
                let key = format!("__PH{}__", counter);
                map.insert(
                    key.clone(),
                    Marker::Directive {
                        keyword: "yield".to_string(),
                        expr: Some(Box::new(JsExpr::Raw(expr.clone()))),
                        target: signal
                            .clone()
                            .or_else(|| signal_param.as_ref().map(|p| format!("%{}", p))),
                    },
                );
                // Emit as expression statement: __PH0__(expr)
                js.push_str(&format!("{}({})", key, expr));
                counter += 1;
            }
            EmitSegment::ForLoop { var, array, body } => {
                // For loops should be preprocessed before reaching here
                // Emit a marker that will be skipped
                js.push_str(&format!("/* %for ${} in %{} {{ {} }} */", var, array, body));
            }
            EmitSegment::Cleanup { content } => {
                // Cleanup blocks should be extracted before reaching here
                // Skip them in the JS output
                js.push_str(&format!("/* %cleanup {{ {} }} */", content));
            }
            // Already handled by segment_to_marker above
            _ => {}
        }
    }

    (js, map)
}

/// Validate JS syntax using SWC parser (no V8 runtime needed).
///
/// Uses swc_ecma_parser for syntax-only validation, avoiding the need
/// for a V8 isolate and eliminating thread-safety concerns.
/// Wraps the code in a function to allow return statements.
fn validate_js(js: &str) -> Result<(), EmitParseError> {
    crate::profile_span!("js_validate_swc");
    if js.trim().is_empty() {
        return Ok(());
    }

    use swc_common::{FileName, SourceMap, input::SourceFileInput};
    use swc_ecma_parser::{EsSyntax, Parser, Syntax};

    // Wrap in an ASYNC function to allow both `return` AND `await` statements
    // (macro emit bodies like @when use `await` to flush microtasks; validating
    // inside a non-async wrapper rejected them and silently dropped the emit).
    let wrapped = format!("(async function() {{\n{}\n}})", js);

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
                let msg = errors
                    .into_iter()
                    .map(|e| format!("{:?}", e.into_kind()))
                    .collect::<Vec<_>>()
                    .join("; ");
                Err(EmitParseError::new(format!("JS syntax error: {}", msg)))
            }
        }
        Err(e) => {
            let msg = format!("JS syntax error: {:?}", e.into_kind());
            Err(EmitParseError::new(msg))
        }
    }
}

/// Preprocess segments to split JsChunks at statement boundaries.
///
/// This ensures each JsChunk either ends with a semicolon (completing a statement)
/// or contains no semicolons (fragment that continues to next segment).
fn split_chunks_at_boundaries(segments: &[EmitSegment]) -> Vec<EmitSegment> {
    let mut result = Vec::new();

    for seg in segments {
        match seg {
            EmitSegment::JsChunk(chunk) => {
                // Split on semicolons at depth zero (respecting strings and nesting)
                let mut remainder = chunk.as_str();
                while !remainder.is_empty() {
                    if let Some(semi_pos) = find_semicolon_at_depth_zero(remainder) {
                        let (before, after) = remainder.split_at(semi_pos + 1);
                        if !before.is_empty() {
                            result.push(EmitSegment::JsChunk(before.to_string()));
                        }
                        remainder = after;
                    } else {
                        // No more semicolons at depth zero — push the rest
                        result.push(EmitSegment::JsChunk(remainder.to_string()));
                        break;
                    }
                }
            }
            other => result.push(other.clone()),
        }
    }

    result
}

/// Convert tokenized segments to IR statements.
///
/// This function processes segments and creates structured IR where possible,
/// falling back to Raw for complex cases.
fn convert_segments_to_ir(
    segments: &[EmitSegment],
    markers: &HashMap<String, Marker<JsExpr>>,
) -> Result<Vec<JsStmt>, EmitParseError> {
    // Preprocess: split chunks at statement boundaries for cleaner parsing
    let mut segments = split_chunks_at_boundaries(segments);

    let mut statements = Vec::new();
    let mut i = 0;

    while i < segments.len() {
        match &segments[i] {
            EmitSegment::Yield {
                expr,
                signal,
                signal_param,
            } => {
                // Parse the yield expression
                let parsed_expr = parse_expression(expr)?;
                statements.push(JsStmt::Expr(JsExpr::Marker(Marker::Directive {
                    keyword: "yield".to_string(),
                    expr: Some(Box::new(parsed_expr)),
                    target: signal
                        .clone()
                        .or_else(|| signal_param.as_ref().map(|p| format!("%{}", p))),
                })));
                i += 1;

                // Skip trailing semicolon if present
                if i < segments.len()
                    && let EmitSegment::JsChunk(chunk) = &segments[i]
                    && chunk.trim() == ";"
                {
                    i += 1;
                }
            }
            EmitSegment::JsChunk(_chunk) => {
                // Try control flow parsing first (if, return, etc.)
                if let Some((stmt, cf_consumed, remaining)) =
                    try_parse_control_flow(&segments[i..], markers)?
                {
                    statements.push(stmt);
                    i += cf_consumed;
                    // Re-inject any remaining text after the control flow block,
                    // split at statement boundaries so each statement is processed
                    // independently. Without this split, multi-statement remainders
                    // get parsed as a single Raw block, dropping everything after
                    // the first semicolon at depth 0.
                    if !remaining.trim().is_empty() {
                        let split = split_chunks_at_boundaries(&[EmitSegment::JsChunk(remaining)]);
                        for (j, chunk) in split.into_iter().enumerate() {
                            segments.insert(i + j, chunk);
                        }
                    }
                    continue;
                }

                // Fall back to general statement parsing
                let (stmt_result, consumed) = try_parse_statement(&segments[i..], markers)?;
                match stmt_result {
                    Some(stmt) => statements.push(stmt),
                    None => {
                        // Empty chunk or whitespace only
                    }
                }
                i += consumed.max(1);
            }
            // Marker segments — delegate to try_parse_statement so they
            // can be combined with following JsChunks (e.g., %&el.method())
            EmitSegment::Param { .. }
            | EmitSegment::Element { .. }
            | EmitSegment::Signal { .. }
            | EmitSegment::VarRef { .. } => {
                let (stmt_result, consumed) = try_parse_statement(&segments[i..], markers)?;
                if let Some(stmt) = stmt_result {
                    statements.push(stmt)
                }
                i += consumed.max(1);
            }
            EmitSegment::ForLoop { var, array, body } => {
                // Recursively parse the body content
                let body_stmts = parse_emit_js(body)?;
                statements.push(JsStmt::ForLoopMeta {
                    var: var.clone(),
                    array: array.clone(),
                    body: body_stmts,
                });
                i += 1;
            }
            EmitSegment::Cleanup { .. } => {
                // Cleanup segments are filtered out before this function is called
                // If we get here, skip it
                i += 1;
            }
        }
    }

    Ok(statements)
}

/// Find the position of a semicolon that's at depth 0 (not inside braces/parens)
fn find_semicolon_at_depth_zero(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut string_char = '"';
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if in_string {
            if c == '\\' && i + 1 < chars.len() {
                i += 2; // Skip escaped char
                continue;
            } else if c == string_char {
                in_string = false;
            }
        } else {
            // Comments: a `;` inside `//` or `/* */` is NOT a statement
            // boundary (FUP-063). Only recognized outside string literals; a
            // lone `/` (division / regex) is left untouched, so normal JS
            // splitting is unaffected.
            if c == '/' && i + 1 < chars.len() {
                match chars[i + 1] {
                    '/' => {
                        // Line comment: skip to end-of-line (or end-of-input).
                        i += 2;
                        while i < chars.len() && chars[i] != '\n' {
                            i += 1;
                        }
                        continue;
                    }
                    '*' => {
                        // Block comment: skip to the closing `*/`.
                        i += 2;
                        while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                            i += 1;
                        }
                        i += 2; // step past the closing `*/`
                        continue;
                    }
                    _ => {}
                }
            }
            match c {
                '"' | '\'' | '`' => {
                    in_string = true;
                    string_char = c;
                }
                '{' | '(' => depth += 1,
                '}' | ')' => depth -= 1,
                ';' if depth == 0 => return Some(i),
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// Count brace depth change in a string (handles strings and comments)
fn brace_depth_change(s: &str) -> i32 {
    let mut depth = 0i32;
    let mut in_string = false;
    let mut string_char = '"';
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if in_string {
            if c == '\\' {
                chars.next(); // Skip escaped char
            } else if c == string_char {
                in_string = false;
            }
        } else {
            match c {
                '"' | '\'' | '`' => {
                    in_string = true;
                    string_char = c;
                }
                '{' | '(' => depth += 1,
                '}' | ')' => depth -= 1,
                _ => {}
            }
        }
    }
    depth
}

// ============================================================================
// Control Flow Parsing
// ============================================================================

/// Check if a string starts with a keyword followed by a delimiter (space, paren, newline).
fn starts_with_keyword(s: &str, keyword: &str) -> bool {
    if !s.starts_with(keyword) {
        return false;
    }
    let after = &s[keyword.len()..];
    after.is_empty()
        || after.starts_with(' ')
        || after.starts_with('(')
        || after.starts_with('\n')
        || after.starts_with('\t')
        || after.starts_with(';')
}

/// Try to parse a control flow statement (if, return) from segments.
///
/// Returns (statement, segments_consumed, remaining_text) where remaining_text
/// is any text after the control flow block's closing brace that should be
/// re-injected for further parsing.
fn try_parse_control_flow(
    segments: &[EmitSegment],
    markers: &HashMap<String, Marker<JsExpr>>,
) -> Result<Option<(JsStmt, usize, String)>, EmitParseError> {
    let first_chunk = match segments.first() {
        Some(EmitSegment::JsChunk(s)) => s,
        _ => return Ok(None),
    };

    let trimmed = first_chunk.trim_start();

    // Detect return statement
    if starts_with_keyword(trimmed, "return") {
        return match try_parse_return(segments, markers)? {
            Some((stmt, consumed)) => Ok(Some((stmt, consumed, String::new()))),
            None => Ok(None),
        };
    }

    // Detect if statement
    if starts_with_keyword(trimmed, "if") {
        return try_parse_if(segments, markers);
    }

    Ok(None)
}

/// Parse a `return` statement, optionally with an expression.
///
/// Handles: `return;`, `return expr;`, `return %param;`, etc.
fn try_parse_return(
    segments: &[EmitSegment],
    markers: &HashMap<String, Marker<JsExpr>>,
) -> Result<Option<(JsStmt, usize)>, EmitParseError> {
    let first_chunk = match segments.first() {
        Some(EmitSegment::JsChunk(s)) => s.clone(),
        _ => return Ok(None),
    };

    let trimmed = first_chunk.trim_start();
    if !starts_with_keyword(trimmed, "return") {
        return Ok(None);
    }

    let after_kw = &trimmed["return".len()..];
    let after_trimmed = after_kw.trim();

    // return; — explicit semicolon
    if after_trimmed == ";" {
        return Ok(Some((JsStmt::Return(None), 1)));
    }

    // return expr; — expression entirely in first chunk
    if !after_trimmed.is_empty() && after_trimmed.ends_with(';') {
        let expr_str = after_trimmed.trim_end_matches(';').trim();
        if expr_str.is_empty() {
            return Ok(Some((JsStmt::Return(None), 1)));
        }
        let expr = build_composite_expr(expr_str, markers);
        return Ok(Some((JsStmt::Return(Some(expr)), 1)));
    }

    // Multi-segment return: "return" may have trailing whitespace with expression
    // in subsequent segments, or expression may start in this chunk and continue
    let mut expr_segments = Vec::new();
    let first_expr_part = after_kw.trim_start();
    if !first_expr_part.is_empty() {
        expr_segments.push(EmitSegment::JsChunk(first_expr_part.to_string()));
    }

    let mut consumed = 1;
    let mut found_semi = false;
    for seg in &segments[1..] {
        consumed += 1;
        match seg {
            EmitSegment::JsChunk(chunk) => {
                if let Some(semi_pos) = find_semicolon_at_depth_zero(chunk) {
                    let before = &chunk[..semi_pos];
                    if !before.trim().is_empty() {
                        expr_segments.push(EmitSegment::JsChunk(before.to_string()));
                    }
                    found_semi = true;
                    break;
                } else {
                    expr_segments.push(seg.clone());
                }
            }
            _ => {
                expr_segments.push(seg.clone());
            }
        }
    }

    // Bare "return" with no expression and no following semicolon found
    if expr_segments.is_empty() {
        if !found_semi {
            consumed = 1;
        }
        return Ok(Some((JsStmt::Return(None), consumed)));
    }

    let (js_str, ph_map) = build_parseable_js(&expr_segments);
    let expr = build_composite_expr(js_str.trim(), &ph_map);
    Ok(Some((JsStmt::Return(Some(expr)), consumed)))
}

/// Parse an `if` statement with optional else/else-if chains.
///
/// Returns (statement, segments_consumed, remaining_text) where remaining_text
/// is any unconsumed text after the if/else block.
fn try_parse_if(
    segments: &[EmitSegment],
    markers: &HashMap<String, Marker<JsExpr>>,
) -> Result<Option<(JsStmt, usize, String)>, EmitParseError> {
    let (cond_segments, body_segments, after_body, consumed) = match extract_cond_and_body(segments)
    {
        Some(result) => result,
        None => return Ok(None),
    };

    // Build condition expression from the segments between ( and )
    let (cond_js, cond_ph_map) = build_parseable_js(&cond_segments);
    let cond_expr = build_composite_expr(cond_js.trim(), &cond_ph_map);

    // Recursively parse body segments
    let body_stmts = convert_segments_to_ir(&body_segments, markers)?;

    // Check for else/else-if
    let (else_stmts, extra_consumed, remaining) =
        try_parse_else(&after_body, &segments[consumed..], markers)?;

    Ok(Some((
        JsStmt::If {
            cond: cond_expr,
            then_: body_stmts,
            else_: else_stmts,
        },
        consumed + extra_consumed,
        remaining,
    )))
}

/// Extract condition (in parens) and body (in braces) from segments
/// beginning with a keyword like `if (cond) { body }`.
///
/// Returns (condition_segments, body_segments, text_after_close_brace, segments_consumed).
fn extract_cond_and_body(
    segments: &[EmitSegment],
) -> Option<(Vec<EmitSegment>, Vec<EmitSegment>, String, usize)> {
    #[derive(PartialEq)]
    enum Phase {
        BeforeParen,
        InParen,
        BetweenParenBrace,
        InBrace,
        Done,
    }

    let mut phase = Phase::BeforeParen;
    let mut paren_depth = 0i32;
    let mut brace_depth = 0i32;
    let mut in_str = false;
    let mut str_char = '"';

    let mut cond_segs: Vec<EmitSegment> = Vec::new();
    let mut body_segs: Vec<EmitSegment> = Vec::new();
    let mut after_text = String::new();
    let mut consumed = 0;

    for seg in segments {
        consumed += 1;

        match seg {
            EmitSegment::JsChunk(chunk) => {
                let chars: Vec<char> = chunk.chars().collect();
                let mut ci = 0;
                let mut seg_start = 0;

                while ci < chars.len() {
                    let c = chars[ci];

                    // Handle string literals
                    if in_str {
                        if c == '\\' && ci + 1 < chars.len() {
                            ci += 2;
                            continue;
                        }
                        if c == str_char {
                            in_str = false;
                        }
                        ci += 1;
                        continue;
                    }
                    if c == '"' || c == '\'' || c == '`' {
                        in_str = true;
                        str_char = c;
                        ci += 1;
                        continue;
                    }

                    match phase {
                        Phase::BeforeParen => {
                            if c == '(' {
                                paren_depth = 1;
                                phase = Phase::InParen;
                                seg_start = ci + 1;
                            }
                            ci += 1;
                        }
                        Phase::InParen => {
                            if c == '(' {
                                paren_depth += 1;
                            } else if c == ')' {
                                paren_depth -= 1;
                                if paren_depth == 0 {
                                    let part: String = chars[seg_start..ci].iter().collect();
                                    if !part.is_empty() {
                                        cond_segs.push(EmitSegment::JsChunk(part));
                                    }
                                    phase = Phase::BetweenParenBrace;
                                    seg_start = ci + 1;
                                }
                            }
                            ci += 1;
                        }
                        Phase::BetweenParenBrace => {
                            // BUG-212: only a `{` IMMEDIATELY after the condition
                            // opens the if-body. Anything else means the `if` is
                            // BODYLESS (`if (!x) return;`) and this parse must not
                            // claim it.
                            //
                            // This phase used to skip every character until it found
                            // a `{` anywhere ahead, which silently spliced the guard
                            // to the next brace in the file. For
                            //
                            //     if (!x) return;
                            //     const f = () => { x.dataset.hit = '1'; };
                            //
                            // it discarded `return; const f = () =>` and adopted the
                            // ARROW's body as the if's body. Three corruptions from
                            // one skip: the `return` vanished, the guard inverted
                            // (work meant for when `x` EXISTS now runs only when it
                            // is missing, dereferencing the pointer just proven
                            // null), and `const f` was dropped so the following
                            // `addEventListener('input', f)` referenced nothing.
                            //
                            // All silent. The emitted guard still reads plausibly in
                            // review because the `if` is right there.
                            if c == '{' {
                                brace_depth = 1;
                                phase = Phase::InBrace;
                                seg_start = ci + 1;
                                ci += 1;
                            } else if c.is_whitespace() {
                                ci += 1;
                            } else {
                                // A bodyless `if`. Refusing here makes the caller
                                // fall back to passing the statement through
                                // verbatim, which is exactly right: the source is
                                // already valid JS and needs no restructuring.
                                return None;
                            }
                        }
                        Phase::InBrace => {
                            if c == '{' {
                                brace_depth += 1;
                                ci += 1;
                            } else if c == '}' {
                                brace_depth -= 1;
                                if brace_depth == 0 {
                                    let part: String = chars[seg_start..ci].iter().collect();
                                    if !part.is_empty() {
                                        body_segs.push(EmitSegment::JsChunk(part));
                                    }
                                    after_text = chars[ci + 1..].iter().collect();
                                    phase = Phase::Done;
                                    // Don't increment ci — we break out
                                    break;
                                }
                                ci += 1;
                            } else {
                                ci += 1;
                            }
                        }
                        Phase::Done => break,
                    }
                }

                if phase == Phase::Done {
                    break;
                }

                // Add remainder of this chunk to the current collection
                match phase {
                    Phase::InParen => {
                        let rem: String = chars[seg_start..].iter().collect();
                        if !rem.is_empty() {
                            cond_segs.push(EmitSegment::JsChunk(rem));
                        }
                    }
                    Phase::InBrace => {
                        let rem: String = chars[seg_start..].iter().collect();
                        if !rem.is_empty() {
                            body_segs.push(EmitSegment::JsChunk(rem));
                        }
                    }
                    _ => {}
                }
            }
            other => {
                match phase {
                    Phase::InParen => cond_segs.push(other.clone()),
                    Phase::InBrace => body_segs.push(other.clone()),
                    Phase::BeforeParen | Phase::BetweenParenBrace => {
                        // Marker outside condition/body — can't parse structurally
                        return None;
                    }
                    Phase::Done => break,
                }
            }
        }
    }

    if phase != Phase::Done {
        return None;
    }

    Some((cond_segs, body_segs, after_text, consumed))
}

/// Parse an `else` or `else if` clause after an if-body's closing brace.
///
/// `after_body` is the text remaining in the chunk after `}`.
/// `remaining_segments` are the unconsumed segments after the if statement.
///
/// Returns (else_body, extra_segments_consumed, remaining_text).
fn try_parse_else(
    after_body: &str,
    remaining_segments: &[EmitSegment],
    markers: &HashMap<String, Marker<JsExpr>>,
) -> Result<(Option<Vec<JsStmt>>, usize, String), EmitParseError> {
    let trimmed = after_body.trim();
    if !trimmed.starts_with("else") {
        // No else clause — return the after_body text as remaining
        return Ok((None, 0, after_body.to_string()));
    }

    let after_else_kw = &trimmed["else".len()..];
    // "else" must be followed by space, newline, or {
    if !after_else_kw.is_empty()
        && !after_else_kw.starts_with(' ')
        && !after_else_kw.starts_with('\n')
        && !after_else_kw.starts_with('\t')
        && !after_else_kw.starts_with('{')
    {
        return Ok((None, 0, after_body.to_string()));
    }

    let after_else = after_else_kw.trim_start();

    // else if (...) { ... }
    if starts_with_keyword(after_else, "if") {
        let mut synth_segments = vec![EmitSegment::JsChunk(after_else.to_string())];
        synth_segments.extend(remaining_segments.iter().cloned());

        match try_parse_if(&synth_segments, markers)? {
            Some((else_if_stmt, synth_consumed, remaining)) => {
                // First segment was synthetic (from after_body text),
                // so real segments consumed = synth_consumed - 1
                let extra = synth_consumed.saturating_sub(1);
                Ok((Some(vec![else_if_stmt]), extra, remaining))
            }
            None => Ok((None, 0, after_body.to_string())),
        }
    }
    // else { ... }
    else if let Some(inside_brace) = after_else.strip_prefix('{') {
        // skip the {
        let mut content_segments = Vec::new();
        let synth_offset = if inside_brace.is_empty() {
            0
        } else {
            content_segments.push(EmitSegment::JsChunk(inside_brace.to_string()));
            1
        };
        content_segments.extend(remaining_segments.iter().cloned());

        match extract_brace_body(&content_segments) {
            Some((body_segs, after_text, brace_consumed)) => {
                let body_stmts = convert_segments_to_ir(&body_segs, markers)?;
                let extra = brace_consumed.saturating_sub(synth_offset);
                Ok((Some(body_stmts), extra, after_text))
            }
            None => Ok((None, 0, after_body.to_string())),
        }
    } else {
        Ok((None, 0, after_body.to_string()))
    }
}

/// Extract body segments from inside braces, starting AFTER the opening `{`.
///
/// Returns (body_segments, text_after_close_brace, segments_consumed).
fn extract_brace_body(segments: &[EmitSegment]) -> Option<(Vec<EmitSegment>, String, usize)> {
    let mut body_segs = Vec::new();
    let mut brace_depth = 1i32;
    let mut in_str = false;
    let mut str_char = '"';
    let mut consumed = 0;

    for seg in segments {
        consumed += 1;
        match seg {
            EmitSegment::JsChunk(chunk) => {
                let chars: Vec<char> = chunk.chars().collect();
                let mut ci = 0;
                let seg_start = 0;

                while ci < chars.len() {
                    let c = chars[ci];

                    if in_str {
                        if c == '\\' && ci + 1 < chars.len() {
                            ci += 2;
                            continue;
                        }
                        if c == str_char {
                            in_str = false;
                        }
                        ci += 1;
                        continue;
                    }
                    if c == '"' || c == '\'' || c == '`' {
                        in_str = true;
                        str_char = c;
                        ci += 1;
                        continue;
                    }

                    if c == '{' {
                        brace_depth += 1;
                    } else if c == '}' {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            let part: String = chars[seg_start..ci].iter().collect();
                            if !part.is_empty() {
                                body_segs.push(EmitSegment::JsChunk(part));
                            }
                            let after: String = chars[ci + 1..].iter().collect();
                            return Some((body_segs, after, consumed));
                        }
                    }
                    ci += 1;
                }

                // Whole chunk is part of body
                let rem: String = chars[seg_start..].iter().collect();
                if !rem.is_empty() {
                    body_segs.push(EmitSegment::JsChunk(rem));
                }
            }
            other => {
                body_segs.push(other.clone());
            }
        }
    }

    None // Unmatched brace
}

/// Try to parse a statement from segments starting at the given position.
/// Returns the parsed statement (if any) and number of segments consumed.
///
/// This function tracks brace/paren depth to properly handle callbacks and
/// nested expressions that contain markers.
fn try_parse_statement(
    segments: &[EmitSegment],
    _markers: &HashMap<String, Marker<JsExpr>>,
) -> Result<(Option<JsStmt>, usize), EmitParseError> {
    if segments.is_empty() {
        return Ok((None, 0));
    }

    // Collect segments until we hit a statement boundary at depth 0
    let mut js_parts = Vec::new();
    let mut marker_positions = Vec::new();
    let mut consumed = 0;
    let mut depth = 0i32; // Track brace/paren nesting depth

    for seg in segments {
        // Try the common helper for simple segment types (Param, Element, Signal, VarRef)
        if let Some(marker) = segment_to_marker(seg) {
            let key = format!("__PH{}__", marker_positions.len());
            marker_positions.push((key.clone(), marker));
            js_parts.push(key);
            consumed += 1;
            continue;
        }

        // Handle segments that need special logic
        match seg {
            EmitSegment::JsChunk(chunk) => {
                // Update depth for this chunk
                depth += brace_depth_change(chunk);

                // Check for statement terminator at depth 0
                if depth <= 0
                    && let Some(semi_pos) = find_semicolon_at_depth_zero(chunk)
                {
                    // Include up to and including the semicolon
                    js_parts.push(chunk[..=semi_pos].to_string());
                    consumed += 1;
                    break;
                }

                js_parts.push(chunk.clone());
                consumed += 1;
            }
            EmitSegment::Yield {
                expr,
                signal,
                signal_param,
            } => {
                // If we're inside a callback (depth > 0), include the yield as part of this statement
                if depth > 0 {
                    let key = format!("__YIELD{}__", marker_positions.len());
                    marker_positions.push((
                        key.clone(),
                        Marker::Directive {
                            keyword: "yield".to_string(),
                            expr: Some(Box::new(
                                parse_expression(expr).unwrap_or(JsExpr::Var(expr.clone())),
                            )),
                            target: signal
                                .clone()
                                .or_else(|| signal_param.as_ref().map(|p| format!("%{}", p))),
                        },
                    ));
                    js_parts.push(key);
                    consumed += 1;
                } else {
                    // At top level, yield is a separate statement
                    break;
                }
            }
            EmitSegment::ForLoop { .. } => {
                // ForLoop is always a separate statement
                break;
            }
            EmitSegment::Cleanup { .. } => {
                // Cleanup should be filtered out before reaching here
                break;
            }
            // Already handled by segment_to_marker above
            _ => {}
        }
    }

    if js_parts.is_empty() || consumed == 0 {
        return Ok((None, consumed.max(1)));
    }

    let combined = js_parts.join("");
    let trimmed = combined.trim();

    if trimmed.is_empty() {
        return Ok((None, consumed));
    }

    // Try to parse as a declaration or simple statement
    let local_markers: HashMap<String, Marker<JsExpr>> = marker_positions.into_iter().collect();

    if let Some(stmt) = try_parse_decl(trimmed, &local_markers)? {
        return Ok((Some(stmt), consumed));
    }

    // Fall back to Composite expression wrapped in Expr statement
    let composite = build_composite_expr(trimmed, &local_markers);
    match composite {
        JsExpr::Raw(s) => Ok((Some(JsStmt::Raw(s)), consumed)),
        _ => Ok((Some(JsStmt::Expr(composite)), consumed)),
    }
}

/// Try to parse as a variable declaration: const/let/var name = expr;
fn try_parse_decl(
    js: &str,
    markers: &HashMap<String, Marker<JsExpr>>,
) -> Result<Option<JsStmt>, EmitParseError> {
    let trimmed = js.trim().trim_end_matches(';');

    // Check for declaration keywords
    let (kind, rest) = if let Some(rest) = trimmed.strip_prefix("const ") {
        (DeclKind::Const, rest.trim())
    } else if let Some(rest) = trimmed.strip_prefix("let ") {
        (DeclKind::Let, rest.trim())
    } else if let Some(rest) = trimmed.strip_prefix("var ") {
        (DeclKind::Var, rest.trim())
    } else {
        return Ok(None);
    };

    // Parse: name = expr
    let eq_pos = match rest.find('=') {
        Some(pos) => pos,
        None => {
            // Declaration without initializer: const name;
            let name = rest.trim().to_string();
            if is_valid_identifier(&name) {
                return Ok(Some(JsStmt::Decl {
                    kind,
                    name,
                    init: None,
                }));
            }
            return Ok(None);
        }
    };

    let name = rest[..eq_pos].trim().to_string();
    let expr_str = rest[eq_pos + 1..].trim();

    if !is_valid_identifier(&name) {
        return Ok(None);
    }

    // Parse the expression
    let expr = parse_expression_with_markers(expr_str, markers)?;

    Ok(Some(JsStmt::Decl {
        kind,
        name,
        init: Some(expr),
    }))
}

/// Parse an expression string, substituting marker identifiers
fn parse_expression_with_markers(
    expr_str: &str,
    markers: &HashMap<String, Marker<JsExpr>>,
) -> Result<JsExpr, EmitParseError> {
    let trimmed = expr_str.trim();

    // Check if it's exactly a marker identifier
    if let Some(marker) = markers.get(trimmed) {
        return Ok(JsExpr::Marker(marker.clone()));
    }

    // Check for simple binary operations with marker on RHS
    // Pattern: simple_expr OP marker (e.g., "1000 / __PH0__")
    // Only matches when the left side is a simple expression (identifier, literal)
    for (key, marker) in markers {
        if let Some(op_pos) = trimmed.rfind(key.as_str()) {
            let before = trimmed[..op_pos].trim();

            // Check for binary operator before the marker
            if let Some((left_str, op)) = parse_binary_op_prefix(before) {
                // Only use this pattern if left side is a simple expression
                // (avoids matching complex nested expressions)
                if let Ok(left) = parse_simple_expr(left_str) {
                    return Ok(JsExpr::Binary {
                        left: Box::new(left),
                        op,
                        right: Box::new(JsExpr::Marker(marker.clone())),
                    });
                }
            }
        }
    }

    // Try to parse as simple expression
    if let Ok(expr) = parse_simple_expr(trimmed) {
        return Ok(expr);
    }

    // Fall back to Composite with structured markers
    Ok(build_composite_expr(trimmed, markers))
}

/// Parse the left side of a binary operation, returning (left_expr_str, operator)
fn parse_binary_op_prefix(s: &str) -> Option<(&str, BinOp)> {
    let trimmed = s.trim();

    // Check for operators at the end
    let ops = [
        ("/ ", BinOp::Div),
        ("* ", BinOp::Mul),
        ("+ ", BinOp::Add),
        ("- ", BinOp::Sub),
        ("% ", BinOp::Mod),
        ("=== ", BinOp::EqStrict),
        ("!== ", BinOp::NeStrict),
        ("== ", BinOp::Eq),
        ("!= ", BinOp::Ne),
        (">= ", BinOp::Ge),
        ("<= ", BinOp::Le),
        ("> ", BinOp::Gt),
        ("< ", BinOp::Lt),
        ("&& ", BinOp::And),
        ("|| ", BinOp::Or),
    ];

    for (op_str, op) in ops {
        if trimmed.ends_with(op_str.trim()) {
            let left = trimmed[..trimmed.len() - op_str.trim().len()].trim();
            return Some((left, op));
        }
    }

    None
}

/// Parse a simple expression (literal, identifier, or basic construct)
fn parse_simple_expr(s: &str) -> Result<JsExpr, EmitParseError> {
    let trimmed = s.trim();

    // Number literal
    if let Ok(n) = trimmed.parse::<f64>() {
        return Ok(JsExpr::Lit(JsLit::Number(n)));
    }

    // Boolean literal
    if trimmed == "true" {
        return Ok(JsExpr::Lit(JsLit::Bool(true)));
    }
    if trimmed == "false" {
        return Ok(JsExpr::Lit(JsLit::Bool(false)));
    }

    // Null/undefined
    if trimmed == "null" {
        return Ok(JsExpr::Lit(JsLit::Null));
    }
    if trimmed == "undefined" {
        return Ok(JsExpr::Lit(JsLit::Undefined));
    }

    // String literal
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        let content = &trimmed[1..trimmed.len() - 1];
        return Ok(JsExpr::Lit(JsLit::String(content.to_string())));
    }

    // Simple identifier
    if is_valid_identifier(trimmed) {
        return Ok(JsExpr::Var(trimmed.to_string()));
    }

    Err(EmitParseError::new(format!(
        "cannot parse expression: {}",
        trimmed
    )))
}

/// Parse an expression string (for %yield expressions)
fn parse_expression(expr_str: &str) -> Result<JsExpr, EmitParseError> {
    parse_expression_with_markers(expr_str, &HashMap::new())
}

/// Build a Composite expression from a string containing marker identifiers.
///
/// Instead of substituting markers back to their original syntax (which would
/// require regex during emission), this builds a structured Composite with
/// interleaved Raw strings and Marker nodes.
fn build_composite_expr(s: &str, markers: &HashMap<String, Marker<JsExpr>>) -> JsExpr {
    // If no markers, just return Raw
    if markers.is_empty() {
        return JsExpr::Raw(s.to_string());
    }

    // Collect markers sorted by their position in the string
    let mut marker_positions: Vec<(usize, &str, &Marker<JsExpr>)> = Vec::new();
    for (key, marker) in markers {
        let mut search_start = 0;
        while let Some(pos) = s[search_start..].find(key.as_str()) {
            let abs_pos = search_start + pos;
            marker_positions.push((abs_pos, key, marker));
            search_start = abs_pos + key.len();
        }
    }

    // If no markers found in string, return Raw
    if marker_positions.is_empty() {
        return JsExpr::Raw(s.to_string());
    }

    // Sort by position
    marker_positions.sort_by_key(|(pos, _, _)| *pos);

    // Build parts
    let mut parts: Vec<JsPart> = Vec::new();
    let mut last_end = 0;

    for (pos, key, marker) in marker_positions {
        // Add raw part before this marker (if any)
        if pos > last_end {
            let raw = s[last_end..pos].to_string();
            if !raw.is_empty() {
                parts.push(JsPart::Raw(raw));
            }
        }
        // Add the marker
        parts.push(JsPart::Marker((*marker).clone()));
        last_end = pos + key.len();
    }

    // Add remaining raw part (if any)
    if last_end < s.len() {
        let raw = s[last_end..].to_string();
        if !raw.is_empty() {
            parts.push(JsPart::Raw(raw));
        }
    }

    // If only one part and it's Raw, return Raw directly
    if parts.len() == 1 {
        if let JsPart::Raw(s) = &parts[0] {
            return JsExpr::Raw(s.clone());
        }
        if let JsPart::Marker(m) = &parts[0] {
            // Single marker — return directly as JsExpr::Marker
            return JsExpr::Marker(m.clone());
        }
    }

    JsExpr::Composite(parts)
}

/// Check if a string is a valid JavaScript identifier
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_alphabetic() && first != '_' && first != '$' {
        return false;
    }
    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let stmts = parse_emit_js("").unwrap();
        assert!(stmts.is_empty());
    }

    #[test]
    fn parse_plain_js_decl() {
        let stmts = parse_emit_js("const x = 42;").unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Decl { kind, name, init } => {
                assert_eq!(*kind, DeclKind::Const);
                assert_eq!(name, "x");
                assert!(matches!(init, Some(JsExpr::Lit(JsLit::Number(n))) if *n == 42.0));
            }
            _ => panic!("Expected Decl"),
        }
    }

    #[test]
    fn parse_let_decl() {
        let stmts = parse_emit_js("let running = true;").unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Decl { kind, name, init } => {
                assert_eq!(*kind, DeclKind::Let);
                assert_eq!(name, "running");
                assert!(matches!(init, Some(JsExpr::Lit(JsLit::Bool(true)))));
            }
            _ => panic!("Expected Decl"),
        }
    }

    #[test]
    fn parse_with_param_marker() {
        let stmts = parse_emit_js("const fps = %fps;").unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Decl { name, init, .. } => {
                assert_eq!(name, "fps");
                match init {
                    Some(JsExpr::Marker(Marker::Param { name, .. })) => {
                        assert_eq!(name, "fps");
                    }
                    _ => panic!("Expected Marker::Param"),
                }
            }
            _ => panic!("Expected Decl"),
        }
    }

    #[test]
    fn parse_param_in_binary_expr() {
        let stmts = parse_emit_js("const interval = 1000 / %fps;").unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Decl { name, init, .. } => {
                assert_eq!(name, "interval");
                match init {
                    Some(JsExpr::Binary { left, op, right }) => {
                        assert!(
                            matches!(left.as_ref(), JsExpr::Lit(JsLit::Number(n)) if *n == 1000.0)
                        );
                        assert_eq!(*op, BinOp::Div);
                        assert!(
                            matches!(right.as_ref(), JsExpr::Marker(Marker::Param { name, .. }) if name == "fps")
                        );
                    }
                    _ => panic!("Expected Binary expression, got {:?}", init),
                }
            }
            _ => panic!("Expected Decl"),
        }
    }

    #[test]
    fn parse_with_element_marker() {
        let stmts = parse_emit_js("const el = %&container;").unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Decl { name, init, .. } => {
                assert_eq!(name, "el");
                match init {
                    Some(JsExpr::Marker(Marker::Element(name))) => {
                        assert_eq!(name, "container");
                    }
                    _ => panic!("Expected Marker::Element"),
                }
            }
            _ => panic!("Expected Decl"),
        }
    }

    #[test]
    fn parse_with_signal_marker() {
        let stmts = parse_emit_js("const val = $progress;").unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Decl { name, init, .. } => {
                assert_eq!(name, "val");
                match init {
                    Some(JsExpr::Marker(Marker::Signal(name))) => {
                        assert_eq!(name, "progress");
                    }
                    _ => panic!("Expected Marker::Signal"),
                }
            }
            _ => panic!("Expected Decl"),
        }
    }

    #[test]
    fn parse_yield_statement() {
        let stmts = parse_emit_js("%yield now -> $t;").unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Expr(JsExpr::Marker(Marker::Directive {
                keyword,
                expr,
                target,
            })) => {
                assert_eq!(keyword, "yield");
                assert_eq!(target.as_deref(), Some("t"));
                assert!(matches!(expr.as_deref(), Some(JsExpr::Var(n)) if n == "now"));
            }
            _ => panic!("Expected Yield marker, got {:?}", stmts[0]),
        }
    }

    #[test]
    fn parse_yield_complex_expr() {
        let stmts = parse_emit_js("%yield t * 100 -> $progress;").unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Expr(JsExpr::Marker(Marker::Directive {
                keyword, target, ..
            })) => {
                assert_eq!(keyword, "yield");
                assert_eq!(target.as_deref(), Some("progress"));
            }
            _ => panic!("Expected Yield marker"),
        }
    }

    #[test]
    fn parse_string_literal() {
        let stmts = parse_emit_js("const name = \"hello\";").unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Decl {
                init: Some(JsExpr::Lit(JsLit::String(s))),
                ..
            } => {
                assert_eq!(s, "hello");
            }
            _ => panic!("Expected string literal"),
        }
    }

    #[test]
    fn parse_identifier_expr() {
        let stmts = parse_emit_js("const x = someVar;").unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Decl {
                init: Some(JsExpr::Var(v)),
                ..
            } => {
                assert_eq!(v, "someVar");
            }
            _ => panic!("Expected Var"),
        }
    }

    #[test]
    fn invalid_js_returns_error() {
        let result = parse_emit_js("const x = ;");
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("syntax error"));
    }

    #[test]
    fn is_valid_identifier_tests() {
        assert!(is_valid_identifier("foo"));
        assert!(is_valid_identifier("_bar"));
        assert!(is_valid_identifier("$baz"));
        assert!(is_valid_identifier("foo123"));
        assert!(is_valid_identifier("foo_bar"));
        assert!(!is_valid_identifier("123foo"));
        assert!(!is_valid_identifier("foo-bar"));
        assert!(!is_valid_identifier(""));
    }

    #[test]
    fn parse_complex_emit_block() {
        // A realistic emit block from a primitive
        let js = r#"let running = %autoStart;
const loop = (now) => {
    if (!$running) return;
    %yield now -> $t;
    requestAnimationFrame(loop);
};"#;

        let result = parse_emit_js(js);
        // Should not error - complex code falls back to Raw
        assert!(result.is_ok());
        let stmts = result.unwrap();

        // First statement should be the let declaration with marker
        assert!(stmts.len() >= 1);
        match &stmts[0] {
            JsStmt::Decl { kind, name, init } => {
                assert_eq!(*kind, DeclKind::Let);
                assert_eq!(name, "running");
                assert!(matches!(
                    init,
                    Some(JsExpr::Marker(Marker::Param { name, .. })) if name == "autoStart"
                ));
            }
            _ => panic!("Expected first statement to be Decl with Marker::Param"),
        }
    }

    // =========================================================================
    // Cleanup Block Parsing Tests
    // =========================================================================

    #[test]
    fn parse_cleanup_simple() {
        let input = r#"el.addEventListener('click', handler);
%cleanup {
    el.removeEventListener('click', handler);
}"#;
        let result = parse_emit_js_with_cleanup(input).unwrap();
        assert!(!result.main.is_empty());
        assert!(!result.cleanup.is_empty());
    }

    #[test]
    fn parse_cleanup_with_nested_braces() {
        let input = r#"observer.observe(el);
%cleanup {
    if (observer) {
        observer.disconnect();
    }
}"#;
        let result = parse_emit_js_with_cleanup(input).unwrap();
        assert!(!result.main.is_empty());
        assert!(!result.cleanup.is_empty());
    }

    #[test]
    fn parse_cleanup_multiple_blocks() {
        let input = r#"setup1();
%cleanup { cleanup1(); }
setup2();
%cleanup { cleanup2(); }"#;
        let result = parse_emit_js_with_cleanup(input).unwrap();
        assert!(result.main.len() >= 2);
        assert!(result.cleanup.len() >= 2);
    }

    #[test]
    fn parse_cleanup_with_placeholders() {
        let input = r#"const el = %&container;
el.addEventListener('click', handler);
%cleanup {
    %&container.removeEventListener('click', handler);
}"#;
        let result = parse_emit_js_with_cleanup(input).unwrap();
        assert!(!result.main.is_empty());
        assert!(!result.cleanup.is_empty());
    }

    #[test]
    fn parse_cleanup_empty() {
        let input = "const x = 42;";
        let result = parse_emit_js_with_cleanup(input).unwrap();
        assert!(!result.main.is_empty());
        assert!(result.cleanup.is_empty());
    }

    #[test]
    fn parse_cleanup_only() {
        let input = "%cleanup { observer.disconnect(); }";
        let result = parse_emit_js_with_cleanup(input).unwrap();
        assert!(result.main.is_empty());
        assert!(!result.cleanup.is_empty());
    }

    #[test]
    fn parse_cleanup_content_parsed_structurally() {
        let input = r#"%cleanup {
    const fps = %fps;
}"#;
        let result = parse_emit_js_with_cleanup(input).unwrap();
        assert_eq!(result.cleanup.len(), 1);
        match &result.cleanup[0] {
            JsStmt::Decl { kind, name, init } => {
                assert_eq!(*kind, DeclKind::Const);
                assert_eq!(name, "fps");
                assert!(matches!(
                    init,
                    Some(JsExpr::Marker(Marker::Param { name, .. })) if name == "fps"
                ));
            }
            _ => panic!("Expected Decl in cleanup, got {:?}", result.cleanup[0]),
        }
    }

    #[test]
    fn parse_element_method_call() {
        // %&el.removeEventListener should produce a single composite expression,
        // not separate `el;` and `.removeEventListener(...)` statements
        let stmts = parse_emit_js("%&el.removeEventListener('click', handler);").unwrap();
        assert_eq!(
            stmts.len(),
            1,
            "Should be single statement, got: {:?}",
            stmts
        );
        match &stmts[0] {
            JsStmt::Expr(JsExpr::Composite(parts)) => {
                assert!(matches!(&parts[0], JsPart::Marker(Marker::Element(name)) if name == "el"));
                assert!(
                    matches!(&parts[1], JsPart::Raw(s) if s.starts_with(".removeEventListener"))
                );
            }
            other => panic!("Expected Composite expression, got: {:?}", other),
        }
    }

    #[test]
    fn parse_cleanup_element_method_call() {
        let input = r#"el.addEventListener('click', handler);
%cleanup {
    %&el.removeEventListener('click', handler);
}"#;
        let result = parse_emit_js_with_cleanup(input).unwrap();
        assert!(!result.cleanup.is_empty());
        assert_eq!(
            result.cleanup.len(),
            1,
            "Should be single cleanup statement, got: {:?}",
            result.cleanup
        );
    }

    #[test]
    fn parse_element_property_access() {
        // %&el.style.display should be a single composite expression
        let stmts = parse_emit_js("%&el.style.display = 'none';").unwrap();
        assert_eq!(
            stmts.len(),
            1,
            "Should be single statement, got: {:?}",
            stmts
        );
    }

    #[test]
    fn parse_emit_js_ignores_cleanup() {
        let input = r#"const x = 42;
%cleanup { doCleanup(); }"#;
        let stmts = parse_emit_js(input).unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Decl { name, .. } => {
                assert_eq!(name, "x");
            }
            _ => panic!("Expected Decl"),
        }
    }

    #[test]
    fn parse_arrow_function_with_nested_placeholders() {
        // This tests the case where an arrow function body contains placeholders
        // deep inside - the parser should not fail trying to match simple binary
        // patterns on the complex expression.
        let input = r#"const setState = (newState) => {
      const oldState = __state;
      if (%&el.__stStateStartTime) {
        %&el.__stStateDurations[oldState] = now - %&el.__stStateStartTime;
      }
    };"#;
        // Should successfully parse without error
        let result = parse_emit_js(input);
        assert!(
            result.is_ok(),
            "Parser should not fail on complex arrow functions: {:?}",
            result
        );

        let stmts = result.unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Decl {
                kind: DeclKind::Const,
                name,
                init: Some(_),
            } => {
                assert_eq!(name, "setState");
            }
            other => panic!("Expected Const decl for setState, got: {:?}", other),
        }
    }

    // =========================================================================
    // Control Flow Parsing Tests
    // =========================================================================

    #[test]
    fn parse_if_with_yield_in_body() {
        // The core bug: %yield inside if-block should produce JsStmt::If
        // with the yield as a structured statement in the body
        let input = r#"if (!%&el) {
    %yield 0.5 -> $progress;
    return;
}"#;
        let stmts = parse_emit_js(input).unwrap();
        assert_eq!(
            stmts.len(),
            1,
            "Should be single if statement, got: {:?}",
            stmts
        );
        match &stmts[0] {
            JsStmt::If { cond, then_, else_ } => {
                // Condition should contain the element reference
                match cond {
                    JsExpr::Composite(parts) => {
                        assert!(
                            parts.iter().any(
                                |p| matches!(p, JsPart::Marker(Marker::Element(n)) if n == "el")
                            ),
                            "Condition should contain element marker, got: {:?}",
                            parts
                        );
                    }
                    _ => panic!("Expected Composite condition, got: {:?}", cond),
                }
                // Body should contain yield + return
                assert!(
                    then_.len() >= 2,
                    "Body should have at least 2 statements (yield + return), got: {:?}",
                    then_
                );
                // First body statement should be the yield
                assert!(
                    matches!(&then_[0], JsStmt::Expr(JsExpr::Marker(Marker::Directive { keyword, .. })) if keyword == "yield"),
                    "First body stmt should be Yield, got: {:?}",
                    then_[0]
                );
                // Second should be return
                assert!(
                    matches!(&then_[1], JsStmt::Return(None)),
                    "Second body stmt should be Return, got: {:?}",
                    then_[1]
                );
                // No else
                assert!(else_.is_none());
            }
            other => panic!("Expected If statement, got: {:?}", other),
        }
    }

    #[test]
    fn parse_if_else_with_yields() {
        let input = r#"if (%active) {
    %yield 1 -> $progress;
} else {
    %yield 0 -> $progress;
}"#;
        let stmts = parse_emit_js(input).unwrap();
        assert_eq!(
            stmts.len(),
            1,
            "Should be single if-else statement, got: {:?}",
            stmts
        );
        match &stmts[0] {
            JsStmt::If { then_, else_, .. } => {
                assert!(!then_.is_empty(), "Then body should not be empty");
                assert!(
                    matches!(&then_[0], JsStmt::Expr(JsExpr::Marker(Marker::Directive { keyword, .. })) if keyword == "yield"),
                    "Then body should have yield, got: {:?}",
                    then_[0]
                );
                assert!(else_.is_some(), "Should have else branch");
                let else_body = else_.as_ref().unwrap();
                assert!(!else_body.is_empty(), "Else body should not be empty");
                assert!(
                    matches!(&else_body[0], JsStmt::Expr(JsExpr::Marker(Marker::Directive { keyword, .. })) if keyword == "yield"),
                    "Else body should have yield, got: {:?}",
                    else_body[0]
                );
            }
            other => panic!("Expected If statement, got: {:?}", other),
        }
    }

    #[test]
    fn parse_if_else_if_chain() {
        let input = r#"if (%axis === 'x') {
    %yield x -> $progress;
} else if (%axis === 'y') {
    %yield y -> $progress;
} else {
    %yield both -> $progress;
}"#;
        let stmts = parse_emit_js(input).unwrap();
        assert_eq!(
            stmts.len(),
            1,
            "Should be single if-else-if statement, got: {:?}",
            stmts
        );
        match &stmts[0] {
            JsStmt::If {
                else_: Some(else_body),
                ..
            } => {
                // else branch should contain another If
                assert_eq!(else_body.len(), 1, "Else should contain one if stmt");
                match &else_body[0] {
                    JsStmt::If {
                        else_: Some(final_else),
                        ..
                    } => {
                        // Final else body should have yield
                        assert!(!final_else.is_empty());
                    }
                    other => panic!("Expected nested If in else, got: {:?}", other),
                }
            }
            other => panic!("Expected If with else, got: {:?}", other),
        }
    }

    #[test]
    fn parse_return_no_expr() {
        let input = "return;";
        let stmts = parse_emit_js(input).unwrap();
        assert_eq!(stmts.len(), 1);
        assert!(
            matches!(&stmts[0], JsStmt::Return(None)),
            "Expected Return(None), got: {:?}",
            stmts[0]
        );
    }

    #[test]
    fn parse_return_with_expr() {
        let input = "return someValue;";
        let stmts = parse_emit_js(input).unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Return(Some(_)) => {}
            other => panic!("Expected Return(Some(_)), got: {:?}", other),
        }
    }

    #[test]
    fn parse_return_with_marker() {
        let input = "return %defaultValue;";
        let stmts = parse_emit_js(input).unwrap();
        assert_eq!(stmts.len(), 1);
        match &stmts[0] {
            JsStmt::Return(Some(expr)) => {
                assert!(
                    matches!(expr, JsExpr::Marker(Marker::Param { name, .. }) if name == "defaultValue"),
                    "Expected Param marker, got: {:?}",
                    expr
                );
            }
            other => panic!("Expected Return with marker, got: {:?}", other),
        }
    }

    #[test]
    fn parse_if_with_plain_js_body() {
        // If with only plain JS (no placeholders) should also produce structured If
        let input = r#"if (condition) {
    doSomething();
    return;
}"#;
        let stmts = parse_emit_js(input).unwrap();
        assert_eq!(
            stmts.len(),
            1,
            "Should be single if statement, got: {:?}",
            stmts
        );
        match &stmts[0] {
            JsStmt::If { then_, .. } => {
                assert!(
                    then_.len() >= 2,
                    "Body should have at least 2 statements, got: {:?}",
                    then_
                );
            }
            other => panic!("Expected If statement, got: {:?}", other),
        }
    }

    #[test]
    fn parse_if_followed_by_more_code() {
        // If statement followed by more statements at the same level
        let input = r#"if (!%&el) {
    %yield 0.5 -> $progress;
    return;
}
const x = 42;"#;
        let stmts = parse_emit_js(input).unwrap();
        assert!(stmts.len() >= 2, "Should have if + const, got: {:?}", stmts);
        assert!(
            matches!(&stmts[0], JsStmt::If { .. }),
            "First should be If, got: {:?}",
            stmts[0]
        );
        assert!(
            matches!(&stmts[1], JsStmt::Decl { name, .. } if name == "x"),
            "Second should be Decl, got: {:?}",
            stmts[1]
        );
    }

    #[test]
    fn parse_mouse_driver_null_guard() {
        // The exact pattern that was broken: mouse-driver null element guard
        let input = r#"if (!%&el) {
    %yield (() => {}) -> $destroy;
    return;
}
%&el.addEventListener('mousemove', handleMove);"#;
        let stmts = parse_emit_js(input).unwrap();
        assert!(
            stmts.len() >= 2,
            "Should have if + addEventListener, got {:?}",
            stmts
        );
        match &stmts[0] {
            JsStmt::If { then_, .. } => {
                assert!(
                    then_.len() >= 2,
                    "If body should have yield + return, got: {:?}",
                    then_
                );
            }
            other => panic!("Expected If statement, got: {:?}", other),
        }
    }

    #[test]
    fn parse_if_with_nested_braces() {
        // Nested braces inside the if body (e.g., object literals) shouldn't
        // cause premature closure of the if block
        let input = r#"if (condition) {
    const obj = { a: 1, b: 2 };
    %yield obj -> $state;
}"#;
        let stmts = parse_emit_js(input).unwrap();
        assert_eq!(
            stmts.len(),
            1,
            "Should be single if statement, got: {:?}",
            stmts
        );
        match &stmts[0] {
            JsStmt::If { then_, .. } => {
                assert!(
                    then_.len() >= 2,
                    "Body should have decl + yield, got: {:?}",
                    then_
                );
            }
            other => panic!("Expected If statement, got: {:?}", other),
        }
    }

    #[test]
    fn parse_fullscreen_quad_preserves_createbuffer() {
        // Regression: the 4 lines between ]); and %yield were being dropped
        let input = r#"const vertices = new Float32Array([
      -1, -1,
       1, -1,
      -1,  1,
       1,  1
    ]);

    const vbo = el._stGL.createBuffer();
    el._stGL.bindBuffer(el._stGL.ARRAY_BUFFER, vbo);
    el._stGL.bufferData(el._stGL.ARRAY_BUFFER, vertices, el._stGL.STATIC_DRAW);
    el._stGL.bindBuffer(el._stGL.ARRAY_BUFFER, null);

    %yield vbo -> $vbo;"#;

        let stmts = parse_emit_js(input).unwrap();
        assert!(
            stmts.len() >= 6,
            "Expected at least 6 statements (vertices, vbo, 3 bindBuffer/bufferData, yield), got {}. Stmts: {:?}",
            stmts.len(),
            stmts
        );
        let emitted = {
            use crate::emit::{EmitOptions, js as js_emit};
            let ctx = js_emit::EmitContext::new("el");
            let opts = EmitOptions::pretty();
            let resolved = js_emit::resolve_stmts(&stmts, &ctx, &opts).unwrap();
            js_emit::emit_stmts(&resolved, &opts).unwrap_or_default()
        };
        assert!(
            emitted.contains("createBuffer"),
            "Emitted code must contain createBuffer. Got:\n{}",
            emitted
        );
    }

    /// Regression: after an if-block, remaining text was re-injected as a single
    /// unsplit JsChunk, causing multi-statement code between `]);` and `%yield` to
    /// be parsed as one Raw block — dropping everything after the first `;` at depth 0.
    #[test]
    fn parse_fullscreen_quad_with_if_preserves_createbuffer() {
        let input = r#"
    if (!el._stGL) {
      %yield null -> $vbo;
      %yield (() => {}) -> $draw;
      return;
    }

    // Fullscreen quad vertices: two triangles covering -1 to 1
    const vertices = new Float32Array([
      -1, -1,    // Bottom-left
       1, -1,    // Bottom-right
      -1,  1,    // Top-left
      -1,  1,    // Top-left
       1, -1,    // Bottom-right
       1,  1     // Top-right
    ]);

    const vbo = el._stGL.createBuffer();
    el._stGL.bindBuffer(el._stGL.ARRAY_BUFFER, vbo);
    el._stGL.bufferData(el._stGL.ARRAY_BUFFER, vertices, el._stGL.STATIC_DRAW);
    el._stGL.bindBuffer(el._stGL.ARRAY_BUFFER, null);

    %yield vbo -> $vbo;

    // Draw function
    const draw = (positionLocation = 0) => {
      el._stGL.bindBuffer(el._stGL.ARRAY_BUFFER, vbo);
      el._stGL.enableVertexAttribArray(positionLocation);
      el._stGL.vertexAttribPointer(positionLocation, 2, el._stGL.FLOAT, false, 0, 0);
      el._stGL.drawArrays(el._stGL.TRIANGLES, 0, 6);
      el._stGL.bindBuffer(el._stGL.ARRAY_BUFFER, null);
    };
    %yield draw -> $draw;
    // Store on element for cross-scope access by child primitives
    el._stDrawQuad = draw;"#;

        let stmts = parse_emit_js(input).unwrap();
        let emitted = {
            use crate::emit::{EmitOptions, js as js_emit};
            let ctx = js_emit::EmitContext::new("el");
            let opts = EmitOptions::pretty();
            let resolved = js_emit::resolve_stmts(&stmts, &ctx, &opts).unwrap();
            js_emit::emit_stmts(&resolved, &opts).unwrap_or_default()
        };
        assert!(
            emitted.contains("createBuffer"),
            "Emitted code must contain createBuffer. Got:\n{}",
            emitted
        );
        assert!(
            emitted.contains("bindBuffer"),
            "Emitted code must contain bindBuffer. Got:\n{}",
            emitted
        );
        assert!(
            emitted.contains("bufferData"),
            "Emitted code must contain bufferData. Got:\n{}",
            emitted
        );
    }

    /// BUG: semicolon inside a string literal adjacent to a %param gets split into
    /// a separate statement. The tokenizer correctly tracks string context, but when
    /// a %param marker follows, the JsChunk boundary falls mid-string, and
    /// split_chunks_at_boundaries sees a bare ';' at depth 0.
    ///
    /// Repro: `const items = ['a'].join(';'); const x = %color;`
    /// Expected: join(';') stays intact as a single statement
    /// Actual: the ';' inside join's argument is treated as a statement boundary
    #[test]
    fn semicolon_inside_string_with_nearby_param_not_split() {
        let input = r#"const items = ['a', 'b'].join(';');
    const x = %color;
    console.log(items);"#;

        let stmts = parse_emit_js(input).unwrap();
        let emitted = {
            use crate::emit::{EmitOptions, js as js_emit};
            let ctx = js_emit::EmitContext::new("el").with_param("color", "\"red\"");
            let opts = EmitOptions::pretty();
            let resolved = js_emit::resolve_stmts(&stmts, &ctx, &opts).unwrap();
            js_emit::emit_stmts(&resolved, &opts).unwrap_or_default()
        };
        assert!(
            emitted.contains("join(';')") || emitted.contains("join(\";\""),
            "The semicolon inside the join() string literal must survive intact. Got:\n{}",
            emitted
        );
    }

    /// FUP-063: a `;` inside a `//` line comment or a `/* */` block comment is
    /// NOT a statement boundary. Before the fix, such a `;` split the chunk and
    /// the comment tail lost its `//` prefix, surfacing as raw JS (SyntaxError).
    #[test]
    fn semicolon_inside_comment_is_not_a_boundary() {
        // Line comment: the `;` after `a` lives in `// a; b`; the real boundary
        // is the `;` after `c` (index of the last char).
        let s = "// a; b\nc;";
        assert_eq!(
            find_semicolon_at_depth_zero(s),
            Some(s.len() - 1),
            "`;` inside a // comment must be skipped; the `;` after c is the boundary"
        );

        // Block comment: the `;` inside `/* a; */` is skipped; the `;` after `d`
        // is the boundary.
        let s = "/* a; */ d;";
        assert_eq!(
            find_semicolon_at_depth_zero(s),
            Some(s.len() - 1),
            "`;` inside a /* */ comment must be skipped"
        );

        // A comment with NO trailing top-level `;` yields None.
        assert_eq!(
            find_semicolon_at_depth_zero("// only a comment; nothing after"),
            None,
            "a `;` that only appears inside a // comment is no boundary at all"
        );

        // Unchanged baselines: a plain `;` and a string `;` still behave.
        assert_eq!(find_semicolon_at_depth_zero("x;"), Some(1));
        // The string `;` is skipped (existing behavior); the trailing `;` is the
        // boundary.
        let s = "'a;b'; ";
        assert_eq!(find_semicolon_at_depth_zero(s), Some(5));

        // A lone `/` (division) is NOT a comment start — splitting unaffected.
        let s = "a = b / c;";
        assert_eq!(find_semicolon_at_depth_zero(s), Some(s.len() - 1));
    }

    /// FUP-063 end-to-end: a `%emit js` line comment containing a `;` must keep
    /// its `//` prefix. Before the fix, the `;` inside the comment split the
    /// chunk and the tail (` no global...`) lost its `//`, surfacing as bare JS
    /// (a runtime SyntaxError). The guarantee is prefix-preservation.
    #[test]
    fn emit_js_comment_with_semicolon_round_trips() {
        let input = "const x = 1; // one bridge; no global\nconst y = 2;";
        let stmts = parse_emit_js(input).unwrap();
        let dump = format!("{stmts:?}");
        // The comment stays behind its `//` prefix as one contiguous unit.
        assert!(
            dump.contains("// one bridge; no global"),
            "comment must remain prefixed by //, got: {stmts:?}"
        );
        // The tail must NOT have been split into a bare-JS chunk (the old bug).
        assert!(
            !dump.contains("Raw(\" no global"),
            "comment tail must not become bare JS, got: {stmts:?}"
        );
        // `const x = 1;` still parses as a real declaration.
        assert!(
            stmts
                .iter()
                .any(|s| matches!(s, JsStmt::Decl { name, .. } if name == "x")),
            "const x decl must survive, got: {stmts:?}"
        );
    }
}
