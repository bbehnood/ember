//! Converts raw Ember source bytes into a flat stream of [`Token`]s.
//!
//! The lexer works over `&[u8]` rather than `&str` since Ember source is
//! restricted to ASCII; this lets identifiers and other slices borrow
//! directly from the input without needing UTF-8 validation on every token.

use thiserror::Error;

/// A 1-based line/column position within the source, used to point at the
/// location of an error in diagnostic messages.
///
/// Both fields are 1-based (the first character of the source is
/// `{ line: 1, col: 1 }`) to match how editors and most compilers report
/// positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub line: u32,
    pub col: u32,
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// A single lexical token, borrowing directly from the source buffer where
/// applicable (e.g. [`Token::Identifier`]).
///
/// `Token` is `Copy` since every variant is small (at most a slice
/// reference or an `i64`), which keeps the parser's lookahead (`peek`,
/// `peek_next`) cheap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token<'a> {
    /// An integer literal, already parsed into an `i64`.
    Number(i64),
    /// A string literal, parsed as a single token
    String(&'a [u8]),
    /// The `true` keyword.
    True,
    /// The `false` keyword.
    False,
    /// An identifier, e.g. a variable name. Borrowed from the source.
    Identifier(&'a [u8]),

    Let,
    Print,

    If,
    Else,
    While,

    Plus,
    Minus,
    Star,
    Slash,

    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    AndAnd,
    OrOr,

    Equal,
    Semicolon,

    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,

    /// Marks the end of the input. Always the last token produced by
    /// [`Lexer::tokenize`], which lets the parser treat "end of input" as
    /// just another token to match against instead of a special case.
    Eof,
}

/// Scans a byte slice of Ember source and produces a stream of [`Token`]s.
///
/// The lexer is a simple single-pass scanner with one character of
/// lookahead (see `Lexer::peek`), which is enough to disambiguate
/// multi-character operators like `==` from `=`.
pub struct Lexer<'a> {
    /// The full source buffer being scanned.
    input: &'a [u8],
    /// Byte offset of the next character to read from `input`.
    current: usize,
    /// 1-based line number of the next character to read.
    line: u32,
    /// 1-based column number of the next character to read.
    col: u32,
}

/// Errors that can occur while lexing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LexError {
    /// A character was encountered that isn't part of any valid token,
    /// e.g. `@`.
    #[error("{pos}: unexpected character '{ch}'")]
    UnexpectedChar { ch: char, pos: Position },

    /// A run of digits didn't fit in an `i64` (or otherwise failed to
    /// parse), e.g. a number with far too many digits.
    #[error("{pos}: invalid number literal")]
    InvalidNumber { pos: Position },

    /// A string literal wasn't terminated correctly, e.g. `"string`
    #[error("{pos}: unterminated string literal '{string}'")]
    UnterminatedString { string: String, pos: Position },
}

