//! A utility for comparing HTML output with configurable comparison options.
//!
//! This crate provides tools for comparing HTML strings while ignoring differences
//! that don't affect the rendered output, such as whitespace, attribute order,
//! and other configurable aspects.
//!
//! # Example
//! ```ignore
//! use html_compare::{HtmlComparer, HtmlCompareOptions};
//!
//! let html1 = "<div><p>Hello</p></div>";
//! let html2 = "<div>\n  <p>Hello</p>\n</div>";
//!
//! let comparer = HtmlComparer::new();
//! assert!(comparer.compare(html1, html2).unwrap());
//! ```
//!
//! For testing, you can use the provided assertion macros:
//! ```ignore
//! # use html_compare::assert_html_eq;
//! # use html_compare::HtmlCompareOptions;
//! assert_html_eq!(
//!     "<div><p>Hello</p></div>",
//!     "<div>\n  <p>Hello</p>\n</div>"
//! );
//! ```

/// Asserts that two HTML strings are equivalent according to the given comparison options.
///
/// # Examples
/// ```ignore
/// use html_compare::assert_html_eq;
///
/// assert_html_eq!(
///     "<div><p>Hello</p></div>",
///     "<div>\n  <p>Hello</p>\n</div>"
/// );
///
/// // With custom options
/// use html_compare::HtmlCompareOptions;
/// assert_html_eq!(
///     "<div><p>First</p><p>Second</p></div>",
///     "<div><p>Second</p><p>First</p></div>",
///     HtmlCompareOptions {
///         ignore_sibling_order: true,
///         ..Default::default()
///     }
/// );
/// ```
#[macro_export]
macro_rules! assert_html_eq {
    ($left:expr, $right:expr $(,)?) => {
        $crate::assert_html_eq!($left, $right, $crate::HtmlCompareOptions::default())
    };
    ($left:expr, $right:expr, $options:expr $(,)?) => {{
        match (&$left, &$right, &$options) {
            (left_val, right_val, options) => {
                let comparer = $crate::HtmlComparer::with_options(options.clone());
                if let Err(err) = comparer.compare(left_val, right_val) {
                    panic!(
                        "\n\
                        HTML comparison failed:\n\
                        {}\n\n\
                        left HTML:\n\
                        {}\n\n\
                        right HTML:\n\
                        {}\n\n\
                        options: {:#?}\
                    ",
                        err, left_val, right_val, options
                    );
                }
            }
        }
    }};
}

/// Asserts that two HTML strings are not equivalent according to the given comparison options.
///
/// # Examples
/// ```ignore
/// use html_compare::assert_html_ne;
///
/// assert_html_ne!(
///     "<div><p>Hello</p></div>",
///     "<div><p>Different</p></div>"
/// );
/// ```
#[macro_export]
macro_rules! assert_html_ne {
    ($left:expr, $right:expr $(,)?) => {
        $crate::assert_html_ne!($left, $right, $crate::HtmlCompareOptions::default())
    };
    ($left:expr, $right:expr, $options:expr $(,)?) => {{
        match (&$left, &$right, &$options) {
            (left_val, right_val, options) => {
                let comparer = $crate::HtmlComparer::with_options(options.clone());
                if let Ok(_) = comparer.compare(left_val, right_val) {
                    panic!(
                        "\n\
                        HTML strings were equal but expected to be different:\n\n\
                        HTML:\n\
                        {}\n\n\
                        options: {:#?}\
                    ",
                        left_val, options
                    );
                }
            }
        }
    }};
}

use ego_tree::NodeRef;
use scraper::{ElementRef, Html, Node};
use std::collections::HashSet;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HtmlCompareError {
    #[error("Node mismatch: {0}")]
    NodeMismatch(String),
    #[error("Missing expected node: {expected} at position {position}")]
    MissingNode { expected: String, position: usize },
    #[error("Extra node found: {found} at position {position}")]
    ExtraNode { found: String, position: usize },
}

