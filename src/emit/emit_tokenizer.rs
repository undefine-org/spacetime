//! Tokenize emit content into JS chunks and Spacetime placeholders.
//!
//! This module splits `%emit js { ... }` content into segments that can be
//! parsed by a standard JavaScript parser while preserving placeholder positions.
//!
//! Markers recognized:
//! - `%param` - primitive parameter reference
//! - `%&element` - element reference
//! - `$signal` - signal read
//! - `%yield expr -> $signal` - signal write
//! - `%cleanup { ... }` - cleanup block

/// Segment of emit content
#[derive(Debug, Clone, PartialEq)]
pub enum EmitSegment {
    /// Raw JavaScript code
    JsChunk(String),
    /// `%name` - parameter reference
    /// `in_string` tracks if this was inside a string literal (Some(quote_char))
    Param {
        name: String,
        field: Option<String>,
        in_string: Option<char>,
    },
    /// `%&name` - element reference
    Element { name: String },
    /// `$name` - signal reference
    Signal { name: String },
    /// `%yield expr -> $name` or `%yield expr -> $%param`
    Yield {
        expr: String,
        /// Signal name (for `$name`)
        signal: Option<String>,
        /// Param name to resolve for signal (for `$%param`)
        signal_param: Option<String>,
    },
    /// `%cleanup { ... }` - cleanup block with its content
    Cleanup { content: String },
    /// `%for $var in %array { body }` - compile-time loop expansion (placeholder for future)
    ForLoop {
        /// Loop variable name (without the $)
        var: String,
        /// Array parameter name (without the %)
        array: String,
        /// Body content (between the braces)
        body: String,
    },
    /// `%$var` or `%$var.field` - loop variable reference
    /// `in_string` tracks if this was inside a string literal (Some(quote_char))
    VarRef {
        /// Variable name (without the $)
        var: String,
        /// Optional field access
        field: Option<String>,
        /// Whether found inside a string literal
        in_string: Option<char>,
    },
}

/// Add string context to an EmitSegment that supports it (Param, VarRef).
/// This marks the segment as having been parsed from within a string literal.
fn add_string_context(seg: EmitSegment, string_char: char) -> EmitSegment {
    match seg {
        EmitSegment::Param { name, field, .. } => EmitSegment::Param {
            name,
            field,
            in_string: Some(string_char),
        },
        EmitSegment::VarRef { var, field, .. } => EmitSegment::VarRef {
            var,
            field,
            in_string: Some(string_char),
        },
        other => other,
    }
}

/// Reserved keywords that start with % but are not param placeholders
const RESERVED_PERCENT_KEYWORDS: &[&str] = &[
    "yield", "if", "else", "elif", "for", "emit", "cleanup", "exports",
];

