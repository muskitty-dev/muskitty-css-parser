//! Token stream (§5.3 L1722-1814).
//!
//! A token stream is a struct representing a stream of tokens and/or
//! component values. It has three fields: `tokens` (a list), `index`
//! (current position), and `marked_indexes` (a stack of saved positions
//! for backtracking).

use crate::types::ParseError;
use muskitty_css_tokenizer::{CssTokenizer, Token, Tokenizer};
use std::cell::{Cell, RefCell};

/// §5.5 解析器最大嵌套深度。
///
/// `consume_a_simple_block` / `consume_a_function` 通过
/// `consume_a_component_value` 间接递归；恶意 CSS（如 10,000 层
/// `{{{{...}}}}` 或 `((((...))))`）可触发栈溢出。此上限用于在
/// 递归到达危险深度前停止下降，记录 [`ParseError::NestingTooDeep`]。
///
/// 参考实现：
/// - Chromium `kMaxCSSTokenizerNestingLevel = 100`
/// - Firefox `kMaxNesting = 200`
/// - Servo `MAX_PARSER_NESTING_DEPTH = 100`
///
/// 审计 F-3：1024 远超所有主流实现（5-10 倍），且每层在语法解析 + 下游
/// （calc 求值、选择器解析）各叠 ~2-3 帧调用栈，1000 层即逼近线程栈余量。
/// 取 Firefox 的 200，与最宽的主流上限平齐。
///
/// [`ParseError::NestingTooDeep`]: crate::types::ParseError::NestingTooDeep
pub const MAX_NESTING_DEPTH: u32 = 200;

/// §5.3 L1725-1754: A token stream.
#[derive(Debug, Clone)]
pub struct TokenStream {
    /// §5.3 L1730-1738: A list of tokens and/or component values. We
    /// model component values as `Token`s here for simplicity; the
    /// §5.5 algorithms that consume "component values" wrap tokens
    /// into [`crate::types::ComponentValue`] at the boundary.
    pub tokens: Vec<Token>,
    /// §5.3 L1740-1748: An index into `tokens`, representing parsing
    /// progress. Starts at 0. Never decreases except via
    /// [`Self::restore_mark`].
    pub index: usize,
    /// §5.3 L1750-1753: A stack of index values for backtracking.
    /// Starts empty.
    marked_indexes: Vec<usize>,
    /// 与 `tokens` 平行的 byte ranges。空 Vec 表示无 source tracking
    /// （`new()` 构造的 stream 无 source）。由 [`Self::with_source`] 填充。
    token_spans: Vec<std::ops::Range<usize>>,
    /// 原始 source text。`None` 表示无 source tracking。
    source: Option<String>,
    /// §5.5 递归保护：当前嵌套深度（`consume_a_simple_block` /
    /// `consume_a_function` 的递归深度）。进入 +1，退出 -1。
    depth: Cell<u32>,
    /// 嵌套深度超限时记录的错误。解析继续（按 §5.5 错误恢复语义
    /// 停止下降），调用方可通过 [`Self::nesting_error`] 读取。
    nesting_error: RefCell<Option<ParseError>>,
}

impl TokenStream {
    /// §5.3: Construct a new token stream over `tokens`. The stream
    /// implicitly appends an EOF token (§5.3 L1811-1813); we model
    /// it by returning `Token::Eof` from [`Self::next_token`] when
    /// index is out of bounds, rather than storing a sentinel.
    ///
    /// 构造的 stream **不带** source-text tracking。如需追踪原始 source
    /// （用于 §5.5.6 `original_text`），请用 [`Self::with_source`]。
    pub fn new(mut tokens: Vec<Token>) -> Self {
        // Ensure an EOF token is present at the end (§5.3 L1811-1813).
        // The tokenizer already emits one, but be defensive.
        if tokens.last().is_none_or(|t| !matches!(t, Token::Eof)) {
            tokens.push(Token::Eof);
        }
        Self {
            tokens,
            index: 0,
            marked_indexes: Vec::new(),
            token_spans: Vec::new(),
            source: None,
            depth: Cell::new(0),
            nesting_error: RefCell::new(None),
        }
    }

