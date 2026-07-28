use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token<'a> {
    Let,
    Number(i64),
    Identifier(&'a [u8]),

    Plus,
    Minus,
    Star,
    Slash,

    Equal,
    Semicolon,

    LeftParen,
    RightParen,

    Eof,
}

pub struct Lexer<'a> {
    input: &'a [u8],
    current: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LexError {
    #[error("unexpected character '{0}'")]
    UnexpectedChar(char),

    #[error("invalid number literal")]
    InvalidNumber,
}

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Let => write!(f, "'let'"),
            Token::Identifier(name) => {
                write!(f, "identifier '{}'", String::from_utf8_lossy(name))
            }
            Token::Number(n) => write!(f, "number '{n}'"),
            Token::Plus => write!(f, "'+'"),
            Token::Minus => write!(f, "'-'"),
            Token::Star => write!(f, "'*'"),
            Token::Slash => write!(f, "'/'"),
            Token::Equal => write!(f, "'='"),
            Token::Semicolon => write!(f, "';'"),
            Token::LeftParen => write!(f, "'('"),
            Token::RightParen => write!(f, "')'"),
            Token::Eof => write!(f, "end of input"),
        }
    }
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, current: 0 }
    }

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

    fn peek(&self) -> Option<u8> {
        self.input.get(self.current).copied()
    }

    fn _peek_next(&self) -> Option<u8> {
        self.input.get(self.current + 1).copied()
    }

    fn advance(&mut self) {
        self.current += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek()
            && ch.is_ascii_whitespace()
        {
            self.advance();
        }
    }

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
            _ => Token::Identifier(ident),
        }
    }

    fn read_number(&mut self) -> Result<Token<'a>, LexError> {
        let start = self.current;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        let string = unsafe {
            // SAFETY: Only valid ASCII gets consumed
            std::str::from_utf8_unchecked(&self.input[start..self.current])
        };

        let number: i64 =
            string.parse().map_err(|_| LexError::InvalidNumber)?;

        Ok(Token::Number(number))
    }

    fn next_token(&mut self) -> Result<Token<'a>, LexError> {
        self.skip_whitespace();

        let token = match self.peek() {
            Some(ch) if ch.is_ascii_digit() => return self.read_number(),

            Some(ch) if ch.is_ascii_alphabetic() || ch == b'_' => {
                return Ok(self.read_identifier());
            }

            Some(ch) => {
                self.advance();

                match ch {
                    b'+' => Token::Plus,

                    b'-' => Token::Minus,

                    b'*' => Token::Star,

                    b'/' => Token::Slash,

                    b'=' => Token::Equal,

                    b';' => Token::Semicolon,

                    b'(' => Token::LeftParen,

                    b')' => Token::RightParen,

                    _ => return Err(LexError::UnexpectedChar(ch as char)),
                }
            }

            None => Token::Eof,
        };

        Ok(token)
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
}
