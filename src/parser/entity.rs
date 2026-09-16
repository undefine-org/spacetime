//! Spatial-ECS entity + facet-path model (FEAT-142 / PLAN-061).
//!
//! An **entity** is a `&name { @component... }` scope: identity is the `&name`,
//! behaviour is the stacked `@directive` components (Wave A). A **facet path** is
//! member-access on an entity reference, `&entity.component.facet(.value)*`:
//!
//! ```text
//! &evernet . peak . summit . y
//! └──┬───┘  └─┬─┘  └──┬──┘  └┬┘
//!  ENTITY   COMP    FACET   VALUE
//! ```
//!
//! - ENTITY / COMPONENT are resolved at COMPILE time against the world registry
//!   (`window.__stWorld.byName`) + the component's published `%exports`.
//! - FACET is a published reactive signal.
//! - any trailing segments are runtime value access on the facet's value.
//!
//! This module is the SINGLE typed home for parsing + classifying that path from
//! the flat `CapturedValue::Element("evernet.peak.summit")` string (Wave B keeps
//! it flat, mirroring `Binding("$data.items")`; this is the one place that splits
//! it — revB guidance: never scatter `split('.')`).

/// The synthetic per-entity marker CLASS selector for `&name { … }`.
///
/// A CLASS (not a `[data-…]` attribute) because a LEADING attribute selector does
/// not emit directive JS in the selector-init path; the interior component
/// directives bind to this class. Entity names are grammar `ident`s, so the class
/// token is always valid.
pub fn entity_selector(name: &str) -> String {
    format!(".st-entity-{}", name)
}

/// Recover an entity name from its synthetic marker-class selector, or `None` if
/// the selector is not an entity marker.
pub fn entity_name_from_selector(selector: &str) -> Option<&str> {
    selector.strip_prefix(".st-entity-")
}

/// A parsed `&entity.component.facet(.value…)` reference.
///
/// Built from the flat dotted string a Wave-B `CapturedValue::Element` carries
/// (the leading `&` is already stripped by the extractor). The classification is
/// purely structural (segment arity); TYPE checking against the component's
/// published facets is layered on top by the resolver (Wave C diagnostics) and
/// the reactive emit is Wave D.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetPath {
    /// The entity name (first segment) — a registry key in `__stWorld.byName`.
    pub entity: String,
    /// The component namespace (second segment), e.g. `peak`. `None` for a bare
    /// `&entity` reference (the entity's default facet, resolved later).
    pub component: Option<String>,
    /// The facet name (third segment), e.g. `summit`. `None` when only
    /// `&entity` or `&entity.component` was written.
    pub facet: Option<String>,
    /// Any trailing value-access segments (`.y`, `.x`) beyond the facet — runtime
    /// member access on the facet's value. Empty for the common case.
    pub tail: Vec<String>,
}

impl FacetPath {
    /// Parse a flat dotted element string (`"evernet.peak.summit"`, `"evernet"`,
    /// `"evernet.peak.summit.y"`) into its segments. Returns `None` only for an
    /// empty string (never produced by the extractor). Whitespace-free by
    /// construction (segments are idents), so the split is unambiguous.
    pub fn parse(dotted: &str) -> Option<FacetPath> {
        let mut segs = dotted.split('.').map(|s| s.to_string());
        let entity = segs.next()?;
        if entity.is_empty() {
            return None;
        }
        let component = segs.next();
        let facet = segs.next();
        let tail: Vec<String> = segs.collect();
        Some(FacetPath {
            entity,
            component,
            facet,
            tail,
        })
    }

    /// Whether this reference is a bare `&entity` (no dotted segments) — i.e. a
    /// plain element reference, NOT a facet path. Single-segment `Element`s must
    /// keep their existing behaviour (`ST.ref(name)`); only DOTTED refs route to
    /// the facet resolver.
    pub fn is_bare_entity(&self) -> bool {
        self.component.is_none()
    }

    /// The number of dotted segments after the entity (`&e` → 0, `&e.c` → 1,
    /// `&e.c.f` → 2, `&e.c.f.v` → 3+).
    pub fn depth(&self) -> usize {
        self.component.is_some() as usize + self.facet.is_some() as usize + self.tail.len()
    }
}

/// Whether a flat `Element` capture string is a DOTTED facet path (has ≥1 `.`),
/// as opposed to a plain single-segment element reference. The seam that decides
/// whether a capture routes to the facet resolver (dotted) or stays a plain
/// `ST.ref` element reference (bare) — revB: reject dotted in a plain-element
/// context, keep bare unchanged.
pub fn is_facet_path(element_capture: &str) -> bool {
    element_capture.contains('.')
}

