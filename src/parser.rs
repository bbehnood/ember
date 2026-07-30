use thiserror::Error;

use crate::{
    ast::{BinaryOp, Expr, Program, Statement},
    lexer::Token,
};

pub struct Parser<'a> {
    tokens: &'a [Token<'a>],
    current: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
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
    pub fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> Result<Program<'a>, ParseError> {
        let mut statements = Vec::new();

        while self.peek() != Token::Eof {
            statements.push(self.parse_statement()?);
        }

        Ok(Program { statements })
    }

    fn peek(&self) -> Token<'a> {
        self.tokens[self.current]
    }

    fn peek_next(&self) -> Token<'a> {
        self.tokens[self.current + 1]
    }

    fn advance(&mut self) {
        self.current += 1;
    }

    fn expect(&mut self, expected: Token<'a>) -> Result<(), ParseError> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: expected.to_string(),
                found: self.peek().to_string(),
            })
        }
    }

    fn expect_identifier(&mut self) -> Result<&'a [u8], ParseError> {
        match self.peek() {
            Token::Identifier(name) => {
                self.advance();
                Ok(name)
            }

            _ => Err(ParseError::ExpectedIdentifier),
        }
    }

    fn parse_statement(&mut self) -> Result<Statement<'a>, ParseError> {
        match self.peek() {
            Token::Let => self.parse_let(),

            Token::Identifier(_) if self.peek_next() == Token::Equal => {
                self.parse_assignment()
            }

            Token::LeftBrace => self.parse_block(),

            Token::Print => self.parse_print(),

            Token::If => self.parse_if(),

            Token::While => self.parse_while(),

            _ => self.parse_expression_statement(),
        }
    }

    fn parse_let(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect(Token::Let)?;

        let name = self.expect_identifier()?;

        self.expect(Token::Equal)?;

        let value = self.parse_expression()?;

        self.expect(Token::Semicolon)?;

        Ok(Statement::Let { name, value })
    }

    fn parse_assignment(&mut self) -> Result<Statement<'a>, ParseError> {
        let name = self.expect_identifier()?;

        self.expect(Token::Equal)?;

        let value = self.parse_expression()?;

        self.expect(Token::Semicolon)?;

        Ok(Statement::Assign { name, value })
    }

    fn parse_print(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect(Token::Print)?;
        self.expect(Token::LeftParen)?;

        let expr = self.parse_expression()?;

        self.expect(Token::RightParen)?;
        self.expect(Token::Semicolon)?;

        Ok(Statement::Print(expr))
    }

    fn parse_block(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect(Token::LeftBrace)?;

        let mut statements = Vec::new();

        while self.peek() != Token::RightBrace && self.peek() != Token::Eof {
            statements.push(self.parse_statement()?);
        }

        self.expect(Token::RightBrace)?;

        Ok(Statement::Block(statements))
    }

    fn parse_if(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect(Token::If)?;

        let condition = self.parse_expression()?;

        let statement = Box::new(self.parse_statement()?);
        let mut else_clause = None;

        if self.peek() == Token::Else {
            self.advance();
            else_clause = Some(Box::new(self.parse_statement()?));
        }

        Ok(Statement::If { condition, statement, else_clause })
    }

    fn parse_while(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect(Token::While)?;

        let condition = self.parse_expression()?;
        let statement = Box::new(self.parse_statement()?);

        Ok(Statement::While { condition, statement })
    }

    fn parse_expression_statement(
        &mut self,
    ) -> Result<Statement<'a>, ParseError> {
        let expr = self.parse_expression()?;

        self.expect(Token::Semicolon)?;

        Ok(Statement::Expression(expr))
    }

    fn parse_expression(&mut self) -> Result<Expr<'a>, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr<'a>, ParseError> {
        let mut expr = self.parse_and()?;

        while self.peek() == Token::OrOr {
            self.advance();

            let rhs = self.parse_and()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: BinaryOp::Or,
                right: Box::new(rhs),
            };
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr<'a>, ParseError> {
        let mut expr = self.parse_comparison()?;

        while self.peek() == Token::AndAnd {
            self.advance();

            let rhs = self.parse_comparison()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: BinaryOp::And,
                right: Box::new(rhs),
            };
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr<'a>, ParseError> {
        let mut expr = self.parse_addition()?;

        loop {
            let op = match self.peek() {
                Token::Less => BinaryOp::Less,
                Token::LessEqual => BinaryOp::LessEqual,
                Token::Greater => BinaryOp::Greater,
                Token::GreaterEqual => BinaryOp::GreaterEqual,
                Token::EqualEqual => BinaryOp::Equal,
                Token::BangEqual => BinaryOp::NotEqual,
                _ => break,
            };

            self.advance();

            let rhs = self.parse_addition()?;

            expr = Expr::Binary {
                left: Box::new(expr),
                operator: op,
                right: Box::new(rhs),
            };
        }

        Ok(expr)
    }

    fn parse_addition(&mut self) -> Result<Expr<'a>, ParseError> {
        let mut expr = self.parse_multiplication()?;

        loop {
            let op = match self.peek() {
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
            let op = match self.peek() {
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
        match self.peek() {
            Token::Number(n) => {
                self.advance();

                Ok(Expr::Number(n))
            }

            Token::True => {
                self.advance();

                Ok(Expr::Boolean(true))
            }

            Token::False => {
                self.advance();
                Ok(Expr::Boolean(false))
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
    use crate::lexer::Token;

    fn parse<'a>(tokens: &'a [Token<'a>]) -> Result<Program<'a>, ParseError> {
        Parser::new(tokens).parse()
    }

    #[test]
    fn let_statement() {
        let tokens = [
            Token::Let,
            Token::Identifier(b"x"),
            Token::Equal,
            Token::Number(42),
            Token::Semicolon,
            Token::Eof,
        ];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::Let {
                    name: b"x",
                    value: Expr::Number(42),
                }]
            }
        );
    }

    #[test]
    fn expression_statement() {
        let tokens = [Token::Number(5), Token::Semicolon, Token::Eof];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::Expression(Expr::Number(5))]
            }
        );
    }

    #[test]
    fn addition() {
        let tokens = [
            Token::Number(1),
            Token::Plus,
            Token::Number(2),
            Token::Semicolon,
            Token::Eof,
        ];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::Expression(Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    operator: BinaryOp::Add,
                    right: Box::new(Expr::Number(2)),
                })]
            }
        );
    }

    #[test]
    fn precedence() {
        let tokens = [
            Token::Number(1),
            Token::Plus,
            Token::Number(2),
            Token::Star,
            Token::Number(3),
            Token::Semicolon,
            Token::Eof,
        ];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::Expression(Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    operator: BinaryOp::Add,
                    right: Box::new(Expr::Binary {
                        left: Box::new(Expr::Number(2)),
                        operator: BinaryOp::Mul,
                        right: Box::new(Expr::Number(3)),
                    }),
                })]
            }
        );
    }

    #[test]
    fn missing_semicolon() {
        let tokens = [Token::Number(1), Token::Eof];

        let err = parse(&tokens).unwrap_err();

        assert!(matches!(err, ParseError::UnexpectedToken { .. }));
    }

    #[test]
    fn missing_identifier() {
        let tokens = [
            Token::Let,
            Token::Number(1),
            Token::Equal,
            Token::Number(2),
            Token::Semicolon,
            Token::Eof,
        ];

        let err = parse(&tokens).unwrap_err();

        assert_eq!(err, ParseError::ExpectedIdentifier);
    }

    #[test]
    fn missing_expression() {
        let tokens = [
            Token::Let,
            Token::Identifier(b"x"),
            Token::Equal,
            Token::Semicolon,
            Token::Eof,
        ];

        let err = parse(&tokens).unwrap_err();

        assert_eq!(err, ParseError::ExpectedExpression);
    }

    #[test]
    fn missing_right_paren() {
        let err = parse(&[
            Token::LeftParen,
            Token::Number(1),
            Token::Plus,
            Token::Number(2),
            Token::Semicolon,
            Token::Eof,
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