/// Tokenize emit content into segments
pub fn tokenize_emit(content: &str) -> Vec<EmitSegment> {
    let mut segments = Vec::new();
    let chars: Vec<char> = content.chars().collect();
    let mut pos = 0;
    let mut in_string = false;
    let mut string_char = '"';
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    // Track template literal interpolation depth
    let mut template_depth = 0; // How many `${}` levels deep we are
    let mut brace_stack: Vec<usize> = Vec::new(); // Stack of brace depths for each template level

    // Track regex literal state
    let mut in_regex = false;
    let mut in_regex_char_class = false; // Inside [...] in a regex

    while pos < chars.len() {
        let c = chars[pos];

        // Handle end of line comment
        if in_line_comment {
            accumulate_js_char(&mut segments, c);
            if c == '\n' {
                in_line_comment = false;
            }
            pos += 1;
            continue;
        }

        // Handle block comment
        if in_block_comment {
            accumulate_js_char(&mut segments, c);
            if c == '*' && pos + 1 < chars.len() && chars[pos + 1] == '/' {
                accumulate_js_char(&mut segments, '/');
                pos += 2;
                in_block_comment = false;
            } else {
                pos += 1;
            }
            continue;
        }

        // Handle regex literals - skip string delimiters inside regexes
        if in_regex {
            if c == '\\' && pos + 1 < chars.len() {
                // Escaped character in regex - skip both chars
                accumulate_js_char(&mut segments, c);
                accumulate_js_char(&mut segments, chars[pos + 1]);
                pos += 2;
                continue;
            }
            if c == '[' && !in_regex_char_class {
                in_regex_char_class = true;
                accumulate_js_char(&mut segments, c);
                pos += 1;
                continue;
            }
            if c == ']' && in_regex_char_class {
                in_regex_char_class = false;
                accumulate_js_char(&mut segments, c);
                pos += 1;
                continue;
            }
            if c == '/' && !in_regex_char_class {
                // End of regex - consume the closing / and any flags
                in_regex = false;
                accumulate_js_char(&mut segments, c);
                pos += 1;
                // Consume regex flags (g, i, m, s, u, y)
                while pos < chars.len() && matches!(chars[pos], 'g' | 'i' | 'm' | 's' | 'u' | 'y') {
                    accumulate_js_char(&mut segments, chars[pos]);
                    pos += 1;
                }
                continue;
            }
            // Inside regex - just accumulate, don't treat quotes as string delimiters
            accumulate_js_char(&mut segments, c);
            pos += 1;
            continue;
        }

        // Handle template literal interpolation: track ${} inside backtick strings
        if in_string
            && string_char == '`'
            && c == '$'
            && pos + 1 < chars.len()
            && chars[pos + 1] == '{'
        {
            // Entering template interpolation - this is NOT string content
            template_depth += 1;
            brace_stack.push(1); // Start with brace depth 1
            accumulate_js_char(&mut segments, c);
            accumulate_js_char(&mut segments, '{');
            pos += 2;
            in_string = false; // Inside ${}, we're NOT in a string
            continue;
        }

        // Track braces when inside template interpolation
        if template_depth > 0 {
            if c == '{' {
                if let Some(depth) = brace_stack.last_mut() {
                    *depth += 1;
                }
            } else if c == '}'
                && let Some(depth) = brace_stack.last_mut()
            {
                *depth -= 1;
                if *depth == 0 {
                    // Exiting this template interpolation level
                    brace_stack.pop();
                    template_depth -= 1;
                    accumulate_js_char(&mut segments, c);
                    pos += 1;
                    // Back inside the template literal string
                    in_string = true;
                    string_char = '`';
                    continue;
                }
            }
        }

        // Check for comment start (only when not in string)
        if !in_string && c == '/' && pos + 1 < chars.len() {
            if chars[pos + 1] == '/' {
                // Start of line comment
                in_line_comment = true;
                accumulate_js_char(&mut segments, c);
                pos += 1;
                continue;
            } else if chars[pos + 1] == '*' {
                // Start of block comment
                in_block_comment = true;
                accumulate_js_char(&mut segments, c);
                accumulate_js_char(&mut segments, '*');
                pos += 2;
                continue;
            }
        }

        // Check for regex literal start
        // A / starts a regex after: ( [ { , = ! && || ? : ; newline, or at start
        // We use a simplified heuristic: if we see / and we're not in a string,
        // look at the previous non-whitespace character
        if c == '/' && !in_string {
            // Look back for previous non-whitespace char
            let prev_char = find_prev_non_whitespace(&chars, pos);
            // These chars indicate the / starts a regex (not division)
            let regex_starters = [
                '(', '[', '{', ',', '=', '!', '&', '|', '?', ':', ';', '\n', '\r',
            ];
            // Also check for keywords like return, if, while, etc. by checking if prev is a letter
            // For simplicity, we'll also start a regex if prev is a non-alphanumeric char
            let starts_regex = match prev_char {
                Some(pc) => {
                    regex_starters.contains(&pc) ||
                           // After operators
                           pc == '+' || pc == '-' || pc == '*' || pc == '<' || pc == '>' ||
                           // At very start
                           pos == 0
                }
                None => true, // Start of content
            };

            if starts_regex {
                in_regex = true;
                in_regex_char_class = false;
                accumulate_js_char(&mut segments, c);
                pos += 1;
                continue;
            }
        }

        // Track string state (handle string delimiters)
        if (c == '"' || c == '\'' || c == '`') && !is_escaped(&chars, pos) {
            if in_string && c == string_char {
                in_string = false;
            } else if !in_string {
                in_string = true;
                string_char = c;
            }
            accumulate_js_char(&mut segments, c);
            pos += 1;
            continue;
        }

        // Handle escaped percent (%%) - produces literal %
        // This works both inside and outside strings
        if chars[pos] == '%' && pos + 1 < chars.len() && chars[pos + 1] == '%' {
            accumulate_js_char(&mut segments, '%');
            pos += 2;
            continue;
        }

        // Handle escaped dollar ($$) - produces a LITERAL `$` (no Signal lowering).
        // Sibling of the `%%` rule above: it is the ONLY way an emit body can write
        // a bare `$ident` that survives to the output verbatim instead of being
        // tokenized as `EmitSegment::Signal` -> `ST.get(el, "ident")`. Needed by the
        // @try macro, whose `$error` catch-binding is a plain JS test-local (a global
        // identifier named `$error`, which @assert reads literally), NOT a page signal.
        // Works both inside and outside strings.
        if chars[pos] == '$' && pos + 1 < chars.len() && chars[pos + 1] == '$' {
            accumulate_js_char(&mut segments, '$');
            pos += 2;
            continue;
        }

        // If inside a string, parse %param and %$var placeholders (for interpolation)
        // but not complex constructs like %for, %cleanup, %yield
        // Also skip single-char params that look like format specifiers (%c, %d, %s, etc.)
        if in_string && chars[pos] == '%' {
            // Try %$var.field (VarRef) first
            if let Some((seg, end)) = try_parse_varref(&chars, pos) {
                segments.push(add_string_context(seg, string_char));
                pos = end;
                continue;
            }
            // Try %param - but skip single-char format specifiers
            if let Some((seg, end)) = try_parse_param(&chars, pos) {
                // Skip single-char params that look like format specifiers (%c, %d, %s, etc.)
                // These are commonly used for CSS console styling or printf-style formatting
                let is_format_specifier = match &seg {
                    EmitSegment::Param { name, .. } => {
                        name.len() == 1
                            && matches!(
                                name.chars().next(),
                                Some(
                                    'c' | 'd' | 's' | 'f' | 'i' | 'o' | 'x' | 'e' | 'g' | 'p' | 'n'
                                )
                            )
                    }
                    _ => false,
                };
                if !is_format_specifier {
                    segments.push(add_string_context(seg, string_char));
                    pos = end;
                    continue;
                }
                // Fall through to accumulate as literal text for format specifiers
            }
        }

        // If inside a string and we didn't match a placeholder, accumulate
        if in_string {
            accumulate_js_char(&mut segments, c);
            pos += 1;
            continue;
        }

        if chars[pos] == '%' {
            // Check for %for, %cleanup, %yield, %&element, or %param
            if let Some((seg, end)) = try_parse_for_loop(&chars, pos) {
                segments.push(seg);
                pos = end;
            } else if let Some((seg, end)) = try_parse_cleanup(&chars, pos) {
                segments.push(seg);
                pos = end;
            } else if let Some((seg, end)) = try_parse_yield(&chars, pos) {
                segments.push(seg);
                pos = end;
            } else if let Some((seg, end)) = try_parse_element(&chars, pos) {
                segments.push(seg);
                pos = end;
            } else if let Some((seg, end)) = try_parse_varref(&chars, pos) {
                segments.push(seg);
                pos = end;
            } else if let Some((seg, end)) = try_parse_param(&chars, pos) {
                segments.push(seg);
                pos = end;
            } else {
                accumulate_js_char(&mut segments, chars[pos]);
                pos += 1;
            }
        } else if chars[pos] == '$' {
            if let Some((seg, end)) = try_parse_signal(&chars, pos) {
                segments.push(seg);
                pos = end;
            } else {
                accumulate_js_char(&mut segments, chars[pos]);
                pos += 1;
            }
        } else {
            accumulate_js_char(&mut segments, chars[pos]);
            pos += 1;
        }
    }

    merge_js_chunks(&mut segments);
    segments
}

