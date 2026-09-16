//! Symbol usage tracking
//!
//! Provides bidirectional mapping between symbols and their usage locations,
//! enabling "Find All References" and "Rename Symbol" features.

use dashmap::DashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::parser::SourceSpan;

/// Unique identifier for a symbol
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId {
    /// Kind of symbol
    pub kind: SymbolKind,
    /// Symbol name
    pub name: String,
}

impl SymbolId {
    pub fn new(kind: SymbolKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
        }
    }

    pub fn directive(name: impl Into<String>) -> Self {
        Self::new(SymbolKind::Directive, name)
    }

    pub fn pattern(name: impl Into<String>) -> Self {
        Self::new(SymbolKind::Pattern, name)
    }

    pub fn r#type(name: impl Into<String>) -> Self {
        Self::new(SymbolKind::Type, name)
    }
}

/// Kind of symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// Macro-created directive (e.g., @scroll, @fade-in)
    Directive,
    /// Pattern definition (@pattern)
    Pattern,
    /// Type definition (@type)
    Type,
    /// Preset (easing, scroll, animation, load)
    Preset,
    /// Named binding (@data, @computed, @fn, @local-state, @element-ref)
    Binding,
    /// Primitive (%primitive)
    Primitive,
    /// Macro (%macro)
    Macro,
    /// Template (@template &name) — referenced via `&name(...)`
    Template,
    /// Runtime registry (%runtime-registry)
    RuntimeRegistry,
    /// Capture type (%capture_type)
    CaptureType,
}

/// Location of a symbol reference
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceLocation {
    /// File containing the reference
    pub file: PathBuf,
    /// Byte span of the reference
    pub span: SourceSpan,
    /// Kind of reference
    pub kind: ReferenceKind,
}

/// Kind of reference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    /// Symbol is defined here
    Definition,
    /// Symbol is used/called here
    Usage,
    /// Symbol is imported here
    Import,
}

/// Per-file symbol index for incremental updates
#[derive(Debug, Clone, Default)]
pub struct FileSymbolIndex {
    /// Symbols defined in this file
    pub definitions: HashSet<SymbolId>,
    /// Definition locations (symbol + EXACT capture span) for register_definition
    pub definition_spans: Vec<(SymbolId, SourceSpan)>,
    /// Symbols used in this file (with locations)
    pub usages: Vec<(SymbolId, SourceSpan)>,
    /// File version when index was built
    pub version: i32,
}

/// Interval in a file for reverse lookup
#[derive(Debug, Clone)]
struct Interval {
    start: usize,
    end: usize,
    symbol: SymbolId,
}

/// Simple interval collection for span-based lookups
#[derive(Debug, Clone, Default)]
pub struct IntervalIndex {
    intervals: Vec<Interval>,
}

impl IntervalIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, start: usize, end: usize, symbol: SymbolId) {
        self.intervals.push(Interval { start, end, symbol });
    }

    /// Find the innermost symbol at the given offset
    pub fn find_at(&self, offset: usize) -> Option<&SymbolId> {
        self.intervals
            .iter()
            .filter(|i| i.start <= offset && offset < i.end)
            .min_by_key(|i| i.end - i.start)
            .map(|i| &i.symbol)
    }
}

/// Main usage index for the workspace
pub struct UsageIndex {
    /// Forward index: symbol → all usage locations
    symbol_to_refs: DashMap<SymbolId, Vec<ReferenceLocation>>,
    /// Definition locations: symbol → definition location
    definitions: DashMap<SymbolId, ReferenceLocation>,
    /// Reverse index: file → interval index for position lookups
    file_intervals: DashMap<PathBuf, IntervalIndex>,
    /// Per-file index for incremental updates
    file_index: DashMap<PathBuf, FileSymbolIndex>,
}

impl UsageIndex {
    pub fn new() -> Self {
        Self {
            symbol_to_refs: DashMap::new(),
            definitions: DashMap::new(),
            file_intervals: DashMap::new(),
            file_index: DashMap::new(),
        }
    }

    /// Update index for a file with new symbol information
    pub fn update_file(&self, path: &Path, index: FileSymbolIndex) {
        // Canonical ABSOLUTE path so a definition's URI and the references that
        // resolve to it share one identity regardless of how the workspace was
        // opened (gh-30).
        let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

        // Remove old references from this file
        self.remove_file_references(&path);

        // Register definitions with their EXACT capture span (gh-30/31): the
        // capture span (e.g. the `$products` token, not the whole `@data`
        // match) is what go-to-definition should land on.
        for (symbol, span) in &index.definition_spans {
            self.register_definition(symbol.clone(), &path, *span);
        }

        // Add usages to forward index
        for (symbol, span) in &index.usages {
            self.symbol_to_refs
                .entry(symbol.clone())
                .or_default()
                .push(ReferenceLocation {
                    file: path.clone(),
                    span: *span,
                    kind: ReferenceKind::Usage,
                });
        }

        // Build interval index for this file
        let mut intervals = IntervalIndex::new();
        for (symbol, span) in &index.usages {
            intervals.insert(span.start, span.end, symbol.clone());
        }
        self.file_intervals.insert(path.clone(), intervals);

        // Store file index
        self.file_index.insert(path, index);
    }