    /// 构造一个带 source-text tracking 的 TokenStream。
    ///
    /// tokenize `source` 并记录每个 token 的 byte range，使
    /// [`Self::source_slice`] 能返回原始 source 片段。供 §5.5.6
    /// `original_text`（custom property）使用。
    pub fn with_source(source: &str) -> Self {
        let mut tz = CssTokenizer::new(source);
        // 建立 char-index → byte-offset 映射（tokenizer 内部用 char 索引）
        let char_to_byte: Vec<usize> = source.char_indices().map(|(i, _)| i).collect();
        let source_len = source.len();

        let mut tokens = Vec::new();
        let mut spans = Vec::new();
        while let Some((token, char_range)) = tz.next_token_with_span() {
            let start_byte = char_to_byte
                .get(char_range.start)
                .copied()
                .unwrap_or(source_len);
            let end_byte = char_to_byte
                .get(char_range.end)
                .copied()
                .unwrap_or(source_len);
            let is_eof = matches!(token, Token::Eof);
            tokens.push(token);
            spans.push(start_byte..end_byte);
            if is_eof {
                break;
            }
        }
        // 防御：确保 EOF 存在（与 new() 一致）
        if tokens.last().is_none_or(|t| !matches!(t, Token::Eof)) {
            tokens.push(Token::Eof);
            spans.push(source_len..source_len);
        }
        Self {
            tokens,
            index: 0,
            marked_indexes: Vec::new(),
            token_spans: spans,
            source: Some(source.to_string()),
            depth: Cell::new(0),
            nesting_error: RefCell::new(None),
        }
    }

    /// 记录嵌套深度超限错误（仅首次覆盖，保留最早的超限点）。
    pub fn record_nesting_error(&self, depth: u32) {
        if self.nesting_error.borrow().is_none() {
            self.nesting_error.replace(Some(ParseError::NestingTooDeep {
                depth,
                limit: MAX_NESTING_DEPTH,
            }));
        }
    }

    /// 返回解析过程中记录的第一个嵌套深度超限错误。
    pub fn nesting_error(&self) -> Option<ParseError> {
        self.nesting_error.borrow().clone()
    }

    /// 进入一层嵌套（`consume_a_simple_block` / `consume_a_function`
    /// 入口调用）。
    ///
    /// 返回 `Some(depth)` 表示该层已超过 [`MAX_NESTING_DEPTH`]：
    /// 错误已被记录，调用方应停止下降（不再递归）并返回空结果。
    /// 返回 `None` 表示正常进入。
    pub fn enter_nesting(&self) -> Option<u32> {
        let depth = self.depth.get() + 1;
        self.depth.set(depth);
        if depth > MAX_NESTING_DEPTH {
            self.record_nesting_error(depth);
            Some(depth)
        } else {
            None
        }
    }

    /// 退出一层嵌套（与 [`Self::enter_nesting`] 成对调用）。
    pub fn leave_nesting(&self) {
        self.depth.set(self.depth.get().saturating_sub(1));
    }

    /// 返回 `tokens[start_index..end_index]` 对应的原始 source text。
    ///
    /// 参数是 **token 索引**（不是 char/byte 索引）。
    /// 返回 `None` 如果无 source tracking 或 index 越界。
    pub fn source_slice(&self, start_index: usize, end_index: usize) -> Option<&str> {
        let source = self.source.as_ref()?;
        if end_index == 0 || start_index >= end_index {
            return Some("");
        }
        let start = self.token_spans.get(start_index)?.start;
        let end = self.token_spans.get(end_index - 1)?.end;
        source.get(start..end)
    }

    /// §5.3 L1769-1773: The item of `tokens` at `index`. If
    /// out-of-bounds, return `Token::Eof`.
    pub fn next_token(&self) -> Token {
        self.tokens.get(self.index).cloned().unwrap_or(Token::Eof)
    }

    /// §5.3 L1775-1777: A token stream is empty if the next token is
    /// `<EOF-token>`.
    pub fn is_empty(&self) -> bool {
        matches!(self.next_token(), Token::Eof)
    }

    /// §5.3 L1779-1782: Let `token` be the next token. Increment
    /// `index`, then return `token`.
    pub fn consume_token(&mut self) -> Token {
        let token = self.next_token();
        if !matches!(token, Token::Eof) {
            self.index += 1;
        }
        token
    }

    /// §5.3 L1784-1786: If not empty, increment `index`.
    pub fn discard_token(&mut self) {
        if !self.is_empty() {
            self.index += 1;
        }
    }

    /// §5.3 L1788-1789: Append `index` to `marked_indexes`.
    pub fn mark(&mut self) {
        self.marked_indexes.push(self.index);
    }

    /// §5.3 L1791-1793: Pop from `marked_indexes` and set `index` to
    /// the popped value. No-op if stack is empty (defensive).
    pub fn restore_mark(&mut self) {
        if let Some(idx) = self.marked_indexes.pop() {
            self.index = idx;
        }
    }

    /// §5.3 L1795-1797: Pop from `marked_indexes` and discard.
    pub fn discard_mark(&mut self) {
        let _ = self.marked_indexes.pop();
    }

    /// §5.3 L1799-1801: While the next token is a
    /// `<whitespace-token>`, discard a token.
    pub fn discard_whitespace(&mut self) {
        while matches!(self.next_token(), Token::Whitespace) {
            self.discard_token();
        }
    }
}