/// Emit the JS expression that RESOLVES a dotted facet-path capture against the
/// spatial world registry: `ST.worldFacet(entity, component, facet)`, plus any
/// value-access `tail` as safe optional member access (`?.y`). This is the SINGLE
/// emitter every consumer (derive lowering, primitive-arg lowering) routes a
/// dotted `Element` through, so the facet-read JS is defined in exactly one place
/// (revC P1/P2-1: all seams agree, tail preserved).
///
/// `dotted` is the flat capture string (`"evernet.peak.summit"`,
/// `"evernet.peak.summit.y"`). Returns `"null"` for an unparseable string.
pub fn emit_facet_read_js(dotted: &str) -> String {
    match FacetPath::parse(dotted) {
        Some(fp) => {
            let base = format!(
                "ST.worldFacet({:?}, {:?}, {:?})",
                fp.entity,
                fp.component.as_deref().unwrap_or(""),
                fp.facet.as_deref().unwrap_or(""),
            );
            if fp.tail.is_empty() {
                base
            } else {
                // Value-access tail (`.y`, `.x`): optional member access so a null
                // facet (unresolved entity) stays null rather than throwing.
                let mut expr = base;
                for seg in &fp.tail {
                    expr.push_str("?.");
                    expr.push_str(seg);
                }
                expr
            }
        }
        None => "null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_roundtrip() {
        assert_eq!(entity_selector("evernet"), ".st-entity-evernet");
        assert_eq!(
            entity_name_from_selector(".st-entity-evernet"),
            Some("evernet")
        );
        assert_eq!(entity_name_from_selector(".other"), None);
    }

    #[test]
    fn parse_bare_entity() {
        let p = FacetPath::parse("evernet").unwrap();
        assert_eq!(p.entity, "evernet");
        assert_eq!(p.component, None);
        assert_eq!(p.facet, None);
        assert!(p.tail.is_empty());
        assert!(p.is_bare_entity());
        assert_eq!(p.depth(), 0);
        assert!(!is_facet_path("evernet"));
    }

    #[test]
    fn parse_entity_component() {
        let p = FacetPath::parse("evernet.peak").unwrap();
        assert_eq!(p.entity, "evernet");
        assert_eq!(p.component.as_deref(), Some("peak"));
        assert_eq!(p.facet, None);
        assert!(!p.is_bare_entity());
        assert_eq!(p.depth(), 1);
        assert!(is_facet_path("evernet.peak"));
    }

    #[test]
    fn parse_full_facet_path() {
        let p = FacetPath::parse("evernet.peak.summit").unwrap();
        assert_eq!(p.entity, "evernet");
        assert_eq!(p.component.as_deref(), Some("peak"));
        assert_eq!(p.facet.as_deref(), Some("summit"));
        assert!(p.tail.is_empty());
        assert_eq!(p.depth(), 2);
        assert!(is_facet_path("evernet.peak.summit"));
    }

    #[test]
    fn parse_value_access_tail() {
        let p = FacetPath::parse("evernet.peak.summit.y").unwrap();
        assert_eq!(p.entity, "evernet");
        assert_eq!(p.component.as_deref(), Some("peak"));
        assert_eq!(p.facet.as_deref(), Some("summit"));
        assert_eq!(p.tail, vec!["y".to_string()]);
        assert_eq!(p.depth(), 3);
    }

    #[test]
    fn parse_empty_is_none() {
        assert_eq!(FacetPath::parse(""), None);
    }

    #[test]
    fn emit_facet_read_shapes() {
        // FEAT-142 WAVE C (revC): the single facet-read emitter. entity.component.facet
        // -> ST.worldFacet; a value-access tail becomes safe optional member access.
        assert_eq!(
            emit_facet_read_js("evernet.peak.summit"),
            r#"ST.worldFacet("evernet", "peak", "summit")"#
        );
        assert_eq!(
            emit_facet_read_js("evernet.peak"),
            r#"ST.worldFacet("evernet", "peak", "")"#
        );
        // Value-access tail (`.y`) is preserved as optional member access (revC P2-1).
        assert_eq!(
            emit_facet_read_js("evernet.peak.summit.y"),
            r#"ST.worldFacet("evernet", "peak", "summit")?.y"#
        );
        assert_eq!(
            emit_facet_read_js("evernet.peak.summit.pos.x"),
            r#"ST.worldFacet("evernet", "peak", "summit")?.pos?.x"#
        );
    }
}
