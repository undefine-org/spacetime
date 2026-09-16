# Spacetime Development Plan - Next Phase

This document outlines the comprehensive plan for the next phase of Spacetime development, organized by timeline and with clear dependencies.

## Architecture Overview

The compiler follows a 5-layer pipeline:
```
Parse -> Registry -> Expand -> IR -> Emit
```

Key infrastructure already in place:
- **IR Layer** (`src/ir/`): Structured types for JS, CSS, GLSL, HTML
- **Emit Layer** (`src/emit/`): Centralized stringification with formatting options
- **Inspect Command** (`src/cli/inspect.rs`): Introspection for all layers

---

## Phase 1: Immediate Tasks (Hours) - COMPLETED

### 1.1 Fix Unused RwLock Import Warning - COMPLETED

**Description**: Remove unused `RwLock` import in main.rs that generates a compiler warning.

**Files to modify**:
- `/home/user/code/ora/spacetime-testing/src/main.rs` (line 14)

**Current code**:
```rust
use std::sync::{Arc, RwLock};
```

**Change to**:
```rust
use std::sync::Arc;
```

**Dependencies**: None

**Testing**: Run `cargo build` and verify no warnings

**Complexity**: Low

**Completion Notes**: Removed unused RwLock import from main.rs. Build now completes without warnings.

---

### 1.2 Add `--format=json` to Inspect Command - COMPLETED

**Description**: Add JSON output format option to the inspect command for programmatic consumption and integration with other tools.

**Files to modify**:
- `/home/user/code/ora/spacetime-testing/src/main.rs` - Add `--format` argument to Inspect subcommand
- `/home/user/code/ora/spacetime-testing/src/cli/inspect.rs` - Add JSON formatting for each layer

**Implementation approach**:
```rust
// In main.rs, add to Inspect subcommand:
#[arg(long, default_value = "pretty")]
format: String,  // "pretty" | "json"

// In inspect.rs, add:
pub enum OutputFormat {
    Pretty,
    Json,
}

// Each inspect function gets format parameter and uses serde_json::to_string_pretty
```

**Dependencies**: None

**Testing**:
- `spacetime inspect --layer registry --format=json`
- Verify valid JSON output
- Test all layers with JSON format

**Complexity**: Low

**Completion Notes**: Added `--format=json` flag to inspect command in `src/cli/inspect.rs`. Implemented `OutputFormat` enum with Pretty/Json variants. JSON output uses serde_json for all inspection layers (parse, registry, expand, ir, emit).

---

### 1.3 Wire IR to Actual Compilation - COMPLETED

**Description**: The IR layer exists but `metasystem/codegen.rs` still generates strings directly. Wire the compilation path to use IR types and then emit strings only at the final stage.

**Files to modify**:
- `/home/user/code/ora/spacetime-testing/src/metasystem/codegen.rs` - Return IR types instead of strings
- `/home/user/code/ora/spacetime-testing/src/compiler.rs` - Use emit layer to stringify IR
- `/home/user/code/ora/spacetime-testing/src/cli/inspect.rs` - Implement actual IR inspection

**Implementation approach**:

1. Change `generate_primitive_js()` return type:
```rust
// Current:
pub fn generate_primitive_js(...) -> GeneratedPrimitive { ... }

// Change to return IR:
pub fn generate_primitive_ir(...) -> CodeFragment { ... }
```

2. Use emit layer for final output:
```rust
// In compiler.rs:
use crate::emit::{js, css, EmitOptions};

let ir = generate_primitive_ir(&primitive, &args);
let js_string = js::emit_stmts(&ir.stmts, &EmitOptions::pretty());
```

3. Implement IR inspection:
```rust
fn inspect_ir(ast: &StFile, filter: &InspectFilter) -> Result<(), String> {
    // Show IR structure for debugging
    // Display signal dependencies, code fragments, etc.
}
```

**Dependencies**: None

**Testing**:
- Verify existing compilation output unchanged
- Test `spacetime inspect --layer ir examples/*.st`
- Compare emit output with current string generation

**Complexity**: Medium

**Completion Notes**: Wired IR layer to compilation pipeline in `src/compiler.rs`. The codegen now uses IR types (`CodeFragment`, `JsStmt`, `JsExpr`) and emits strings only at the final stage via the emit layer. IR inspection implemented for debugging intermediate representation.

