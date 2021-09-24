//! Procedural macro for formatting fragmented strings.
//!
//! Can be used like this:
//! ```
//! # use format_procmacro::frag_format;
//! let foo = "foo";
//! let result = frag_format!("%s%s%d", foo, "bar", 42);
//! assert_eq!(result, "%s%s%d__foo__bar__42");
//! ```
//!
//! Do not use this crate directly.
//! The macro is reexported in the main `fragstrings` crate:
//! ```no_compile
//! # // This doctest is disabled because the crate is not in scope.
//! use fragstrings::frag_format;
//! ```

use proc_macro2::{TokenStream, TokenTree};
use quote::format_ident;
use quote::quote;

use std::iter;

use itertools::Itertools;

use crate::fmt_strings::{parse_format_string, FormatItem};
use crate::literals::parse_string_literal;
use crate::punct::parse_punctuated_args;

#[proc_macro]
pub fn frag_format(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = args.into();
    let output = match frag_format_impl(args) {
        Ok(res) => res,
        Err(err) => err.into_compile_error(),
    };
    output.into()
}

enum CompileError {
    NoArgs,
    UnrecognizedToken,
    StringLiteralExpected,
    BadStringLiteral,
    BadFormatString,
    ArgCountMismatch,
}

impl CompileError {
    fn into_compile_error(self) -> TokenStream {
        use CompileError::*;
        let msg = match self {
            NoArgs => "Empty arguments",
            UnrecognizedToken => "Unrecognized token",
            StringLiteralExpected => "String literal expected",
            BadStringLiteral => "Bad string literal",
            BadFormatString => "Bad format string",
            ArgCountMismatch => "Number of arguments mismatches number of format items",
        };
        // Extra curly braces are required here,
        // because output is required to be an assignable expression.
        quote! { { compile_error!(#msg); } }
    }
}

fn frag_format_impl(args: TokenStream) -> Result<TokenStream, CompileError> {
    let args = parse_punctuated_args(args);

    let mut args = args.into_iter();
    let fmt_string_literal = match args.next() {
        None => return Err(CompileError::NoArgs),
        Some(stream) => {
            let mut iter = stream.into_iter();
            let literal = match iter.next() {
                None => return Err(CompileError::NoArgs),
                Some(TokenTree::Literal(lit)) => lit.to_string(),
                _ => return Err(CompileError::StringLiteralExpected),
            };
            if iter.next().is_some() {
                return Err(CompileError::UnrecognizedToken);
            }
            literal
        }
    };

    let fmt_string =
        parse_string_literal(&fmt_string_literal).ok_or(CompileError::BadStringLiteral)?;

    let fmt_items = parse_format_string(fmt_string).ok_or(CompileError::BadFormatString)?;

    let args = args.collect::<Vec<_>>();

    if fmt_items.len() != args.len() {
        return Err(CompileError::ArgCountMismatch);
    }

    let n = fmt_items.len();

    let vars = (0..n).map(|i| format_ident!("_{}", i)).collect::<Vec<_>>();

    let var_decls = vars
        .iter()
        .zip(fmt_items.into_iter())
        .zip(args.into_iter())
        .map(|((var, it), arg)| match it {
            FormatItem::Str => {
                quote! { let #var: &str = ::core::convert::AsRef::<str>::as_ref(&( #arg )); }
            }
            FormatItem::Int => quote! { let #var: i64 = { #arg } as i64; },
        })
        .collect::<Vec<_>>();

    #[allow(unstable_name_collisions)]
    let fmt_string = iter::once(fmt_string)
        .chain(iter::repeat("{}").take(n))
        .intersperse("__")
        .collect::<String>();

    let res = quote! {
        {
            #( #var_decls )*
            ::std::format!(#fmt_string, #( #vars ),*)
        }
    };

    Ok(res)
}

mod punct {
    use itertools::Itertools;
    use proc_macro2::{TokenStream, TokenTree};

    pub(super) fn parse_punctuated_args(args: TokenStream) -> Vec<TokenStream> {
        args.into_iter()
            .group_by(|it| match it {
                TokenTree::Punct(punct) => punct.as_char() == ',',
                _ => false,
            })
            .into_iter()
            .filter_map(|(is_comma, arg)| {
                if is_comma {
                    None
                } else {
                    let mut stream = TokenStream::new();
                    stream.extend(arg);
                    Some(stream)
                }
            })
            .collect_vec()
    }

    #[test]
    fn test_parse_punctuated_args() {
        use quote::quote;

        fn test(input: TokenStream, expected: Vec<&str>) {
            let parsed = parse_punctuated_args(input);
            let result = parsed
                .into_iter()
                .map(|stream| stream.to_string())
                .collect_vec();
            assert_eq!(result, expected);
        }

        test(quote! { foo, bar, baz }, vec!["foo", "bar", "baz"]);
        test(quote! { one, 2+2, two }, vec!["one", "2 + 2", "two"]);
        test(quote! { x, (1+2)*3, y }, vec!["x", "(1 + 2) * 3", "y"]);
        test(quote! { a, x -> y, b }, vec!["a", "x -> y", "b"]);
    }
}

// Naive parsing, can't handle Unicode, but sufficient for the format strings.
mod literals {
    pub(super) fn parse_string_literal(lit: &str) -> Option<&str> {
        let mut s = lit;
        if s.starts_with('b') {
            s = &s[1..];
        } else if s.starts_with('r') {
            s = &s[1..];
            while s.starts_with('#') && s.ends_with('#') && s.len() >= 2 {
                let n = s.len() - 1;
                s = &s[1..n];
            }
        }
        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
            let n = s.len() - 1;
            s = &s[1..n];
            Some(s)
        } else {
            None
        }
    }

    #[test]
    fn test_parse_string_literal() {
        assert_eq!(parse_string_literal(r#####""""#####), Some(""));
        assert_eq!(parse_string_literal(r#####""foo""#####), Some("foo"));
        assert_eq!(parse_string_literal(r#####"b"foo""#####), Some("foo"));
        assert_eq!(parse_string_literal(r#####"r"foo""#####), Some("foo"));
        assert_eq!(parse_string_literal(r#####"r#"foo"#"#####), Some("foo"));
        assert_eq!(parse_string_literal(r#####"r##"foo"##"#####), Some("foo"));
        assert_eq!(parse_string_literal(r#####"r###"foo"###"#####), Some("foo"));

        assert_eq!(parse_string_literal(r#####""#####), None);
        assert_eq!(parse_string_literal(r#####"""#####), None);
        assert_eq!(parse_string_literal(r#####"'foo'"#####), None);
        assert_eq!(parse_string_literal(r#####"'foo"#####), None);
        assert_eq!(parse_string_literal(r#####"foo'"#####), None);
        assert_eq!(parse_string_literal(r#####""foo"#####), None);
        assert_eq!(parse_string_literal(r#####"foo""#####), None);
        assert_eq!(parse_string_literal(r#####"r#"foo""#####), None);
        assert_eq!(parse_string_literal(r#####"r"foo"#"#####), None);
    }
}

mod fmt_strings {
    use itertools::Itertools;

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub(super) enum FormatItem {
        Str,
        Int,
    }

    pub(super) fn parse_format_string(fmt: &str) -> Option<Vec<FormatItem>> {
        let n = fmt.len();
        if n == 0 || n % 2 != 0 {
            return None;
        }
        let mut res = Vec::with_capacity(n / 2);
        for (ch1, ch2) in fmt.bytes().tuples() {
            if ch1 != b'%' {
                return None;
            }
            let item = match ch2 {
                b's' => FormatItem::Str,
                b'd' => FormatItem::Int,
                _ => return None,
            };
            res.push(item);
        }
        Some(res)
    }

    #[test]
    fn test_parse_format_string() {
        use FormatItem::{Int, Str};

        assert_eq!(parse_format_string(""), None);

        assert_eq!(parse_format_string("%s"), Some(vec![Str]));
        assert_eq!(parse_format_string("%d"), Some(vec![Int]));
        assert_eq!(parse_format_string("%s%d"), Some(vec![Str, Int]));
        assert_eq!(parse_format_string("%d%s"), Some(vec![Int, Str]));
        assert_eq!(parse_format_string("%s%s"), Some(vec![Str, Str]));
        assert_eq!(parse_format_string("%d%d"), Some(vec![Int, Int]));

        assert_eq!(parse_format_string("%"), None);
        assert_eq!(parse_format_string("%%"), None);
        assert_eq!(parse_format_string("%f"), None);
        assert_eq!(parse_format_string("%x"), None);
        assert_eq!(parse_format_string("%s%x"), None);
        assert_eq!(parse_format_string("%sx"), None);
        assert_eq!(parse_format_string("%sxx"), None);
        assert_eq!(parse_format_string("%s foo"), None);
        assert_eq!(parse_format_string("%s "), None);
        assert_eq!(parse_format_string(" %s"), None);
    }
}
