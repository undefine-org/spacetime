//! AST visitor for collecting symbol usages
//!
//! Traverses a parsed StFile to extract all symbol references.

use std::collections::HashSet;
use std::path::Path;

use crate::parser::{
    BehaviorBlock, NestedScope, PatternBodyItem, PatternDef, PresetDef, PresetType, ScopeBlock,
    SourceSpan, StFile, meta_ast::MetaDef,
};
use crate::syntax::FormMatch;

use super::usage_index::{FileSymbolIndex, SymbolId, SymbolKind};

/// Visitor that collects symbol usages from an AST
pub struct UsageVisitor {
    /// Collected symbol definitions
    definitions: HashSet<SymbolId>,
    /// Collected symbol definitions with their EXACT capture spans
    definition_spans: Vec<(SymbolId, SourceSpan)>,
    /// Collected symbol usages with their spans
    usages: Vec<(SymbolId, SourceSpan)>,
}

impl UsageVisitor {
    pub fn new() -> Self {
        Self {
            definitions: HashSet::new(),
            definition_spans: Vec::new(),
            usages: Vec::new(),
        }
    }

    /// Visit an entire file and return the symbol index
    pub fn visit_file(file: &StFile, _path: &Path) -> FileSymbolIndex {
        let mut visitor = Self::new();
        visitor.visit_st_file(file);

        FileSymbolIndex {
            definitions: visitor.definitions,
            definition_spans: visitor.definition_spans,
            usages: visitor.usages,
            version: 0,
        }
    }

    fn visit_st_file(&mut self, file: &StFile) {
        // Collect definitions from patterns
        for pattern in &file.patterns {
            self.visit_pattern_def(pattern);
        }

        // Collect definitions from presets
        for preset in &file.presets {
            self.visit_preset_def(preset);
        }

        // Collect definitions from meta_defs
        for meta_def in &file.meta_defs {
            self.visit_meta_def(meta_def);
        }

        // Collect definitions and usages from FormMatches. `file.matches` is
        // the FLAT list — it already contains every match that also lives in a
        // scope's `matches` (the parser pushes each match to both). Walking
        // `file.matches` AND the scopes would record every scoped directive
        // TWICE, duplicating the references (gh-31). `local-state` matches live
        // only in scopes, but they register nothing (the no-op branch below).
        for fm in &file.matches {
            self.visit_form_match(fm);
        }
    }

    fn visit_pattern_def(&mut self, pattern: &PatternDef) {
        // Register the pattern definition
        self.definitions
            .insert(SymbolId::new(SymbolKind::Pattern, pattern.name.clone()));

        // Visit pattern body for usages
        for item in &pattern.body {
            self.visit_pattern_body_item(item);
        }
    }

    fn visit_pattern_body_item(&mut self, item: &PatternBodyItem) {
        match item {
            PatternBodyItem::Call(call) => {
                // This is a call to another pattern
                self.usages.push((
                    SymbolId::new(SymbolKind::Pattern, call.pattern_name.clone()),
                    call.span,
                ));
            }
            PatternBodyItem::Include(include) => {
                // @include is a pattern reference
                self.usages.push((
                    SymbolId::new(SymbolKind::Pattern, include.pattern_name.clone()),
                    include.span,
                ));
            }
            PatternBodyItem::For(for_ast) => {
                for nested in &for_ast.body {
                    self.visit_pattern_body_item(nested);
                }
            }
            PatternBodyItem::If(if_ast) => {
                for nested in &if_ast.body {
                    self.visit_pattern_body_item(nested);
                }
            }
            PatternBodyItem::Match(match_ast) => {
                for arm in &match_ast.arms {
                    for nested in &arm.body {
                        self.visit_pattern_body_item(nested);
                    }
                }
            }
            // Other items (StateMachine, State, Transition, etc.) are local definitions
            _ => {}
        }
    }

    fn visit_preset_def(&mut self, preset: &PresetDef) {
        let kind = match preset.preset_type {
            PresetType::Easing | PresetType::Scroll | PresetType::Animation | PresetType::Load => {
                SymbolKind::Preset
            }
        };
        self.definitions
            .insert(SymbolId::new(kind, preset.name.clone()));
    }

