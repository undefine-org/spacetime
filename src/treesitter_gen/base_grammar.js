/// <reference types="tree-sitter-cli/dsl" />
// @ts-check
// Tree-sitter grammar for Spacetime - Generated from meta-AST
// This file is a template. Directive rules are injected at {{DIRECTIVE_RULES}}

module.exports = grammar({
  name: 'spacetime',

  externals: $ => [
    $.emit_content,
  ],

  extras: $ => [
    /\s/,
    $.line_comment,
    $.block_comment,
  ],

  conflicts: $ => [
    [$.meta_directive],
    // The named definition forms share `meta_directive`'s ambiguity: after
    // `% <kw> name`, a following bare identifier could begin `_meta_inline_args`
    // or a new item. Same GLR resolution as `meta_directive` (FEAT-118).
    [$.macro_def],
    [$.primitive_def],
    [$.capture_type_def],
  ],

  word: $ => $.identifier,

  rules: {
    source_file: $ => repeat($._item),

    _item: $ => choice(
      $.directive,
      $.generic_directive,
      $.emit_directive,
      $.macro_def,
      $.primitive_def,
      $.capture_type_def,
      $.meta_directive,
      $.scope_block,
    ),

    // Known directive names - generated from stdlib %macro definitions
    _known_directive_name: $ => choice(
      // {{DIRECTIVE_CHOICE}}
    ),

    // Directive with known name (higher precedence for syntax highlighting)
    // prec.right: prefer SHIFT into _directive_args over reducing directive early
    directive: $ => prec.right(1, seq(
      '@',
      $._known_directive_name,
      optional($._directive_args),
      optional($.block),
      optional(';')
    )),

    // Fallback for directives not in stdlib (user-defined or unknown)
    generic_directive: $ => prec.right(-1, seq(
      '@',
      $.identifier,
      optional($._directive_args),
      optional($.block),
      optional(';')
    )),

    _directive_args: $ => prec.left(2, choice(
      seq('(', optional($._args_inner), ')'),
      seq($._inline_directive_args, optional(seq('(', optional($._args_inner), ')')))
    )),

    _inline_directive_args: $ => prec.right(0, seq(
      repeat1($._inline_arg),
      optional(seq(':', $._args_inner))
    )),

    _args_inner: $ => prec.left(seq(
      $._arg,
      repeat(seq(choice(',', 'as', 'in'), $._arg)),
      optional(',')
    )),

    _arg: $ => choice(
      // $name:type = default (form parameter with type and default)
      seq($.variable_ref, ':', $.identifier, optional(seq('=', $._expression))),
      // name: value (named argument)
      seq($.identifier, ':', $._expression),
      // positional value
      $._expression
    ),

    _inline_arg: $ => choice(
      $.string,
      $.template_string,
      $.duration,
      $.dimension,
      $.number,
      $.variable_ref,
      $.element_ref,
      $.preset_ref,
      'in',
      'as',
      prec(2, $.identifier)
    ),

    // %emit js/css/glsl { raw content }
    emit_directive: $ => seq(
      '%',
      'emit',
      $.identifier,
      '{',
      optional($.emit_content),
      '}'
    ),

    // Named definition forms (FEAT-118): `%macro`/`%primitive`/`%capture_type`
    // get their OWN node kinds with a `name` field, so the Spell LanguageProfile
    // (`export-spell-profile`) can resolve §function/§decl to real tree-sitter
    // nodes and extract symbol names — instead of every `%`-construct collapsing
    // into one generic `meta_directive`. Mirrors the existing `emit_directive`
    // specialization. Other `%`-kinds fall through to `meta_directive`.
    macro_def: $ => seq(
      '%',
      'macro',
      field('name', $.identifier),
      optional($._meta_inline_args),
      optional($.meta_block)
    ),

    primitive_def: $ => seq(
      '%',
      'primitive',
      field('name', $.identifier),
      optional($._meta_inline_args),
      optional($.meta_block)
    ),

    capture_type_def: $ => seq(
      '%',
      choice('capture_type', 'captureType'),
      field('name', $.identifier),
      optional($._meta_inline_args),
      optional($.meta_block)
    ),

    // %meta patterns (excluding %emit and the named definition forms above)
    meta_directive: $ => seq(
      '%',
      $.identifier,
      optional($._meta_inline_args),
      optional($.meta_block)
    ),

    _meta_inline_args: $ => prec.left(choice(
      seq('(', optional($._args_inner), ')'),
      repeat1($._meta_inline_arg)
    )),

    _meta_inline_arg: $ => choice(
      $.variable_ref,
      $.identifier,
      'in',
    ),

    meta_block: $ => seq(
      '{',
      repeat($._meta_item),
      '}'
    ),

    _meta_item: $ => choice(
      $.directive,
      $.generic_directive,
      $.macro_def,
      $.primitive_def,
      $.capture_type_def,
      $.meta_directive,
      $.property_decl,
    ),

    // Scope blocks: .selector { ... } or element { ... }
    scope_block: $ => choice(
      seq($.selector_list, $.block),
      prec.dynamic(-10, seq($.identifier, $.block))
    ),

    selector_list: $ => seq(
      repeat1($.selector),
      repeat(seq(',', repeat1($.selector)))
    ),

    block: $ => seq('{', repeat($._block_item), '}'),

    _block_item: $ => choice(
      $.directive,
      $.generic_directive,
      $.property_decl,
      $.value_decl,
      $.nested_scope,
    ),

    nested_scope: $ => seq(choice($.selector_list, $.element_ref), $.block),

    property_decl: $ => prec.left(seq(
      $.property_name,
      ':',
      $._property_value,
      optional(';')
    )),

    _property_value: $ => prec.left(repeat1(choice(
      $.transition_arrow,
      $._value,
    ))),

    transition_arrow: $ => seq($._value, '->', $._value),

    value_decl: $ => seq($.variable_ref, ':', $._value, ';'),

    // Values
    _value: $ => choice(
      $.function_call,
      $.color,
      $.duration,
      $.dimension,
      $.percentage,
      $.number,
      $.string,
      $.template_string,
      $.variable_ref,
      $.element_ref,
      $.preset_ref,
      $.identifier
    ),

    // Expression (for more complex contexts)
    _expression: $ => choice(
      $._value,
      $.binary_expr,
      $.paren_expr,
    ),

    binary_expr: $ => prec.left(1, seq(
      $._expression,
      choice('+', '-', '*', '/', '===', '!==', '==', '!=', '<', '>', '<=', '>=', '&&', '||'),
      $._expression
    )),

    paren_expr: $ => seq('(', $._expression, ')'),

    function_call: $ => prec(2, seq($.identifier, '(', optional($._args_inner), ')')),

    // Template invocation: &template-name($arg1, key: $val)
    template_invocation: $ => seq(
      '&',
      $.identifier,
      optional(seq('(', optional($._args_inner), ')'))
    ),

    // Pattern matching for signals
    pattern_match: $ => seq(
      $.variable_ref,
      'is',
      $.identifier,
      optional(seq('{', repeat($.identifier), '}'))
    ),

    // Easing values (presets or cubic-bezier)
    easing_value: $ => choice(
      $.preset_ref,
      seq('cubic-bezier', '(', $.number, ',', $.number, ',', $.number, ',', $.number, ')')
    ),

    // Type reference
    type_ref: $ => seq(
      $.identifier,
      optional(seq('<', $.type_ref, repeat(seq(',', $.type_ref)), '>'))
    ),

    // Param list: ($value, &element, $optional?)
    param_list: $ => seq(
      '(',
      optional(seq(
        $._param_item,
        repeat(seq(',', $._param_item)),
        optional(',')
      )),
      ')'
    ),

    _param_item: $ => seq(
      choice($.variable_ref, $.element_ref),
      optional('?')
    ),

    // Field declaration for struct-like constructs
    field_decl: $ => seq(
      $.identifier,
      ':',
      $.type_ref,
      optional(';')
    ),

    // State declaration
    state_decl: $ => seq(
      $.identifier,
      optional(seq(':', $._value)),
      optional(';')
    ),

    // Transition declaration
    transition_decl: $ => seq(
      $.identifier,
      '->',
      $.identifier,
      optional($.block),
      optional(';')
    ),

    // Keyframe block
    keyframe_block: $ => seq(
      '{',
      repeat($.keyframe_stop),
      '}'
    ),

    keyframe_stop: $ => seq(
      choice($.percentage, $.number, 'from', 'to'),
      $.block
    ),

    // Mutation action
    mutation_action: $ => seq(
      $.variable_ref,
      choice('=', '+=', '-=', '*=', '/='),
      $._value,
      optional(';')
    ),

    // Template content (for template captures)
    template_content: $ => repeat1(choice(
      $._value,
      $.nested_scope,
    )),

    // HTML block (simplified)
    html_block: $ => seq(
      '<',
      $.identifier,
      repeat($._html_attr),
      choice(
        '/>',
        seq('>', repeat($._html_content), '</', $.identifier, '>')
      )
    ),

    _html_attr: $ => seq($.identifier, optional(seq('=', choice($.string, $.variable_ref)))),

    _html_content: $ => choice(
      $.html_block,
      $.variable_ref,
      /[^<]+/
    ),

    // JS block (simplified - actual content handled by external scanner)
    js_block: $ => seq('{', optional($.emit_content), '}'),

    // Comments
    line_comment: $ => seq('//', /.*/),
    block_comment: $ => seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/'),

    // Terminals
    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_-]*/,
    property_name: $ => /[a-zA-Z-][a-zA-Z0-9-]*/,

    variable_ref: $ => seq('$', $.identifier),
    element_ref: $ => seq('&', $.identifier),
    preset_ref: $ => seq('~', $.identifier),

    selector: $ => /[.#\[][^\s{};,]*/,

    string: $ => /"[^"]*"/,
    template_string: $ => /`[^`]*`/,
    number: $ => /-?\d+(\.\d+)?/,
    duration: $ => /-?\d+(\.\d+)?(ms|s)/,
    dimension: $ => /-?\d+(\.\d+)?(px|em|rem|vh|vw|vmin|vmax|%|deg|rad|turn|fr)/,
    percentage: $ => /\d+%/,
    color: $ => /#[0-9a-fA-F]{3,8}/,

    // {{DIRECTIVE_RULES}}
  },
});