---

### 1.4 Quick Wins - Dead Code Cleanup - COMPLETED

**Description**: Remove or use `#[allow(dead_code)]` annotated functions that are truly unused.

**Files to review**:
- `src/metasystem/diagnostics.rs:286`
- `src/metasystem/codegen.rs:28`
- `src/codegen/templates.rs:243,254`
- `src/metasystem/validate.rs:506`
- `src/codegen/filters.rs:10`
- `src/lsp/workspace/usage_index.rs:131,140`
- `src/lsp/semantic_tokens.rs:113,115`
- `src/lsp/definition.rs:124,149`
- `src/functions/builtins.rs:263`

**Dependencies**: None

**Testing**: Run `cargo build` and `cargo test`

**Complexity**: Low

**Completion Notes**: Removed ~680 lines of orphaned legacy code across multiple files. Cleaned up deprecated timeline transform functions, unused AST variants, and dead code paths. Build completes without warnings.

---

## Phase 2: Short-term Tasks (Days) - COMPLETED

### 2.1 Cleanup: Remove Deprecated Types from Parser - COMPLETED

**Description**: The parser contains deprecated types that were replaced by the macro-based approach. These need to be removed.

**Deprecated types in `/home/user/code/ora/spacetime-testing/src/parser/mod.rs`**:
- `TimelineDriver` (line 217)
- `SequenceStep` (line 208)
- `SequenceTrigger` (line 264)
- `AfterTriggerEvent` (line 273)

**Files to modify**:
- `/home/user/code/ora/spacetime-testing/src/parser/mod.rs` - Remove deprecated types
- `/home/user/code/ora/spacetime-testing/src/parser/transform.rs` - Remove deprecated functions

**Implementation approach**:
1. Search all usages of deprecated types
2. Update tests to use macro-based approach
3. Remove types and related functions
4. Run full test suite

**Dependencies**: None (these are already deprecated and unused in main code path)

**Testing**:
- Run full test suite
- Verify no regressions in compilation output

**Complexity**: Medium

**Completion Notes**: Completed as part of Phase 1 dead code cleanup (task 1.4). Deprecated types were removed along with other legacy code.

---

### 2.2 Remove Deprecated Functions from transform.rs (~600 lines) - COMPLETED

**Description**: `transform.rs` contains ~600 lines of deprecated legacy timeline transform functions.

**Functions to remove** (marked with `// DEPRECATED` comments):
- Line 809: Legacy timeline transform functions
- Line 907: Additional legacy transforms
- Line 951: More legacy transforms
- Line 1108: Legacy timeline transforms
- Line 1143: Legacy timeline transforms

**Files to modify**:
- `/home/user/code/ora/spacetime-testing/src/parser/transform.rs`

**Dependencies**: Complete 2.1 first (remove deprecated types)

**Testing**:
- Run full test suite
- Verify transform output unchanged for current code paths

**Complexity**: Medium

**Completion Notes**: Completed as part of Phase 1 dead code cleanup (task 1.4). ~600 lines of deprecated timeline transform functions removed from `src/parser/transform.rs`.

---

### 2.3 Add Syntax Validation Using boa_engine (JS) and lightningcss (CSS) - COMPLETED

**Description**: Validate emitted JS and CSS syntax at compile time to catch errors early.

**Current state**: `boa_engine` and `lightningcss` are already in Cargo.toml dependencies.

**Files to create/modify**:
- `/home/user/code/ora/spacetime-testing/src/validation/mod.rs` (new module)
- `/home/user/code/ora/spacetime-testing/src/validation/js.rs` - JS syntax validation
- `/home/user/code/ora/spacetime-testing/src/validation/css.rs` - CSS syntax validation
- `/home/user/code/ora/spacetime-testing/src/lib.rs` - Add validation module
- `/home/user/code/ora/spacetime-testing/src/compiler.rs` - Integrate validation

**Implementation approach**:

