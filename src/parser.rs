//! Turns a flat token stream from [`crate::lexer`] into a
//! [`crate::ast::Program`].
//!
//! This is a straightforward recursive-descent parser. Expressions are
//! parsed using precedence climbing: each precedence level has its own
//! `parse_*` method, and each one calls into the next-tighter-binding level
//! for its operands. From loosest to tightest binding, the chain is:
//! `parse_expression` -> `parse_or` -> `parse_and` -> `parse_comparison` ->
//! `parse_addition` -> `parse_multiplication` -> `parse_primary`.

use thiserror::Error;

use crate::{
    ast::{BinaryOp, Expr, Program, Statement},
    lexer::{Position, Token},
};

/// Consumes a slice of [`Token`]s and produces a [`crate::ast::Program`].
///
/// Unlike the lexer, the parser doesn't own its input - it borrows the
/// token slice produced by [`crate::lexer::Lexer::tokenize_with_positions`]
/// (split into `tokens` and `positions`) and walks it with a single cursor
/// (`current`), using one token of lookahead (`peek_next`) where needed to
/// disambiguate grammar rules.
///
/// `tokens` and `positions` must be the same length, with `positions[i]`
/// giving the source location where `tokens[i]` starts - this is exactly
/// the shape [`crate::lexer::Lexer::tokenize_with_positions`] returns
/// (after unzipping). This invariant is only checked with a `debug_assert`
/// in [`Self::new`] since the parser is not exposed to untrusted input
/// directly; callers always construct both slices from the same lexer run.
pub struct Parser<'a, 'b> {
    tokens: &'b [Token<'a>],
    positions: &'b [Position],
    current: usize,
}

/// Errors that can occur while parsing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    /// A specific token was expected at the current position but a
    /// different one was found, e.g. a missing `)` or `;`.
    #[error("{pos}: expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: String, pos: Position },

    /// An identifier was expected (e.g. after `let`) but something else was
    /// found.
    #[error("{pos}: expected an identifier")]
    ExpectedIdentifier { pos: Position },

    /// The start of an expression was expected but the current token can't
    /// begin one (e.g. a stray operator or closing brace).
    #[error("{pos}: expected an expression")]
    ExpectedExpression { pos: Position },
}

impl<'a, 'b> Parser<'a, 'b> {
    #[must_use]
    pub fn new(tokens: &'b [Token<'a>], positions: &'b [Position]) -> Self {
        debug_assert_eq!(
            tokens.len(),
            positions.len(),
            "tokens and positions must be the same length"
        );

