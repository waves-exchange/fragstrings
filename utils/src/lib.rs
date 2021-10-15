//! DO NOT USE THIS CRATE DIRECTLY.
//! It is an internal dependency of macro crates.
//!
//! Utility functions for parsing fragstrings macros arguments.

pub mod punct {
    use itertools::Itertools;
    use proc_macro2::{TokenStream, TokenTree};

    pub fn parse_punctuated_args(args: TokenStream) -> Vec<TokenStream> {
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
pub mod literals {
    pub fn parse_string_literal(lit: &str) -> Option<&str> {
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

pub mod fmt_strings {
    use itertools::Itertools;

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum FormatItem {
        Any,
        Str,
        Int,
    }

    pub fn parse_format_string(fmt: &str) -> Option<Vec<FormatItem>> {
        if fmt.is_empty() {
            return None;
        }
        let n = fmt.len();
        let mut ends_with_wildcard = false;
        if n % 2 != 0 {
            if fmt.ends_with('*') {
                ends_with_wildcard = true;
            } else {
                return None;
            }
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
        if ends_with_wildcard {
            if res.is_empty() {
                return None;
            }
            res.push(FormatItem::Any);
        }
        Some(res)
    }

    #[test]
    fn test_parse_format_string() {
        use FormatItem::{Any, Int, Str};

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

        assert_eq!(parse_format_string("*"), None);
        assert_eq!(parse_format_string("*%s"), None);
        assert_eq!(parse_format_string("*%d"), None);
        assert_eq!(parse_format_string("%s*"), Some(vec![Str, Any]));
        assert_eq!(parse_format_string("%d*"), Some(vec![Int, Any]));
        assert_eq!(parse_format_string("%s%d*"), Some(vec![Str, Int, Any]));
    }
}
