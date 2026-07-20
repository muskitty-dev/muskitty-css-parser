//! §5.4.1 / §5.4.2 grammar-hook tests (CP-7).
//!
//! Verifies the two generic grammar entry points defined in
//! [`muskitty_css_parser::grammar`]:
//!
//! - §5.4.1 `parse_a_grammar` — normalize → consume a list of
//!   component values → match against a `Grammar`.
//! - §5.4.2 `parse_a_comma_separated_list_with_grammar` — normalize →
//!   split on top-level commas → match each group against a `Grammar`,
//!   preserving per-group failures.
//!
//! Tests use stub grammars (`CountingGrammar`, `FirstTokenGrammar`,
//! `RejectingGrammar`) so the generic plumbing can be verified
//! independently of any specific CSS grammar (Selectors, Values,
//! Colors, etc.).

use muskitty_css_parser::{
    parse_a_comma_separated_list_with_grammar, parse_a_grammar, ComponentValue, Grammar,
    ParseError, Token,
};

// ===========================================================================
// Stub grammars
// ===========================================================================

/// Returns the count of component values received.
struct CountingGrammar;

impl Grammar for CountingGrammar {
    type Output = usize;
    fn parse(&self, input: &[ComponentValue]) -> Result<usize, ParseError> {
        Ok(input.len())
    }
}

/// Returns the first component value as a cloned `Token` (best-effort:
/// extracts the inner token if it's a `PreservedToken`). Returns
/// `Err(ParseError)` if the list is empty.
struct FirstTokenGrammar;

impl Grammar for FirstTokenGrammar {
    type Output = Token;
    fn parse(&self, input: &[ComponentValue]) -> Result<Token, ParseError> {
        match input.first() {
            Some(ComponentValue::PreservedToken(t)) => Ok(t.clone()),
            Some(_) => Ok(Token::Eof), // non-token CV (function/block) — stub
            None => Err(ParseError::new("empty input")),
        }
    }
}

/// Always fails — used to verify failure propagation.
struct RejectingGrammar;

impl Grammar for RejectingGrammar {
    type Output = ();
    fn parse(&self, _input: &[ComponentValue]) -> Result<(), ParseError> {
        Err(ParseError::new("rejected by grammar"))
    }
}

// ===========================================================================
// §5.4.1 parse_a_grammar
// ===========================================================================

/// §5.4.1: Empty input → empty component-value list → grammar receives
/// empty slice.
#[test]
fn parse_a_grammar_empty_input() {
    let n = parse_a_grammar("", &CountingGrammar).unwrap();
    assert_eq!(n, 0);
}

/// §5.4.1: A single ident → one component value.
#[test]
fn parse_a_grammar_single_ident() {
    let n = parse_a_grammar("a", &CountingGrammar).unwrap();
    assert_eq!(n, 1);
}

/// §5.4.1: "a b c" → 5 component values (3 idents + 2 whitespace).
#[test]
fn parse_a_grammar_three_idents_with_whitespace() {
    let n = parse_a_grammar("a b c", &CountingGrammar).unwrap();
    assert_eq!(n, 5);
}

/// §5.4.1: Commas are preserved as component values (not split).
/// "a, b" → 4 component values: Ident, Comma, Whitespace, Ident.
#[test]
fn parse_a_grammar_comma_not_split() {
    let n = parse_a_grammar("a, b", &CountingGrammar).unwrap();
    assert_eq!(n, 4);
}

/// §5.4.1: Whitespace-only input → 1 whitespace component value.
#[test]
fn parse_a_grammar_whitespace_only() {
    let n = parse_a_grammar("   ", &CountingGrammar).unwrap();
    assert_eq!(n, 1);
}

/// §5.4.1: Grammar failure propagation.
#[test]
fn parse_a_grammar_failure_propagated() {
    let result = parse_a_grammar("a", &RejectingGrammar);
    assert!(result.is_err());
}

/// §5.4.1: Grammar can inspect component-value content. The first
/// token of "red" should be `Token::Ident("red")`.
#[test]
fn parse_a_grammar_first_token_extracted() {
    let tok = parse_a_grammar("red", &FirstTokenGrammar).unwrap();
    assert!(matches!(tok, Token::Ident(ref s) if s == "red"));
}

/// §5.4.1: FirstTokenGrammar fails on empty input (no first token).
#[test]
fn parse_a_grammar_first_token_empty_fails() {
    let result = parse_a_grammar("", &FirstTokenGrammar);
    assert!(result.is_err());
}

/// §5.4.1: Comments are stripped during tokenization (§4.3.2B), so
/// "a/* x */b" tokenizes to [Ident("a"), Ident("b")] with no inserted
/// whitespace — the comment is gone but no whitespace is synthesized.
#[test]
fn parse_a_grammar_comments_stripped() {
    let n = parse_a_grammar("a/* x */b", &CountingGrammar).unwrap();
    assert_eq!(n, 2);
}

