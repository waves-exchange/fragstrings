//! Procedural macros for formatting and parsing fragmented strings.
//!
//! This is the main crate which reexports macros from implementation crates.
//!
//! # Formatting
//!
//! ```
//! use fragstrings::frag_format;
//! let foo = "foo";
//! let result = frag_format!("%s%s%d", foo, "bar", 42);
//! assert_eq!(result, "%s%s%d__foo__bar__42");
//! ```

pub use format_procmacro::frag_format;
