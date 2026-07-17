#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    Let,
    Identifier(&'a [u8]),
    Number(i64),

    Plus,
    Minus,
    Star,
    Slash,

    Equal,
    Semicolon,

    LeftParen,
    RightParen,

    EOF,
}

pub struct Lexer<'a> {
    bytes: &'a [u8],
    position: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LexError {
    UnexpectedChar(char),
    InvalidNumber,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Lexer<'a> {
        Lexer { bytes: input.as_bytes(), position: 0 }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token<'a>>, LexError> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let eof = token == Token::EOF;

            tokens.push(token);

            if eof {
                break;
            }
        }

        Ok(tokens)
    }

    fn current(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn _peek(&self) -> Option<u8> {
        self.bytes.get(self.position + 1).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.current()?;
        self.position += 1;
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current() {
            if c.is_ascii_whitespace() {
                self.position += 1;
            } else {
                break;
            }
        }
    }

    fn read_identifier(&mut self) -> Token<'a> {
        let start = self.position;

        while let Some(c) = self.current() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                self.advance();
            } else {
                break;
            }
        }

        let ident = &self.bytes[start..self.position];

        match ident {
            b"let" => Token::Let,
            _ => Token::Identifier(ident),
        }
    }

    fn read_number(&mut self) -> Result<Token<'a>, LexError> {
        let start = self.position;

        while let Some(c) = self.current() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        let string = std::str::from_utf8(&self.bytes[start..self.position])
            .expect("Only ASCII digits are being consumed");

        let number =
            string.parse::<i64>().map_err(|_| LexError::InvalidNumber)?;

        Ok(Token::Number(number))
    }

    fn next_token(&mut self) -> Result<Token<'a>, LexError> {
        self.skip_whitespace();

        let token = match self.current() {
            Some(c) if c.is_ascii_digit() => return self.read_number(),

            Some(c) if c.is_ascii_alphabetic() || c == b'_' => {
                return Ok(self.read_identifier());
            }

            Some(c) => {
                self.advance();

                match c {
                    b'+' => Token::Plus,

                    b'-' => Token::Minus,

                    b'*' => Token::Star,

                    b'/' => Token::Slash,

                    b'=' => Token::Equal,

                    b';' => Token::Semicolon,

                    b'(' => Token::LeftParen,

                    b')' => Token::RightParen,

                    _ => return Err(LexError::UnexpectedChar(c as char)),
                }
            }
            None => Token::EOF,
        };

        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let mut lexer = Lexer::new("");

        assert_eq!(lexer.tokenize().unwrap(), vec![Token::EOF]);
    }

    #[test]
    fn lex_keyword() {
        let mut lexer = Lexer::new("let");

        assert_eq!(lexer.tokenize().unwrap(), vec![Token::Let, Token::EOF,]);
    }

    #[test]
    fn lex_identifier() {
        let mut lexer = Lexer::new("hello");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::Identifier(b"hello"), Token::EOF,]
        );
    }

    #[test]
    fn lex_identifier_with_underscore_and_digits() {
        let mut lexer = Lexer::new("_foo123");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::Identifier(b"_foo123"), Token::EOF,]
        );
    }

    #[test]
    fn lex_number() {
        let mut lexer = Lexer::new("12345");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::Number(12345), Token::EOF,]
        );
    }

    #[test]
    fn lex_operators() {
        let mut lexer = Lexer::new("+-*/=;()");

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
                Token::EOF,
            ]
        );
    }

    #[test]
    fn lex_variable_declaration() {
        let mut lexer = Lexer::new("let x = 42;");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![
                Token::Let,
                Token::Identifier(b"x"),
                Token::Equal,
                Token::Number(42),
                Token::Semicolon,
                Token::EOF,
            ]
        );
    }

    #[test]
    fn lex_expression() {
        let mut lexer = Lexer::new("1 + 2 * (3 - 4) / 5");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![
                Token::Number(1),
                Token::Plus,
                Token::Number(2),
                Token::Star,
                Token::LeftParen,
                Token::Number(3),
                Token::Minus,
                Token::Number(4),
                Token::RightParen,
                Token::Slash,
                Token::Number(5),
                Token::EOF,
            ]
        );
    }

    #[test]
    fn ignores_whitespace() {
        let mut lexer = Lexer::new(" \n\t let   foo \r\n =  10 ; ");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![
                Token::Let,
                Token::Identifier(b"foo"),
                Token::Equal,
                Token::Number(10),
                Token::Semicolon,
                Token::EOF,
            ]
        );
    }

    #[test]
    fn unexpected_character() {
        let mut lexer = Lexer::new("@");

        assert_eq!(lexer.tokenize(), Err(LexError::UnexpectedChar('@')));
    }

    #[test]
    fn unexpected_character_after_valid_tokens() {
        let mut lexer = Lexer::new("let x = @");

        assert_eq!(lexer.tokenize(), Err(LexError::UnexpectedChar('@')));
    }

    #[test]
    fn invalid_number_overflow() {
        let mut lexer = Lexer::new("999999999999999999999999999999999999");

        assert_eq!(lexer.tokenize(), Err(LexError::InvalidNumber));
    }

    #[test]
    fn multiple_identifiers() {
        let mut lexer = Lexer::new("foo bar baz");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![
                Token::Identifier(b"foo"),
                Token::Identifier(b"bar"),
                Token::Identifier(b"baz"),
                Token::EOF,
            ]
        );
    }

    #[test]
    fn identifier_named_like_keyword_prefix() {
        let mut lexer = Lexer::new("letter");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::Identifier(b"letter"), Token::EOF,]
        );
    }

    #[test]
    fn consecutive_numbers() {
        let mut lexer = Lexer::new("123 456");

        assert_eq!(
            lexer.tokenize().unwrap(),
            vec![Token::Number(123), Token::Number(456), Token::EOF,]
        );
    }
}