// ===========================================================================
// §5.4.2 parse_a_comma_separated_list_with_grammar
// ===========================================================================

/// §5.4.2 step 2: Empty input → empty list.
#[test]
fn parse_a_comma_separated_list_empty_input() {
    let result: Vec<Result<usize, ParseError>> =
        parse_a_comma_separated_list_with_grammar("", &CountingGrammar);
    assert!(result.is_empty());
}

/// §5.4.2 step 2: Whitespace-only input → empty list (§5.4.2 L1956).
#[test]
fn parse_a_comma_separated_list_whitespace_only() {
    let result: Vec<Result<usize, ParseError>> =
        parse_a_comma_separated_list_with_grammar("   ", &CountingGrammar);
    assert!(result.is_empty());
}

/// §5.4.2 step 2: Whitespace + tabs + newlines (all whitespace) →
/// empty list.
#[test]
fn parse_a_comma_separated_list_mixed_whitespace_only() {
    let result: Vec<Result<usize, ParseError>> =
        parse_a_comma_separated_list_with_grammar(" \t\n  ", &CountingGrammar);
    assert!(result.is_empty());
}

/// §5.4.2 step 3: Single group (no comma).
#[test]
fn parse_a_comma_separated_list_single_group() {
    let result: Vec<Result<usize, ParseError>> =
        parse_a_comma_separated_list_with_grammar("a", &CountingGrammar);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].as_ref().unwrap(), &1);
}

/// §5.4.2 step 3: Three groups separated by commas.
/// "a, b, c" → groups: [a], [ws, b], [ws, c] → counts 1, 2, 2.
#[test]
fn parse_a_comma_separated_list_three_groups() {
    let result: Vec<Result<usize, ParseError>> =
        parse_a_comma_separated_list_with_grammar("a, b, c", &CountingGrammar);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].as_ref().unwrap(), &1);
    assert_eq!(result[1].as_ref().unwrap(), &2);
    assert_eq!(result[2].as_ref().unwrap(), &2);
}

/// §5.4.2 step 3: Leading whitespace before first group is preserved
/// within the group (§5.4.10 does not discard leading whitespace).
/// "  a, b" → groups: [ws, a], [ws, b] → counts 2, 2.
#[test]
fn parse_a_comma_separated_list_leading_whitespace_preserved() {
    let result: Vec<Result<usize, ParseError>> =
        parse_a_comma_separated_list_with_grammar("  a, b", &CountingGrammar);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].as_ref().unwrap(), &2);
    assert_eq!(result[1].as_ref().unwrap(), &2);
}

/// §5.4.2 step 4: Per-group failures are preserved in the result list.
/// RejectingGrammar → every group is `Err(ParseError)`, but the Vec
/// itself is returned (no early-abort).
#[test]
fn parse_a_comma_separated_list_per_group_failure_preserved() {
    let result: Vec<Result<(), ParseError>> =
        parse_a_comma_separated_list_with_grammar("a, b, c", &RejectingGrammar);
    assert_eq!(result.len(), 3);
    for r in &result {
        assert!(r.is_err());
    }
}

/// §5.4.2: Trailing comma without trailing whitespace does NOT yield
/// an extra empty group — §5.4.10's loop checks `while input is not
/// empty`, and after consuming the comma the stream is at EOF.
/// "a, b," → 2 groups: [a], [ws, b].
#[test]
fn parse_a_comma_separated_list_trailing_comma_no_empty_group() {
    let result: Vec<Result<usize, ParseError>> =
        parse_a_comma_separated_list_with_grammar("a, b,", &CountingGrammar);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].as_ref().unwrap(), &1);
    assert_eq!(result[1].as_ref().unwrap(), &2);
}

/// §5.4.2: Trailing comma followed by trailing whitespace DOES yield
/// an extra group consisting of the trailing whitespace (§5.4.10 step
/// 2 sees a non-EOF token after the comma and re-enters the loop).
/// "a, b, " → 3 groups: [a], [ws, b], [ws].
#[test]
fn parse_a_comma_separated_list_trailing_comma_with_whitespace() {
    let result: Vec<Result<usize, ParseError>> =
        parse_a_comma_separated_list_with_grammar("a, b, ", &CountingGrammar);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].as_ref().unwrap(), &1);
    assert_eq!(result[1].as_ref().unwrap(), &2);
    assert_eq!(result[2].as_ref().unwrap(), &1);
}

/// §5.4.2: Function-call-like input. `rgba(1, 2, 3)` is parsed as a
/// single component value (a `Function`), so the commas inside are
/// NOT top-level. Single group → count 1.
#[test]
fn parse_a_comma_separated_list_function_args_not_split() {
    let result: Vec<Result<usize, ParseError>> =
        parse_a_comma_separated_list_with_grammar("rgba(1, 2, 3)", &CountingGrammar);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].as_ref().unwrap(), &1);
}
