/**
 * Tree-sitter external scanner for Spacetime
 *
 * Handles brace-balanced content for:
 * - %emit js/css/glsl { ... }
 */

#include "tree_sitter/parser.h"
#include <wctype.h>

enum TokenType {
    EMIT_CONTENT,
};

/**
 * Scan brace-balanced content until the closing brace
 * Returns true if content was found, false otherwise
 */
static bool scan_braced_content(TSLexer *lexer) {
    int depth = 1;
    bool has_content = false;

    while (depth > 0 && !lexer->eof(lexer)) {
        switch (lexer->lookahead) {
            case '{':
                depth++;
                has_content = true;
                lexer->advance(lexer, false);
                break;
            case '}':
                depth--;
                if (depth > 0) {
                    has_content = true;
                    lexer->advance(lexer, false);
                }
                // Don't consume the final closing brace
                break;
            case '/':
                // Handle comments within content
                lexer->advance(lexer, false);
                has_content = true;
                if (lexer->lookahead == '/') {
                    // Line comment - consume until newline
                    while (!lexer->eof(lexer) && lexer->lookahead != '\n') {
                        lexer->advance(lexer, false);
                    }
                } else if (lexer->lookahead == '*') {
                    // Block comment - consume until */
                    lexer->advance(lexer, false);
                    while (!lexer->eof(lexer)) {
                        if (lexer->lookahead == '*') {
                            lexer->advance(lexer, false);
                            if (lexer->lookahead == '/') {
                                lexer->advance(lexer, false);
                                break;
                            }
                        } else {
                            lexer->advance(lexer, false);
                        }
                    }
                }
                break;
            case '"':
                // String literal - consume until closing quote
                lexer->advance(lexer, false);
                has_content = true;
                while (!lexer->eof(lexer) && lexer->lookahead != '"') {
                    if (lexer->lookahead == '\\') {
                        lexer->advance(lexer, false);
                        if (!lexer->eof(lexer)) {
                            lexer->advance(lexer, false);
                        }
                    } else {
                        lexer->advance(lexer, false);
                    }
                }
                if (lexer->lookahead == '"') {
                    lexer->advance(lexer, false);
                }
                break;
            case '\'':
                // Single-quoted string
                lexer->advance(lexer, false);
                has_content = true;
                while (!lexer->eof(lexer) && lexer->lookahead != '\'') {
                    if (lexer->lookahead == '\\') {
                        lexer->advance(lexer, false);
                        if (!lexer->eof(lexer)) {
                            lexer->advance(lexer, false);
                        }
                    } else {
                        lexer->advance(lexer, false);
                    }
                }
                if (lexer->lookahead == '\'') {
                    lexer->advance(lexer, false);
                }
                break;
            case '`':
                // Template literal - handle ${} interpolation
                lexer->advance(lexer, false);
                has_content = true;
                while (!lexer->eof(lexer) && lexer->lookahead != '`') {
                    if (lexer->lookahead == '$') {
                        lexer->advance(lexer, false);
                        if (lexer->lookahead == '{') {
                            // Nested interpolation - count braces
                            int interp_depth = 1;
                            lexer->advance(lexer, false);
                            while (interp_depth > 0 && !lexer->eof(lexer)) {
                                if (lexer->lookahead == '{') interp_depth++;
                                else if (lexer->lookahead == '}') interp_depth--;
                                if (interp_depth > 0) lexer->advance(lexer, false);
                            }
                            if (lexer->lookahead == '}') {
                                lexer->advance(lexer, false);
                            }
                        }
                    } else if (lexer->lookahead == '\\') {
                        lexer->advance(lexer, false);
                        if (!lexer->eof(lexer)) {
                            lexer->advance(lexer, false);
                        }
                    } else {
                        lexer->advance(lexer, false);
                    }
                }
                if (lexer->lookahead == '`') {
                    lexer->advance(lexer, false);
                }
                break;
            default:
                has_content = true;
                lexer->advance(lexer, false);
                break;
        }
    }

    return has_content;
}

void *tree_sitter_spacetime_external_scanner_create(void) {
    return NULL;
}

void tree_sitter_spacetime_external_scanner_destroy(void *payload) {
    // Nothing to clean up
}

unsigned tree_sitter_spacetime_external_scanner_serialize(
    void *payload,
    char *buffer
) {
    return 0;
}

void tree_sitter_spacetime_external_scanner_deserialize(
    void *payload,
    const char *buffer,
    unsigned length
) {
    // Nothing to deserialize
}

bool tree_sitter_spacetime_external_scanner_scan(
    void *payload,
    TSLexer *lexer,
    const bool *valid_symbols
) {
    // Skip leading whitespace
    while (iswspace(lexer->lookahead)) {
        lexer->advance(lexer, true);
    }

    // Only match emit_content if we're looking at actual content (not at end/close brace)
    // This prevents the scanner from matching during error recovery
    if (valid_symbols[EMIT_CONTENT]) {
        // Don't match if we're at closing brace or end of file
        if (lexer->eof(lexer) || lexer->lookahead == '}') {
            return false;
        }

        // Don't match if content looks like regular Spacetime syntax
        // (starts with @, %, ., #, or is a comment)
        if (lexer->lookahead == '@' ||
            lexer->lookahead == '%' ||
            lexer->lookahead == '.' ||
            lexer->lookahead == '#' ||
            lexer->lookahead == '/') {
            return false;
        }

        lexer->result_symbol = EMIT_CONTENT;
        return scan_braced_content(lexer);
    }

    return false;
}
