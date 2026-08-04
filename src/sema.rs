use std::collections::HashMap;

use thiserror::Error;

use crate::ast::{BinaryOp, Expr, Program, Statement};

pub struct Sema<'a> {
    scopes: Vec<HashMap<&'a [u8], Type>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Number,
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SemaError {
    #[error("undefined variable '{0}'")]
    UndefinedVariable(String),

    #[error("variable '{0}' is already defined")]
    DuplicateVariable(String),

    #[error("expected {expected}, found {found}")]
    UnexpectedType { expected: Type, found: Type },

    #[error("mismatched types: '{left}' and '{right}'")]
    MismatchedTypes { left: Type, right: Type },
}

fn name_as_string(name: &[u8]) -> String {
    std::str::from_utf8(name)
        .expect("The lexer only consumes valid ASCII")
        .to_owned()
}

impl BinaryOp {
    fn check(self, left: Type, right: Type) -> Result<Type, SemaError> {
        match self {
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                if left != Type::Number || right != Type::Number {
                    return Err(SemaError::UnexpectedType {
                        expected: Type::Number,
                        found: right,
                    });
                }

                Ok(Type::Number)
            }

            BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => {
                if left != Type::Number || right != Type::Number {
                    return Err(SemaError::UnexpectedType {
                        expected: Type::Number,
                        found: if left != Type::Number { left } else { right },
                    });
                }

                Ok(Type::Boolean)
            }

            BinaryOp::Equal | BinaryOp::NotEqual => {
                if left != right {
                    return Err(SemaError::MismatchedTypes { left, right });
                }

                Ok(Type::Boolean)
            }

            BinaryOp::And | BinaryOp::Or => {
                if left != Type::Boolean || right != Type::Boolean {
                    return Err(SemaError::UnexpectedType {
                        expected: Type::Boolean,
                        found: if left != Type::Number { left } else { right },
                    });
                }

                Ok(Type::Boolean)
            }
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Number => write!(f, "number"),
            Type::Boolean => write!(f, "boolean"),
        }
    }
}

impl<'a> Sema<'a> {
    #[must_use]
    pub fn new() -> Self {
        Self { scopes: vec![HashMap::new()] }
    }

    pub fn check(&mut self, program: &Program<'a>) -> Result<(), SemaError> {
        for stmt in &program.statements {
            self.check_statement(stmt)?;
        }

        Ok(())
    }

    fn check_statement(
        &mut self,
        stmt: &Statement<'a>,
    ) -> Result<(), SemaError> {
        match stmt {
            Statement::Let { name, value } => {
                let value_type = self.check_expr(value)?;

                if self.scopes.last().unwrap().contains_key(name) {
                    return Err(SemaError::DuplicateVariable(name_as_string(
                        name,
                    )));
                }

                self.scopes.last_mut().unwrap().insert(name, value_type);

                Ok(())
            }

            Statement::Assign { name, value } => {
                self.check_expr(value)?;

                if !self
                    .scopes
                    .iter()
                    .rev()
                    .any(|scope| scope.contains_key(name))
                {
                    return Err(SemaError::UndefinedVariable(name_as_string(
                        name,
                    )));
                }

                Ok(())
            }

            Statement::Print(expr) => {
                self.check_expr(expr)?;

                Ok(())
            }

            Statement::Block(statements) => {
                self.scopes.push(HashMap::new());
                for stmt in statements {
                    self.check_statement(stmt)?;
                }

                self.scopes.pop();

                Ok(())
            }

            Statement::If { condition, .. } => {
                let condition = self.check_expr(condition)?;

                if condition != Type::Boolean {
                    return Err(SemaError::UnexpectedType {
                        expected: Type::Boolean,
                        found: condition,
                    });
                }

                Ok(())
            }

            Statement::While { condition, .. } => {
                let condition = self.check_expr(condition)?;

                if condition != Type::Boolean {
                    return Err(SemaError::UnexpectedType {
                        expected: Type::Boolean,
                        found: condition,
                    });
                }

                Ok(())
            }

            Statement::Expression(expr) => {
                self.check_expr(expr)?;

                Ok(())
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr<'a>) -> Result<Type, SemaError> {
        match expr {
            Expr::Number(_) => Ok(Type::Number),

            Expr::Boolean(_) => Ok(Type::Boolean),

            Expr::Identifier(name) => self
                .scopes
                .iter()
                .rev()
                .find_map(|scope| scope.get(name).copied())
                .ok_or_else(|| {
                    SemaError::UndefinedVariable(name_as_string(name))
                }),

            Expr::Binary { left, right, operator } => {
                let left = self.check_expr(left)?;
                let right = self.check_expr(right)?;
                let result = operator.check(left, right)?;

                Ok(result)
            }
        }
    }
}

impl Default for Sema<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BinaryOp, Expr, Program, Statement};

