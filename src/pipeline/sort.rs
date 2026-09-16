//! Sort Layer (Layer 3)
//!
//! Sorts ResolvedPrimitives by (phase, order).
//!
//! This layer ensures that:
//! 1. Global phase primitives run before Selector phase
//! 2. Within each phase, primitives are ordered by their `order` field
//! 3. Primitives with the same (phase, order) maintain their relative order (stable sort)

use super::types::ResolvedPrimitive;

// =============================================================================
// Sorting Logic
// =============================================================================

/// Sort primitives by (phase, order)
///
/// This uses a stable sort to ensure deterministic output when primitives
/// have the same (phase, order) tuple.
pub fn sort(mut primitives: Vec<ResolvedPrimitive>) -> Vec<ResolvedPrimitive> {
    primitives.sort_by(|a, b| {
        // First compare by phase
        match a.phase.cmp(&b.phase) {
            std::cmp::Ordering::Equal => {
                // If phases are equal, compare by order
                a.order.cmp(&b.order)
            }
            other => other,
        }
    });

    primitives
}

/// Sort primitives in-place by (phase, order)
pub fn sort_in_place(primitives: &mut [ResolvedPrimitive]) {
    primitives.sort_by(|a, b| match a.phase.cmp(&b.phase) {
        std::cmp::Ordering::Equal => a.order.cmp(&b.order),
        other => other,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::types::{BoundArgs, Phase};

    fn make_primitive(phase: Phase, order: u32, name: &str) -> ResolvedPrimitive {
        ResolvedPrimitive::new(name, BoundArgs::new(), phase, order)
    }

    #[test]
    fn test_sort_empty() {
        let primitives = Vec::new();
        let sorted = sort(primitives);
        assert_eq!(sorted.len(), 0);
    }

    #[test]
    fn test_sort_single() {
        let primitives = vec![make_primitive(Phase::Global, 0, "test")];
        let sorted = sort(primitives);
        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0].primitive_name, "test");
    }

    #[test]
    fn test_sort_by_phase() {
        let primitives = vec![
            make_primitive(Phase::Selector, 0, "selector1"),
            make_primitive(Phase::Global, 0, "global1"),
            make_primitive(Phase::Selector, 0, "selector2"),
            make_primitive(Phase::Global, 0, "global2"),
        ];

        let sorted = sort(primitives);

        // All Global phase should come before Selector phase
        assert_eq!(sorted[0].primitive_name, "global1");
        assert_eq!(sorted[0].phase, Phase::Global);
        assert_eq!(sorted[1].primitive_name, "global2");
        assert_eq!(sorted[1].phase, Phase::Global);
        assert_eq!(sorted[2].primitive_name, "selector1");
        assert_eq!(sorted[2].phase, Phase::Selector);
        assert_eq!(sorted[3].primitive_name, "selector2");
        assert_eq!(sorted[3].phase, Phase::Selector);
    }

    #[test]
    fn test_sort_by_order() {
        let primitives = vec![
            make_primitive(Phase::Global, 3, "third"),
            make_primitive(Phase::Global, 1, "first"),
            make_primitive(Phase::Global, 2, "second"),
        ];

        let sorted = sort(primitives);

        assert_eq!(sorted[0].primitive_name, "first");
        assert_eq!(sorted[0].order, 1);
        assert_eq!(sorted[1].primitive_name, "second");
        assert_eq!(sorted[1].order, 2);
        assert_eq!(sorted[2].primitive_name, "third");
        assert_eq!(sorted[2].order, 3);
    }

    #[test]
    fn test_sort_by_phase_and_order() {
        let primitives = vec![
            make_primitive(Phase::Selector, 2, "selector-2"),
            make_primitive(Phase::Global, 2, "global-2"),
            make_primitive(Phase::Selector, 1, "selector-1"),
            make_primitive(Phase::Global, 1, "global-1"),
            make_primitive(Phase::Selector, 3, "selector-3"),
            make_primitive(Phase::Global, 3, "global-3"),
        ];

        let sorted = sort(primitives);

        // Should be sorted by phase first, then order
        assert_eq!(sorted[0].primitive_name, "global-1");
        assert_eq!(sorted[0].phase, Phase::Global);
        assert_eq!(sorted[0].order, 1);

        assert_eq!(sorted[1].primitive_name, "global-2");
        assert_eq!(sorted[1].phase, Phase::Global);
        assert_eq!(sorted[1].order, 2);

        assert_eq!(sorted[2].primitive_name, "global-3");
        assert_eq!(sorted[2].phase, Phase::Global);
        assert_eq!(sorted[2].order, 3);

        assert_eq!(sorted[3].primitive_name, "selector-1");
        assert_eq!(sorted[3].phase, Phase::Selector);
        assert_eq!(sorted[3].order, 1);

        assert_eq!(sorted[4].primitive_name, "selector-2");
        assert_eq!(sorted[4].phase, Phase::Selector);
        assert_eq!(sorted[4].order, 2);

        assert_eq!(sorted[5].primitive_name, "selector-3");
        assert_eq!(sorted[5].phase, Phase::Selector);
        assert_eq!(sorted[5].order, 3);
    }

    #[test]
    fn test_sort_stable() {
        // Test that sort is stable - primitives with same (phase, order) keep relative order
        let primitives = vec![
            make_primitive(Phase::Global, 0, "first"),
            make_primitive(Phase::Global, 0, "second"),
            make_primitive(Phase::Global, 0, "third"),
        ];

        let sorted = sort(primitives);

        assert_eq!(sorted[0].primitive_name, "first");
        assert_eq!(sorted[1].primitive_name, "second");
        assert_eq!(sorted[2].primitive_name, "third");
    }

    #[test]
    fn test_sort_in_place() {
        let mut primitives = vec![
            make_primitive(Phase::Selector, 2, "selector-2"),
            make_primitive(Phase::Global, 1, "global-1"),
            make_primitive(Phase::Selector, 1, "selector-1"),
        ];

        sort_in_place(&mut primitives);

        assert_eq!(primitives[0].primitive_name, "global-1");
        assert_eq!(primitives[1].primitive_name, "selector-1");
        assert_eq!(primitives[2].primitive_name, "selector-2");
    }
}
