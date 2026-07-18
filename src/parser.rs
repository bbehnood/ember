use crate::{
    ast::{BinaryOp, Expr, Program, Statement},
    lexer::Token,
};

pub struct Parser<'a> {
    tokens: &'a [Token<'a>],
    position: usize,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ParseError {
    #[error("expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: String },

    #[error("expected an identifier")]
    ExpectedIdentifier,

    #[error("expected an expression")]
    ExpectedExpression,
}

impl<'a> Parser<'a> {
    #[must_use]
    pub fn new(tokens: &'a [Token<'a>]) -> Self {
        Self { tokens, position: 0 }
    }

    pub fn parse_program(&mut self) -> Result<Program<'a>, ParseError> {
        let mut statements = Vec::new();

        while self.current() != Token::EOF {
            statements.push(self.parse_statement()?);
        }

        Ok(Program { statements })
    }

    fn current(&self) -> Token<'a> {
        self.tokens.get(self.position).copied().unwrap_or(Token::EOF)
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn expect(&mut self, expected: Token<'a>) -> Result<(), ParseError> {
        if self.current() == expected {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: expected.to_string(),
                found: self.current().to_string(),
            })
        }
    }

    fn expect_identifier(&mut self) -> Result<&'a [u8], ParseError> {
        match self.current() {
            Token::Identifier(name) => {
                self.advance();
                Ok(name)
            }
            _ => Err(ParseError::ExpectedIdentifier),
        }
    }

    fn parse_statement(&mut self) -> Result<Statement<'a>, ParseError> {
        match self.current() {
            Token::Let => self.parse_let(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_expression(&mut self) -> Result<Expr<'a>, ParseError> {
        self.parse_addition()
    }

    fn parse_expression_statement(
        &mut self,
    ) -> Result<Statement<'a>, ParseError> {
        let expr = self.parse_expression()?;

        self.expect(Token::Semicolon)?;

        Ok(Statement::Expression(expr))
    }

    fn parse_let(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect(Token::Let)?;

        let name = self.expect_identifier()?;

        self.expect(Token::Equal)?;

        let value = self.parse_expression()?;

        self.expect(Token::Semicolon)?;

        Ok(Statement::Let { name, value })
    }

    fn parse_addition(&mut self) -> Result<Expr<'a>, ParseError> {
        let mut expr = self.parse_multiplication()?;

        loop {
            let op = match self.current() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };

            self.advance();

            let rhs = self.parse_multiplication()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: op,
                right: Box::new(rhs),
            };
        }

        Ok(expr)
    }

    fn parse_multiplication(&mut self) -> Result<Expr<'a>, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            let op = match self.current() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                _ => break,
            };

            self.advance();

            let rhs = self.parse_primary()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: op,
                right: Box::new(rhs),
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr<'a>, ParseError> {
        match self.current() {
            Token::Number(n) => {
                self.advance();

                Ok(Expr::Number(n))
            }

            Token::Identifier(name) => {
                self.advance();

                Ok(Expr::Identifier(name))
            }

            Token::LeftParen => {
                self.advance();

                let expr = self.parse_expression()?;

                self.expect(Token::RightParen)?;

                Ok(expr)
            }

            _ => Err(ParseError::ExpectedExpression),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::{BinaryOp, Expr, Program, Statement},
        lexer::Token,
    };

    fn parse<'a>(tokens: &'a [Token]) -> Result<Program<'a>, ParseError> {
        Parser::new(tokens).parse_program()
    }

    #[test]
    fn parses_empty_program() {
        let program = parse(&[Token::EOF]).unwrap();

        assert!(program.statements.is_empty());
    }

    #[test]
    fn parses_let_statement() {
        let program = parse(&[
            Token::Let,
            Token::Identifier(b"x"),
            Token::Equal,
            Token::Number(42),
            Token::Semicolon,
            Token::EOF,
        ])
        .unwrap();

        assert_eq!(
            program.statements,
            vec![Statement::Let { name: b"x", value: Expr::Number(42) }]
        );
    }

    #[test]
    fn parses_expression_statement() {
        let program =
            parse(&[Token::Number(123), Token::Semicolon, Token::EOF]).unwrap();

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expr::Number(123))]
        );
    }

    #[test]
    fn parses_addition_and_multiplication_precedence() {
        let program = parse(&[
            Token::Number(1),
            Token::Plus,
            Token::Number(2),
            Token::Star,
            Token::Number(3),
            Token::Semicolon,
            Token::EOF,
        ])
        .unwrap();

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expr::Binary {
                left: Box::new(Expr::Number(1)),
                operator: BinaryOp::Add,
                right: Box::new(Expr::Binary {
                    left: Box::new(Expr::Number(2)),
                    operator: BinaryOp::Mul,
                    right: Box::new(Expr::Number(3)),
                }),
            })]
        );
    }

    #[test]
    fn parses_parenthesized_expression() {
        let program = parse(&[
            Token::LeftParen,
            Token::Number(1),
            Token::Plus,
            Token::Number(2),
            Token::RightParen,
            Token::Star,
            Token::Number(3),
            Token::Semicolon,
            Token::EOF,
        ])
        .unwrap();

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expr::Binary {
                left: Box::new(Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    operator: BinaryOp::Add,
                    right: Box::new(Expr::Number(2)),
                }),
                operator: BinaryOp::Mul,
                right: Box::new(Expr::Number(3)),
            })]
        );
    }

    #[test]
    fn parses_left_associative_subtraction() {
        let program = parse(&[
            Token::Number(1),
            Token::Minus,
            Token::Number(2),
            Token::Minus,
            Token::Number(3),
            Token::Semicolon,
            Token::EOF,
        ])
        .unwrap();

        assert_eq!(
            program.statements,
            vec![Statement::Expression(Expr::Binary {
                left: Box::new(Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    operator: BinaryOp::Sub,
                    right: Box::new(Expr::Number(2)),
                }),
                operator: BinaryOp::Sub,
                right: Box::new(Expr::Number(3)),
            })]
        );
    }

    #[test]
    fn error_missing_identifier_after_let() {
        let err = parse(&[
            Token::Let,
            Token::Equal,
            Token::Number(1),
            Token::Semicolon,
            Token::EOF,
        ])
        .unwrap_err();

        assert_eq!(err, ParseError::ExpectedIdentifier);
    }

    #[test]
    fn error_missing_expression() {
        let err = parse(&[
            Token::Let,
            Token::Identifier(b"x"),
            Token::Equal,
            Token::Semicolon,
            Token::EOF,
        ])
        .unwrap_err();

        assert_eq!(err, ParseError::ExpectedExpression);
    }

    #[test]
    fn error_missing_semicolon() {
        let err = parse(&[Token::Number(1), Token::EOF]).unwrap_err();

        assert_eq!(
            err,
            ParseError::UnexpectedToken {
                expected: Token::Semicolon.to_string(),
                found: Token::EOF.to_string(),
            }
        );
    }

    #[test]
    fn error_missing_right_paren() {
        let err = parse(&[
            Token::LeftParen,
            Token::Number(1),
            Token::Plus,
            Token::Number(2),
            Token::Semicolon,
            Token::EOF,
        ])
        .unwrap_err();

        assert_eq!(
            err,
            ParseError::UnexpectedToken {
                expected: Token::RightParen.to_string(),
                found: Token::Semicolon.to_string(),
            }
        );
    }
}
