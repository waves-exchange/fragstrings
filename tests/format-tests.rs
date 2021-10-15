use format_procmacro::frag_format;

#[test]
fn test_frag_format() {
    // frag_format!(); // Compile error
    // frag_format!(""); // Compile error
    // frag_format!("%s"); // Compile error
    // frag_format!("%s" foo, "%s__test"); // Compile error
    // frag_format!("%d%d", 42); // Compile error
    // frag_format!(42); // Compile error
    // frag_format!(xxx); // Compile error
    // frag_format!("xxx"); // Compile error
    // frag_format!("*"); // Compile error
    // frag_format!("*%d", 42); // Compile error
    // frag_format!("%d*", 42); // Compile error

    assert_eq!(frag_format!("%s", "test"), "%s__test");
    assert_eq!(frag_format!("%d", 42), "%d__42");

    let data_int = 42;
    let data_str = "test";
    let data_string = "test".to_string();
    let frag_int = frag_format!("%d", data_int);
    let frag_str = frag_format!("%s", data_str);
    let frag_string = frag_format!("%s", data_string);
    assert_eq!(frag_int, "%d__42");
    assert_eq!(frag_str, "%s__test");
    assert_eq!(frag_string, "%s__test");

    assert_eq!(frag_format!(r"%s", "test"), "%s__test");
    assert_eq!(frag_format!(r#"%s"#, "test"), "%s__test");
    assert_eq!(frag_format!(r##"%s"##, "test"), "%s__test");
    assert_eq!(frag_format!(r###"%s"###, "test"), "%s__test");
    assert_eq!(frag_format!(b"%s", "test"), "%s__test");

    assert_eq!(frag_format!("%d", 2 + 2), "%d__4");
    assert_eq!(frag_format!("%d", (2 + 2) * 2), "%d__8");
    assert_eq!(frag_format!("%d", int_fn(1, 2) * 3), "%d__9");

    assert_eq!(frag_format!("%s%d", "test", 42), "%s%d__test__42");
    assert_eq!(frag_format!("%d%s", 42, "test"), "%d%s__42__test");

    assert_eq!(frag_format!(/* Comment */ "%s", "test"), "%s__test");
    assert_eq!(frag_format!("%s" /* Comment */, "test"), "%s__test");
    assert_eq!(frag_format!("%s", "test" /* Comment */), "%s__test");
}

fn int_fn(a: i32, b: i32) -> i32 {
    a + b
}