fn try_parse_for_loop(chars: &[char], pos: usize) -> Option<(EmitSegment, usize)> {
    let prefix = "%for";
    if !starts_with_str(chars, pos, prefix) {
        return None;
    }
    let after_for = pos + prefix.len();
    if after_for >= chars.len() {
        return None;
    }
    // Skip whitespace after %for
    let mut cursor = after_for;
    while cursor < chars.len() && chars[cursor].is_whitespace() {
        cursor += 1;
    }
    // Expect $var
    if cursor >= chars.len() || chars[cursor] != '$' {
        return None;
    }
    cursor += 1;
    let (var, after_var) = parse_identifier(chars, cursor)?;
    cursor = after_var;
    // Skip whitespace
    while cursor < chars.len() && chars[cursor].is_whitespace() {
        cursor += 1;
    }
    // Expect "in"
    if !starts_with_str(chars, cursor, "in") {
        return None;
    }
    cursor += 2;
    // Make sure "in" is not part of a longer identifier
    if cursor < chars.len() && is_ident_continue(chars[cursor]) {
        return None;
    }
    // Skip whitespace
    while cursor < chars.len() && chars[cursor].is_whitespace() {
        cursor += 1;
    }
    // Expect %array
    if cursor >= chars.len() || chars[cursor] != '%' {
        return None;
    }
    cursor += 1;
    let (array, after_array) = parse_identifier(chars, cursor)?;
    cursor = after_array;
    // Skip whitespace
    while cursor < chars.len() && chars[cursor].is_whitespace() {
        cursor += 1;
    }
    // Expect opening brace
    if cursor >= chars.len() || chars[cursor] != '{' {
        return None;
    }
    let brace_start = cursor;
    let brace_end = find_matching_brace(chars, brace_start)?;
    let body: String = chars[brace_start + 1..brace_end].iter().collect();
    Some((
        EmitSegment::ForLoop {
            var,
            array,
            body: body.trim().to_string(),
        },
        brace_end + 1,
    ))
}

fn try_parse_cleanup(chars: &[char], pos: usize) -> Option<(EmitSegment, usize)> {
    let prefix = "%cleanup";
    if !starts_with_str(chars, pos, prefix) {
        return None;
    }
    let after_cleanup = pos + prefix.len();
    if after_cleanup >= chars.len() {
        return None;
    }
    let mut cursor = after_cleanup;
    while cursor < chars.len() && chars[cursor].is_whitespace() {
        cursor += 1;
    }
    if cursor >= chars.len() || chars[cursor] != '{' {
        return None;
    }
    let brace_start = cursor;
    let brace_end = find_matching_brace(chars, brace_start)?;
    let content: String = chars[brace_start + 1..brace_end].iter().collect();
    Some((
        EmitSegment::Cleanup {
            content: content.trim().to_string(),
        },
        brace_end + 1,
    ))
}

fn find_matching_brace(chars: &[char], open_pos: usize) -> Option<usize> {
    let mut depth = 1;
    let mut pos = open_pos + 1;
    let mut in_string = false;
    let mut string_char = '"';
    while pos < chars.len() && depth > 0 {
        let c = chars[pos];
        if (c == '"' || c == '\'' || c == '`') && (pos == 0 || chars[pos - 1] != '\\') {
            if in_string && c == string_char {
                in_string = false;
            } else if !in_string {
                in_string = true;
                string_char = c;
            }
        }
        if !in_string {
            if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth -= 1;
            }
        }
        pos += 1;
    }
    if depth == 0 { Some(pos - 1) } else { None }
}

fn try_parse_yield(chars: &[char], pos: usize) -> Option<(EmitSegment, usize)> {
    let prefix = "%yield";
    if !starts_with_str(chars, pos, prefix) {
        return None;
    }
    let after_yield = pos + prefix.len();
    if after_yield >= chars.len() {
        return None;
    }
    let mut cursor = after_yield;
    while cursor < chars.len() && chars[cursor].is_whitespace() {
        cursor += 1;
    }
    let arrow_start = find_arrow(chars, cursor)?;
    let expr: String = chars[cursor..arrow_start].iter().collect();
    let expr = expr.trim().to_string();
    let after_arrow = arrow_start + 2;
    let mut signal_start = after_arrow;
    while signal_start < chars.len() && chars[signal_start].is_whitespace() {
        signal_start += 1;
    }
    if signal_start >= chars.len() || chars[signal_start] != '$' {
        return None;
    }

    // Check for $%param pattern (param-based signal name)
    if signal_start + 1 < chars.len() && chars[signal_start + 1] == '%' {
        // Parse param name after $%
        let (param, end) = parse_identifier(chars, signal_start + 2)?;
        return Some((
            EmitSegment::Yield {
                expr,
                signal: None,
                signal_param: Some(param),
            },
            end,
        ));
    }

    // Regular $signal pattern
    let (signal, end) = parse_identifier(chars, signal_start + 1)?;
    Some((
        EmitSegment::Yield {
            expr,
            signal: Some(signal),
            signal_param: None,
        },
        end,
    ))
}

fn try_parse_element(chars: &[char], pos: usize) -> Option<(EmitSegment, usize)> {
    if pos + 1 >= chars.len() || chars[pos] != '%' || chars[pos + 1] != '&' {
        return None;
    }
    let (name, end) = parse_identifier(chars, pos + 2)?;
    Some((EmitSegment::Element { name }, end))
}

/// Parse `%$var` or `%$var.field` - loop variable reference
fn try_parse_varref(chars: &[char], pos: usize) -> Option<(EmitSegment, usize)> {
    // Must start with %$
    if pos + 1 >= chars.len() || chars[pos] != '%' || chars[pos + 1] != '$' {
        return None;
    }
    // Parse variable name
    let (var, after_var) = parse_identifier(chars, pos + 2)?;
    // Check for optional .field
    if after_var < chars.len() && chars[after_var] == '.' {
        // Parse field name
        if let Some((field, end)) = parse_identifier(chars, after_var + 1) {
            return Some((
                EmitSegment::VarRef {
                    var,
                    field: Some(field),
                    in_string: None,
                },
                end,
            ));
        }
    }
    Some((
        EmitSegment::VarRef {
            var,
            field: None,
            in_string: None,
        },
        after_var,
    ))
}

