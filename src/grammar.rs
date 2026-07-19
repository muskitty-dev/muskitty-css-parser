//! §5.4.1 / §5.4.2 grammar-based parse entry points.
//!
//! Two generic entry points defined by CSS Syntax Module Level 3:
//!
//! - §5.4.1 (L1895-1944) `parse something according to a CSS grammar`
//!   (aka [=CSS/parse=]): normalize → `parse a list of component
//!   values` → match the resulting component-value list against a
//!   grammar. Returns the grammar-specific output or failure.
//!
//! - §5.4.2 (L1949-2001) `parse a comma-separated list according to a
//!   CSS grammar` (aka [=CSS/parse a list=]): normalize → if input is
//!   whitespace-only return an empty list; otherwise `parse a
//!   comma-separated list of component values`, then match each group
//!   against the grammar. Returns a `Vec` of `Result`s preserving
//!   per-group failures.
//!
//! # Why a `Grammar` trait?
//!
//! The CSS Syntax spec leaves the actual grammar-matching to the spec
//! that defines the grammar (e.g. Selectors, Values & Units). We model
//! this as a trait so any crate can plug in its own grammar by
//! implementing [`Grammar`]:
//!
//! ```text
//! pub trait Grammar {
//!     type Output;
//!     fn parse(&self, input: &[ComponentValue]) -> Result<Self::Output, ParseError>;
//! }
//! ```
//!
//! # Spec references
//!
//! Source: `D:\CSSWG\css-syntax-3\Overview.md`, §5.4.1 L1895-1944,
//! §5.4.2 L1949-2001.

use crate::algorithms::consume_a_list_of_component_values;
use crate::entry_points::normalize_from_string;
use crate::token_stream::TokenStream;
use crate::types::{ComponentValue, ParseError};
use muskitty_css_tokenizer::Token;

/// §5.4.1: A CSS grammar production.
///
/// Implementors receive the component-value list produced by
/// [`parse_a_grammar`] / [`parse_a_comma_separated_list_with_grammar`]
/// and decide whether it matches the grammar. Returning `Ok` yields
/// the grammar-specific output; returning `Err` signals failure per
/// §5.4.1 L1944 ("otherwise, return failure").
pub trait Grammar {
    /// The grammar-specific parse result (e.g. `SelectorList` for the
    /// Selectors grammar, a color value for the CSS Color grammar).
    type Output;

    /// Match `input` (a list of component values) against this grammar.
    fn parse(&self, input: &[ComponentValue]) -> Result<Self::Output, ParseError>;
}

/// §5.4.1 (L1895-1944): Parse something according to a CSS grammar.
///
/// Algorithm (spec verbatim):
///
/// 1. Normalize `input`, and set `input` to the result.
/// 2. Parse a list of component values from `input`, and let `result`
///    be the return value.
/// 3. Attempt to match `result` against `grammar`. If this is
///    successful, return the matched result; otherwise, return
///    failure.
///
/// # Examples
///
/// ```ignore
/// use muskitty_css_parser::{Grammar, parse_a_grammar, ComponentValue, ParseError};
///
/// struct CountingGrammar;
/// impl Grammar for CountingGrammar {
///     type Output = usize;
///     fn parse(&self, input: &[ComponentValue]) -> Result<usize, ParseError> {
///         Ok(input.len())
///     }
/// }
///
/// assert_eq!(parse_a_grammar("a b c", &CountingGrammar).unwrap(), 5);
/// // 5 = 3 idents + 2 whitespace tokens.
/// ```
pub fn parse_a_grammar<G: Grammar>(input: &str, grammar: &G) -> Result<G::Output, ParseError> {
    // §5.4.1 step 1: Normalize input.
    let mut stream = normalize_from_string(input);
    // §5.4.1 step 2: Parse a list of component values.
    let result = consume_a_list_of_component_values(&mut stream, None, false);
    // §5.4.1 step 3: Match against grammar.
    grammar.parse(&result)
}

/// §5.4.2 (L1949-2001): Parse a comma-separated list according to a
/// CSS grammar.
///
/// Algorithm (spec verbatim):
///
/// 1. Normalize `input`, and set `input` to the result.
/// 2. If `input` contains only whitespace tokens, return an empty
///    list.
/// 3. Parse a comma-separated list of component values from `input`,
///    and let `list` be the return value.
/// 4. For each `item` of `list`, replace `item` with the result of
///    [=CSS/parsing=] `item` with `grammar`.
/// 5. Return `list`.
///
/// Each `item` is itself a `Vec<ComponentValue>` (one comma-group of
/// the comma-separated parse). The grammar is applied to each group
/// independently; failures are preserved per-group rather than
/// failing the entire call, per §5.4.2 L1956-1967.
pub fn parse_a_comma_separated_list_with_grammar<G: Grammar>(
    input: &str,
    grammar: &G,
) -> Vec<Result<G::Output, ParseError>> {
    // §5.4.2 step 1: Normalize input.
    let mut stream = normalize_from_string(input);
    // §5.4.2 step 2: If input contains only whitespace tokens, return
    // an empty list. Use mark/restore so we don't consume the leading
    // whitespace of a non-whitespace-only input (§5.4.10 preserves
    // leading whitespace within each group).
    stream.mark();
    stream.discard_whitespace();
    let only_whitespace = stream.is_empty();
    stream.restore_mark();
    if only_whitespace {
        return Vec::new();
    }
    // §5.4.2 step 3: Parse a comma-separated list of component values.
    let groups = consume_a_comma_separated_list_of_component_values(&mut stream);
    // §5.4.2 step 4: Apply grammar to each group.
    // §5.4.2 step 5: Return list.
    groups
        .into_iter()
        .map(|group| grammar.parse(&group))
        .collect()
}

/// §5.4.2 step 3 helper: Parse a comma-separated list of component
/// values from a token stream.
///
/// This is the same algorithm as
/// [`crate::entry_points::parse_a_comma_separated_list_of_component_values`]
/// but operating on an already-normalized `TokenStream` rather than a
/// raw `&str` (so the caller can normalize once and reuse the stream).
fn consume_a_comma_separated_list_of_component_values(
    stream: &mut TokenStream,
) -> Vec<Vec<ComponentValue>> {
    let mut groups = Vec::new();
    while !stream.is_empty() {
        // §5.4.10 L2199-2201: consume a list of component values with
        // `,` as the stop token.
        let group = consume_a_list_of_component_values(stream, Some(Token::Comma), false);
        groups.push(group);
        // §5.4.10 L2202: discard a token (the comma). On EOF, this is
        // a no-op (§5.3 L1784-1786).
        stream.discard_token();
    }
    groups
}