impl<'a> Lexer<'a> {
    #[must_use]
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, current: 0, line: 1, col: 1 }
    }

    /// Scans the entire input and returns the full list of tokens,
    /// terminated by a trailing [`Token::Eof`].
    ///
    /// Stops and returns an error as soon as an invalid token is
    /// encountered; the lexer does not attempt error recovery.
    ///
    /// This discards the position of each token - use
    /// [`Self::tokenize_with_positions`] when that's needed (e.g. to give
    /// [`crate::parser::ParseError`] a location to report).
    pub fn tokenize(&mut self) -> Result<Vec<Token<'a>>, LexError> {
        Ok(self
            .tokenize_with_positions()?
            .into_iter()
            .map(|(token, _)| token)
            .collect())
    }

    /// Scans the entire input like [`Self::tokenize`], but also returns the
    /// [`Position`] where each token starts, in lockstep with the returned
    /// tokens (so `tokens[i]` starts at `positions[i]`).
    pub fn tokenize_with_positions(
        &mut self,
    ) -> Result<Vec<(Token<'a>, Position)>, LexError> {
        let mut tokens = Vec::new();
        let mut eof = false;

        while !eof {
            let (token, pos) = self.next_token()?;
            eof = token == Token::Eof;

            tokens.push((token, pos));
        }

        Ok(tokens)
    }

    /// Returns the byte at the current position without consuming it, or
    /// `None` if the input has been exhausted.
    fn peek(&self) -> Option<u8> {
        self.input.get(self.current).copied()
    }

    /// Returns the current line/column, i.e. the position of the next
    /// character [`Self::advance`] would consume.
    fn position(&self) -> Position {
        Position { line: self.line, col: self.col }
    }

    /// Consumes one byte, moving `current` forward and updating `line`/`col`
    /// to track the position of the *next* byte. A newline resets `col`
    /// back to 1 and advances `line`; anything else just advances `col`.
    fn advance(&mut self) {
        if self.peek() == Some(b'\n') {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }

        self.current += 1;
    }

    /// Consumes any run of whitespace at the current position. Ember has no
    /// significant whitespace, so this is simply discarded between tokens.
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek()
            && ch.is_ascii_whitespace()
        {
            self.advance();
        }
    }

    /// Scans an identifier or keyword starting at the current position.
    ///
    /// Assumes the caller has already checked that the current character is
    /// a valid identifier start (alphabetic or `_`). Keywords (`let`,
    /// `print`, `if`, etc.) are recognized here by matching the scanned text
    /// against the keyword list; anything else becomes a
    /// [`Token::Identifier`].
    fn read_identifier(&mut self) -> Token<'a> {
        let start = self.current;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == b'_' {
                self.advance();
            } else {
                break;
            }
        }

        let ident = &self.input[start..self.current];

        match ident {
            b"let" => Token::Let,
            b"print" => Token::Print,

            b"if" => Token::If,
            b"else" => Token::Else,
            b"while" => Token::While,

            b"true" => Token::True,
            b"false" => Token::False,

            _ => Token::Identifier(ident),
        }
    }

    /// Scans an integer literal starting at the current position.
    ///
    /// Only consumes ASCII digits (no decimals, signs, or exponents - unary
    /// minus is handled by the parser, not the lexer). Returns
    /// [`LexError::InvalidNumber`] if the digits don't fit in an `i64`.
    ///
    /// `pos` is the position of the first digit, used to locate the error
    /// if the literal is invalid.
    fn read_number(&mut self, pos: Position) -> Result<Token<'a>, LexError> {
        let start = self.current;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        let string = std::str::from_utf8(&self.input[start..self.current])
            .expect("Only valid ASCII gets consumed");

        let number: i64 =
            string.parse().map_err(|_| LexError::InvalidNumber { pos })?;

        Ok(Token::Number(number))
    }

    /// Scans a string literal starting from the current position.
    ///
    /// Assumes the current character is a double quote. `pos` is the
    /// position of that opening quote, used to locate the error if the
    /// string is never closed.
    fn read_string(&mut self, pos: Position) -> Result<Token<'a>, LexError> {
        self.advance();

        let start = self.current;

        while let Some(ch) = self.peek() {
            if ch != b'"' {
                self.advance();
            } else {
                break;
            }
        }

        let string = &self.input[start..self.current];

        if self.peek() != Some(b'"') {
            return Err(LexError::UnterminatedString {
                string: String::from_utf8_lossy(
                    &self.input[start - 1..self.current],
                )
                .to_string(),
                pos,
            });
        }

        self.advance();

        Ok(Token::String(string))
    }

    /// Scans and returns the next single token from the input, skipping
    /// leading whitespace, along with the [`Position`] where that token
    /// starts. Returns [`Token::Eof`] once the input is exhausted.
    fn next_token(&mut self) -> Result<(Token<'a>, Position), LexError> {
        self.skip_whitespace();

        let pos = self.position();

        let token = match self.peek() {
            Some(ch) if ch.is_ascii_digit() => self.read_number(pos)?,

            Some(ch) if ch.is_ascii_alphabetic() || ch == b'_' => {
                self.read_identifier()
            }

            Some(b'"') => self.read_string(pos)?,

            Some(ch) => {
                self.advance();

                match ch {
                    b'+' => Token::Plus,

                    b'-' => Token::Minus,

                    b'*' => Token::Star,

                    b'/' => Token::Slash,

                    // Two-character operators are matched by peeking ahead
                    // for the second character before falling through to
                    // the single-character variant below (e.g. `=` vs
                    // `==`).
                    b'=' if self.peek() == Some(b'=') => {
                        self.advance();
                        Token::EqualEqual
                    }

                    b'!' if self.peek() == Some(b'=') => {
                        self.advance();
                        Token::BangEqual
                    }

                    b'<' if self.peek() == Some(b'=') => {
                        self.advance();
                        Token::LessEqual
                    }

                    b'>' if self.peek() == Some(b'=') => {
                        self.advance();
                        Token::GreaterEqual
                    }

                    b'&' if self.peek() == Some(b'&') => {
                        self.advance();
                        Token::AndAnd
                    }

                    b'|' if self.peek() == Some(b'|') => {
                        self.advance();
                        Token::OrOr
                    }

                    b'<' => Token::Less,

                    b'>' => Token::Greater,

                    b'=' => Token::Equal,

                    b';' => Token::Semicolon,

                    b'(' => Token::LeftParen,

                    b')' => Token::RightParen,

                    b'{' => Token::LeftBrace,

                    b'}' => Token::RightBrace,

                    // Note: a lone `!` or `&` or `|` (not followed by their
                    // paired character) falls through to here, since Ember
                    // has no unary `!` or bitwise `&`/`|` operators.
                    _ => {
                        return Err(LexError::UnexpectedChar {
                            ch: ch as char,
                            pos,
                        });
                    }
                }
            }

            None => Token::Eof,
        };

        Ok((token, pos))
    }
}

