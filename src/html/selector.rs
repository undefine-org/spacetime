//! Selector matching with template expansion and similarity detection.
//!
//! Provides intelligent selector validation that understands custom elements
//! and their template expansions, plus "did you mean?" suggestions.

use super::model::HtmlContext;

/// Result of matching a selector against HTML context.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectorMatch {
    /// Selector matches directly in the document
    Direct,
    /// Selector matches inside a template's shadow DOM
    Template(String),
    /// Selector matches after template expansion (internal class of used component)
    Expanded(String),
    /// Selector doesn't match, but similar selectors were found
    NotFound(Vec<String>),
    /// Selector doesn't match and no similar selectors exist
    NoMatch,
}

impl SelectorMatch {
    /// Returns true if the selector matched in any way
    pub fn is_match(&self) -> bool {
        matches!(
            self,
            SelectorMatch::Direct | SelectorMatch::Template(_) | SelectorMatch::Expanded(_)
        )
    }

    /// Get similar selectors if no match was found
    pub fn suggestions(&self) -> Option<&[String]> {
        match self {
            SelectorMatch::NotFound(sug) => Some(sug),
            _ => None,
        }
    }
}

impl HtmlContext {
    /// Check if a selector matches any element in the context.
    ///
    /// This performs template-aware matching:
    /// 1. First checks direct matches in the document
    /// 2. Then checks if selector matches template internal classes/IDs
    /// 3. Returns suggestions if no match found
    pub fn matches(&self, selector: &str) -> SelectorMatch {
        let selector = selector.trim();

        // Handle comma-separated selector lists: `.a, .b` — match if ANY part matches
        if selector.contains(',') {
            for part in selector.split(',') {
                let part = part.trim();
                if !part.is_empty() {
                    let result = self.matches(part);
                    if result.is_match() {
                        return result;
                    }
                }
            }
            // None matched — collect suggestions from the full selector
            let similar = self.find_similar(selector);
            return if similar.is_empty() {
                SelectorMatch::NoMatch
            } else {
                SelectorMatch::NotFound(similar)
            };
        }

        // Handle compound selectors - check the final part
        let final_part = extract_final_selector(selector);

        // 1. Check direct match in document
        if self.selector_matches_document(final_part) {
            return SelectorMatch::Direct;
        }

        // 2. Check if it matches any template's internal structure
        for (name, template) in &self.templates.components {
            if self.selector_matches_template(final_part, template) {
                // Check if this template is actually used in the document
                if self.document.custom_elements.contains(name) {
                    return SelectorMatch::Expanded(name.clone());
                } else {
                    return SelectorMatch::Template(name.clone());
                }
            }
        }

        // 3. No match - find similar selectors
        let similar = self.find_similar(selector);
        if similar.is_empty() {
            SelectorMatch::NoMatch
        } else {
            SelectorMatch::NotFound(similar)
        }
    }

    /// Check if a scoped selector matches within a parent scope.
    ///
    /// This is used for validating selectors inside @scroll, @on, etc. blocks.
    pub fn matches_in_scope(&self, selector: &str, parent: &str) -> SelectorMatch {
        // First verify the parent scope exists
        if !self.matches(parent).is_match() {
            return SelectorMatch::NoMatch;
        }

        // For now, just check if the child selector exists anywhere
        // A more sophisticated implementation would verify hierarchy
        self.matches(selector)
    }

    /// Check if selector matches document directly
    fn selector_matches_document(&self, selector: &str) -> bool {
        super::parser::selector_exists(&self.document, selector)
    }

    /// Check if selector matches a template's internal structure
    fn selector_matches_template(
        &self,
        selector: &str,
        template: &super::model::ComponentTemplate,
    ) -> bool {
        // ID selector
        if let Some(id) = selector.strip_prefix('#') {
            return template.internal_ids.contains(id);
        }

        // Class selector
        if selector.starts_with('.') {
            let classes: Vec<&str> = selector[1..].split('.').filter(|s| !s.is_empty()).collect();
            return classes
                .iter()
                .all(|c| template.internal_classes.contains(*c));
        }

        // Tag selector (including nested custom elements)
        if !selector.contains('[') && !selector.contains(':') && !selector.contains('.') {
            return template.nested_components.contains(selector);
        }

        false
    }

    /// Find selectors similar to the given one.
    ///
    /// Uses Levenshtein distance to suggest typo fixes.
    fn find_similar(&self, selector: &str) -> Vec<String> {
        let final_part = extract_final_selector(selector);
        let candidates = self.collect_all_selectors();

        find_similar_selector(final_part, &candidates, 3)
    }

    /// Collect all possible selectors from document and templates
    fn collect_all_selectors(&self) -> Vec<String> {
        let mut selectors = Vec::new();

        // From document
        for id in &self.document.ids {
            selectors.push(format!("#{}", id));
        }
        for class in &self.document.classes {
            selectors.push(format!(".{}", class));
        }
        for tag in &self.document.tags {
            selectors.push(tag.clone());
        }

        // From templates (for expanded matching)
        for template in self.templates.components.values() {
            for id in &template.internal_ids {
                selectors.push(format!("#{}", id));
            }
            for class in &template.internal_classes {
                selectors.push(format!(".{}", class));
            }
        }

        selectors
    }
}