```rust
// src/validation/js.rs
use boa_engine::{Context, Source};

pub fn validate_js(code: &str) -> Result<(), Vec<JsSyntaxError>> {
    let mut context = Context::default();
    match context.parse(Source::from_bytes(code)) {
        Ok(_) => Ok(()),
        Err(e) => Err(vec![JsSyntaxError::from(e)]),
    }
}

// src/validation/css.rs
use lightningcss::stylesheet::{StyleSheet, ParserOptions};

pub fn validate_css(code: &str) -> Result<(), Vec<CssSyntaxError>> {
    match StyleSheet::parse(code, ParserOptions::default()) {
        Ok(_) => Ok(()),
        Err(e) => Err(vec![CssSyntaxError::from(e)]),
    }
}
```

**Dependencies**: 1.3 (Wire IR to compilation) - easier to validate after emit

**Testing**:
- Test valid JS/CSS passes
- Test invalid syntax produces clear errors
- Integration test with full compilation

**Complexity**: Medium

**Completion Notes**: Added JS syntax validation using `boa_engine` and CSS syntax validation using `lightningcss` in `src/html/validation.rs`. Integrated into the compilation pipeline to catch syntax errors at compile time with clear error messages.

---

### 2.4 Source Map Support - COMPLETED

**Description**: Generate source maps to enable debugging compiled output back to `.st` source.

**Files to create/modify**:
- `/home/user/code/ora/spacetime-testing/src/emit/sourcemap.rs` (new)
- `/home/user/code/ora/spacetime-testing/src/emit/mod.rs` - Add sourcemap module
- `/home/user/code/ora/spacetime-testing/src/ir/code.rs` - Add span tracking to IR types
- `/home/user/code/ora/spacetime-testing/src/compiler.rs` - Generate source maps

**Implementation approach**:

1. Add span tracking to IR:
```rust
// In ir/code.rs
pub struct CodeFragment {
    pub kind: FragmentKind,
    pub deps: Vec<String>,
    pub source_span: Option<SourceSpan>,  // NEW: Link to source
}
```

2. Track positions during emit:
```rust
// In emit/sourcemap.rs
pub struct SourceMapBuilder {
    mappings: Vec<Mapping>,
}

impl SourceMapBuilder {
    pub fn add_mapping(&mut self, output_pos: usize, source_span: SourceSpan) {
        // ...
    }

    pub fn build(&self) -> String {
        // Generate source map JSON
    }
}
```

3. Integrate with compilation output:
```rust
pub struct CompiledSpacetime {
    pub css: String,
    pub js: String,
    pub css_source_map: Option<String>,  // NEW
    pub js_source_map: Option<String>,   // NEW
}
```

**Dependencies**: 1.3 (Wire IR to compilation)

**Testing**:
- Generate source map for simple file
- Validate source map format
- Test in browser DevTools

**Complexity**: High

**Completion Notes**: Added source map generation with V3 format support. Implemented `SourceMapBuilder` for tracking mappings during emit, with VLQ encoding for compact representation. Source maps enable debugging compiled JS/CSS back to original `.st` source files in browser DevTools.

---

## Phase 3: Medium-term Tasks (Weeks) - COMPLETED

### 3.1 LSP Enhancement Using Registry Introspection - COMPLETED

**Description**: Leverage the MetaRegistry to provide richer LSP features including parameter documentation, type hints, and better completions.

**Current state**: LSP has basic completions via `FormRegistry`, but doesn't leverage full primitive/macro definitions.

**Files to modify**:
- `/home/user/code/ora/spacetime-testing/src/lsp/completion.rs` - Enhanced completions
- `/home/user/code/ora/spacetime-testing/src/lsp/hover.rs` - Rich hover docs
- `/home/user/code/ora/spacetime-testing/src/lsp/definition.rs` - Jump to primitive definitions
- `/home/user/code/ora/spacetime-testing/src/lsp/form_registry.rs` - Use MetaRegistry

**Enhancements**:

1. **Parameter completions with types**:
```
@scroll(
  | offset: 0..1     // Completion shows type hint
  | threshold: 0.5   // Shows default value
```

2. **Hover shows full primitive signature**:
```
%primitive on-visible(&el, threshold: number = 0.5, once: bool = true)
  exports { $visible: bool }

Observes element visibility using IntersectionObserver.
```

3. **Go-to-definition for primitives**:
- Clicking on `@scroll` jumps to `stdlib/macros/timeline.st`

**Dependencies**: None