/// Formats a token the way it should appear in diagnostic messages, e.g.
/// `expected 'if', found identifier 'x'` (see [`crate::parser::ParseError`]).
impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Let => write!(f, "'let'"),
            Token::Print => write!(f, "'print'"),
            Token::If => write!(f, "'if'"),
            Token::Else => write!(f, "'else'"),
            Token::While => write!(f, "'while'"),
            Token::Identifier(name) => {
                write!(f, "identifier '{}'", String::from_utf8_lossy(name))
            }
            Token::Number(n) => write!(f, "number '{n}'"),
            Token::String(s) => {
                write!(f, "string '{}'", String::from_utf8_lossy(s))
            }
            Token::True => write!(f, "boolean 'true'"),
            Token::False => write!(f, "boolean 'false'"),
            Token::Plus => write!(f, "'+'"),
            Token::Minus => write!(f, "'-'"),
            Token::Star => write!(f, "'*'"),
            Token::Slash => write!(f, "'/'"),
            Token::EqualEqual => write!(f, "'=='"),
            Token::BangEqual => write!(f, "'!='"),
            Token::Less => write!(f, "'<'"),
            Token::LessEqual => write!(f, "'<='"),
            Token::Greater => write!(f, "'>'"),
            Token::GreaterEqual => write!(f, "'>='"),
            Token::AndAnd => write!(f, "'&&'"),
            Token::OrOr => write!(f, "'||'"),
            Token::Equal => write!(f, "'='"),
            Token::Semicolon => write!(f, "';'"),
            Token::LeftParen => write!(f, "'('"),
            Token::RightParen => write!(f, "')'"),
            Token::LeftBrace => write!(f, "'{{'"),
            Token::RightBrace => write!(f, "'}}'"),
            Token::Eof => write!(f, "end of input"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let mut lexer = Lexer::new(b"");

        assert_eq!(lexer.tokenize().unwrap(), vec![Token::Eof]);
    }

    #[test]
    fn whitespace_only() {
        let mut lexer = Lexer::new(b" \t\n ");

        assert_eq!(lexer.tokenize().unwrap(), vec![Token::Eof])
    }

    #[test]
    fn keyword_let() {
        let mut lexer = Lexer::new(b"let");

        assert_eq!(lexer.tokenize().unwrap(), vec![Token::Let, Token::Eof])
    }

    #[test]
    fn identifiers() {
        let mut lexer = Lexer::new(b"foo bar _baz");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![
                Token::Identifier(b"foo"),
                Token::Identifier(b"bar"),
                Token::Identifier(b"_baz"),
                Token::Eof
            ]
        )
    }

    #[test]
    fn numbers() {
        let mut lexer = Lexer::new(b"123 456");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::Number(123), Token::Number(456), Token::Eof]
        )
    }

    #[test]
    fn operators() {
        let mut lexer = Lexer::new(b"+-*/=;()");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![
                Token::Plus,
                Token::Minus,
                Token::Star,
                Token::Slash,
                Token::Equal,
                Token::Semicolon,
                Token::LeftParen,
                Token::RightParen,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn simple_program() {
        let mut lexer = Lexer::new(b"let x = 42;");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![
                Token::Let,
                Token::Identifier(b"x"),
                Token::Equal,
                Token::Number(42),
                Token::Semicolon,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn keyword_prefix() {
        let mut lexer = Lexer::new(b"letter");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::Identifier(b"letter"), Token::Eof,]
        );
    }

    #[test]
    fn identifier_with_digits() {
        let mut lexer = Lexer::new(b"_foo123");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::Identifier(b"_foo123"), Token::Eof,]
        );
    }

    #[test]
    fn unexpected_character() {
        let mut lexer = Lexer::new(b"@");

        assert_eq!(
            lexer.tokenize(),
            Err(LexError::UnexpectedChar {
                ch: '@',
                pos: Position { line: 1, col: 1 }
            })
        );
    }

    #[test]
    fn unexpected_character_position_on_later_line() {
        let mut lexer = Lexer::new(b"let x = 1;\nlet y = @;");

        assert_eq!(
            lexer.tokenize(),
            Err(LexError::UnexpectedChar {
                ch: '@',
                pos: Position { line: 2, col: 9 }
            })
        );
    }

    #[test]
    fn integer_overflow() {
        let mut lexer = Lexer::new(b"999999999999999999999999999999999");

        assert_eq!(
            lexer.tokenize(),
            Err(LexError::InvalidNumber { pos: Position { line: 1, col: 1 } })
        );
    }

    #[test]
    fn adjacent_tokens() {
        let mut lexer = Lexer::new(b"x=123");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![
                Token::Identifier(b"x"),
                Token::Equal,
                Token::Number(123),
                Token::Eof
            ]
        )
    }

    #[test]
    fn comparison_operators() {
        let mut lexer = Lexer::new(b"== != <= >=");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![
                Token::EqualEqual,
                Token::BangEqual,
                Token::LessEqual,
                Token::GreaterEqual,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn relational_operators() {
        let mut lexer = Lexer::new(b"< >");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::Less, Token::Greater, Token::Eof]
        );
    }

    #[test]
    fn logical_operators() {
        let mut lexer = Lexer::new(b"&& ||");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::AndAnd, Token::OrOr, Token::Eof]
        );
    }

    #[test]
    fn braces() {
        let mut lexer = Lexer::new(b"{}");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::LeftBrace, Token::RightBrace, Token::Eof]
        );
    }

    #[test]
    fn boolean_literals() {
        let mut lexer = Lexer::new(b"true false");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::True, Token::False, Token::Eof]
        );
    }

    #[test]
    fn keyword_print() {
        let mut lexer = Lexer::new(b"print");

        assert_eq!(lexer.tokenize().unwrap(), vec![Token::Print, Token::Eof])
    }

    #[test]
    fn keyword_if_else() {
        let mut lexer = Lexer::new(b"if else");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::If, Token::Else, Token::Eof]
        )
    }

    #[test]
    fn keyword_while() {
        let mut lexer = Lexer::new(b"while");

        assert_eq!(lexer.tokenize().unwrap(), vec![Token::While, Token::Eof])
    }

    #[test]
    fn string() {
        let mut lexer = Lexer::new(b"\"hello\"");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::String(b"hello"), Token::Eof]
        );
    }

    #[test]
    fn unterminated_string() {
        let mut lexer = Lexer::new(b"\"hello");

        assert_eq!(
            lexer.tokenize(),
            Err(LexError::UnterminatedString {
                string: "\"hello".to_string(),
                pos: Position { line: 1, col: 1 }
            })
        );
    }

    #[test]
    fn tokenize_with_positions_tracks_line_and_col() {
        let mut lexer = Lexer::new(b"let x = 1;\nprint(x);");

        assert_eq!(
            lexer.tokenize_with_positions().unwrap(),
            vec![
                (Token::Let, Position { line: 1, col: 1 }),
                (Token::Identifier(b"x"), Position { line: 1, col: 5 }),
                (Token::Equal, Position { line: 1, col: 7 }),
                (Token::Number(1), Position { line: 1, col: 9 }),
                (Token::Semicolon, Position { line: 1, col: 10 }),
                (Token::Print, Position { line: 2, col: 1 }),
                (Token::LeftParen, Position { line: 2, col: 6 }),
                (Token::Identifier(b"x"), Position { line: 2, col: 7 }),
                (Token::RightParen, Position { line: 2, col: 8 }),
                (Token::Semicolon, Position { line: 2, col: 9 }),
                (Token::Eof, Position { line: 2, col: 10 }),
            ]
        );
    }
}