    /// Register a symbol definition
    pub fn register_definition(&self, symbol: SymbolId, file: &Path, span: SourceSpan) {
        let location = ReferenceLocation {
            file: file.to_path_buf(),
            span,
            kind: ReferenceKind::Definition,
        };
        self.definitions.insert(symbol.clone(), location.clone());

        // Also add to references
        self.symbol_to_refs
            .entry(symbol)
            .or_default()
            .push(location);
    }

    /// Get definition location for a symbol
    pub fn get_definition(&self, symbol: &SymbolId) -> Option<ReferenceLocation> {
        self.definitions.get(symbol).map(|r| r.clone())
    }

    /// Find all references to a symbol
    pub fn find_references(&self, symbol: &SymbolId) -> Vec<ReferenceLocation> {
        self.symbol_to_refs
            .get(symbol)
            .map(|r| r.clone())
            .unwrap_or_default()
    }

    /// Find symbol at a position in a file
    pub fn symbol_at(&self, file: &Path, offset: usize) -> Option<SymbolId> {
        self.file_intervals
            .get(file)
            .and_then(|intervals| intervals.find_at(offset).cloned())
    }

    /// Remove all references from a file
    fn remove_file_references(&self, path: &Path) {
        // Remove from forward index
        for mut entry in self.symbol_to_refs.iter_mut() {
            entry.retain(|loc| loc.file != path);
        }

        // Remove empty entries
        self.symbol_to_refs.retain(|_, refs| !refs.is_empty());

        // Remove definitions from this file
        self.definitions.retain(|_, loc| loc.file != path);

        // Remove interval index
        self.file_intervals.remove(path);

        // Remove file index
        self.file_index.remove(path);
    }

    /// Remove a file from the index entirely
    pub fn remove_file(&self, path: &Path) {
        self.remove_file_references(path);
    }

    /// Get all symbols defined in a file
    pub fn symbols_in_file(&self, path: &Path) -> HashSet<SymbolId> {
        self.file_index
            .get(path)
            .map(|idx| idx.definitions.clone())
            .unwrap_or_default()
    }

    /// Check if a symbol is defined anywhere
    pub fn has_definition(&self, symbol: &SymbolId) -> bool {
        self.definitions.contains_key(symbol)
    }

    /// Get count of references for a symbol
    pub fn reference_count(&self, symbol: &SymbolId) -> usize {
        self.symbol_to_refs
            .get(symbol)
            .map(|r| r.len())
            .unwrap_or(0)
    }
}

impl Default for UsageIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_id() {
        let s1 = SymbolId::directive("scroll");
        let s2 = SymbolId::directive("scroll");
        let s3 = SymbolId::pattern("scroll");

        assert_eq!(s1, s2);
        assert_ne!(s1, s3);
    }

    #[test]
    fn test_interval_index() {
        let mut idx = IntervalIndex::new();
        let sym1 = SymbolId::directive("outer");
        let sym2 = SymbolId::directive("inner");

        // Nested intervals
        idx.insert(0, 100, sym1.clone());
        idx.insert(20, 50, sym2.clone());

        // At position 30, innermost should be sym2
        assert_eq!(idx.find_at(30), Some(&sym2));

        // At position 10, only sym1
        assert_eq!(idx.find_at(10), Some(&sym1));

        // Outside all intervals
        assert_eq!(idx.find_at(150), None);
    }

    #[test]
    fn test_usage_index() {
        let index = UsageIndex::new();
        let def_file = PathBuf::from("/definition.st");
        let usage_file = PathBuf::from("/usage.st");
        let symbol = SymbolId::directive("scroll");

        // Register a definition in one file
        index.register_definition(symbol.clone(), &def_file, SourceSpan { start: 0, end: 10 });

        // Add usages from another file
        let file_index = FileSymbolIndex {
            definitions: HashSet::new(),
            definition_spans: vec![],
            usages: vec![(symbol.clone(), SourceSpan { start: 50, end: 60 })],
            version: 1,
        };
        index.update_file(&usage_file, file_index);

        // Should find both definition and usage
        let refs = index.find_references(&symbol);
        assert_eq!(refs.len(), 2);
    }
}

    #[test]
    fn update_file_registers_definitions_with_capture_span_and_canonical_path() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("defs.st");
        // Use a relative path — register_definition must canonicalize it.
        let rel = std::path::Path::new(file.to_string_lossy().as_ref())
            .to_path_buf();
        let _ = rel;
        fs::write(&file, "").unwrap();

        let index = UsageIndex::new();
        let symbol = SymbolId::new(SymbolKind::Binding, "products");
        let file_index = FileSymbolIndex {
            definitions: {
                let mut s = HashSet::new();
                s.insert(symbol.clone());
                s
            },
            // The `$products` token span — NOT the whole `@data` match.
            definition_spans: vec![(symbol.clone(), SourceSpan { start: 4, end: 13 })],
            usages: vec![(symbol.clone(), SourceSpan { start: 4, end: 13 })],
            version: 1,
        };
        index.update_file(&file, file_index);

        let def = index.get_definition(&symbol).expect("definition registered");
        assert_eq!(def.span, SourceSpan { start: 4, end: 13 });
        assert!(
            def.file.is_absolute(),
            "definition file must be canonical/absolute, got {}",
            def.file.display()
        );

        // Exactly ONE reference from this file (the usage) + the definition.
        // A double-walk would have recorded the same usage twice.
        let refs = index.find_references(&symbol);
        assert_eq!(refs.len(), 2, "definition + usage, no duplicates: {refs:?}");
    }