fn try_parse_param(chars: &[char], pos: usize) -> Option<(EmitSegment, usize)> {
    if chars[pos] != '%' {
        return None;
    }
    if pos + 1 >= chars.len() {
        return None;
    }
    let next = chars[pos + 1];
    if !is_ident_start(next) {
        return None;
    }
    // A `%param` name stops at `-`: `%name-width` is `%name` + literal `-width`,
    // mirroring the CSS tokenizer (css_parser::is_ident_char). Hyphens are not
    // valid in JS identifiers, so a hyphenated `%param` can never be a real
    // parameter — and CSS-var tails like `--st-%name-width` in JS emit bodies
    // depend on the param resolving before the `-width` suffix.
    let mut end = pos + 1;
    while end < chars.len() && is_param_ident_continue(chars[end]) {
        end += 1;
    }
    let name: String = chars[pos + 1..end].iter().collect();
    if RESERVED_PERCENT_KEYWORDS.contains(&name.as_str()) {
        return None;
    }
    // Check for optional .field (mirrors try_parse_varref pattern)
    if end < chars.len()
        && chars[end] == '.'
        && let Some((field, field_end)) = parse_identifier(chars, end + 1)
    {
        return Some((
            EmitSegment::Param {
                name,
                field: Some(field),
                in_string: None,
            },
            field_end,
        ));
    }
    Some((
        EmitSegment::Param {
            name,
            field: None,
            in_string: None,
        },
        end,
    ))
}

/// Valid in a `%param` name: alphanumeric or `_`. A hyphen TERMINATES the param
/// so `--st-%name-width` resolves `%name` then `-width` (CSS custom-property
/// vars in emit bodies). Matches css_parser::is_ident_char.
fn is_param_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn try_parse_signal(chars: &[char], pos: usize) -> Option<(EmitSegment, usize)> {
    if chars[pos] != '$' {
        return None;
    }
    if pos + 1 >= chars.len() {
        return None;
    }
    let next = chars[pos + 1];
    if !is_ident_start(next) {
        return None;
    }
    let (name, end) = parse_identifier(chars, pos + 1)?;
    Some((EmitSegment::Signal { name }, end))
}

fn parse_identifier(chars: &[char], pos: usize) -> Option<(String, usize)> {
    if pos >= chars.len() || !is_ident_start(chars[pos]) {
        return None;
    }
    let mut end = pos;
    while end < chars.len() && is_ident_continue(chars[end]) {
        end += 1;
    }
    let name: String = chars[pos..end].iter().collect();
    Some((name, end))
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}
fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

/// Find the previous non-whitespace character before the given position
fn find_prev_non_whitespace(chars: &[char], pos: usize) -> Option<char> {
    if pos == 0 {
        return None;
    }
    let mut check_pos = pos - 1;
    loop {
        if !chars[check_pos].is_whitespace() {
            return Some(chars[check_pos]);
        }
        if check_pos == 0 {
            return None;
        }
        check_pos -= 1;
    }
}

/// Check if the character at pos is escaped by a backslash
fn is_escaped(chars: &[char], pos: usize) -> bool {
    if pos == 0 {
        return false;
    }
    let mut backslash_count = 0;
    let mut check_pos = pos - 1;
    loop {
        if chars[check_pos] == '\\' {
            backslash_count += 1;
        } else {
            break;
        }
        if check_pos == 0 {
            break;
        }
        check_pos -= 1;
    }
    backslash_count % 2 == 1
}

fn starts_with_str(chars: &[char], pos: usize, s: &str) -> bool {
    let s_chars: Vec<char> = s.chars().collect();
    if pos + s_chars.len() > chars.len() {
        return false;
    }
    for (i, c) in s_chars.iter().enumerate() {
        if chars[pos + i] != *c {
            return false;
        }
    }
    true
}

fn find_arrow(chars: &[char], start: usize) -> Option<usize> {
    let mut pos = start;
    while pos + 1 < chars.len() {
        if chars[pos] == '-' && chars[pos + 1] == '>' {
            return Some(pos);
        }
        pos += 1;
    }
    None
}

fn accumulate_js_char(segments: &mut Vec<EmitSegment>, c: char) {
    if let Some(EmitSegment::JsChunk(s)) = segments.last_mut() {
        s.push(c);
    } else {
        segments.push(EmitSegment::JsChunk(c.to_string()));
    }
}

