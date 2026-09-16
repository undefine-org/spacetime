; Spacetime syntax highlighting
; Matches the simplified grammar structure

; === Comments ===
(line_comment) @comment
(block_comment) @comment

; === Keywords (@directives) ===
(directive "@" @keyword)
(directive (identifier) @keyword)

; === Metasystem keywords (%directives) ===
(meta_directive "%" @keyword.directive)
(meta_directive (identifier) @keyword.directive)

; === Emit directive (%emit) ===
(emit_directive "%" @keyword.directive)
(emit_directive "emit" @keyword.directive)
(emit_directive (identifier) @string.special)  ; language name
(emit_content) @string

; === Variables ($name) ===
(variable_ref "$" @punctuation.special)
(variable_ref (identifier) @variable)

; === Element refs (&name) ===
(element_ref "&" @punctuation.special)
(element_ref (identifier) @variable.special)

; === Preset refs (~name) ===
(preset_ref "~" @punctuation.special)
(preset_ref (identifier) @constant)

; === Strings ===
(string) @string
(template_string) @string.special

; === Numbers ===
(number) @number
(duration) @number
(percentage) @number
(color) @constant

; === Properties ===
(property_name) @property
(property_decl (property_name) @property)

; === Functions ===
(function_call (identifier) @function)

; === Operators ===
"->" @operator
":" @punctuation.delimiter
";" @punctuation.delimiter
"," @punctuation.delimiter
"(" @punctuation.bracket
")" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket

; === Selectors ===
(selector) @type
(selector_list (selector) @type)
