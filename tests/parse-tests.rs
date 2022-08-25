use parse_procmacro::frag_parse;

#[test]
fn test_frag_parse() {
    // Expected compile errors

    // frag_parse!(); // Compile error
    // frag_parse!(""); // Compile error
    // frag_parse!("%s"); // Compile error
    // frag_parse!("%s" foo, "%s__test"); // Compile error

    // frag_parse!("%d", 42); // Compile error
    // frag_parse!(42, s); // Compile error
    // frag_parse!(xxx, s); // Compile error
    // frag_parse!("xxx", s); // Compile error

    // Functions returning fragstrings - for tests

    fn value_str_fn() -> &'static str {
        "%s%s%s__foo__bar__baz"
    }

    fn value_string_fn() -> String {
        "%d%d%d__1__2__3".to_string()
    }

    fn other_string_fn() -> String {
        "%s%d%s__foo__42__bar".to_string()
    }

    // Test cases

    let value = frag_parse!("%s", "%s__test").expect("failed to parse");
    assert_eq!(value, "test");

    let value = frag_parse!("%d", "%d__42").expect("parse error");
    assert_eq!(value, 42_i64);

    let frag_str = "%s__test";
    let value = frag_parse!("%s", frag_str).expect("failed to parse");
    assert_eq!(value, "test");

    let frag_string = "%s__test".to_string();
    let value = frag_parse!("%s", frag_string).expect("failed to parse");
    assert_eq!(value, "test");

    let (frag1, frag2) = frag_parse!("%s%d", "%s%d__test__42").expect("failed to parse");
    assert_eq!(frag1, "test");
    assert_eq!(frag2, 42);

    let (frag1, frag2) = frag_parse!("%d%s", "%d%s__42__test").expect("failed to parse");
    assert_eq!(frag1, 42);
    assert_eq!(frag2, "test");

    let (frag1, frag2, frag3) = frag_parse!("%s%s%s", value_str_fn()).expect("failed to parse");
    assert_eq!(frag1, "foo");
    assert_eq!(frag2, "bar");
    assert_eq!(frag3, "baz");

    let (frag1, frag2, frag3) = frag_parse!("%d%d%d", value_string_fn()).expect("failed to parse");
    assert_eq!(frag1, 1);
    assert_eq!(frag2, 2);
    assert_eq!(frag3, 3);

    let (frag1, frag2, frag3) = frag_parse!("%s%d%s", other_string_fn()).expect("failed to parse");
    assert_eq!(frag1, "foo");
    assert_eq!(frag2, 42);
    assert_eq!(frag3, "bar");

    assert!(frag_parse!("%d", "%d").is_none());
    assert!(frag_parse!("%d", "%d__").is_none());
    assert!(frag_parse!("%d", "%d__1").is_some());
    assert!(frag_parse!("%d", "%d__foo").is_none());
    assert!(frag_parse!("%d", "%s__foo").is_none());
    assert!(frag_parse!("%s", "%s__foo").is_some());
    assert!(frag_parse!("%d%s", "%d%s__42").is_none());
    assert!(frag_parse!("%d%s", "%d%s__42__foo").is_some());
    assert!(frag_parse!("%d%s", "%d%s__42__foo__bar").is_none());

    assert!(frag_parse!(r"%s", "%s__test").is_some());
    assert!(frag_parse!(r#"%s"#, "%s__test").is_some());
    assert!(frag_parse!(r##"%s"##, "%s__test").is_some());
    assert!(frag_parse!(r###"%s"###, "%s__test").is_some());
    assert!(frag_parse!(b"%s", "%s__test").is_some());
}

#[test]
fn test_frag_parse_non_strict() {
    // OK: usual parsing
    let (frag1, frag2) = frag_parse!("%s%d", "%s%d__test__42").expect("failed to parse");
    assert_eq!(frag1, "test");
    assert_eq!(frag2, 42);

    // Bad: extra parameters beyond described, no asterisk
    assert!(frag_parse!("%s%d", "%s%d%s__test__42__foo").is_none());

    // OK: asterisk + no extra parameters
    let (frag1, frag2) = frag_parse!("%s%d*", "%s%d__test__42").expect("failed to parse");
    assert_eq!(frag1, "test");
    assert_eq!(frag2, 42);

    // OK: asterisk + extra parameter
    let (frag1, frag2) = frag_parse!("%s%d*", "%s%d%s__test__42__foo").expect("failed to parse");
    assert_eq!(frag1, "test");
    assert_eq!(frag2, 42);

    // OK: asterisk + two extra parameters
    let (frag1, frag2) = frag_parse!("%s%d*", "%s%d%s%s__test__42__foo__bar").expect("failed to parse");
    assert_eq!(frag1, "test");
    assert_eq!(frag2, 42);
}

#[test]
fn test_frag_parse_optional() {
    // OK: Optional parameter present
    let (frag1, frag2) = frag_parse!("%s%d?", "%s%d__test__42").expect("failed to parse");
    assert_eq!(frag1, "test");
    assert_eq!(frag2, Some(42));

    // OK: Optional parameter absent
    let (frag1, frag2) = frag_parse!("%s%d?", "%s__test").expect("failed to parse");
    assert_eq!(frag1, "test");
    assert_eq!(frag2, None);

    // Bad: Parameter is described but missing, this is NOT how optional works
    //assert!(frag_parse!("%s%d?", "%s%d__test").is_none()); //TODO FIXME This is a known bug

    // Bad: There must be at least one mandatory item -- this is checked at compile time
    // assert!(frag_parse!("%s?", "%s__test").is_none()); // Compile error -- expected
    // assert!(frag_parse!("%s?%d?", "%s%d__test__42").is_none()); // Compile error -- expected

    // OK: Two optional parameters, one absent
    let (frag1, frag2, frag3) = frag_parse!("%s%d?%s?", "%s%d__test__42").expect("failed to parse");
    assert_eq!(frag1, "test");
    assert_eq!(frag2, Some(42));
    assert_eq!(frag3, None);

    // OK: Two optional parameters, both present
    let (frag1, frag2, frag3) = frag_parse!("%s%d?%s?", "%s%d%s__test__42__foo").expect("failed to parse");
    assert_eq!(frag1, "test");
    assert_eq!(frag2, Some(42));
    assert_eq!(frag3, Some("foo".into()));

    // Bad: extra parameter beyond optional, no asterisk
    assert!(frag_parse!("%s%d?", "%s%d%s__test__42__foo").is_none());

    // OK: Optional parameter + asterisk without extra parameters
    let (frag1, frag2) = frag_parse!("%s%d?*", "%s%d__test__42").expect("failed to parse");
    assert_eq!(frag1, "test");
    assert_eq!(frag2, Some(42));

    // OK: Optional parameter + asterisk with extra unused parameter
    let (frag1, frag2) = frag_parse!("%s%d?*", "%s%d%s__test__42__foo").expect("failed to parse");
    assert_eq!(frag1, "test");
    assert_eq!(frag2, Some(42));

    //TODO FIXME This is a known bug - parameter type mismatch must not be accepted
    assert!(frag_parse!("%s%d", "%s%s__test__42").is_none());
    //assert!(frag_parse!("%s%d?", "%s%s__test__42").is_none());
    //assert!(frag_parse!("%s%d?*", "%s%s__test__42").is_none());
}
