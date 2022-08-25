# Fragmented Strings formatting & parsing
Procedural macros to format and parse _Waves.Exchange_ fragmented strings with compile-time checking.

## Overview
This enables you to write code like this:

```rust
    let key = frag_format!("%d%d%s", amount_asset_internal_id, price_asset_internal_id, "locked");

    let (amount_asset_locked, price_asset_locked, lp_token_locked) = frag_parse!("%d%d%d", value)?;
```

The format specifier (`%d%d%s` in the example above) must be a string literal and is checked at compile time.
Argument count and types are also checked at compile time.

The `frag_format!()` macro returns a `String`.

The `frag_parse!()` macro returns an `Option<(tuple)>`, where tuple has items which corresponds
to the format descriptor.


## Adding dependency to your code
Add the following to your `Cargo.toml`'s dependencies section:
```
fragstrings = { git = "https://github.com/waves-exchange/fragstrings", tag = "v0.1.1" }
```

### Features
Parsing and formatting of fragmented strings is split into two features: `parse` and `format`.
Both of them are active by default. 
To enable only specific parts, disable default features and opt-in accordingly.

```
fragstrings = { git = "https://github.com/waves-exchange/fragstrings", tag = "v0.1.1", default-features = false, features = ["format"] }
```

```
fragstrings = { git = "https://github.com/waves-exchange/fragstrings", tag = "v0.1.1", default-features = false, features = ["parse"] }
```


## Fragmented strings syntax
Fragmented string is an encoding scheme for storing several  numeric and string values in a string.

A fragmented string consists of two parts: fragments descriptors and fragments values, divided by `__`.

The fragments descriptors part describes the fragments value types and consists of concatenated C-like format specifiers. 
Now it supports the types described below:
* `%s` - for strings;
* `%d` - for signed 64-bit integers.

The fragments' values are separated by `__`.
Every fragment value has to have respective fragment descriptor.

Example: `%s%s%d__order__height__1000`.
The fragments descriptor here is `%s%s%d`, and there are 3 fragments: two strings and one number.
Fragment 0 is a string `order`, fragment 1 is a string `height` and fragment 2 is an integer `1000`.

* Empty strings are allowed.
* Negative integers are allowed.


### Special syntax extension for parsing fragmented strings
For extensibility purposes, format specifier for the parse macro is allowed to end with a '*',
which means that any unspecified fragments can appear in the value, which are silently ignored
without any errors reported.

Example:
```rust
    let (foo, bar) = frag_parse!("%s%s*", "%s%s%s__foo__bar__baz")?;
```

In this example the fragment "baz" is ignored because it is masked by an asterisk
in the format specifier.


## Running tests
To run all unit tests, execute `cargo test --workspace` in the workspace root.
