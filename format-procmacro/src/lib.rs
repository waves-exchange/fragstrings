//! DO NOT USE THIS CRATE DIRECTLY.
//! It is an internal dependency of the main crate.
//!
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

use utils::{
    fmt_strings::{parse_format_string, FormatItem},
    literals::parse_string_literal,
    punct::parse_punctuated_args,
};

/// Procedural macro for formatting fragmented strings.
///
/// Can be used like this:
/// ```
/// # use format_procmacro::frag_format;
/// let foo = "foo";
/// let result = frag_format!("%s%s%d", foo, "bar", 42);
/// assert_eq!(result, "%s%s%d__foo__bar__42");
/// ```
///
/// The returned value is `String`.
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

    let fmt_string = parse_string_literal(&fmt_string_literal).ok_or(CompileError::BadStringLiteral)?;

    let fmt_items = parse_format_string(fmt_string).ok_or(CompileError::BadFormatString)?;

    if fmt_items.ends_with(&[FormatItem::Any]) {
        return Err(CompileError::BadFormatString);
    }

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
            FormatItem::Any => unreachable!(),
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