    fn check(program: Program<'static>) -> Result<(), SemaError> {
        Sema::new().check(&program)
    }

    #[test]
    fn empty_program() {
        let program = Program { statements: vec![] };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn single_variable() {
        let program = Program {
            statements: vec![Statement::Let {
                name: b"x",
                value: Expr::Number(42),
            }],
        };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn variable_use_after_declaration() {
        let program = Program {
            statements: vec![
                Statement::Let { name: b"x", value: Expr::Number(1) },
                Statement::Expression(Expr::Identifier(b"x")),
            ],
        };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn variable_in_binary_expression() {
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
    fn undefined_variable() {
        let program = Program {
            statements: vec![Statement::Expression(Expr::Identifier(b"x"))],
        };

        assert_eq!(
            check(program),
            Err(SemaError::UndefinedVariable("x".to_string()))
        );
    }

    #[test]
    fn variable_used_before_declaration() {
        let program = Program {
            statements: vec![
                Statement::Expression(Expr::Identifier(b"x")),
                Statement::Let { name: b"x", value: Expr::Number(1) },
            ],
        };

        assert_eq!(
            check(program),
            Err(SemaError::UndefinedVariable("x".to_string()))
        );
    }

    #[test]
    fn duplicate_variable() {
        let program = Program {
            statements: vec![
                Statement::Let { name: b"x", value: Expr::Number(1) },
                Statement::Let { name: b"x", value: Expr::Number(2) },
            ],
        };

        assert_eq!(
            check(program),
            Err(SemaError::DuplicateVariable("x".to_string()))
        );
    }

    #[test]
    fn self_reference() {
        let program = Program {
            statements: vec![Statement::Let {
                name: b"x",
                value: Expr::Identifier(b"x"),
            }],
        };

        assert_eq!(
            check(program),
            Err(SemaError::UndefinedVariable("x".to_string()))
        );
    }

    #[test]
    fn undefined_variable_in_binary_expression() {
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

        assert_eq!(
            check(program),
            Err(SemaError::UndefinedVariable("y".to_string()))
        );
    }

    #[test]
    fn unexpected_type() {
        let program = Program {
            statements: vec![Statement::Expression(Expr::Binary {
                left: Box::new(Expr::Boolean(true)),
                operator: BinaryOp::Add,
                right: Box::new(Expr::Boolean(false)),
            })],
        };

        assert_eq!(
            check(program),
            Err(SemaError::UnexpectedType {
                expected: Type::Number,
                found: Type::Boolean
            })
        );
    }

    #[test]
    fn comparison_of_numbers() {
        let program = Program {
            statements: vec![Statement::Expression(Expr::Binary {
                left: Box::new(Expr::Number(1)),
                operator: BinaryOp::Less,
                right: Box::new(Expr::Number(2)),
            })],
        };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn comparison_type_mismatch() {
        let program = Program {
            statements: vec![Statement::Expression(Expr::Binary {
                left: Box::new(Expr::Boolean(true)),
                operator: BinaryOp::Less,
                right: Box::new(Expr::Number(2)),
            })],
        };

        assert_eq!(
            check(program),
            Err(SemaError::UnexpectedType {
                expected: Type::Number,
                found: Type::Boolean,
            })
        );
    }

    #[test]
    fn equality_of_booleans() {
        let program = Program {
            statements: vec![Statement::Expression(Expr::Binary {
                left: Box::new(Expr::Boolean(true)),
                operator: BinaryOp::Equal,
                right: Box::new(Expr::Boolean(false)),
            })],
        };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn equality_type_mismatch() {
        let program = Program {
            statements: vec![Statement::Expression(Expr::Binary {
                left: Box::new(Expr::Number(1)),
                operator: BinaryOp::Equal,
                right: Box::new(Expr::Boolean(true)),
            })],
        };

        assert_eq!(
            check(program),
            Err(SemaError::MismatchedTypes {
                left: Type::Number,
                right: Type::Boolean,
            })
        );
    }

    #[test]
    fn logical_and_of_booleans() {
        let program = Program {
            statements: vec![Statement::Expression(Expr::Binary {
                left: Box::new(Expr::Boolean(true)),
                operator: BinaryOp::And,
                right: Box::new(Expr::Boolean(false)),
            })],
        };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn logical_operator_type_mismatch() {
        let program = Program {
            statements: vec![Statement::Expression(Expr::Binary {
                left: Box::new(Expr::Number(1)),
                operator: BinaryOp::And,
                right: Box::new(Expr::Number(2)),
            })],
        };

        assert_eq!(
            check(program),
            Err(SemaError::UnexpectedType {
                expected: Type::Boolean,
                found: Type::Number,
            })
        );
    }

    #[test]
    fn if_with_boolean_condition() {
        let program = Program {
            statements: vec![Statement::If {
                condition: Expr::Boolean(true),
                statement: Box::new(Statement::Block(vec![])),
                else_clause: None,
            }],
        };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn if_with_non_boolean_condition() {
        let program = Program {
            statements: vec![Statement::If {
                condition: Expr::Number(1),
                statement: Box::new(Statement::Block(vec![])),
                else_clause: None,
            }],
        };

        assert_eq!(
            check(program),
            Err(SemaError::UnexpectedType {
                expected: Type::Boolean,
                found: Type::Number,
            })
        );
    }

    #[test]
    fn while_with_boolean_condition() {
        let program = Program {
            statements: vec![Statement::While {
                condition: Expr::Boolean(true),
                statement: Box::new(Statement::Block(vec![])),
            }],
        };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn while_with_non_boolean_condition() {
        let program = Program {
            statements: vec![Statement::While {
                condition: Expr::Number(1),
                statement: Box::new(Statement::Block(vec![])),
            }],
        };

        assert_eq!(
            check(program),
            Err(SemaError::UnexpectedType {
                expected: Type::Boolean,
                found: Type::Number,
            })
        );
    }

    #[test]
    fn assign_to_defined_variable() {
        let program = Program {
            statements: vec![
                Statement::Let { name: b"x", value: Expr::Number(1) },
                Statement::Assign { name: b"x", value: Expr::Number(2) },
            ],
        };

        assert_eq!(check(program), Ok(()));
    }

    #[test]
    fn assign_to_undefined_variable() {
        let program = Program {
            statements: vec![Statement::Assign {
                name: b"x",
                value: Expr::Number(1),
            }],
        };

        assert_eq!(
            check(program),
            Err(SemaError::UndefinedVariable("x".to_string()))
        );
    }

    #[test]
    fn variable_out_of_scope_after_block() {
        let program = Program {
            statements: vec![
                Statement::Block(vec![Statement::Let {
                    name: b"x",
                    value: Expr::Number(1),
                }]),
                Statement::Expression(Expr::Identifier(b"x")),
            ],
        };

        assert_eq!(
            check(program),
            Err(SemaError::UndefinedVariable("x".to_string()))
        );
    }
}