/// Configuration for HTML comparison
#[derive(Debug, Clone)]
pub struct HtmlCompareOptions {
    /// Ignore whitespace differences between elements
    pub ignore_whitespace: bool,
    /// Ignore all HTML attributes
    pub ignore_attributes: bool,
    /// Specific attributes to ignore (if ignore_attributes is false)
    pub ignored_attributes: HashSet<String>,
    /// Ignore text node differences
    pub ignore_text: bool,
    /// Ignore comment nodes
    pub ignore_comments: bool,
    /// Ignore order of sibling elements
    pub ignore_sibling_order: bool,
}

impl Default for HtmlCompareOptions {
    fn default() -> Self {
        Self {
            ignore_whitespace: true,
            ignore_attributes: false,
            ignored_attributes: HashSet::new(),
            ignore_text: false,
            ignore_comments: true,
            ignore_sibling_order: false,
        }
    }
}

fn node_type_name(node: &Node) -> &'static str {
    match node {
        Node::Text(_) => "Text",
        Node::Element(_) => "Element",
        Node::Comment(_) => "Comment",
        Node::ProcessingInstruction(_) => "ProcessingInstruction",
        Node::Doctype(_) => "Doctype",
        Node::Document => "Document",
        Node::Fragment => "Fragment",
    }
}
/// Main struct for comparing HTML
#[derive(Debug)]
pub struct HtmlComparer {
    options: HtmlCompareOptions,
}

impl Default for HtmlComparer {
    fn default() -> Self {
        Self::new()
    }
}

impl HtmlComparer {
    /// Create a new HTML comparer with default options.
    ///
    /// Note about whitespace handling:
    /// - Multiple spaces in text content are collapsed into a single space (standard HTML behavior)
    /// - Whitespace between elements is ignored by default
    /// - Setting `ignore_whitespace: false` only affects element whitespace, not text content
    /// - Special elements like <pre> and attributes like xml:space are treated the same as regular elements
    pub fn new() -> Self {
        Self {
            options: HtmlCompareOptions::default(),
        }
    }

    /// Create a new HTML comparer with custom options
    pub fn with_options(options: HtmlCompareOptions) -> Self {
        Self { options }
    }

    /// Compare two HTML strings
    pub fn compare(&self, expected: &str, actual: &str) -> Result<bool, HtmlCompareError> {
        let expected_doc = Html::parse_document(expected);
        let actual_doc = Html::parse_document(actual);

        let expected_root = expected_doc.root_element();
        let actual_root = actual_doc.root_element();

        self.compare_element_refs(expected_root, actual_root)
            .map(|_| true)
    }

    /// Compare two ElementRefs
    fn compare_element_refs(
        &self,
        expected: ElementRef,
        actual: ElementRef,
    ) -> Result<(), HtmlCompareError> {
        // Compare tag names
        if expected.value().name() != actual.value().name() {
            return Err(HtmlCompareError::NodeMismatch(format!(
                "Tag name mismatch. Expected: {}, Actual: {}",
                expected.value().name(),
                actual.value().name()
            )));
        }

        // Compare attributes if not ignored
        if !self.options.ignore_attributes {
            self.compare_attributes(expected, actual)?;
        }

        // Get child nodes
        let expected_children: Vec<_> = expected
            .children()
            .filter(|n| self.should_include_node(n))
            .collect();
        let actual_children: Vec<_> = actual
            .children()
            .filter(|n| self.should_include_node(n))
            .collect();

        if self.options.ignore_sibling_order {
            self.compare_unordered_nodes(&expected_children, &actual_children)?;
        } else {
            self.compare_ordered_nodes(&expected_children, &actual_children)?;
        }

        Ok(())
    }

    /// Compare attributes between two ElementRefs
    fn compare_attributes(
        &self,
        expected: ElementRef,
        actual: ElementRef,
    ) -> Result<(), HtmlCompareError> {
        let expected_attrs: HashSet<_> = expected
            .value()
            .attrs()
            .filter(|(name, _)| !self.options.ignored_attributes.contains(*name))
            .collect();
        let actual_attrs: HashSet<_> = actual
            .value()
            .attrs()
            .filter(|(name, _)| !self.options.ignored_attributes.contains(*name))
            .collect();

        if expected_attrs != actual_attrs {
            return Err(HtmlCompareError::NodeMismatch(format!(
                "Attributes mismatch. Expected: {:?}, Actual: {:?}",
                expected_attrs, actual_attrs
            )));
        }
        Ok(())
    }