    fn visit_meta_def(&mut self, meta_def: &MetaDef) {
        match meta_def {
            MetaDef::Primitive(p) => {
                self.definitions
                    .insert(SymbolId::new(SymbolKind::Primitive, p.name.clone()));
            }
            MetaDef::Macro(m) => {
                self.definitions
                    .insert(SymbolId::new(SymbolKind::Macro, m.name.clone()));
            }
            MetaDef::Migration(mig) => {
                // PLAN-076: indexed under the migration id — the same name
                // its synthetic parse-side macro carries.
                self.definitions
                    .insert(SymbolId::new(SymbolKind::Macro, mig.id.clone()));
            }
            MetaDef::Preset(p) => {
                self.definitions
                    .insert(SymbolId::new(SymbolKind::Preset, p.name.clone()));
            }
            MetaDef::RuntimeRegistry(r) => {
                // Register the runtime registry definition itself
                self.definitions
                    .insert(SymbolId::new(SymbolKind::RuntimeRegistry, r.name.clone()));
            }
            MetaDef::CaptureType(ct) => {
                // Register the capture type definition
                self.definitions
                    .insert(SymbolId::new(SymbolKind::CaptureType, ct.name.clone()));
            }
            MetaDef::Vendor(_) => {
                // Vendored deps are not indexed as LSP symbols (yet).
            }
            MetaDef::CommentType(_) => {
                // Comment types describe TRIVIA (`//@` comments), which never
                // reach emitted output — so they are not code symbols and an
                // LSP usage index would have nothing to point at. The comments
                // roster is served by the pill / `check` / MCP instead.
            }
            MetaDef::ScalarType(_) => {
                // Scalar types are DATA (a registry table), not code symbols —
                // like comment types they never reach emitted output, so an
                // LSP usage index has nothing to point at.
            }
        }
    }

    /// Visit a FormMatch and categorize it based on macro_name.
    ///
    /// Type definitions register as `Type`, macros with a "name" capture
    /// register as `Binding` definitions, and everything else registers
    /// as a `Directive` usage. The @each macro also tracks its data source
    /// as a `Binding` usage.
    fn visit_form_match(&mut self, fm: &FormMatch) {
        match fm.macro_name.as_str() {
            // Type definitions get their own SymbolKind
            "type" => {
                if let Some(name) = fm.get_ident("name") {
                    let symbol = SymbolId::new(SymbolKind::Type, name.to_string());
                    self.definitions.insert(symbol.clone());
                    self.record_definition(symbol, fm);
                }
            }
            // Binding-producing macros: register the named definition. The name
            // may arrive as an Ident (`@data $x : type{}` legacy) OR a Binding
            // (`$name:binding`, the unified `@data <kind>` surface) — mirror the
            // compiler's `analyze_data_sources` (gh-31): strip the leading `$`
            // so the definition and an `@each($name …)` source reference share
            // one symbol identity, or the definition is never linked.
            "data" | "computed" | "fn" => {
                if let Some(name) = fm
                    .get_ident("name")
                    .or_else(|| fm.get_binding("name"))
                    .map(|s| s.strip_prefix('$').unwrap_or(s))
                {
                    let symbol = SymbolId::new(SymbolKind::Binding, name.to_string());
                    self.definitions.insert(symbol.clone());
                    self.record_definition(symbol, fm);
                }
            }
            // Scope-local bindings (no global registration)
            "local-state" | "element-ref" => {}
            // A template DECLARATION: `@template &card($label) { ... }`. The
            // name capture is the bare template name ("card"). Register the
            // definition so a `&card(...)` invocation (same-file or imported)
            // resolves to it (gh-30 / BUG-341).
            "template" => {
                if let Some(name) = fm.get_ident("name") {
                    let symbol = SymbolId::new(SymbolKind::Template, name.to_string());
                    self.definitions.insert(symbol.clone());
                    self.record_definition(symbol, fm);
                }
            }
            // @each tracks its data source as a Binding usage. The span is the
            // SOURCE CAPTURE (`$items` token), not the whole `@each` block —
            // the old `fm.span` made every reference cover the entire block.
            "each" => {
                if let Some(source) = fm.get_binding("source").or_else(|| fm.get_ident("source")) {
                    let source_name = source.strip_prefix('$').unwrap_or(source);
                    if !source_name.is_empty() {
                        let span = fm.get_capture_span("source").unwrap_or(fm.span);
                        self.usages.push((
                            SymbolId::new(SymbolKind::Binding, source_name.to_string()),
                            span,
                        ));
                    }
                }
            }
            // A template INVOCATION: `&card("Hi")` (bare) or `&ref &card("Hi")`
            // (named). The `name` capture is the template being invoked.
            "template-invoke-bare" | "template-invoke-named" => {
                if let Some(name) = fm.get_ident("name").or_else(|| fm.get_string("name")) {
                    // Dynamic dispatch `&$name(...)` has no static definition.
                    let name = name.strip_prefix('$').unwrap_or(name);
                    if !name.is_empty() {
                        let span = fm.get_capture_span("name").unwrap_or(fm.span);
                        self.usages.push((
                            SymbolId::new(SymbolKind::Template, name.to_string()),
                            span,
                        ));
                    }
                }
            }
            // Event handlers — no symbol registration needed
            _ if fm.macro_name.starts_with("on-") => {}
            // Unknown macros register as directive usages
            _ => {
                if !fm.macro_name.is_empty() {
                    self.usages.push((
                        SymbolId::new(SymbolKind::Directive, fm.macro_name.clone()),
                        fm.span,
                    ));
                }
            }
        }
    }