        Self { tokens, positions, current: 0 }
    }

    /// Parses the entire token stream into a [`Program`], i.e. a sequence
    /// of top-level statements running up to [`Token::Eof`].
    pub fn parse(&mut self) -> Result<Program<'a>, ParseError> {
        let mut statements = Vec::new();

        while self.peek() != Token::Eof {
            statements.push(self.parse_statement()?);
        }

        Ok(Program { statements })
    }

    /// Returns the token at the current cursor position without consuming
    /// it. Never fails: [`Token::Eof`] is always present as a sentinel at
    /// the end of the stream, so this is always in bounds.
    fn peek(&self) -> Token<'a> {
        self.tokens[self.current]
    }

    /// Returns the token one past the current cursor position, used to
    /// look ahead when a single token isn't enough to decide which grammar
    /// rule applies (see the identifier-vs-assignment check in
    /// [`Self::parse_statement`]).
    fn peek_next(&self) -> Token<'a> {
        self.tokens[self.current + 1]
    }

    /// Returns the position of the token at the current cursor position,
    /// i.e. where [`Self::peek`]'s result starts in the source. Used to
    /// locate errors raised at the current cursor position.
    fn current_pos(&self) -> Position {
        self.positions[self.current]
    }

    /// Moves the cursor forward by one token.
    fn advance(&mut self) {
        self.current += 1;
    }

    /// Consumes the current token if it matches `expected`, otherwise
    /// returns a [`ParseError::UnexpectedToken`] describing the mismatch.
    fn expect(&mut self, expected: Token<'a>) -> Result<(), ParseError> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: expected.to_string(),
                found: self.peek().to_string(),
                pos: self.current_pos(),
            })
        }
    }

    /// Consumes the current token if it's an [`Token::Identifier`] and
    /// returns the borrowed name, otherwise returns
    /// [`ParseError::ExpectedIdentifier`].
    fn expect_identifier(&mut self) -> Result<&'a [u8], ParseError> {
        match self.peek() {
            Token::Identifier(name) => {
                self.advance();
                Ok(name)
            }

            _ => {
                Err(ParseError::ExpectedIdentifier { pos: self.current_pos() })
            }
        }
    }

    /// Dispatches to the appropriate `parse_*` method for a statement based
    /// on the current token.
    ///
    /// Distinguishing an assignment (`x = 1;`) from a bare expression
    /// statement (`x;`) requires one token of lookahead: both start with an
    /// identifier, so [`Self::peek_next`] is used to check for the
    /// following `=`.
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

    /// Parses a `let name = expr;` declaration.
    fn parse_let(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect(Token::Let)?;

        let name = self.expect_identifier()?;

        self.expect(Token::Equal)?;

        let value = self.parse_expression()?;

        self.expect(Token::Semicolon)?;

        Ok(Statement::Let { name, value })
    }

    /// Parses a `name = expr;` assignment. Called only after
    /// [`Self::parse_statement`] has already confirmed via lookahead that
    /// the statement starts with `identifier =`.
    fn parse_assignment(&mut self) -> Result<Statement<'a>, ParseError> {
        let name = self.expect_identifier()?;

        self.expect(Token::Equal)?;

        let value = self.parse_expression()?;

        self.expect(Token::Semicolon)?;

        Ok(Statement::Assign { name, value })
    }

    /// Parses a `print(expr);` statement.
    fn parse_print(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect(Token::Print)?;
        self.expect(Token::LeftParen)?;

        let expr = self.parse_expression()?;

        self.expect(Token::RightParen)?;
        self.expect(Token::Semicolon)?;

        Ok(Statement::Print(expr))
    }

    /// Parses a `{ ... }` block: a brace-delimited sequence of statements.
    /// Stops at a matching `}` or, defensively, at EOF (in which case
    /// [`Self::expect`] below will produce the appropriate error).
    fn parse_block(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect(Token::LeftBrace)?;

        let mut statements = Vec::new();

        while self.peek() != Token::RightBrace && self.peek() != Token::Eof {
            statements.push(self.parse_statement()?);
        }

        self.expect(Token::RightBrace)?;

        Ok(Statement::Block(statements))
    }

    /// Parses an `if condition statement [else statement]` construct.
    ///
    /// Note that `statement` here is a full statement, not necessarily a
    /// block - so both branches are typically (but not required to be)
    /// `{ ... }` blocks, matching how the grammar is structured.
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

    /// Parses a `while condition statement` loop.
    fn parse_while(&mut self) -> Result<Statement<'a>, ParseError> {
        self.expect(Token::While)?;

        let condition = self.parse_expression()?;
        let statement = Box::new(self.parse_statement()?);

        Ok(Statement::While { condition, statement })
    }

    /// Parses a bare expression followed by a semicolon, e.g. `1 + 2;`.
    /// This is the fallback case in [`Self::parse_statement`] when no other
    /// statement keyword matches.
    fn parse_expression_statement(
        &mut self,
    ) -> Result<Statement<'a>, ParseError> {
        let expr = self.parse_expression()?;

        self.expect(Token::Semicolon)?;

        Ok(Statement::Expression(expr))
    }

    /// Entry point for expression parsing. Starts at the loosest-binding
    /// level (`||`) and recurses down through tighter-binding levels; see
    /// the module-level docs for the full precedence chain.
    fn parse_expression(&mut self) -> Result<Expr<'a>, ParseError> {
        self.parse_or()
    }

    /// Parses `||` expressions (loosest-binding, left-associative). Operands
    /// are parsed one level tighter, at [`Self::parse_and`].
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

    /// Parses `&&` expressions, binding tighter than `||` but looser than
    /// comparisons.
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

    /// Parses comparison expressions (`<`, `<=`, `>`, `>=`, `==`, `!=`).
    /// All comparison operators sit at the same precedence level and are
    /// left-associative (so `a < b < c` parses as `(a < b) < c`, though
    /// that particular expression would later be rejected by
    /// [`crate::sema::Sema`] since `<` produces a boolean, not a number).
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

    /// Parses `+` and `-` expressions, binding tighter than comparisons but
    /// looser than `*`/`/`.
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

    /// Parses `*` and `/` expressions, the tightest-binding operators
    /// besides parenthesized/primary expressions.
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

    /// Parses a primary expression: a literal, identifier, or a
    /// parenthesized sub-expression. This is the base case of the
    /// precedence-climbing chain.
    fn parse_primary(&mut self) -> Result<Expr<'a>, ParseError> {
        match self.peek() {
            Token::Number(n) => {
                self.advance();

                Ok(Expr::Number(n))
            }

            Token::String(s) => {
                self.advance();

                Ok(Expr::String(s))
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

                // Parenthesized expressions re-enter at the top of the
                // precedence chain, since anything can appear inside `(...)`.
                let expr = self.parse_expression()?;

                self.expect(Token::RightParen)?;

                Ok(expr)
            }

            _ => {
                Err(ParseError::ExpectedExpression { pos: self.current_pos() })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Token;

    /// Parses a hand-written token slice for tests that don't care about
    /// positions. Every token is given the same placeholder `Position`
    /// (its `Default`, i.e. `{ line: 0, col: 0 }`) since these tests
    /// construct tokens directly rather than going through the lexer;
    /// tests that need to assert on a `ParseError`'s position use this
    /// same placeholder value.
    fn parse<'a>(tokens: &[Token<'a>]) -> Result<Program<'a>, ParseError> {
        let positions = vec![Position::default(); tokens.len()];

        Parser::new(tokens, &positions).parse()
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

        assert_eq!(
            err,
            ParseError::ExpectedIdentifier { pos: Position::default() }
        );
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

        assert_eq!(
            err,
            ParseError::ExpectedExpression { pos: Position::default() }
        );
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
                pos: Position::default(),
            }
        );
    }

    #[test]
    fn boolean_literal() {
        let tokens = [Token::True, Token::Semicolon, Token::Eof];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::Expression(Expr::Boolean(true))]
            }
        );
    }

    #[test]
    fn comparison() {
        let tokens = [
            Token::Number(1),
            Token::Less,
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
                    operator: BinaryOp::Less,
                    right: Box::new(Expr::Number(2)),
                })]
            }
        );
    }

    #[test]
    fn logical_and() {
        let tokens = [
            Token::True,
            Token::AndAnd,
            Token::False,
            Token::Semicolon,
            Token::Eof,
        ];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::Expression(Expr::Binary {
                    left: Box::new(Expr::Boolean(true)),
                    operator: BinaryOp::And,
                    right: Box::new(Expr::Boolean(false)),
                })]
            }
        );
    }

    #[test]
    fn logical_or() {
        let tokens = [
            Token::True,
            Token::OrOr,
            Token::False,
            Token::Semicolon,
            Token::Eof,
        ];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::Expression(Expr::Binary {
                    left: Box::new(Expr::Boolean(true)),
                    operator: BinaryOp::Or,
                    right: Box::new(Expr::Boolean(false)),
                })]
            }
        );
    }

    #[test]
    fn logical_precedence_over_comparison() {
        let tokens = [
            Token::Number(1),
            Token::Less,
            Token::Number(2),
            Token::AndAnd,
            Token::Number(3),
            Token::Less,
            Token::Number(4),
            Token::Semicolon,
            Token::Eof,
        ];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::Expression(Expr::Binary {
                    left: Box::new(Expr::Binary {
                        left: Box::new(Expr::Number(1)),
                        operator: BinaryOp::Less,
                        right: Box::new(Expr::Number(2)),
                    }),
                    operator: BinaryOp::And,
                    right: Box::new(Expr::Binary {
                        left: Box::new(Expr::Number(3)),
                        operator: BinaryOp::Less,
                        right: Box::new(Expr::Number(4)),
                    }),
                })]
            }
        );
    }

    #[test]
    fn assignment() {
        let tokens = [
            Token::Identifier(b"x"),
            Token::Equal,
            Token::Number(1),
            Token::Semicolon,
            Token::Eof,
        ];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::Assign {
                    name: b"x",
                    value: Expr::Number(1),
                }]
            }
        );
    }

    #[test]
    fn print_statement() {
        let tokens = [
            Token::Print,
            Token::LeftParen,
            Token::Number(1),
            Token::RightParen,
            Token::Semicolon,
            Token::Eof,
        ];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program { statements: vec![Statement::Print(Expr::Number(1))] }
        );
    }

    #[test]
    fn block_statement() {
        let tokens = [
            Token::LeftBrace,
            Token::Number(1),
            Token::Semicolon,
            Token::RightBrace,
            Token::Eof,
        ];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::Block(vec![
                    Statement::Expression(Expr::Number(1))
                ])]
            }
        );
    }

    #[test]
    fn empty_block_statement() {
        let tokens = [Token::LeftBrace, Token::RightBrace, Token::Eof];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program { statements: vec![Statement::Block(vec![])] }
        );
    }

    #[test]
    fn if_statement() {
        let tokens = [
            Token::If,
            Token::True,
            Token::LeftBrace,
            Token::RightBrace,
            Token::Eof,
        ];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::If {
                    condition: Expr::Boolean(true),
                    statement: Box::new(Statement::Block(vec![])),
                    else_clause: None,
                }]
            }
        );
    }

    #[test]
    fn if_else_statement() {
        let tokens = [
            Token::If,
            Token::True,
            Token::LeftBrace,
            Token::RightBrace,
            Token::Else,
            Token::LeftBrace,
            Token::RightBrace,
            Token::Eof,
        ];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::If {
                    condition: Expr::Boolean(true),
                    statement: Box::new(Statement::Block(vec![])),
                    else_clause: Some(Box::new(Statement::Block(vec![]))),
                }]
            }
        );
    }

    #[test]
    fn while_statement() {
        let tokens = [
            Token::While,
            Token::True,
            Token::LeftBrace,
            Token::RightBrace,
            Token::Eof,
        ];

        let program = parse(&tokens).unwrap();

        assert_eq!(
            program,
            Program {
                statements: vec![Statement::While {
                    condition: Expr::Boolean(true),
                    statement: Box::new(Statement::Block(vec![])),
                }]
            }
        );
    }

    /// End-to-end check (lexer + parser together) that a [`ParseError`]
    /// reports the real position of the offending token, rather than the
    /// placeholder `Position::default()` used by the hand-written-token
    /// tests above.
    #[test]
    fn error_reports_real_position_from_lexer() {
        use crate::lexer::Lexer;

        let source = b"let x = 1\nlet y = 2;";
        let tokens_with_pos =
            Lexer::new(source).tokenize_with_positions().unwrap();

        let tokens: Vec<Token> =
            tokens_with_pos.iter().map(|(t, _)| *t).collect();
        let positions: Vec<Position> =
            tokens_with_pos.iter().map(|(_, p)| *p).collect();

        let err = Parser::new(&tokens, &positions).parse().unwrap_err();

        // The missing `;` after `let x = 1` is discovered once the parser
        // reaches `let` on the second line, since that's the first token
        // that isn't a valid continuation of the statement.
        assert_eq!(
            err,
            ParseError::UnexpectedToken {
                expected: Token::Semicolon.to_string(),
                found: Token::Let.to_string(),
                pos: Position { line: 2, col: 1 },
            }
        );
    }
}
