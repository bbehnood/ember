//! Converts raw Ember source bytes into a flat stream of [`Token`]s.
//!
//! The lexer works over `&[u8]` rather than `&str` since Ember source is
//! restricted to ASCII; this lets identifiers and other slices borrow
//! directly from the input without needing UTF-8 validation on every token.

use thiserror::Error;

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
}

/// Errors that can occur while lexing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LexError {
    /// A character was encountered that isn't part of any valid token,
    /// e.g. `@`.
    #[error("unexpected character '{0}'")]
    UnexpectedChar(char),

    /// A run of digits didn't fit in an `i64` (or otherwise failed to
    /// parse), e.g. a number with far too many digits.
    #[error("invalid number literal")]
    InvalidNumber,

    /// A string literal wasn't terminated correctly, e.g. `"string`
    #[error("unterminated string literal '{0}'")]
    UnterminatedString(String),
}

impl<'a> Lexer<'a> {
    #[must_use]
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, current: 0 }
    }

    /// Scans the entire input and returns the full list of tokens,
    /// terminated by a trailing [`Token::Eof`].
    ///
    /// Stops and returns an error as soon as an invalid token is
    /// encountered; the lexer does not attempt error recovery.
    pub fn tokenize(&mut self) -> Result<Vec<Token<'a>>, LexError> {
        let mut tokens = Vec::new();
        let mut eof = false;

        while !eof {
            let token = self.next_token()?;
            eof = token == Token::Eof;

            tokens.push(token);
        }

        Ok(tokens)
    }

    /// Returns the byte at the current position without consuming it, or
    /// `None` if the input has been exhausted.
    fn peek(&self) -> Option<u8> {
        self.input.get(self.current).copied()
    }

    /// Consumes one byte, moving `current` forward.
    fn advance(&mut self) {
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
    fn read_number(&mut self) -> Result<Token<'a>, LexError> {
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
            string.parse().map_err(|_| LexError::InvalidNumber)?;

        Ok(Token::Number(number))
    }

    /// Scans a string literal starting from the current position.
    ///
    /// Assumes the current character is a double quote.
    fn read_string(&mut self) -> Result<Token<'a>, LexError> {
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
            return Err(LexError::UnterminatedString(
                String::from_utf8_lossy(&self.input[start - 1..self.current])
                    .to_string(),
            ));
        }

        self.advance();

        Ok(Token::String(string))
    }

    /// Scans and returns the next single token from the input, skipping
    /// leading whitespace. Returns [`Token::Eof`] once the input is
    /// exhausted.
    fn next_token(&mut self) -> Result<Token<'a>, LexError> {
        self.skip_whitespace();

        let token = match self.peek() {
            Some(ch) if ch.is_ascii_digit() => return self.read_number(),

            Some(ch) if ch.is_ascii_alphabetic() || ch == b'_' => {
                return Ok(self.read_identifier());
            }

            Some(b'"') => self.read_string()?,

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
                    _ => return Err(LexError::UnexpectedChar(ch as char)),
                }
            }

            None => Token::Eof,
        };

        Ok(token)
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

        assert_eq!(lexer.tokenize(), Err(LexError::UnexpectedChar('@')));
    }

    #[test]
    fn integer_overflow() {
        let mut lexer = Lexer::new(b"999999999999999999999999999999999");

        assert_eq!(lexer.tokenize(), Err(LexError::InvalidNumber),);
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
            Err(LexError::UnterminatedString("\"hello".to_string()))
        );
    }
}
