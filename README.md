# html-compare-rs

A Rust library for comparing HTML content with configurable comparison options. Useful for testing HTML output while ignoring differences that don't affect the rendered result.

## Features

- Compare HTML strings while ignoring unimportant differences
- Configurable comparison options for whitespace, attributes, comments, and more
- Built-in test assertions macros for ergonomic testing
- Handles standard HTML parsing edge cases and malformed HTML
- Clear error messages for debugging differences

## Installation

Just `cargo add --dev html-compare-rs` or add this to your `Cargo.toml`:

```toml
[dev-dependencies]
html-compare-rs = "0.1.0"
```

## Quick Start

```rust
use html_compare::{assert_html_eq, HtmlCompareOptions};

// Basic comparison
assert_html_eq!(
    "<div><p>Hello</p></div>",
    "<div>\n  <p>Hello</p>\n</div>"
);

// Custom comparison options
assert_html_eq!(
    "<div class='test'><p>First</p><p>Second</p></div>",
    "<div class='different'><p>Second</p><p>First</p></div>",
    HtmlCompareOptions {
        ignore_attributes: true,
        ignore_sibling_order: true,
        ..Default::default()
    }
);
```

## Usage

### Basic Comparison

The library provides both a programmatic API and test assertions:

```rust
use html_compare::{HtmlComparer, assert_html_eq};

// Using the assertion macro (recommended for tests)
assert_html_eq!(
    "<div><p>Hello</p></div>",
    "<div><p>Hello</p></div>"
);

// Using the API directly
let comparer = HtmlComparer::new();
assert!(comparer.compare(
    "<div><p>Hello</p></div>",
    "<div><p>Hello</p></div>"
).is_ok());
```

### Configuration Options

Control how HTML is compared with `HtmlCompareOptions`:

```rust
use html_compare::{HtmlCompareOptions, assert_html_eq};

let options = HtmlCompareOptions {
    // Ignore whitespace between elements (default: true)
    ignore_whitespace: true,
    // Ignore all HTML attributes (default: false)
    ignore_attributes: false,
    // Ignore specific attributes
    ignored_attributes: {
        let mut set = std::collections::HashSet::new();
        set.insert("class".to_string());
        set
    },
    // Ignore text content (default: false)
    ignore_text: false,
    // Ignore HTML comments (default: true)
    ignore_comments: true,
    // Ignore sibling element order (default: false)
    ignore_sibling_order: false,
};

assert_html_eq!(
    "<div class='a'>First</div>",
    "<div class='b'>First</div>",
    options
);
```

### Built-in Presets

Common comparison configurations are available as presets:

```rust
use html_compare::{assert_html_eq, presets};

// Relaxed comparison - ignores formatting, attributes, and order
assert_html_eq!(
    "<div class='a'><p>First</p><p>Second</p></div>",
    "<div class='b'><p>Second</p><p>First</p></div>",
    presets::relaxed()
);

// Strict comparison - only ignores whitespace
assert_html_eq!(
    "<div class='test'>Content</div>",
    "<div class='test'>Content</div>",
    presets::strict()
);

// Markdown comparison - ignores IDs but preserves other attributes
assert_html_eq!(
    "<h1 id='header-1'>Title</h1>",
    "<h1 id='different'>Title</h1>",
    presets::markdown()
);
```

## Whitespace Handling

By default, the library follows standard HTML whitespace rules:
- Multiple spaces in text content are collapsed into a single space
- Whitespace between elements is ignored when `ignore_whitespace` is true
- Leading and trailing whitespace in text content is trimmed

```rust
assert_html_eq!(
    "<p>Hello   World</p>",
    "<p>Hello World</p>"
);

assert_html_eq!(
    "<div>\n  <p>\n    Hello\n  </p>\n</div>",
    "<div><p>Hello</p></div>"
);
```

## Error Messages

When comparisons fail, detailed error messages help identify the differences:

```rust
use html_compare::assert_html_eq;

// This will panic with a detailed error message:
assert_html_eq!(
    "<div class='test'>Content</div>",
    "<div class='different'>Content</div>"
);
// Error:
// HTML comparison failed:
// Node mismatch: Attributes mismatch...
// 
// left HTML:
// <div class='test'>Content</div>
// 
// right HTML:
// <div class='different'>Content</div>
//
// options: ...
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