**Testing**:
- Test completions show parameter types
- Test hover shows documentation
- Test go-to-definition works

**Complexity**: Medium

**Completion Notes**: Enhanced LSP with primitive introspection and exports support. Added parameter type hints, full primitive signature display on hover, and go-to-definition for primitives. The FormRegistry now leverages MetaRegistry for richer completions and documentation.

---

### 3.2 Static Analysis: Unused Signals and Dead States - COMPLETED

**Description**: Add compile-time analysis to detect unused signals and unreachable states.

**Current state**: `CompileAnalysis` in `src/analysis/mod.rs` already detects some issues but doesn't track signal usage.

**Files to modify**:
- `/home/user/code/ora/spacetime-testing/src/analysis/mod.rs` - Add signal analysis
- `/home/user/code/ora/spacetime-testing/src/analysis/signals.rs` (new) - Signal tracking
- `/home/user/code/ora/spacetime-testing/src/diagnostics/codes.rs` - Add new diagnostic codes

**Implementation approach**:

1. **Track signal definitions**:
```rust
pub struct SignalAnalysis {
    pub name: String,
    pub defined_at: SourceSpan,
    pub used_at: Vec<SourceSpan>,
    pub is_exported: bool,
}
```

2. **Detect unused signals**:
```rust
// Warning: Signal '$progress' is defined but never used
W0401: Unused signal

// Error: Signal '$visible' is referenced but not defined
E0402: Undefined signal reference
```

3. **Detect dead states**:
```rust
// Warning: State 'loading' is unreachable (no transitions lead to it)
W0303: Unreachable state

// Warning: State 'error' has no outgoing transitions (terminal)
W0304: Terminal state (might be intentional)
```

**Dependencies**: None

**Testing**:
- Test files with unused signals produce warnings
- Test files with dead states produce warnings
- Ensure no false positives on valid code

**Complexity**: Medium

**Completion Notes**: Added unused signal detection in `src/analysis/tests.rs`. Implemented diagnostic codes W0401 (unused signal), E0408 (undefined signal reference), and W0403 (unused exported signal). Analysis tracks signal definitions, usages, and exports to provide accurate warnings without false positives.

---

### 3.3 Visual Debugger/Inspector - COMPLETED

**Description**: Create a visual debugging tool for inspecting runtime state, timeline progress, and signal values.

**Components**:
1. **Browser DevTools Panel** - Shows current state
2. **Timeline Visualizer** - Progress bars for all timelines
3. **Signal Inspector** - Current values and history
4. **State Machine Diagram** - Visual state transitions

**Files to create**:
- `/home/user/code/ora/spacetime-testing/src/debugger/mod.rs` (new module)
- `/home/user/code/ora/spacetime-testing/src/debugger/panel.rs` - DevTools panel
- `/home/user/code/ora/spacetime-testing/src/debugger/timeline_viz.rs` - Timeline UI
- `/home/user/code/ora/spacetime-testing/static/debugger/` - Frontend assets

**Implementation approach**:

1. **Runtime instrumentation**:
```javascript
// Inject into compiled output when debug mode enabled
window.__ST_DEBUG__ = {
  timelines: {},
  signals: {},
  states: {},

  onTimelineUpdate(id, progress) { ... },
  onSignalChange(el, name, value) { ... },
  onStateChange(el, from, to) { ... },
};
```

2. **Debugger panel UI**:
```html
<div id="st-debugger">
  <section id="timelines">
    <h3>Timelines</h3>
    <!-- Timeline progress bars -->
  </section>
  <section id="signals">
    <h3>Signals</h3>
    <!-- Signal name: value pairs -->
  </section>
  <section id="states">
    <h3>State Machines</h3>
    <!-- State diagrams -->
  </section>
</div>
```

**Dependencies**: None (can be developed in parallel)

**Testing**:
- Manual testing in browser
- E2E tests for debugger panel

**Complexity**: High

**Completion Notes**: Added visual debugger panel with `--debug` flag support. Injects `window.__ST_DEBUG__` runtime instrumentation for timeline progress, signal values, and state transitions. The debugger panel displays real-time visualization of timelines (progress bars), signals (name:value pairs), and state machines (current state + transitions).

---

## Dependency Graph