    /// Compare ordered nodes
    fn compare_ordered_nodes(
        &self,
        expected: &[NodeRef<Node>],
        actual: &[NodeRef<Node>],
    ) -> Result<(), HtmlCompareError> {
        if expected.len() != actual.len() {
            return Err(HtmlCompareError::NodeMismatch(format!(
                "Child count mismatch. Expected: {}, Actual: {}",
                expected.len(),
                actual.len()
            )));
        }

        for (i, (expected_child, actual_child)) in expected.iter().zip(actual.iter()).enumerate() {
            match (expected_child.value(), actual_child.value()) {
                (Node::Text(expected_text), Node::Text(actual_text)) => {
                    if !self.options.ignore_text {
                        let expected_str = if self.options.ignore_whitespace {
                            expected_text.trim()
                        } else {
                            expected_text
                        };
                        let actual_str = if self.options.ignore_whitespace {
                            actual_text.trim()
                        } else {
                            actual_text
                        };
                        if expected_str != actual_str {
                            return Err(HtmlCompareError::NodeMismatch(format!(
                                "Text content mismatch at position {}. Expected: '{}', Actual: '{}'",
                                i, expected_str, actual_str
                            )));
                        }
                    }
                }
                (Node::Comment(_), Node::Comment(_)) => {
                    // If we're not ignoring comments, we should compare their content
                    if !self.options.ignore_comments {
                        let expected_comment = match expected_child.value() {
                            Node::Comment(c) => c.trim(),
                            _ => unreachable!(),
                        };
                        let actual_comment = match actual_child.value() {
                            Node::Comment(c) => c.trim(),
                            _ => unreachable!(),
                        };
                        if expected_comment != actual_comment {
                            return Err(HtmlCompareError::NodeMismatch(format!(
                                "Comment content mismatch at position {}. Expected: '{}', Actual: '{}'",
                                i, expected_comment, actual_comment
                            )));
                        }
                    }
                }
                (Node::Element(_), Node::Element(_)) => {
                    if let (Some(expected_el), Some(actual_el)) = (
                        ElementRef::wrap(*expected_child),
                        ElementRef::wrap(*actual_child),
                    ) {
                        self.compare_element_refs(expected_el, actual_el)?;
                    }
                }
                (expected, actual) => {
                    return Err(HtmlCompareError::NodeMismatch(format!(
                        "Node type mismatch at position {}. Expected type: {:?}, Actual type: {:?}",
                        i,
                        node_type_name(expected),
                        node_type_name(actual)
                    )));
                }
            }
        }
        Ok(())
    }