fn merge_js_chunks(segments: &mut Vec<EmitSegment>) {
    let mut merged = Vec::with_capacity(segments.len());
    let mut current_chunk = String::new();
    for seg in segments.drain(..) {
        match seg {
            EmitSegment::JsChunk(s) => {
                current_chunk.push_str(&s);
            }
            other => {
                if !current_chunk.is_empty() {
                    merged.push(EmitSegment::JsChunk(std::mem::take(&mut current_chunk)));
                }
                merged.push(other);
            }
        }
    }
    if !current_chunk.is_empty() {
        merged.push(EmitSegment::JsChunk(current_chunk));
    }
    *segments = merged;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_cleanup_simple() {
        let segments = tokenize_emit("%cleanup { el.removeEventListener('click', handler); }");
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            EmitSegment::Cleanup { content } => {
                assert_eq!(content, "el.removeEventListener('click', handler);");
            }
            _ => panic!("Expected Cleanup segment, got {:?}", segments[0]),
        }
    }

    #[test]
    fn tokenize_cleanup_with_nested_braces() {
        let input = r#"%cleanup {
            if (observer) {
                observer.disconnect();
            }
        }"#;
        let segments = tokenize_emit(input);
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            EmitSegment::Cleanup { content } => {
                assert!(content.contains("if (observer)"));
                assert!(content.contains("observer.disconnect()"));
            }
            _ => panic!("Expected Cleanup segment"),
        }
    }

    #[test]
    fn tokenize_cleanup_with_string_containing_braces() {
        let input = r#"%cleanup { console.log("{ not a brace }"); }"#;
        let segments = tokenize_emit(input);
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            EmitSegment::Cleanup { content } => {
                assert!(content.contains(r#"console.log("{ not a brace }")"#));
            }
            _ => panic!("Expected Cleanup segment"),
        }
    }

    #[test]
    fn tokenize_cleanup_with_surrounding_code() {
        let input = r#"el.addEventListener('click', handler);
%cleanup {
    el.removeEventListener('click', handler);
}
console.log('done');"#;
        let segments = tokenize_emit(input);
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], EmitSegment::JsChunk(s) if s.contains("addEventListener")));
        assert!(
            matches!(&segments[1], EmitSegment::Cleanup { content } if content.contains("removeEventListener"))
        );
        assert!(matches!(&segments[2], EmitSegment::JsChunk(s) if s.contains("console.log")));
    }

    #[test]
    fn tokenize_cleanup_with_placeholders() {
        let input = r#"%cleanup {
            %&el.removeEventListener('click', handler);
            cancelAnimationFrame($frameId);
        }"#;
        let segments = tokenize_emit(input);
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            EmitSegment::Cleanup { content } => {
                assert!(content.contains("%&el"));
                assert!(content.contains("$frameId"));
            }
            _ => panic!("Expected Cleanup segment"),
        }
    }

    #[test]
    fn tokenize_multiple_cleanup_blocks() {
        let input = r#"setup1();
%cleanup { cleanup1(); }
setup2();
%cleanup { cleanup2(); }"#;
        let segments = tokenize_emit(input);
        assert_eq!(segments.len(), 4);
        assert!(matches!(&segments[0], EmitSegment::JsChunk(s) if s.contains("setup1")));
        assert!(
            matches!(&segments[1], EmitSegment::Cleanup { content } if content.contains("cleanup1"))
        );
        assert!(matches!(&segments[2], EmitSegment::JsChunk(s) if s.contains("setup2")));
        assert!(
            matches!(&segments[3], EmitSegment::Cleanup { content } if content.contains("cleanup2"))
        );
    }

    #[test]
    fn tokenize_cleanup_malformed_no_brace() {
        let segments = tokenize_emit("%cleanup el.remove();");
        assert_eq!(segments.len(), 1);
        assert!(matches!(&segments[0], EmitSegment::JsChunk(_)));
    }

    #[test]
    fn tokenize_cleanup_empty_body() {
        let segments = tokenize_emit("%cleanup { }");
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            EmitSegment::Cleanup { content } => {
                assert!(content.is_empty());
            }
            _ => panic!("Expected Cleanup segment"),
        }
    }

    #[test]
    fn tokenize_param() {
        let segments = tokenize_emit("const fps = %fps;");
        assert_eq!(
            segments,
            vec![
                EmitSegment::JsChunk("const fps = ".into()),
                EmitSegment::Param {
                    name: "fps".into(),
                    field: None,
                    in_string: None
                },
                EmitSegment::JsChunk(";".into()),
            ]
        );
    }

    #[test]
    fn tokenize_element() {
        let segments = tokenize_emit("const el = %&container;");
        assert_eq!(
            segments,
            vec![
                EmitSegment::JsChunk("const el = ".into()),
                EmitSegment::Element {
                    name: "container".into()
                },
                EmitSegment::JsChunk(";".into()),
            ]
        );
    }

    #[test]
    fn tokenize_signal() {
        let segments = tokenize_emit("if ($running) { stop(); }");
        assert_eq!(
            segments,
            vec![
                EmitSegment::JsChunk("if (".into()),
                EmitSegment::Signal {
                    name: "running".into()
                },
                EmitSegment::JsChunk(") { stop(); }".into()),
            ]
        );
    }

    #[test]
    fn tokenize_escaped_dollar_is_literal_not_signal() {
        // BUG-112: `$$error` must collapse to a LITERAL `$error` JsChunk (a plain JS
        // identifier), NOT an `EmitSegment::Signal` that would lower to
        // `ST.get(el, "error")`. This is the sibling of the `%%`->`%` escape and the
        // only way an emit body (e.g. @try's catch binding) emits a bare `$ident`
        // that survives verbatim. Single `$` still tokenizes as a Signal.
        let segments = tokenize_emit("$$error = __e;");
        assert_eq!(
            segments,
            vec![EmitSegment::JsChunk("$error = __e;".into()),]
        );
        // And it must NOT produce a Signal segment anywhere.
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Signal { .. })),
            "$$ escape must not yield a Signal: {segments:?}"
        );
    }

    #[test]
    fn tokenize_yield() {
        let segments = tokenize_emit("%yield now -> $t;");
        assert_eq!(
            segments,
            vec![
                EmitSegment::Yield {
                    expr: "now".into(),
                    signal: Some("t".into()),
                    signal_param: None
                },
                EmitSegment::JsChunk(";".into()),
            ]
        );
    }

    #[test]
    fn tokenize_yield_with_param_signal() {
        // Test for %yield expr -> $%param pattern
        let segments = tokenize_emit("%yield progress -> $%name;");
        assert_eq!(
            segments,
            vec![
                EmitSegment::Yield {
                    expr: "progress".into(),
                    signal: None,
                    signal_param: Some("name".into())
                },
                EmitSegment::JsChunk(";".into()),
            ]
        );
    }

    // =========================================================================
    // ForLoop Tokenizer Tests
    // =========================================================================

    #[test]
    fn tokenize_for_loop_simple() {
        let segments = tokenize_emit("%for $item in %items { console.log($item); }");
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            EmitSegment::ForLoop { var, array, body } => {
                assert_eq!(var, "item");
                assert_eq!(array, "items");
                assert_eq!(body, "console.log($item);");
            }
            _ => panic!("Expected ForLoop segment, got {:?}", segments[0]),
        }
    }

    #[test]
    fn tokenize_for_loop_with_nested_braces() {
        let input = r#"%for $item in %items {
            if (condition) {
                doSomething();
            }
        }"#;
        let segments = tokenize_emit(input);
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            EmitSegment::ForLoop { var, array, body } => {
                assert_eq!(var, "item");
                assert_eq!(array, "items");
                assert!(body.contains("if (condition)"));
                assert!(body.contains("doSomething()"));
            }
            _ => panic!("Expected ForLoop segment"),
        }
    }

    #[test]
    fn tokenize_for_loop_with_string_braces() {
        let input = r#"%for $item in %items { console.log("{ not a brace }"); }"#;
        let segments = tokenize_emit(input);
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            EmitSegment::ForLoop { body, .. } => {
                assert!(body.contains(r#"console.log("{ not a brace }")"#));
            }
            _ => panic!("Expected ForLoop segment"),
        }
    }

    #[test]
    fn tokenize_for_loop_with_surrounding_code() {
        let input = r#"setup();
%for $item in %items {
    process($item);
}
cleanup();"#;
        let segments = tokenize_emit(input);
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], EmitSegment::JsChunk(s) if s.contains("setup")));
        assert!(
            matches!(&segments[1], EmitSegment::ForLoop { var, array, .. } if var == "item" && array == "items")
        );
        assert!(matches!(&segments[2], EmitSegment::JsChunk(s) if s.contains("cleanup")));
    }

    #[test]
    fn tokenize_for_loop_with_placeholders() {
        let input = r#"%for $item in %items {
            %&el.appendChild($item);
            %yield $item -> $current;
        }"#;
        let segments = tokenize_emit(input);
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            EmitSegment::ForLoop { body, .. } => {
                assert!(body.contains("%&el"));
                assert!(body.contains("$item"));
                assert!(body.contains("%yield"));
            }
            _ => panic!("Expected ForLoop segment"),
        }
    }

    #[test]
    fn tokenize_for_loop_malformed_no_dollar() {
        // Missing $ before variable name - should not parse as ForLoop
        // The tokenizer continues and parses what it can (e.g., %items as Param)
        let segments = tokenize_emit("%for item in %items { }");
        // Should NOT be a ForLoop - instead it falls through to regular tokenization
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, EmitSegment::ForLoop { .. }))
        );
        // %items will be tokenized as a Param
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "items"))
        );
    }

    #[test]
    fn tokenize_for_loop_malformed_no_percent() {
        // Missing % before array name - should not parse as ForLoop
        let segments = tokenize_emit("%for $item in items { }");
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, EmitSegment::ForLoop { .. }))
        );
        // $item will be tokenized as a Signal
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Signal { name } if name == "item"))
        );
    }

    #[test]
    fn tokenize_for_loop_malformed_no_brace() {
        // Missing opening brace - should not parse as ForLoop
        let segments = tokenize_emit("%for $item in %items console.log($item);");
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, EmitSegment::ForLoop { .. }))
        );
        // $item and %items will be tokenized as Signal and Param
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "items"))
        );
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Signal { name } if name == "item"))
        );
    }

    #[test]
    fn tokenize_for_loop_empty_body() {
        let segments = tokenize_emit("%for $item in %items { }");
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            EmitSegment::ForLoop { var, array, body } => {
                assert_eq!(var, "item");
                assert_eq!(array, "items");
                assert!(body.is_empty());
            }
            _ => panic!("Expected ForLoop segment"),
        }
    }

    #[test]
    fn tokenize_for_loop_with_property_access() {
        // Test case where body uses %$var.property syntax for property access
        let input = r#"%for $item in %items {
            console.log(%$item.name, %$item.value);
        }"#;
        let segments = tokenize_emit(input);
        assert_eq!(segments.len(), 1);
        match &segments[0] {
            EmitSegment::ForLoop { body, .. } => {
                assert!(body.contains("%$item.name"));
                assert!(body.contains("%$item.value"));
            }
            _ => panic!("Expected ForLoop segment"),
        }
    }

    // =========================================================================
    // String-Awareness Tests (placeholders inside strings should NOT be parsed)
    // =========================================================================

    #[test]
    fn tokenize_percent_c_in_double_quote_string() {
        // %c is CSS console styling, not a Spacetime param
        let segments = tokenize_emit(r#"console.log("%c styled", "color: red");"#);
        assert_eq!(segments.len(), 1);
        assert!(matches!(&segments[0], EmitSegment::JsChunk(s) if s.contains("%c")));
        // Should NOT have a Param { name: "c" }
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "c"))
        );
    }

    #[test]
    fn tokenize_percent_c_in_template_literal() {
        // Template literal backticks should also be handled
        let segments = tokenize_emit(r#"console.error(`%c[Error]%c`, 'color: red', '');"#);
        assert_eq!(segments.len(), 1);
        assert!(matches!(&segments[0], EmitSegment::JsChunk(s) if s.contains("%c")));
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { .. }))
        );
    }

    #[test]
    fn tokenize_dollar_sign_in_string() {
        // $foo in a string should not become a Signal
        let segments = tokenize_emit(r#"console.log("Price: $100");"#);
        assert_eq!(segments.len(), 1);
        // Should NOT have any Signal segments
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Signal { .. }))
        );
    }

    #[test]
    fn tokenize_param_outside_string_still_works() {
        // Params outside strings should still be parsed
        let segments = tokenize_emit(r#"const x = %param; console.log("hello");"#);
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], EmitSegment::JsChunk(s) if s == "const x = "));
        assert!(matches!(&segments[1], EmitSegment::Param { name, .. } if name == "param"));
        assert!(matches!(&segments[2], EmitSegment::JsChunk(s) if s.contains("console.log")));
    }

    #[test]
    fn tokenize_escaped_quote_in_string() {
        // Escaped quotes should not end the string
        let segments = tokenize_emit(r#"console.log("say \"hi\" with %c");"#);
        assert_eq!(segments.len(), 1);
        // %c is inside the string, should not be parsed as param
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "c"))
        );
    }

    #[test]
    fn tokenize_single_quote_string() {
        let segments = tokenize_emit(r#"console.log('%c styled', 'color: blue');"#);
        assert_eq!(segments.len(), 1);
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { .. }))
        );
    }

    #[test]
    fn tokenize_mixed_strings_and_params() {
        // Mix of strings (with %c) and real params
        let input = r#"console.error("%c Error", "color: red"); const val = %name;"#;
        let segments = tokenize_emit(input);
        // Should have: JsChunk, Param(name), JsChunk
        assert_eq!(segments.len(), 3);
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "name"))
        );
        // Should NOT have Param { name: "c" }
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "c"))
        );
    }

    #[test]
    fn tokenize_varref_simple() {
        // Test standalone VarRef (used for Data params like $gl)
        let segments = tokenize_emit("const gl = %$gl;");
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], EmitSegment::JsChunk(s) if s == "const gl = "));
        assert!(
            matches!(&segments[1], EmitSegment::VarRef { var, field, .. } if var == "gl" && field.is_none())
        );
        assert!(matches!(&segments[2], EmitSegment::JsChunk(s) if s == ";"));
    }

    #[test]
    fn tokenize_varref_with_field() {
        // Test VarRef with field access
        let segments = tokenize_emit("const ctx = %$gl.context;");
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], EmitSegment::JsChunk(s) if s == "const ctx = "));
        assert!(
            matches!(&segments[1], EmitSegment::VarRef { var, field, .. } if var == "gl" && *field == Some("context".to_string()))
        );
        assert!(matches!(&segments[2], EmitSegment::JsChunk(s) if s == ";"));
    }

    // =========================================================================
    // Comment Handling Tests
    // =========================================================================

    #[test]
    fn tokenize_apostrophe_in_line_comment() {
        // Apostrophes in comments should not affect string tracking
        let segments = tokenize_emit(
            r#"// we're testing
const x = %param;"#,
        );
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], EmitSegment::JsChunk(s) if s.contains("// we're testing")));
        assert!(matches!(&segments[1], EmitSegment::Param { name, .. } if name == "param"));
        assert!(matches!(&segments[2], EmitSegment::JsChunk(s) if s == ";"));
    }

    #[test]
    fn tokenize_apostrophe_in_block_comment() {
        // Apostrophes in block comments should not affect string tracking
        let segments = tokenize_emit(
            r#"/* it's a test */
const x = %param;"#,
        );
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], EmitSegment::JsChunk(s) if s.contains("/* it's a test */")));
        assert!(matches!(&segments[1], EmitSegment::Param { name, .. } if name == "param"));
        assert!(matches!(&segments[2], EmitSegment::JsChunk(s) if s == ";"));
    }

    #[test]
    fn tokenize_param_after_comment_with_apostrophe() {
        // The actual bug case: param after comment containing apostrophe
        let segments = tokenize_emit(
            r#"// Enable if we're ready
if (%enabled) { doIt(); }"#,
        );
        assert_eq!(segments.len(), 3);
        assert!(
            matches!(&segments[0], EmitSegment::JsChunk(s) if s.contains("// Enable if we're ready"))
        );
        assert!(matches!(&segments[1], EmitSegment::Param { name, .. } if name == "enabled"));
        assert!(matches!(&segments[2], EmitSegment::JsChunk(s) if s.contains("doIt()")));
    }

    #[test]
    fn tokenize_placeholder_in_comment_ignored() {
        // Placeholders inside comments should be treated as part of the comment
        let segments = tokenize_emit(
            r#"// %param not parsed
const x = %real;"#,
        );
        assert_eq!(segments.len(), 3);
        // The comment contains %param but it should not be parsed
        assert!(
            matches!(&segments[0], EmitSegment::JsChunk(s) if s.contains("// %param not parsed"))
        );
        // Only %real should be parsed as a param
        assert!(matches!(&segments[1], EmitSegment::Param { name, .. } if name == "real"));
    }

    // =========================================================================
    // Template Literal Interpolation Tests
    // =========================================================================

    #[test]
    fn tokenize_template_literal_with_interpolation() {
        // Template literals with ${} interpolations containing strings
        let input = r#"const x = `${fn}(${args.join(', ')})`; const y = %param;"#;
        let segments = tokenize_emit(input);
        // The %param should be parsed as a Param
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "param")),
            "Should find %param outside template literal"
        );
    }

    #[test]
    fn tokenize_param_after_template_literal() {
        // This mimics the apply-animations pattern
        let input = r#"return `${fn}(${interpolatedArgs.join(', ')})`;
}

