use std::collections::HashSet;

use crate::ast::{Expr, Program, Statement};

pub struct Sema<'a> {
    variables: HashSet<&'a [u8]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SemaError<'a> {
    UndefinedVariable(&'a [u8]),
    DuplicateVariable(&'a [u8]),
}

impl<'a> Sema<'a> {
    pub fn new() -> Self {
        Sema { variables: HashSet::new() }
    }

    pub fn check_program(
        &mut self,
        program: &Program<'a>,
    ) -> Result<(), SemaError<'a>> {
        for stmt in &program.statements {
            self.check_statement(stmt)?;
        }

        Ok(())
    }

    fn check_statement(
        &mut self,
        stmt: &Statement<'a>,
    ) -> Result<(), SemaError<'a>> {
        match stmt {
            Statement::Let { name, value } => {
                self.check_expr(value)?;

                if self.variables.contains(name) {
                    return Err(SemaError::DuplicateVariable(name));
                }

                self.variables.insert(*name);

                Ok(())
            }

            Statement::Expression(expr) => {
                self.check_expr(expr)?;

                Ok(())
            }
        }
    }

    fn check_expr(&self, expr: &Expr<'a>) -> Result<(), SemaError<'a>> {
        match expr {
            Expr::Number(_) => Ok(()),

            Expr::Identifier(name) => {
                if !self.variables.contains(name) {
                    return Err(SemaError::UndefinedVariable(name));
                }

                Ok(())
            }

            Expr::Binary { left, right, .. } => {
                self.check_expr(left)?;
                self.check_expr(right)?;

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BinaryOp, Expr, Program, Statement};

    fn check(program: Program<'static>) -> Result<(), SemaError<'static>> {
        Sema::new().check_program(&program)
    }

    #[test]
    fn accepts_empty_program() {
        let program = Program { statements: vec![] };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn accepts_single_variable() {
        let program = Program {
            statements: vec![Statement::Let {
                name: b"x",
                value: Expr::Number(42),
            }],
        };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn accepts_variable_use_after_declaration() {
        let program = Program {
            statements: vec![
                Statement::Let { name: b"x", value: Expr::Number(1) },
                Statement::Expression(Expr::Identifier(b"x")),
            ],
        };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn accepts_variable_in_binary_expression() {
        let program = Program {
            statements: vec![
                Statement::Let { name: b"x", value: Expr::Number(1) },
                Statement::Expression(Expr::Binary {
                    left: Box::new(Expr::Identifier(b"x")),
                    operator: BinaryOp::Add,
                    right: Box::new(Expr::Number(2)),
                }),
            ],
        };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn rejects_undefined_variable() {
        let program = Program {
            statements: vec![Statement::Expression(Expr::Identifier(b"x"))],
        };

        assert_eq!(check(program), Err(SemaError::UndefinedVariable(b"x")));
    }

    #[test]
    fn rejects_variable_used_before_declaration() {
        let program = Program {
            statements: vec![
                Statement::Expression(Expr::Identifier(b"x")),
                Statement::Let { name: b"x", value: Expr::Number(1) },
            ],
        };

        assert_eq!(check(program), Err(SemaError::UndefinedVariable(b"x")));
    }

    #[test]
    fn rejects_duplicate_variable() {
        let program = Program {
            statements: vec![
                Statement::Let { name: b"x", value: Expr::Number(1) },
                Statement::Let { name: b"x", value: Expr::Number(2) },
            ],
        };

        assert_eq!(check(program), Err(SemaError::DuplicateVariable(b"x")));
    }

    #[test]
    fn rejects_self_reference() {
        let program = Program {
            statements: vec![Statement::Let {
                name: b"x",
                value: Expr::Identifier(b"x"),
            }],
        };

        assert_eq!(check(program), Err(SemaError::UndefinedVariable(b"x")));
    }

    #[test]
    fn rejects_undefined_variable_in_binary_expression() {
        let program = Program {
            statements: vec![
                Statement::Let { name: b"x", value: Expr::Number(1) },
                Statement::Expression(Expr::Binary {
                    left: Box::new(Expr::Identifier(b"x")),
                    operator: BinaryOp::Add,
                    right: Box::new(Expr::Identifier(b"y")),
                }),
            ],
        };

        assert_eq!(check(program), Err(SemaError::UndefinedVariable(b"y")));
    }
}