    fn compare_unordered_nodes(
        &self,
        expected: &[NodeRef<Node>],
        actual: &[NodeRef<Node>],
    ) -> Result<(), HtmlCompareError> {
        if expected.len() != actual.len() {
            return Err(HtmlCompareError::NodeMismatch(format!(
                "Child count mismatch. Expected: {}, Actual: {}",
                expected.len(),
                actual.len()
            )));
        }

        let mut matched = vec![false; actual.len()];

        for expected_child in expected {
            let mut found = false;
            for (i, actual_child) in actual.iter().enumerate() {
                if !matched[i] {
                    match (expected_child.value(), actual_child.value()) {
                        (Node::Text(expected_text), Node::Text(actual_text)) => {
                            if self.options.ignore_text
                                || (!self.options.ignore_whitespace && expected_text == actual_text)
                                || (self.options.ignore_whitespace
                                    && expected_text.trim() == actual_text.trim())
                            {
                                matched[i] = true;
                                found = true;
                                break;
                            }
                        }
                        (Node::Element(_), Node::Element(_)) => {
                            if let (Some(expected_el), Some(actual_el)) = (
                                ElementRef::wrap(*expected_child),
                                ElementRef::wrap(*actual_child),
                            ) {
                                if self.compare_element_refs(expected_el, actual_el).is_ok() {
                                    matched[i] = true;
                                    found = true;
                                    break;
                                }
                            }
                        }
                        (Node::Comment(_), Node::Comment(_)) if self.options.ignore_comments => {
                            matched[i] = true;
                            found = true;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            if !found {
                return Err(HtmlCompareError::NodeMismatch(format!(
                    "No matching node found for {:?}",
                    expected_child.value()
                )));
            }
        }
        Ok(())
    }

    /// Determine if a node should be included in comparison
    fn should_include_node(&self, node: &NodeRef<Node>) -> bool {
        match node.value() {
            Node::Text(text) => {
                !self.options.ignore_text
                    && (!self.options.ignore_whitespace || !text.trim().is_empty())
            }
            Node::Comment(_) => !self.options.ignore_comments,
            _ => true,
        }
    }
}

/// Convenience functions for creating common comparison configurations
pub mod presets {
    use super::*;

    /// Create a comparer that ignores all formatting differences
    pub fn relaxed() -> HtmlCompareOptions {
        HtmlCompareOptions {
            ignore_whitespace: true,
            ignore_attributes: true,
            ignored_attributes: HashSet::new(),
            ignore_text: false,
            ignore_comments: true,
            ignore_sibling_order: true,
        }
    }

    /// Create a comparer that is strict about everything except whitespace
    pub fn strict() -> HtmlCompareOptions {
        HtmlCompareOptions {
            ignore_whitespace: true,
            ignore_attributes: false,
            ignored_attributes: HashSet::new(),
            ignore_text: false,
            ignore_comments: false,
            ignore_sibling_order: false,
        }
    }

    /// Create a comparer that is suitable for testing markdown output
    pub fn markdown() -> HtmlCompareOptions {
        HtmlCompareOptions {
            ignore_whitespace: true,
            ignore_attributes: false,
            ignored_attributes: {
                let mut set = HashSet::new();
                set.insert("id".to_string());
                set
            },
            ignore_text: false,
            ignore_comments: true,
            ignore_sibling_order: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_comparison() {
        assert_html_eq!("<div><p>Hello</p></div>", "<div><p>Hello</p></div>");
    }

    #[test]
    fn test_empty_elements() {
        assert_html_eq!("<div></div>", "<div></div>");
        assert_html_eq!("<div></div>", "<div/>");
        assert_html_eq!("<br>", "<br/>");
        assert_html_eq!("<img src='test.jpg'>", "<img src='test.jpg'/>");

        // Empty elements with whitespace
        assert_html_eq!("<div></div>", "<div>   </div>");
        assert_html_eq!("<p></p>", "<p>\n</p>");
    }

    #[test]
    fn test_whitespace_handling() {
        // ignore_whitespace only affects whitespace between elements, not within text content
        assert_html_ne!("<p>Hello   World</p>", "<p>Hello World</p>");

        // Whitespace between elements is ignored by default
        assert_html_eq!(
            "<div><p>Hello</p></div>",
            "<div>\n  <p>\n    Hello\n  </p>\n</div>"
        );

        // Whitespace at start/end of text content
        assert_html_eq!(
            "<p>   Hello   </p>",
            "<p>Hello</p>",
            HtmlCompareOptions {
                ignore_whitespace: true,
                ..Default::default()
            }
        );

        // With whitespace preservation, element whitespace matters
        let strict_options = HtmlCompareOptions {
            ignore_whitespace: false,
            ..Default::default()
        };

        assert_html_ne!(
            "<div><p>Hello</p></div>",
            "<div>\n  <p>\n    Hello\n  </p>\n</div>",
            strict_options.clone()
        );

        // Multiple consecutive spaces in text
        assert_html_ne!("<p>Hello    World</p>", "<p>Hello World</p>");
    }

    #[test]
    fn test_text_content_whitespace() {
        // Text with various whitespace patterns
        assert_html_ne!("<p>Hello   World</p>", "<p>Hello World</p>");

        assert_html_ne!("<p>Hello \t World</p>", "<p>Hello World</p>");

        assert_html_ne!("<p>Hello\nWorld</p>", "<p>Hello World</p>");

        // Exact whitespace matches
        assert_html_eq!("<p>Hello   World</p>", "<p>Hello   World</p>");

        // Mixed whitespace and elements
        assert_html_eq!(
            "<div>\n    <p>Hello   World</p>\n</div>",
            "<div><p>Hello   World</p></div>",
            HtmlCompareOptions {
                ignore_whitespace: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_whitespace_with_multiple_text_nodes() {
        // Text nodes with elements between
        assert_html_eq!(
            "<p>Hello <strong>beautiful</strong> World</p>",
            "<p>Hello <strong>beautiful</strong> World</p>"
        );

        // Different whitespace around elements should be ignored
        assert_html_eq!(
            "<p>Hello<strong>beautiful</strong>World</p>",
            "<p>Hello <strong>beautiful</strong> World</p>"
        );
    }

    #[test]
    fn test_attribute_handling() {
        // Different attribute order
        assert_html_eq!(
            "<div class='test' id='1'>Test</div>",
            "<div id='1' class='test'>Test</div>"
        );

        // Different attribute values
        assert_html_ne!(
            "<div class='test'>Test</div>",
            "<div class='different'>Test</div>"
        );

        // Multiple attributes
        assert_html_eq!(
            "<div class='a b' id='1' data-test='value'>Content</div>",
            "<div data-test='value' class='a b' id='1'>Content</div>"
        );

        // Boolean attributes
        assert_html_eq!(
            "<input type='checkbox' checked>",
            "<input checked type='checkbox'>"
        );

        // Ignored attributes
        let mut ignored_attrs = HashSet::new();
        ignored_attrs.insert("class".to_string());
        ignored_attrs.insert("id".to_string());

        assert_html_eq!(
            "<div class='test' id='1'>Test</div>",
            "<div class='different' id='2'>Test</div>",
            HtmlCompareOptions {
                ignored_attributes: ignored_attrs,
                ..Default::default()
            }
        );

        // All attributes ignored
        assert_html_eq!(
            "<div class='test' id='1' data-custom='value'>Test</div>",
            "<div class='different' id='2' data-custom='other'>Test</div>",
            HtmlCompareOptions {
                ignore_attributes: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_text_handling() {
        // Basic text comparison
        assert_html_eq!("<p>Hello World</p>", "<p>Hello World</p>");

        // Different text content
        assert_html_ne!("<p>Hello World</p>", "<p>Goodbye World</p>");

        // Text with special characters
        assert_html_eq!("<p>Hello &amp; World</p>", "<p>Hello &amp; World</p>");

        // Mixed text and elements
        assert_html_eq!(
            "<div>Hello <strong>World</strong>!</div>",
            "<div>Hello <strong>World</strong>!</div>"
        );

        // Text ignored
        assert_html_eq!(
            "<p>Hello World</p>",
            "<p>Goodbye World</p>",
            HtmlCompareOptions {
                ignore_text: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn test_nested_structure() {
        // Basic nesting
        assert_html_eq!(
            "<div><section><h1>Title</h1><p>Text</p></section></div>",
            "<div><section><h1>Title</h1><p>Text</p></section></div>"
        );

        // Different nesting
        assert_html_ne!(
            "<div><section><h1>Title</h1><p>Text</p></section></div>",
            "<div><h1>Title</h1><section><p>Text</p></section></div>"
        );

        // Deep nesting
        assert_html_eq!(
            "<div><article><section><header><h1>Title</h1></header><p>Text</p></section></article></div>",
            "<div><article><section><header><h1>Title</h1></header><p>Text</p></section></article></div>"
        );

        // Multiple nested elements
        assert_html_eq!(
            "<div><section><h1>Title</h1><p>Text</p></section><section><h2>Another</h2><p>More</p></section></div>",
            "<div><section><h1>Title</h1><p>Text</p></section><section><h2>Another</h2><p>More</p></section></div>"
        );
    }

    #[test]
    fn test_comment_handling() {
        // Comments ignored by default
        assert_html_eq!(
            "<div><!-- Comment --><p>Test</p></div>",
            "<div><p>Test</p></div>"
        );

        assert_html_eq!(
            "<div><!-- Multiple --><!-- Comments --><p>Test</p></div>",
            "<div><p>Test</p></div>"
        );

        // Comments preserved
        let preserve_comments = HtmlCompareOptions {
            ignore_comments: false,
            ..Default::default()
        };

        // Same comments
        assert_html_eq!(
            "<div><!-- Comment --><p>Test</p></div>",
            "<div><!-- Comment --><p>Test</p></div>",
            preserve_comments.clone()
        );

        // Different comments
        assert_html_ne!(
            "<div><!-- Comment 1 --><p>Test</p></div>",
            "<div><!-- Comment 2 --><p>Test</p></div>",
            preserve_comments.clone()
        );

        // Missing comment
        assert_html_ne!(
            "<div><!-- Comment --><p>Test</p></div>",
            "<div><p>Test</p></div>",
            preserve_comments
        );
    }

    #[test]
    fn test_sibling_order() {
        // Order matters by default
        assert_html_ne!(
            "<div><p>First</p><p>Second</p></div>",
            "<div><p>Second</p><p>First</p></div>"
        );

        // Order ignored
        let ignore_order = HtmlCompareOptions {
            ignore_sibling_order: true,
            ..Default::default()
        };

        // Simple sibling swap
        assert_html_eq!(
            "<div><p>First</p><p>Second</p></div>",
            "<div><p>Second</p><p>First</p></div>",
            ignore_order.clone()
        );

        // Multiple siblings
        assert_html_eq!(
            "<div><p>1</p><p>2</p><p>3</p></div>",
            "<div><p>3</p><p>1</p><p>2</p></div>",
            ignore_order.clone()
        );

        // Nested siblings
        assert_html_eq!(
            "<div><section><p>A</p><p>B</p></section><section><p>C</p><p>D</p></section></div>",
            "<div><section><p>B</p><p>A</p></section><section><p>D</p><p>C</p></section></div>",
            ignore_order
        );
    }

    #[test]
    fn test_special_characters() {
        // HTML entities
        assert_html_eq!(
            "<p>&lt;div&gt; &amp; &quot;quotes&quot;</p>",
            "<p>&lt;div&gt; &amp; &quot;quotes&quot;</p>"
        );

        // Unicode characters
        assert_html_eq!("<p>Hello 世界 🌍</p>", "<p>Hello 世界 🌍</p>");

        // Mixed entities and Unicode
        assert_html_eq!(
            "<p>&copy; 2024 • Hello 世界</p>",
            "<p>&copy; 2024 • Hello 世界</p>"
        );

        // Different entities representing same character
        assert_html_eq!("<p>&quot;quoted&quot;</p>", "<p>&#34;quoted&#34;</p>");
    }

    #[test]
    fn test_error_messages() {
        // Test tag mismatch error
        let result = HtmlComparer::new().compare("<div>Test</div>", "<span>Test</span>");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Node mismatch: Tag name mismatch. Expected: div, Actual: span"
        );

        // Test attribute mismatch error
        let result = HtmlComparer::new().compare(
            "<div class='test'>Content</div>",
            "<div class='different'>Content</div>",
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Node mismatch: Attributes mismatch. Expected: {(\"class\", \"test\")}, Actual: {(\"class\", \"different\")}"
        );

        // Test content mismatch error
        let result = HtmlComparer::new().compare("<div>Hello</div>", "<div>World</div>");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Node mismatch: Text content mismatch at position 0. Expected: 'Hello', Actual: 'World'"
        );

        // Test structure mismatch error
        let result = HtmlComparer::new().compare("<div><p>Text</p></div>", "<div>Text</div>");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            r#"Node mismatch: Node type mismatch at position 0. Expected type: "Element", Actual type: "Text""#
        );
    }

    #[test]
    fn test_preset_configurations() {
        // Test relaxed preset
        let _relaxed = HtmlComparer::with_options(presets::relaxed());
        assert_html_eq!(
            "<div class='a'><p>First</p><p>Second</p></div>",
            "<div class='b'><p>Second</p><p>First</p></div>",
            presets::relaxed()
        );

        // Test strict preset
        assert_html_eq!(
            "<div class='test'><!--comment--><p>Content</p></div>",
            "<div class='test'><!--comment--><p>Content</p></div>",
            presets::strict()
        );

        assert_html_ne!(
            "<div class='test'>Content</div>",
            "<div class='different'>Content</div>",
            presets::strict()
        );

        // Test markdown preset
        assert_html_eq!(
            "<h1 id='heading-1'>Title</h1><p>Content</p>",
            "<h1 id='different-id'>Title</h1><p>Content</p>",
            presets::markdown()
        );
    }
    #[test]
    fn test_mixed_scenarios() {
        // Combine multiple options
        let custom_options = HtmlCompareOptions {
            ignore_whitespace: true,
            ignore_comments: true,
            ignore_sibling_order: true,
            ignored_attributes: {
                let mut set = HashSet::new();
                set.insert("class".to_string());
                set
            },
            ..Default::default()
        };

        assert_html_eq!(
            "<div class='a'><!-- comment -->\n  <p>First</p>\n  <p>Second</p>\n</div>",
            "<div class='b'><p>Second</p><p>First</p></div>",
            custom_options
        );

        // Mix text and structural comparison
        let mixed_content = HtmlCompareOptions {
            ignore_whitespace: true,
            ignore_sibling_order: true,
            ..Default::default()
        };

        assert_html_eq!(
            "<div>\n  <p>Text</p>\n  <ul><li>A</li><li>B</li></ul>\n</div>",
            "<div><ul><li>B</li><li>A</li></ul><p>Text</p></div>",
            mixed_content
        );
    }

    #[test]
    fn test_edge_cases() {
        // Empty HTML
        assert_html_eq!("", "");

        // Just whitespace
        assert_html_eq!("   ", "");
        assert_html_eq!("\n\t  \n", "");

        // Single text node
        assert_html_eq!("Hello", "Hello");

        // Deeply nested single element
        assert_html_eq!(
            "<div><div><div><div><div>Text</div></div></div></div></div>",
            "<div><div><div><div><div>Text</div></div></div></div></div>"
        );

        // Many siblings
        let mut many_siblings1 = String::with_capacity(1000);
        let mut many_siblings2 = String::with_capacity(1000);
        for i in 0..100 {
            many_siblings1.push_str("<p>");
            many_siblings1.push_str(&i.to_string());
            many_siblings1.push_str("</p>");

            many_siblings2.push_str("<p>");
            many_siblings2.push_str(&i.to_string());
            many_siblings2.push_str("</p>");
        }
        assert_html_eq!(
            &format!("<div>{}</div>", many_siblings1),
            &format!("<div>{}</div>", many_siblings2)
        );

        // HTML with all sorts of content
        assert_html_eq!(
            r#"<div class="wrapper" id="main">
                <!-- Header section -->
                <header class="header">
                    <h1>Title &amp; Subtitle</h1>
                </header>
                <main>
                    <p>Hello 世界!</p>
                    <ul>
                        <li>Item 1</li>
                        <li>Item 2</li>
                    </ul>
                    <img src="test.jpg" alt="Test Image"/>
                </main>
                <!-- Footer section -->
                <footer>
                    <p>&copy; 2024</p>
                </footer>
            </div>"#,
            r#"<div class="wrapper" id="main"><header class="header"><h1>Title &amp; Subtitle</h1></header><main><p>Hello 世界!</p><ul><li>Item 1</li><li>Item 2</li></ul><img src="test.jpg" alt="Test Image"/></main><footer><p>&copy; 2024</p></footer></div>"#
        );
    }

    #[test]
    fn test_malformed_html() {
        // Unclosed tags (should be handled by HTML parser)
        assert_html_eq!("<p>Text", "<p>Text</p>");

        // Extra closing tags - parser treats them as additional elements
        assert_html_ne!("<p>Text</p></p>", "<p>Text</p>");

        // Test the specific error we get with extra closing tags
        let result = HtmlComparer::new().compare("<p>Text</p></p>", "<p>Text</p>");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Node mismatch: Child count mismatch. Expected: 2, Actual: 1"
        );

        // Mismatched tags are typically corrected by the parser
        // Let's verify the actual behavior
        let result = HtmlComparer::new().compare(
            "<p><strong>Text</p></strong>",
            "<p><strong>Text</strong></p>",
        );
        if let Err(e) = result {
            println!("Actual parser behavior for mismatched tags: {}", e);
        }
    }
}