const x = %driver;"#;
        let segments = tokenize_emit(input);
        // %driver should be parsed as a Param
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "driver")),
            "Should find %driver after template literal. Segments: {:?}",
            segments
        );
    }

    #[test]
    fn tokenize_complex_template_literal_interpolation() {
        // Multiple template literals with nested strings
        let input = r#"const a = `Hello ${name}`;
const b = `${fn}(${args.join(', ')})`;
const c = %param;"#;
        let segments = tokenize_emit(input);
        // %param should be parsed as a Param
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "param")),
            "Should find %param after template literals. Segments: {:?}",
            segments
        );
    }

    // =========================================================================
    // Regex Literal Tests (quotes inside regex should NOT affect string tracking)
    // =========================================================================

    #[test]
    fn tokenize_regex_with_quote_in_char_class() {
        // The " inside [...] is part of the regex, not a string
        let input = r#"const x = selector.replace(/[.#\[\]=">~+\s]/g, '-'); const y = %param;"#;
        let segments = tokenize_emit(input);
        // %param should be parsed as a Param (regex should not corrupt string state)
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "param")),
            "Should find %param after regex with quote. Segments: {:?}",
            segments
        );
    }

    #[test]
    fn tokenize_param_after_regex_with_flags() {
        // Regex with flags
        let input = r#"const match = s.match(/rgba?\s*\(/g); const x = %driver;"#;
        let segments = tokenize_emit(input);
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "driver")),
            "Should find %driver after regex. Segments: {:?}",
            segments
        );
    }

    #[test]
    fn tokenize_multiple_regexes_with_param() {
        // Multiple regexes followed by a param
        let input = r#"const a = /foo/; const b = /bar"baz/g; const c = %param;"#;
        let segments = tokenize_emit(input);
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "param")),
            "Should find %param after multiple regexes. Segments: {:?}",
            segments
        );
    }

    #[test]
    fn tokenize_division_not_confused_with_regex() {
        // Division should not be confused with regex
        let input = r#"const x = 10 / 2; const y = %param;"#;
        let segments = tokenize_emit(input);
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "param")),
            "Should find %param after division. Segments: {:?}",
            segments
        );
    }

    // =========================================================================
    // In-String Placeholder Tests
    // =========================================================================

    #[test]
    fn tokenize_param_inside_single_quote_string() {
        // %param inside a single-quoted string should be parsed with in_string context
        let segments = tokenize_emit(r#"console.log('Hello %name');"#);
        assert!(
            segments.iter().any(|s| matches!(
                s, EmitSegment::Param { name, field: None, in_string: Some('\'') } if name == "name"
            )),
            "Should find Param with in_string=Some(') for %name inside string. Segments: {:?}",
            segments
        );
    }

    #[test]
    fn tokenize_param_inside_double_quote_string() {
        // %param inside a double-quoted string should be parsed with in_string context
        let segments = tokenize_emit(r#"const url = "%url";"#);
        assert!(
            segments.iter().any(|s| matches!(
                s, EmitSegment::Param { name, field: None, in_string: Some('"') } if name == "url"
            )),
            "Should find Param with in_string=Some(\") for %url inside string. Segments: {:?}",
            segments
        );
    }

    #[test]
    fn tokenize_varref_inside_string() {
        // %$var.field inside a string should be parsed with in_string context
        let segments = tokenize_emit(r#"el.querySelector("%$b.sel")"#);
        assert!(
            segments.iter().any(|s| matches!(
                s, EmitSegment::VarRef { var, field, in_string: Some('"') }
                if var == "b" && *field == Some("sel".to_string())
            )),
            "Should find VarRef with in_string context. Segments: {:?}",
            segments
        );
    }

    #[test]
    fn tokenize_param_outside_string_no_in_string() {
        // %param outside strings should have in_string: None
        let segments = tokenize_emit("const x = %param;");
        assert!(
            segments.iter().any(|s| matches!(
                s, EmitSegment::Param { name, field: None, in_string: None } if name == "param"
            )),
            "Should find Param with in_string=None outside string. Segments: {:?}",
            segments
        );
    }

    // =========================================================================
    // Escape Mechanism Tests
    // =========================================================================

    #[test]
    fn tokenize_escape_percent() {
        // %% should produce a literal % in output
        let segments = tokenize_emit("console.log('100%%');");
        // Should contain a JsChunk with '100%' not '100%%'
        assert!(segments.iter().any(|s| matches!(s, EmitSegment::JsChunk(chunk) if chunk.contains("100%") && !chunk.contains("%%"))),
            "Should produce literal % for %%. Segments: {:?}", segments);
        // Should NOT have any Param segments
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { .. })),
            "Should not have any Param segments for %%"
        );
    }

    #[test]
    fn tokenize_escape_before_param_name() {
        // %%name should be literal %name, not a param
        let segments = tokenize_emit("const x = '%%name';");
        // Should NOT have a Param for 'name'
        assert!(
            !segments
                .iter()
                .any(|s| matches!(s, EmitSegment::Param { name, .. } if name == "name")),
            "%%name should not be parsed as Param. Segments: {:?}",
            segments
        );
        // Should contain %name in a JsChunk
        assert!(
            segments
                .iter()
                .any(|s| matches!(s, EmitSegment::JsChunk(chunk) if chunk.contains("%name"))),
            "Should have literal %name in output. Segments: {:?}",
            segments
        );
    }

    #[test]
    fn tokenize_multiple_in_string_params() {
        // Multiple params in the same string
        let segments = tokenize_emit(r#"const msg = '%first %last';"#);
        let param_count = segments
            .iter()
            .filter(|s| {
                matches!(
                    s,
                    EmitSegment::Param {
                        in_string: Some('\''),
                        ..
                    }
                )
            })
            .count();
        assert_eq!(
            param_count, 2,
            "Should find 2 params inside string. Segments: {:?}",
            segments
        );
    }
}

