//! §5.5.6 original_text 测试 — 验证 custom property 的原始 source text 被正确捕获。

use muskitty_css_parser::parse_a_declaration;

#[test]
fn custom_property_captures_original_text() {
    let decl = parse_a_declaration("--foo: 10px solid red").unwrap();
    assert_eq!(decl.name, "--foo");
    // original_text 是 value tokens 的原始 source（前导空格已被 Step 4
    // discard_whitespace 跳过，不含前导空格）
    assert_eq!(
        decl.original_text.as_deref(),
        Some("10px solid red"),
        "original_text should be the raw source of value tokens (no leading ws)"
    );
}

#[test]
fn non_custom_property_has_no_original_text() {
    let decl = parse_a_declaration("color: red").unwrap();
    assert_eq!(
        decl.original_text, None,
        "non-custom properties should not have original_text"
    );
}

#[test]
fn custom_property_with_calc_preserves_original_text() {
    let decl = parse_a_declaration("--bar: calc(100% - 20px)").unwrap();
    assert_eq!(decl.name, "--bar");
    assert_eq!(
        decl.original_text.as_deref(),
        Some("calc(100% - 20px)"),
        "calc() source text preserved as-is (no leading ws)"
    );
}

#[test]
fn custom_property_with_no_space_after_colon() {
    let decl = parse_a_declaration("--x:10px").unwrap();
    assert_eq!(decl.name, "--x");
    // Step 4 discards whitespace after colon, but source_slice captures
    // from value_start_index (after ws discard) to value_end_index.
    // So original_text should be "10px" (no leading space).
    assert_eq!(decl.original_text.as_deref(), Some("10px"));
}

#[test]
fn custom_property_empty_value() {
    // --foo: ;  → empty value (just semicolon)
    let decl = parse_a_declaration("--foo: ;").unwrap();
    assert_eq!(decl.name, "--foo");
    // value is empty, original_text should be empty or whitespace-only
    // value_start_index == value_end_index (no tokens consumed before ;)
    assert!(decl.value.is_empty());
}

#[test]
fn custom_property_with_important_still_has_original_text() {
    let decl = parse_a_declaration("--foo: 10px !important").unwrap();
    assert_eq!(decl.name, "--foo");
    assert!(decl.important);
    // original_text captures everything before !important was stripped.
    // After Step 7 strips trailing whitespace, value is "10px".
    // But original_text is captured from source_slice, which includes
    // the original source up to the semicolon (or where !important was).
    // The value_end_index points to the token after value consumption,
    // which is the `!` of `!important` or the `;`.
    assert!(
        decl.original_text.is_some(),
        "custom property with !important should still have original_text"
    );
}

#[test]
fn custom_property_with_comment_in_value() {
    let decl = parse_a_declaration("--foo: 10px /* comment */ red").unwrap();
    assert_eq!(decl.name, "--foo");
    // original_text preserves the comment (source is raw)
    let text = decl.original_text.as_deref().unwrap();
    assert!(text.contains("10px"), "should contain 10px");
    assert!(text.contains("comment"), "should preserve comment");
    assert!(text.contains("red"), "should contain red");
}
