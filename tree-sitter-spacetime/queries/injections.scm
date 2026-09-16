; Language injections for embedded code

; JavaScript in %emit js { ... }
((emit_directive
  (identifier) @_lang
  (emit_content) @injection.content)
 (#eq? @_lang "js")
 (#set! injection.language "javascript"))

; CSS in %emit css { ... }
((emit_directive
  (identifier) @_lang
  (emit_content) @injection.content)
 (#eq? @_lang "css")
 (#set! injection.language "css"))

; GLSL in %emit glsl { ... }
((emit_directive
  (identifier) @_lang
  (emit_content) @injection.content)
 (#eq? @_lang "glsl")
 (#set! injection.language "glsl"))
