# muskitty-css-parser

[![crates.io](https://img.shields.io/crates/v/muskitty-css-parser.svg)](https://crates.io/crates/muskitty-css-parser)
[![Documentation](https://docs.rs/muskitty-css-parser/badge.svg)](https://docs.rs/muskitty-css-parser)
[![License](https://img.shields.io/crates/l/muskitty-css-parser.svg)](https://github.com/muskitty-dev/muskitty-css-parser/blob/main/LICENSE)
[![CI](https://github.com/muskitty-dev/muskitty-css-parser/actions/workflows/ci.yml/badge.svg)](https://github.com/muskitty-dev/muskitty-css-parser/actions/workflows/ci.yml)

A from-scratch CSS parser written in pure Rust, implementing the
parser layer of the [CSS Syntax Module Level 3 §5](https://drafts.csswg.org/css-syntax-3/#tokenization)
on top of [`muskitty-css-tokenizer`](https://crates.io/crates/muskitty-css-tokenizer).

Part of the [MusKitty](https://github.com/muskitty-dev) browser engine project.

## Status

| Component | Spec Coverage | Test Pass Rate |
|-----------|---------------|----------------|
| **Parser** (§5) | 8 entry points + 13 consume algorithms | 62/62 tests |

- Zero `unsafe` code
- Zero C/C++ dependencies
- One runtime dependency: `muskitty-css-tokenizer`
- Rust stable toolchain only
- MSRV 1.82

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
muskitty-css-parser = "0.1.0"
```

Or run:

```bash
cargo add muskitty-css-parser
```

## Quick Start

```rust
use muskitty_css_parser::{parse_stylesheet, parse_rule, Rule};

// Parse a full stylesheet (§5.4.3)
let ss = parse_stylesheet("a { color: red; } body { font-size: 12px; }");
assert_eq!(ss.rules.len(), 2);

// Parse a single rule (§5.4.6)
let rule = parse_rule("@media print { body { color: black; } }").unwrap();
assert!(matches!(rule, Rule::AtRule(_)));
```

## Architecture

```
muskitty-css-parser/
  src/
    types.rs          Stylesheet, Rule, QualifiedRule, AtRule, Declaration,
                      ComponentValue, SimpleBlock, Function, ParseError, BlockKind
    token_stream.rs  TokenStream — §5.1 input stream wrapper over tokenizer
    algorithms.rs     §5.5 consume_* algorithms (qualified rule, at-rule,
                      declaration, simple block, function, unicode-range, etc.)
    entry_points.rs   §5.4 parse_* entry points
    lib.rs            Public API: 7 top-level functions + re-exports
```

### What is a CSS Parser?

The CSS parser is the second stage of the two-stage model in CSS Syntax §3.1:

1. **Tokenization** (delegated to `muskitty-css-tokenizer`) — produces a
   token stream after §5.3 preprocessing.
2. **Parsing** (this crate) — consumes tokens and produces CSS objects
   such as stylesheets, rules, declarations, and component values.

The parser exposes 8 entry points (§5.4) and 13 consume algorithms (§5.5).

### Spec Coverage

All §5 parser entry points and consume algorithms are implemented:

**Entry points (§5.4):**

- §5.4.1 Parse a stylesheet
- §5.4.2 Parse a list of rules
- §5.4.3 Parse a rule
- §5.4.4 Parse a declaration
- §5.4.5 Parse a list of declarations
- §5.4.6 Parse a component value
- §5.4.7 Parse a list of component values
- §5.4.8 Parse a comma-separated list of component values

**Consume algorithms (§5.5):**

- §5.5.1 Consume a stylesheet's contents
- §5.5.2 Consume a list of rules
- §5.5.3 Consume an at-rule
- §5.5.4 Consume a qualified rule
- §5.5.5 Consume a declaration
- §5.5.6 Consume the remnants of a bad declaration
- §5.5.7 Consume a list of component values
- §5.5.8 Consume a simple block
- §5.5.9 Consume a block's contents
- §5.5.10 Consume a component value
- §5.5.11 Consume a function
- §5.5.12 Consume a unicode-range value (referenced from §5.4.6)

## Building

```bash
cargo check
cargo build
```

## Testing

```bash
# Integration tests (55 tests across 6 files)
cargo test --tests

# Doctests (7 tests)
cargo test --doc

# All tests (62 tests)
cargo test
```

## Design Principles

1. **CSSWG is ground truth** — Implementation follows the spec exactly.
2. **Spec-compliant, not test-compliant** — Tests verify the code; code is never modified to pass a test unless the spec proves the test is wrong.
3. **Zero runtime dependencies** (besides `muskitty-css-tokenizer`) — Pure safe Rust.
4. **Zero unsafe** — Pure safe Rust.
5. **Surgical changes** — Every diff is as small as the task requires.

## Spec Reference

This implementation references:

- [CSS Syntax Module Level 3](https://drafts.csswg.org/css-syntax-3/) — Primary authority
  - §3.1: CSS Parsing Overview (two-stage model)
  - §5: Parser Algorithms (entry points + consume algorithms)
  - §5.3: Input Stream Preprocessing (in tokenizer)

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

Copyright 2026 MusCat / MusKitty Bit-Torch Community
