# muskitty-css-parser

[English](README.md) | [简体中文](README.zh-CN.md)

[![crates.io](https://img.shields.io/crates/v/muskitty-css-parser.svg)](https://crates.io/crates/muskitty-css-parser)
[![Documentation](https://docs.rs/muskitty-css-parser/badge.svg)](https://docs.rs/muskitty-css-parser)
[![License](https://img.shields.io/crates/l/muskitty-css-parser.svg)](https://github.com/muskitty-dev/muskitty-css-parser/blob/main/LICENSE)
[![CI](https://github.com/muskitty-dev/muskitty-css-parser/actions/workflows/ci.yml/badge.svg)](https://github.com/muskitty-dev/muskitty-css-parser/actions/workflows/ci.yml)

一个用纯 Rust 从零编写的 CSS 解析器，在 [`muskitty-css-tokenizer`](https://crates.io/crates/muskitty-css-tokenizer) 之上实现了 [CSS Syntax Module Level 3 §5](https://drafts.csswg.org/css-syntax-3/#tokenization) 的解析器层。

它是 [MusKitty](https://github.com/muskitty-dev) 浏览器引擎项目的组成部分。

## 状态

| 组件 | 规范覆盖 | 测试通过率 |
|-----------|---------------|----------------|
| **Parser** (§5) | 8 个入口点 + 13 个消费算法 | 62/62 测试 |

- 零 `unsafe` 代码
- 零 C/C++ 依赖
- 单个运行时依赖：`muskitty-css-tokenizer`
- 仅使用 Rust stable 工具链
- MSRV 1.82

## 安装

将以下内容添加到你的 `Cargo.toml`：

```toml
[dependencies]
muskitty-css-parser = "0.1.0"
```

或运行：

```bash
cargo add muskitty-css-parser
```

## 快速上手

```rust
use muskitty_css_parser::{parse_stylesheet, parse_rule, Rule};

// Parse a full stylesheet (§5.4.3)
let ss = parse_stylesheet("a { color: red; } body { font-size: 12px; }");
assert_eq!(ss.rules.len(), 2);

// Parse a single rule (§5.4.6)
let rule = parse_rule("@media print { body { color: black; } }").unwrap();
assert!(matches!(rule, Rule::AtRule(_)));
```

## 架构

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

### 什么是 CSS 解析器？

CSS 解析器是 CSS Syntax §3.1 两阶段模型中的第二阶段：

1. **分词**（委托给 `muskitty-css-tokenizer`）— 在 §5.3 预处理之后生成 token 流。
2. **解析**（本 crate）— 消费 token 并生成 CSS 对象，
   如样式表、规则、声明和组件值。

该解析器暴露了 8 个入口点（§5.4）和 13 个消费算法（§5.5）。

### 规范覆盖

所有 §5 解析器入口点和消费算法均已实现：

**入口点（§5.4）：**

- §5.4.1 Parse a stylesheet
- §5.4.2 Parse a list of rules
- §5.4.3 Parse a rule
- §5.4.4 Parse a declaration
- §5.4.5 Parse a list of declarations
- §5.4.6 Parse a component value
- §5.4.7 Parse a list of component values
- §5.4.8 Parse a comma-separated list of component values

**消费算法（§5.5）：**

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

## 构建

```bash
cargo check
cargo build
```

## 测试

```bash
# Integration tests (55 tests across 6 files)
cargo test --tests

# Doctests (7 tests)
cargo test --doc

# All tests (62 tests)
cargo test
```

## 设计原则

1. **CSSWG 是唯一权威** — 实现严格遵循规范。
2. **对齐规范，而非对齐测试** — 测试用于验证代码；除非规范证明测试有误，否则绝不为了通过测试而修改代码。
3. **零运行时依赖**（除 `muskitty-css-tokenizer` 外）— 纯安全的 Rust。
4. **零 unsafe** — 纯安全的 Rust。
5. **外科手术式修改** — 每次改动都尽可能小，仅满足任务所需。

## 规范参考

本实现参考了：

- [CSS Syntax Module Level 3](https://drafts.csswg.org/css-syntax-3/) — 主要权威
  - §3.1: CSS Parsing Overview (two-stage model)
  - §5: Parser Algorithms (entry points + consume algorithms)
  - §5.3: Input Stream Preprocessing (in tokenizer)

## 许可证

基于 Apache License, Version 2.0 授权。详见 [LICENSE](LICENSE)。

Copyright 2026 MusCat / MusKitty Bit-Torch Community
