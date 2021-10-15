//! DO NOT USE THIS CRATE DIRECTLY.
//! It is an internal dependency of the main crate.
//!
//! Procedural macro for parsing fragmented strings.
//!
//! Can be used like this:
//! ```
//! # use parse_procmacro::frag_parse;
//! let (foo, bar, baz) = frag_parse!("%s%s%d", "%s%s%d__foo__bar__42").unwrap();
//! assert_eq!(foo, "foo");
//! assert_eq!(bar, "bar");
//! assert_eq!(baz, 42);
//! ```
//!
//! The macro is reexported in the main `fragstrings` crate:
//! ```no_compile
//! # // This doctest is disabled because the crate is not in scope.
//! use fragstrings::frag_parse;
//! ```

use proc_macro2::{TokenStream, TokenTree};
use quote::format_ident;
use quote::quote;

use utils::{
    fmt_strings::{parse_format_string, FormatItem},
    literals::parse_string_literal,
    punct::parse_punctuated_args,
};

/// Procedural macro for parsing fragmented strings.
///
/// Can be used like this:
/// ```
/// # use parse_procmacro::frag_parse;
/// let (foo, bar, baz) = frag_parse!("%s%s%d", "%s%s%d__foo__bar__42").unwrap();
/// assert_eq!(foo, "foo");
/// assert_eq!(bar, "bar");
/// assert_eq!(baz, 42);
/// ```
///
/// The returned value is `Option<(tuple)>`, where tuple has items which corresponds
/// to the format descriptor.
#[proc_macro]
pub fn frag_parse(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = args.into();
    let output = match frag_parse_impl(args) {
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
    StringExpressionExpected,
    TooManyArguments,
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
            StringExpressionExpected => "String expression expected",
            TooManyArguments => "Too many arguments",
        };
        // Extra curly braces are required here,
        // because output is required to be an assignable expression.
        quote! { { compile_error!(#msg); } }
    }
}

fn frag_parse_impl(args: TokenStream) -> Result<TokenStream, CompileError> {
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

    let formatted_value_expr = match args.next() {
        None => return Err(CompileError::StringExpressionExpected),
        Some(stream) => stream,
    };

    if args.next().is_some() {
        return Err(CompileError::TooManyArguments);
    }

    let fmt_string =
        parse_string_literal(&fmt_string_literal).ok_or(CompileError::BadStringLiteral)?;

    let mut fmt_items = parse_format_string(fmt_string).ok_or(CompileError::BadFormatString)?;

    let mut ends_with_wildcard = false;
    let mut fmt_string_len = fmt_string.len();
    if fmt_items.ends_with(&[FormatItem::Any]) {
        ends_with_wildcard = true;
        fmt_items.pop();
        fmt_string_len -= 1;
    }

    let n = fmt_items.len();

    let vars = (0..n).map(|i| format_ident!("_{}", i)).collect::<Vec<_>>();

    let var_decls = vars
        .iter()
        .zip(fmt_items.into_iter())
        .map(|(var, it)| match it {
            FormatItem::Str => {
                quote! {
                    let #var: ::std::string::String = if let Some(value) = fragments.next() {
                        value.to_owned()
                    } else {
                        ok = false;
                        "".to_owned()
                    };
                }
            }
            FormatItem::Int => {
                quote! {
                    let #var: i64 = if let Some(value) = fragments.next() {
                        match value.parse() {
                            Ok(value) => value,
                            Err(_) => {
                                ok = false;
                                0
                            }
                        }
                    } else {
                        ok = false;
                        0
                    };
                }
            }
            FormatItem::Any => unreachable!(),
        })
        .collect::<Vec<_>>();

    let pattern_check = if ends_with_wildcard {
        let prefix = &fmt_string[0..fmt_string_len];
        quote! { pattern.len() >= #fmt_string_len && &pattern[0..#fmt_string_len] == #prefix }
    } else {
        quote! { pattern == #fmt_string }
    };

    let res = quote! {
        {
            let input: &str = &(#formatted_value_expr);
            let mut fragments = input.split("__");
            let ok = if let Some(pattern) = fragments.next() {
                #pattern_check
            } else {
                false
            };
            if ok {
                let mut ok = true;
                #( #var_decls )*
                let all_good = if #ends_with_wildcard {
                    true
                } else {
                    fragments.next().is_none()
                };
                if ok && all_good {
                    Some( ( #( #vars ),* ) )
                } else {
                    None
                }
            } else {
                None
            }
        }
    };

    Ok(res)
}