#[test]
fn tokenize_fullscreen_quad_preserves_createbuffer() {
    let content = r#"
    const vertices = new Float32Array([
      -1, -1,
       1, -1,
      -1,  1,
      -1,  1,
       1, -1,
       1,  1
    ]);

    const vbo = el._stGL.createBuffer();
    el._stGL.bindBuffer(el._stGL.ARRAY_BUFFER, vbo);
    el._stGL.bufferData(el._stGL.ARRAY_BUFFER, vertices, el._stGL.STATIC_DRAW);
    el._stGL.bindBuffer(el._stGL.ARRAY_BUFFER, null);

    %yield vbo -> $vbo;"#;

    let segments = tokenize_emit(content);
    let all_js: String = segments
        .iter()
        .filter_map(|s| match s {
            EmitSegment::JsChunk(c) => Some(c.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        all_js.contains("createBuffer"),
        "Tokenized output must contain createBuffer. JS chunks: {}",
        all_js
    );
}

#[test]
fn test_parse_param_with_field_accessor() {
    let segments = tokenize_emit("gl.clearColor(%clearColor.rgba);");
    // Should have: JsChunk("gl.clearColor("), Param{name:"clearColor", field:Some("rgba")}, JsChunk(");");
    let param_seg = segments
        .iter()
        .find(|s| matches!(s, EmitSegment::Param { .. }));
    assert!(param_seg.is_some(), "Should find a Param segment");
    match param_seg.unwrap() {
        EmitSegment::Param { name, field, .. } => {
            assert_eq!(name, "clearColor");
            assert_eq!(field.as_deref(), Some("rgba"));
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_parse_param_without_field() {
    let segments = tokenize_emit("x = %fps;");
    let param_seg = segments
        .iter()
        .find(|s| matches!(s, EmitSegment::Param { .. }));
    assert!(param_seg.is_some());
    match param_seg.unwrap() {
        EmitSegment::Param { name, field, .. } => {
            assert_eq!(name, "fps");
            assert_eq!(*field, None);
        }
        _ => unreachable!(),
    }
}