    /// Record a definition with the EXACT capture span of its name token
    /// (gh-30): go-to-definition lands on `$products`, not the whole `@data`.
    fn record_definition(&mut self, symbol: SymbolId, fm: &FormMatch) {
        let span = fm.get_capture_span("name").unwrap_or(fm.span);
        self.definition_spans.push((symbol, span));
    }

    fn visit_scope(&mut self, scope: &ScopeBlock) {
        // Visit behavior block
        self.visit_behavior(&scope.behavior);

        // Visit FormMatches in this scope
        for fm in &scope.matches {
            self.visit_form_match(fm);
        }

        // Visit nested scopes
        for nested in &scope.nested_scopes {
            self.visit_nested_scope(nested);
        }
    }

    fn visit_nested_scope(&mut self, nested: &NestedScope) {
        // Visit behavior
        self.visit_behavior(&nested.behavior);

        // Visit FormMatches in this nested scope
        for fm in &nested.matches {
            self.visit_form_match(fm);
        }

        // Visit sub-nested scopes
        for sub in &nested.nested_scopes {
            self.visit_nested_scope(sub);
        }
    }

    fn visit_behavior(&mut self, _behavior: &BehaviorBlock) {
        // State machines, states, transitions are local definitions
        // We could track pattern calls here if behavior supported them
    }
}