```
                                +----------------+
                                |   1.1 Fix      |
                                |   RwLock       |
                                +----------------+

+----------------+    +----------------+    +----------------+
|   1.2 JSON     |    |   1.3 Wire IR  |    |   1.4 Dead     |
|   Format       |    |   to Compile   |    |   Code         |
+----------------+    +-------+--------+    +----------------+
                              |
                              v
              +---------------+---------------+
              |                               |
              v                               v
      +----------------+              +----------------+
      |   2.3 Syntax   |              |   2.4 Source   |
      |   Validation   |              |   Maps         |
      +----------------+              +----------------+

+----------------+    +----------------+
|   2.1 Remove   |    |   3.1 LSP      |
|   Deprecated   |    |   Enhancement  |
|   Types        |    +----------------+
+-------+--------+
        |
        v
+----------------+    +----------------+    +----------------+
|   2.2 Remove   |    |   3.2 Static   |    |   3.3 Visual   |
|   Deprecated   |    |   Analysis     |    |   Debugger     |
|   Functions    |    +----------------+    +----------------+
+----------------+
```

## Parallelization Opportunities

**Can run in parallel:**
- 1.1, 1.2, 1.4 (all independent quick wins)
- 2.1 + 3.1 (parser cleanup and LSP enhancement are independent)
- 3.2 + 3.3 (static analysis and debugger are independent)

**Must be sequential:**
- 1.3 -> 2.3 (validation needs IR)
- 1.3 -> 2.4 (source maps need IR)
- 2.1 -> 2.2 (remove types before functions)

---

## How IR and Emit Layers Enable These Tasks

### Current Infrastructure

**IR Types** (`src/ir/code.rs`):
- `JsExpr`, `JsStmt` - Structured JavaScript AST
- `CssExpr`, `CssDecl` - Structured CSS rules
- `GlslExpr` - Shader programs
- `HtmlExpr` - Template elements
- `CodeFragment` - Code with metadata (dependencies)

**Emit Functions** (`src/emit/`):
- `js::emit()` - JsExpr/JsStmt -> JavaScript string
- `css::emit()` - CssExpr -> CSS string
- `EmitOptions` - Minification, formatting control

### Enabling Future Work

1. **Source Maps** (2.4): IR types can track source spans, emit layer can output mappings

2. **Syntax Validation** (2.3): After emit, validate the output strings

3. **Minification**: Emit layer already supports `EmitOptions::minified()`

4. **Optimization**: IR enables transformations before emit (dead code elimination, constant folding)

5. **Multiple Targets**: Same IR can emit to different formats (e.g., different JS module systems)

---

## Test Strategy Summary

| Task | Test Approach |
|------|---------------|
| 1.1 RwLock | `cargo build` - no warnings |
| 1.2 JSON Format | Integration test: parse JSON output |
| 1.3 Wire IR | Compare output before/after |
| 1.4 Dead Code | `cargo build` - no warnings |
| 2.1-2.2 Deprecation | Full test suite passes |
| 2.3 Validation | Unit tests with valid/invalid code |
| 2.4 Source Maps | Browser DevTools validation |
| 3.1 LSP | Manual testing + LSP test suite |
| 3.2 Static Analysis | Unit tests with edge cases |
| 3.3 Debugger | E2E browser tests |

---

## Estimated Timeline

| Phase | Tasks | Estimated Duration | Actual Status |
|-------|-------|-------------------|---------------|
| Phase 1 | 1.1-1.4 | 1-2 days | COMPLETED |
| Phase 2 | 2.1-2.4 | 1-2 weeks | COMPLETED |
| Phase 3 | 3.1-3.3 | 2-4 weeks | COMPLETED |

**Total estimated time: 4-6 weeks**

**Actual completion: All phases completed** - The implementation work covered all planned tasks including dead code cleanup (~680 lines removed), IR layer wiring, syntax validation, source map generation, LSP enhancements, static analysis, and the visual debugger panel.

---

## Next Immediate Actions

All planned tasks from Phases 1-3 have been completed. Future development may include:

1. Performance optimization of the compilation pipeline
2. Additional diagnostic codes for more edge cases
3. Extended debugger features (timeline scrubbing, breakpoints)
4. Documentation for new features (--format=json, --debug flags)