/// Extract the final (rightmost) part of a compound selector.
///
/// Examples:
/// - `.parent .child` -> `.child`
/// - `.parent > .child` -> `.child`
/// - `.single` -> `.single`
fn extract_final_selector(selector: &str) -> &str {
    // Handle descendant combinator
    if let Some(pos) = selector.rfind(' ') {
        let candidate = selector[pos + 1..].trim();
        if !candidate.is_empty() {
            return extract_final_selector(candidate);
        }
    }

    // Handle child combinator
    if let Some(pos) = selector.rfind('>') {
        let candidate = selector[pos + 1..].trim();
        if !candidate.is_empty() {
            return extract_final_selector(candidate);
        }
    }

    selector.trim()
}

/// Find selectors similar to the target using Levenshtein distance.
///
/// Returns up to `max_results` suggestions, sorted by similarity.
pub fn find_similar_selector(
    target: &str,
    candidates: &[String],
    max_results: usize,
) -> Vec<String> {
    let target_normalized = normalize_selector(target);

    let mut scored: Vec<(String, usize)> = candidates
        .iter()
        .filter_map(|c| {
            let normalized = normalize_selector(c);
            let distance = levenshtein_distance(&target_normalized, &normalized);
            // Only include if reasonably similar (within 50% of target length)
            let threshold = (target_normalized.len() / 2).max(3);
            if distance <= threshold {
                Some((c.clone(), distance))
            } else {
                None
            }
        })
        .collect();

    scored.sort_by_key(|(_, d)| *d);
    scored.truncate(max_results);
    scored.into_iter().map(|(s, _)| s).collect()
}

/// Normalize a selector for comparison (remove prefix, lowercase)
fn normalize_selector(selector: &str) -> String {
    selector
        .trim()
        .trim_start_matches('#')
        .trim_start_matches('.')
        .to_lowercase()
}

/// Calculate Levenshtein (edit) distance between two strings.
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::{parse_html, parse_templates};

    fn create_test_context() -> HtmlContext {
        let index = r#"
            <div id="app" class="container">
                <ik-projects>
                    <div class="ik-projects__header"></div>
                </ik-projects>
            </div>
        "#;

        let templates = r#"
            <template data-component="ik-projects">
                <section class="ik-projects">
                    <div class="ik-projects__grid">
                        <slot></slot>
                    </div>
                </section>
            </template>
        "#;

        HtmlContext {
            document: parse_html(index).unwrap(),
            templates: parse_templates(templates).unwrap(),
            source_path: "test.html".to_string(),
        }
    }

    #[test]
    fn test_direct_match() {
        let ctx = create_test_context();

        assert!(ctx.matches(".container").is_match());
        assert!(ctx.matches("#app").is_match());
        assert!(ctx.matches("div").is_match());
    }

    #[test]
    fn test_template_expanded_match() {
        let ctx = create_test_context();

        // .ik-projects__grid is internal to the template, but ik-projects is used
        let result = ctx.matches(".ik-projects__grid");
        assert!(result.is_match());
        assert!(matches!(result, SelectorMatch::Expanded(_)));
    }

    #[test]
    fn test_compound_selector() {
        let ctx = create_test_context();

        // Should match the final part
        assert!(ctx.matches(".container .ik-projects__header").is_match());
    }

    #[test]
    fn test_no_match_with_suggestions() {
        let ctx = create_test_context();

        let result = ctx.matches(".ik-projects__gird"); // typo
        assert!(!result.is_match());

        if let SelectorMatch::NotFound(suggestions) = result {
            assert!(suggestions.contains(&".ik-projects__grid".to_string()));
        } else {
            panic!("Expected NotFound with suggestions");
        }
    }

    #[test]
    fn test_extract_final_selector() {
        assert_eq!(extract_final_selector(".parent .child"), ".child");
        assert_eq!(extract_final_selector(".parent > .child"), ".child");
        assert_eq!(extract_final_selector(".single"), ".single");
        assert_eq!(extract_final_selector(".a .b > .c .d"), ".d");
    }

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("grid", "gird"), 2);
    }

    #[test]
    fn test_find_similar() {
        let candidates = vec![
            ".ik-projects__grid".to_string(),
            ".ik-projects__header".to_string(),
            ".container".to_string(),
        ];

        let similar = find_similar_selector(".ik-projects__gird", &candidates, 3);
        assert!(!similar.is_empty());
        assert_eq!(similar[0], ".ik-projects__grid");
    }

    #[test]
    fn test_comma_separated_selectors() {
        let ctx = create_test_context();

        // Both parts exist
        assert!(ctx.matches(".container, #app").is_match());
        // First part exists
        assert!(ctx.matches(".container, .nonexistent").is_match());
        // Second part exists
        assert!(ctx.matches(".nonexistent, .container").is_match());
        // Neither exists
        assert!(!ctx.matches(".foo, .bar").is_match());
    }

    #[test]
    fn test_pseudo_class_selectors() {
        let ctx = create_test_context();

        // :first-child on a class that exists
        assert!(ctx.matches(".container:first-child").is_match());
        // :hover on a class that exists
        assert!(ctx.matches(".container:hover").is_match());
        // Comma-separated with pseudo-classes
        assert!(
            ctx.matches(".container:first-child, .ik-projects__header:last-child")
                .is_match()
        );
    }
}