impl Default for UsageVisitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::{CapturedValue, FormMatch};
    use std::path::PathBuf;

    #[test]
    fn test_empty_file() {
        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        assert!(index.definitions.is_empty());
        assert!(index.usages.is_empty());
    }

    #[test]
    fn test_macro_call_usage() {
        let scroll_fm = FormMatch::new("scroll").with_span(SourceSpan::new(10, 20));

        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![scroll_fm],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        assert!(index.definitions.is_empty());
        assert_eq!(index.usages.len(), 1);
        assert_eq!(index.usages[0].0.name, "scroll");
        assert_eq!(index.usages[0].0.kind, SymbolKind::Directive);
    }

    #[test]
    fn test_type_definition_via_form_match() {
        let type_fm = FormMatch::new("type")
            .capture("name", CapturedValue::Ident("Product".to_string()))
            .with_span(SourceSpan::new(0, 50));

        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![type_fm],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        assert_eq!(index.definitions.len(), 1);
        assert!(
            index
                .definitions
                .contains(&SymbolId::new(SymbolKind::Type, "Product"))
        );
    }

    #[test]
    fn test_data_definition_via_form_match() {
        let data_fm = FormMatch::new("data")
            .capture("name", CapturedValue::Ident("products".to_string()))
            .with_span(SourceSpan::new(0, 50));

        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![data_fm],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        assert_eq!(index.definitions.len(), 1);
        assert!(
            index
                .definitions
                .contains(&SymbolId::new(SymbolKind::Binding, "products"))
        );
    }

    #[test]
    fn test_computed_definition_via_form_match() {
        let computed_fm = FormMatch::new("computed")
            .capture("name", CapturedValue::Ident("totalPrice".to_string()))
            .with_span(SourceSpan::new(0, 50));

        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![computed_fm],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        assert_eq!(index.definitions.len(), 1);
        assert!(
            index
                .definitions
                .contains(&SymbolId::new(SymbolKind::Binding, "totalPrice"))
        );
    }

    #[test]
    fn test_function_definition_via_form_match() {
        let fn_fm = FormMatch::new("fn")
            .capture("name", CapturedValue::Ident("handleClick".to_string()))
            .with_span(SourceSpan::new(0, 50));

        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![fn_fm],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        assert_eq!(index.definitions.len(), 1);
        assert!(
            index
                .definitions
                .contains(&SymbolId::new(SymbolKind::Binding, "handleClick"))
        );
    }

    #[test]
    fn test_each_block_references_data_source() {
        let each_fm = FormMatch::new("each")
            .capture("source", CapturedValue::Binding("$products".to_string()))
            .with_span(SourceSpan::new(100, 150));

        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![each_fm],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        assert_eq!(index.usages.len(), 1);
        assert_eq!(index.usages[0].0.name, "products");
        assert_eq!(index.usages[0].0.kind, SymbolKind::Binding);
    }

    #[test]
    fn test_mixed_file_with_all_directive_types() {
        let type_fm = FormMatch::new("type")
            .capture("name", CapturedValue::Ident("User".to_string()))
            .with_span(SourceSpan::new(0, 30));

        let data_fm = FormMatch::new("data")
            .capture("name", CapturedValue::Ident("users".to_string()))
            .with_span(SourceSpan::new(40, 80));

        let computed_fm = FormMatch::new("computed")
            .capture("name", CapturedValue::Ident("userCount".to_string()))
            .with_span(SourceSpan::new(90, 130));

        let fn_fm = FormMatch::new("fn")
            .capture("name", CapturedValue::Ident("loadUsers".to_string()))
            .with_span(SourceSpan::new(140, 180));

        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![type_fm, data_fm, computed_fm, fn_fm],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        assert_eq!(index.definitions.len(), 4);
        assert!(
            index
                .definitions
                .contains(&SymbolId::new(SymbolKind::Type, "User"))
        );
        assert!(
            index
                .definitions
                .contains(&SymbolId::new(SymbolKind::Binding, "users"))
        );
        assert!(
            index
                .definitions
                .contains(&SymbolId::new(SymbolKind::Binding, "userCount"))
        );
        assert!(
            index
                .definitions
                .contains(&SymbolId::new(SymbolKind::Binding, "loadUsers"))
        );
    }

    #[test]
    fn test_form_match_with_missing_name_capture() {
        // Edge case: FormMatch has wrong type for name
        let bad_fm = FormMatch::new("type")
            .capture("name", CapturedValue::String("NotAnIdent".to_string()))
            .with_span(SourceSpan::new(0, 30));

        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![bad_fm],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        // Should not crash, but should not register the symbol
        assert!(index.definitions.is_empty());
    }

    #[test]
    fn test_form_match_with_no_name_capture() {
        // Edge case: FormMatch has no name capture at all
        let bad_fm = FormMatch::new("type")
            .capture("fields", CapturedValue::Properties(vec![]))
            .with_span(SourceSpan::new(0, 30));

        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![bad_fm],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        // Should not crash, should not register any symbol
        assert!(index.definitions.is_empty());
    }

    #[test]
    fn test_unknown_macro_registers_as_directive_usage() {
        let custom_fm = FormMatch::new("custom-directive")
            .capture("arg", CapturedValue::Ident("value".to_string()))
            .with_span(SourceSpan::new(0, 30));

        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![custom_fm],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        assert_eq!(index.usages.len(), 1);
        assert_eq!(index.usages[0].0.name, "custom-directive");
        assert_eq!(index.usages[0].0.kind, SymbolKind::Directive);
    }

    #[test]
    fn test_nested_scope_with_form_matches() {
        use crate::parser::BehaviorBlock;

        let data_fm = FormMatch::new("data")
            .capture("name", CapturedValue::Ident("items".to_string()))
            .with_span(SourceSpan::new(50, 70));

        let nested = NestedScope {
            kind: Default::default(),
            selector: ".item".to_string(),
            composed_selector: ".item".to_string(),
            behavior: BehaviorBlock::default(),
            css_declarations: vec![],
            form_refs: Vec::new(),
            nested_scopes: vec![],
            matches: vec![data_fm.clone()],
            collection_ref: None,
            span: SourceSpan::new(40, 100),
        };

        let scope = ScopeBlock {
            kind: Default::default(),
            selector: ".container".to_string(),
            behavior: BehaviorBlock::default(),
            css_declarations: vec![],
            form_refs: Vec::new(),
            nested_scopes: vec![nested],
            matches: vec![],
            span: SourceSpan::new(0, 110),
            source_file: None,
            exports: vec![],
            refs: vec![],
            states: vec![],
            html: String::new(),
        };
        // The parser pushes every match to the flat `file.matches` AND the
        // scope's `matches` — the flat list is the single walk the visitor
        // uses (gh-31), so the fixture mirrors that by listing the match in
        // both places.
        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![scope],
            matches: vec![data_fm],
            form_refs: Vec::new(),
            html_blocks: vec![],
            raw_css_blocks: vec![],
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        assert_eq!(index.definitions.len(), 1);
        assert!(
            index
                .definitions
                .contains(&SymbolId::new(SymbolKind::Binding, "items"))
        );
        // The definition is registered ONCE with the exact name capture span.
        assert_eq!(index.definition_spans.len(), 1);
        assert_eq!(index.definition_spans[0].0, SymbolId::new(SymbolKind::Binding, "items"));
    }

    /// gh-31: a scoped directive appears in BOTH `file.matches` (flat) and the
    /// scope's `matches`. The visitor must walk the flat list only, or every
    /// scoped reference is recorded twice.
    #[test]
    fn scoped_match_listed_twice_is_recorded_once() {
        let scroll_fm = FormMatch::new("scroll").with_span(SourceSpan::new(5, 30));

        let scope = ScopeBlock {
            kind: Default::default(),
            selector: ".x".to_string(),
            behavior: crate::parser::BehaviorBlock::default(),
            css_declarations: vec![],
            form_refs: Vec::new(),
            nested_scopes: vec![],
            matches: vec![scroll_fm.clone()],
            span: SourceSpan::new(0, 40),
            source_file: None,
            exports: vec![],
            refs: vec![],
            states: vec![],
            html: String::new(),
        };
        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![scope],
            matches: vec![scroll_fm],
            form_refs: Vec::new(),
            html_blocks: vec![],
            raw_css_blocks: vec![],
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        let scroll_usages: Vec<_> = index
            .usages
            .iter()
            .filter(|(s, _)| s.name == "scroll")
            .collect();
        assert_eq!(
            scroll_usages.len(),
            1,
            "a match present in both the flat list and a scope must be recorded once"
        );
    }

    #[test]
    fn test_on_event_handler_form_match() {
        let on_click_fm = FormMatch::new("on-click")
            .capture("action", CapturedValue::Expr("handleClick()".to_string()))
            .with_span(SourceSpan::new(0, 30));

        let file = StFile {
            imports: vec![],
            presets: vec![],
            patterns: vec![],
            meta_defs: vec![],
            scopes: vec![],
            html_blocks: vec![],
            raw_css_blocks: vec![],
            matches: vec![on_click_fm],
            form_refs: Vec::new(),
            module_manifest: None,
            diagnostics: vec![],
            file_exports: vec![],
            span: SourceSpan::default(),
        };

        let index = UsageVisitor::visit_file(&file, &PathBuf::from("/test.st"));
        // Event handlers don't register symbols currently, but shouldn't crash
        assert!(index.definitions.is_empty());
    }
}
