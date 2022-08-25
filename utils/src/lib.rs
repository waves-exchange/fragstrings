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
            let result = parsed.into_iter().map(|stream| stream.to_string()).collect_vec();
            assert_eq!(result, expected);
        }

        test(quote! { foo, bar, baz }, vec!["foo", "bar", "baz"]);
        test(quote! { one, 2+2, two }, vec!["one", "2 + 2", "two"]);
        test(quote! { x, (1+2)*3, y }, vec!["x", "(1 + 2) * 3", "y"]);
        test(quote! { x, (1, 2, 3), y }, vec!["x", "(1 , 2 , 3)", "y"]);
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

    use self::FormatEnding::{Closed, Open};
    use self::FormatItemOpt::{Mandatory, Optional};
    use self::FormatItemType::{Int, Str};

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct FormatString(pub Vec<FormatItem>, pub FormatEnding);

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub struct FormatItem(pub FormatItemType, pub FormatItemOpt);

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum FormatItemType {
        Str,
        Int,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum FormatItemOpt {
        Mandatory,
        Optional,
    }

    #[derive(Copy, Clone, PartialEq, Eq, Debug)]
    pub enum FormatEnding {
        Closed,
        Open,
    }

    pub fn parse_format_string(fmt: &str) -> Option<Vec<FormatItemType>> {
        let res = parse_format_string_ex(fmt);
        // Remove all the extra stuff, if present
        if let Some(FormatString(ref items, ending)) = res {
            if ending != Closed {
                return None;
            }
            if items.iter().any(|item| item.1 == Optional) {
                return None;
            }
        }

        res.map(|FormatString(items, _)| items.into_iter().map(|item| item.0).collect_vec())
    }

    pub fn parse_format_string_ex(fmt: &str) -> Option<FormatString> {
        if fmt.is_empty() {
            return None;
        }

        let approx_capacity = fmt.len() / 2;
        let mut items = Vec::with_capacity(approx_capacity);
        let mut ending = Closed;
        let mut iter = fmt.bytes().peekable();
        loop {
            match iter.next() {
                None => break,
                Some(ch) => {
                    if ch == b'*' {
                        // Asterisk, if present, must be the last item in the format string
                        if iter.next().is_some() {
                            return None;
                        }

                        // Asterisk, if present, must be not the only item in the format string
                        if items.is_empty() {
                            return None;
                        }

                        // Otherwise mark format string as open-ended and finish parsing
                        ending = Open;
                        break;
                    }

                    // All format descriptors must start with an '%'
                    if ch != b'%' {
                        return None;
                    }

                    // Next character is mandatory, otherwise abort parsing
                    let ch = iter.next()?;
                    let item_type = match ch {
                        b's' => Str,
                        b'd' => Int,
                        _ => return None,
                    };

                    // Optional '?' character
                    let item_opt = if iter.peek() == Some(&b'?') {
                        let _ = iter.next(); // Consume it
                        Optional
                    } else {
                        Mandatory
                    };

                    // Optional items, if present, must all be in the end of the format string
                    if item_opt == Mandatory {
                        if let Some(&FormatItem(_, last_opt)) = items.last() {
                            if last_opt == Optional {
                                return None;
                            }
                        }
                    }

                    // Store the item
                    items.push(FormatItem(item_type, item_opt));
                }
            }
        }

        // All items can not be optional, there must be at least one mandatory item
        if let Some(first) = items.first() {
            if first.1 == Optional {
                return None;
            }
        } else {
            // No items at all - error
            return None;
        }

        Some(FormatString(items, ending))
    }

    #[test]
    fn test_parse_format_string() {
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
        assert_eq!(parse_format_string("%b"), None);
        assert_eq!(parse_format_string("%x"), None);
        assert_eq!(parse_format_string("%s%x"), None);
        assert_eq!(parse_format_string("%sx"), None);
        assert_eq!(parse_format_string("%sxx"), None);
        assert_eq!(parse_format_string("%s foo"), None);
        assert_eq!(parse_format_string("%s "), None);
        assert_eq!(parse_format_string(" %s"), None);
    }

    #[rustfmt::skip] // FIXME review settings of the rustfmt
    #[test]
    fn test_parse_format_string_ex() {
        // Parse so that all items are mandatory
        let pm = |s: &str| {
            parse_format_string_ex(s).map(|FormatString(items, ending)| {
                let ok = items.iter().all(|item| item.1 == Mandatory);
                assert!(ok, "All items in this format string supposed to be parsed as mandatory: {}", s);
                let items = items.into_iter().map(|item| item.0).collect_vec();
                (items, ending)
            })
        };

        // Parse with possible optional items
        let po = |s: &str| {
            parse_format_string_ex(s).map(|FormatString(items, ending)| {
                let items = items.into_iter().map(|item| (item.0, item.1)).collect_vec();
                (items, ending)
            })
        };

        assert_eq!(pm(""), None);

        assert_eq!(pm("%s"), Some((vec![Str], Closed)));
        assert_eq!(pm("%d"), Some((vec![Int], Closed)));
        assert_eq!(pm("%s%d"), Some((vec![Str, Int], Closed)));
        assert_eq!(pm("%d%s"), Some((vec![Int, Str], Closed)));
        assert_eq!(pm("%s%s"), Some((vec![Str, Str], Closed)));
        assert_eq!(pm("%d%d"), Some((vec![Int, Int], Closed)));

        assert_eq!(pm("*"), None);
        assert_eq!(pm("*%s"), None);
        assert_eq!(pm("*%d"), None);
        assert_eq!(pm("%s*"), Some((vec![Str], Open)));
        assert_eq!(pm("%d*"), Some((vec![Int], Open)));
        assert_eq!(pm("%s%d*"), Some((vec![Str, Int], Open)));

        assert_eq!(po("?"), None);
        assert_eq!(po("*?"), None);
        assert_eq!(po("?*"), None);
        assert_eq!(po("%?"), None);
        assert_eq!(po("?%s"), None);
        assert_eq!(po("%s?"), None);
        assert_eq!(po("%d?"), None);
        assert_eq!(po("%s?*"), None);
        assert_eq!(po("%d?*"), None);
        assert_eq!(po("%s?%s?"), None);
        assert_eq!(po("%d?%d?"), None);
        assert_eq!(po("%s?%s?*"), None);
        assert_eq!(po("%d?%d?*"), None);
        assert_eq!(po("%s%d?"), Some((vec![(Str, Mandatory), (Int, Optional)], Closed)));
        assert_eq!(po("%d%s?"), Some((vec![(Int, Mandatory), (Str, Optional)], Closed)));
        assert_eq!(po("%s%d?*"), Some((vec![(Str, Mandatory), (Int, Optional)], Open)));
        assert_eq!(po("%d%s?*"), Some((vec![(Int, Mandatory), (Str, Optional)], Open)));
        assert_eq!(po("%s%s%d?"), Some((vec![(Str, Mandatory), (Str, Mandatory), (Int, Optional)], Closed)));
        assert_eq!(po("%s%s?%d?"), Some((vec![(Str, Mandatory), (Str, Optional), (Int, Optional)], Closed)));
        assert_eq!(po("%s%s%d?*"), Some((vec![(Str, Mandatory), (Str, Mandatory), (Int, Optional)], Open)));
        assert_eq!(po("%s%s?%d?*"), Some((vec![(Str, Mandatory), (Str, Optional), (Int, Optional)], Open)));
        assert_eq!(po("%s?%s"), None);
        assert_eq!(po("%s?%s*"), None);
    }
}
