use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, Program, Statement};

pub struct Interpreter<'a> {
    scopes: Vec<HashMap<&'a [u8], Value>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    Number(i64),
    Boolean(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeError {
    #[error("attempt to divide by zero")]
    DivideByZero,

    #[error("arithmetic overflow")]
    ArithmeticOverflow,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(n) => write!(f, "{n}"),
            Self::Boolean(b) => write!(f, "{b}"),
        }
    }
}

impl<'a> Interpreter<'a> {
    #[must_use]
    pub fn new() -> Self {
        Self { scopes: vec![HashMap::new()] }
    }

    pub fn run(&mut self, program: &Program<'a>) -> Result<(), RuntimeError> {
        for stmt in &program.statements {
            self.execute_statement(stmt)?;
        }

        Ok(())
    }

    fn execute_statement(
        &mut self,
        stmt: &Statement<'a>,
    ) -> Result<(), RuntimeError> {
        match stmt {
            Statement::Let { name, value } => {
                let value = self.eval(value)?;
                self.scopes.last_mut().unwrap().insert(name, value);

                Ok(())
            }

            Statement::Assign { name, value } => {
                let value = self.eval(value)?;
                *self
                    .scopes
                    .iter_mut()
                    .rev()
                    .find_map(|scope| scope.get_mut(name))
                    .expect("Undefined variables should be caught at sema") =
                    value;

                Ok(())
            }

            Statement::Print(expr) => {
                let value = self.eval(expr)?;
                println!("{value}");
                Ok(())
            }

            Statement::Block(statements) => {
                self.scopes.push(HashMap::new());
                for stmt in statements {
                    self.execute_statement(stmt)?;
                }

                self.scopes.pop();

                Ok(())
            }

            Statement::If { condition, statement, else_clause } => {
                if self.eval(condition)? == Value::Boolean(true) {
                    self.execute_statement(statement)?;
                } else {
                    if let Some(stmt) = else_clause {
                        self.execute_statement(stmt)?;
                    }
                }

                Ok(())
            }

            Statement::While { condition, statement } => {
                while self.eval(condition)? == Value::Boolean(true) {
                    self.execute_statement(statement)?;
                }

                Ok(())
            }

            Statement::Expression(expr) => {
                self.eval(expr)?;
                Ok(())
            }
        }
    }

    fn eval(&mut self, expr: &Expr<'a>) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),

            Expr::Boolean(b) => Ok(Value::Boolean(*b)),

            Expr::Identifier(name) => Ok(self
                .scopes
                .iter()
                .rev()
                .find_map(|scope| scope.get(name).copied())
                .expect("Undefined variables should be caught at sema")),

            Expr::Binary { left, operator, right } => {
                let left = self.eval(left)?;
                let right = self.eval(right)?;

                match operator {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div => match (left, right) {
                        (Value::Number(left), Value::Number(right)) => {
                            let result = match operator {
                                BinaryOp::Add => left.checked_add(right),
                                BinaryOp::Sub => left.checked_sub(right),
                                BinaryOp::Mul => left.checked_mul(right),
                                BinaryOp::Div => {
                                    if right == 0 {
                                        return Err(RuntimeError::DivideByZero);
                                    }

                                    left.checked_div(right)
                                }

                                _ => unreachable!(),
                            };

                            result
                                .map(Value::Number)
                                .ok_or(RuntimeError::ArithmeticOverflow)
                        }

                        _ => unreachable!(),
                    },

                    BinaryOp::Equal => Ok(Value::Boolean(left == right)),

                    BinaryOp::NotEqual => Ok(Value::Boolean(left != right)),

                    BinaryOp::Less => match (left, right) {
                        (Value::Number(left), Value::Number(right)) => {
                            Ok(Value::Boolean(left < right))
                        }
                        _ => unreachable!(),
                    },

                    BinaryOp::LessEqual => match (left, right) {
                        (Value::Number(left), Value::Number(right)) => {
                            Ok(Value::Boolean(left <= right))
                        }
                        _ => unreachable!(),
                    },

                    BinaryOp::Greater => match (left, right) {
                        (Value::Number(left), Value::Number(right)) => {
                            Ok(Value::Boolean(left > right))
                        }
                        _ => unreachable!(),
                    },

                    BinaryOp::GreaterEqual => match (left, right) {
                        (Value::Number(left), Value::Number(right)) => {
                            Ok(Value::Boolean(left >= right))
                        }
                        _ => unreachable!(),
                    },

                    BinaryOp::And => Ok(
                        if left == Value::Boolean(true)
                            && right == Value::Boolean(true)
                        {
                            Value::Boolean(true)
                        } else {
                            Value::Boolean(false)
                        },
                    ),

                    BinaryOp::Or => Ok(
                        if left == Value::Boolean(true)
                            || right == Value::Boolean(true)
                        {
                            Value::Boolean(true)
                        } else {
                            Value::Boolean(false)
                        },
                    ),
                }
            }
        }
    }
}

impl Default for Interpreter<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BinaryOp, Expr, Program, Statement};

    fn eval(expr: Expr<'static>) -> Result<Value, RuntimeError> {
        Interpreter::new().eval(&expr)
    }

    fn run(program: Program<'static>) -> Result<(), RuntimeError> {
        Interpreter::new().run(&program)
    }

    #[test]
    fn number_literal() {
        assert_eq!(eval(Expr::Number(42)), Ok(Value::Number(42)));
    }

    #[test]
    fn boolean_literal() {
        assert_eq!(eval(Expr::Boolean(true)), Ok(Value::Boolean(true)))
    }

    #[test]
    fn addition() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(1)),
            operator: BinaryOp::Add,
            right: Box::new(Expr::Number(2)),
        };

        assert_eq!(eval(expr), Ok(Value::Number(3)));
    }

    #[test]
    fn subtraction() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(5)),
            operator: BinaryOp::Sub,
            right: Box::new(Expr::Number(3)),
        };

        assert_eq!(eval(expr), Ok(Value::Number(2)));
    }

    #[test]
    fn multiplication() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(4)),
            operator: BinaryOp::Mul,
            right: Box::new(Expr::Number(5)),
        };

        assert_eq!(eval(expr), Ok(Value::Number(20)));
    }

    #[test]
    fn division() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(10)),
            operator: BinaryOp::Div,
            right: Box::new(Expr::Number(3)),
        };

        assert_eq!(eval(expr), Ok(Value::Number(3)));
    }

    #[test]
    fn nested_expression_precedence() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(1)),
            operator: BinaryOp::Add,
            right: Box::new(Expr::Binary {
                left: Box::new(Expr::Number(2)),
                operator: BinaryOp::Mul,
                right: Box::new(Expr::Number(3)),
            }),
        };

        assert_eq!(eval(expr), Ok(Value::Number(7)));
    }

    #[test]
    fn divide_by_zero() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(1)),
            operator: BinaryOp::Div,
            right: Box::new(Expr::Number(0)),
        };

        assert_eq!(eval(expr), Err(RuntimeError::DivideByZero));
    }

    #[test]
    fn addition_overflow() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(i64::MAX)),
            operator: BinaryOp::Add,
            right: Box::new(Expr::Number(1)),
        };

        assert_eq!(eval(expr), Err(RuntimeError::ArithmeticOverflow));
    }

    #[test]
    fn subtraction_overflow() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(i64::MIN)),
            operator: BinaryOp::Sub,
            right: Box::new(Expr::Number(1)),
        };

        assert_eq!(eval(expr), Err(RuntimeError::ArithmeticOverflow));
    }

    #[test]
    fn multiplication_overflow() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(i64::MAX)),
            operator: BinaryOp::Mul,
            right: Box::new(Expr::Number(2)),
        };

        assert_eq!(eval(expr), Err(RuntimeError::ArithmeticOverflow));
    }

    #[test]
    fn division_overflow() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(i64::MIN)),
            operator: BinaryOp::Div,
            right: Box::new(Expr::Number(-1)),
        };

        assert_eq!(eval(expr), Err(RuntimeError::ArithmeticOverflow));
    }

    #[test]
    fn divide_by_zero_and_overflow() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(i64::MIN)),
            operator: BinaryOp::Div,
            right: Box::new(Expr::Number(0)),
        };

        assert_eq!(eval(expr), Err(RuntimeError::DivideByZero));
    }

    #[test]
    fn empty_program() {
        let program = Program { statements: vec![] };

        assert_eq!(run(program), Ok(()));
    }

    #[test]
    fn let_and_use_variable() {
        let program = Program {
            statements: vec![
                Statement::Let { name: b"x", value: Expr::Number(10) },
                Statement::Expression(Expr::Identifier(b"x")),
            ],
        };

        assert_eq!(run(program), Ok(()));
    }

    #[test]
    fn let_statement() {
        let mut interpreter = Interpreter::new();

        interpreter
            .run(&Program {
                statements: vec![Statement::Let {
                    name: b"x",
                    value: Expr::Binary {
                        left: Box::new(Expr::Number(2)),
                        operator: BinaryOp::Mul,
                        right: Box::new(Expr::Number(21)),
                    },
                }],
            })
            .unwrap();

        assert_eq!(
            interpreter.eval(&Expr::Identifier(b"x")),
            Ok(Value::Number(42))
        );
    }

    #[test]
    fn rebinding_in_nested_block() {
        let mut interpreter = Interpreter::new();

        interpreter
            .run(&Program {
                statements: vec![
                    Statement::Let { name: b"x", value: Expr::Number(1) },
                    Statement::Block(vec![Statement::Let {
                        name: b"x",
                        value: Expr::Number(2),
                    }]),
                ],
            })
            .unwrap();

        assert_eq!(
            interpreter.eval(&Expr::Identifier(b"x")),
            Ok(Value::Number(1))
        );
    }

    #[test]
    fn outer_scope_variables() {
        let program = Program {
            statements: vec![
                Statement::Let { name: b"x", value: Expr::Number(5) },
                Statement::Block(vec![Statement::Expression(
                    Expr::Identifier(b"x"),
                )]),
            ],
        };

        assert_eq!(run(program), Ok(()));
    }

    #[test]
    fn nested_blocks() {
        let program = Program {
            statements: vec![Statement::Block(vec![Statement::Block(vec![
                Statement::Let { name: b"x", value: Expr::Number(1) },
            ])])],
        };

        assert_eq!(run(program), Ok(()));
    }

    #[test]
    fn runtime_error_in_let() {
        let program = Program {
            statements: vec![Statement::Let {
                name: b"x",
                value: Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    operator: BinaryOp::Div,
                    right: Box::new(Expr::Number(0)),
                },
            }],
        };

        assert_eq!(run(program), Err(RuntimeError::DivideByZero));
    }

    #[test]
    fn runtime_error_in_block() {
        let program = Program {
            statements: vec![Statement::Block(vec![Statement::Expression(
                Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    operator: BinaryOp::Div,
                    right: Box::new(Expr::Number(0)),
                },
            )])],
        };

        assert_eq!(run(program), Err(RuntimeError::DivideByZero));
    }

    #[test]
    fn print_statement() {
        let program =
            Program { statements: vec![Statement::Print(Expr::Number(42))] };

        assert_eq!(run(program), Ok(()));
    }

    #[test]
    fn equal() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(2)),
            operator: BinaryOp::Equal,
            right: Box::new(Expr::Number(2)),
        };

        assert_eq!(eval(expr), Ok(Value::Boolean(true)));
    }

    #[test]
    fn not_equal() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(2)),
            operator: BinaryOp::NotEqual,
            right: Box::new(Expr::Number(3)),
        };

        assert_eq!(eval(expr), Ok(Value::Boolean(true)));
    }

    #[test]
    fn less() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(1)),
            operator: BinaryOp::Less,
            right: Box::new(Expr::Number(2)),
        };

        assert_eq!(eval(expr), Ok(Value::Boolean(true)));
    }

    #[test]
    fn less_equal() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(2)),
            operator: BinaryOp::LessEqual,
            right: Box::new(Expr::Number(2)),
        };

        assert_eq!(eval(expr), Ok(Value::Boolean(true)));
    }

    #[test]
    fn greater() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(3)),
            operator: BinaryOp::Greater,
            right: Box::new(Expr::Number(2)),
        };

        assert_eq!(eval(expr), Ok(Value::Boolean(true)));
    }

    #[test]
    fn greater_equal() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Number(2)),
            operator: BinaryOp::GreaterEqual,
            right: Box::new(Expr::Number(2)),
        };

        assert_eq!(eval(expr), Ok(Value::Boolean(true)));
    }

    #[test]
    fn logical_and() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Boolean(true)),
            operator: BinaryOp::And,
            right: Box::new(Expr::Boolean(false)),
        };

        assert_eq!(eval(expr), Ok(Value::Boolean(false)));
    }

    #[test]
    fn logical_or() {
        let expr = Expr::Binary {
            left: Box::new(Expr::Boolean(false)),
            operator: BinaryOp::Or,
            right: Box::new(Expr::Boolean(true)),
        };

        assert_eq!(eval(expr), Ok(Value::Boolean(true)));
    }

    #[test]
    fn if_true_branch() {
        let mut interpreter = Interpreter::new();

        interpreter
            .run(&Program {
                statements: vec![Statement::If {
                    condition: Expr::Boolean(true),
                    statement: Box::new(Statement::Let {
                        name: b"x",
                        value: Expr::Number(1),
                    }),
                    else_clause: Some(Box::new(Statement::Let {
                        name: b"x",
                        value: Expr::Number(2),
                    })),
                }],
            })
            .unwrap();

        assert_eq!(
            interpreter.eval(&Expr::Identifier(b"x")),
            Ok(Value::Number(1))
        );
    }

    #[test]
    fn if_false_branch_with_else() {
        let mut interpreter = Interpreter::new();

        interpreter
            .run(&Program {
                statements: vec![Statement::If {
                    condition: Expr::Boolean(false),
                    statement: Box::new(Statement::Let {
                        name: b"x",
                        value: Expr::Number(1),
                    }),
                    else_clause: Some(Box::new(Statement::Let {
                        name: b"x",
                        value: Expr::Number(2),
                    })),
                }],
            })
            .unwrap();

        assert_eq!(
            interpreter.eval(&Expr::Identifier(b"x")),
            Ok(Value::Number(2))
        );
    }

    #[test]
    fn if_false_branch_without_else() {
        let program = Program {
            statements: vec![Statement::If {
                condition: Expr::Boolean(false),
                statement: Box::new(Statement::Expression(Expr::Number(1))),
                else_clause: None,
            }],
        };

        assert_eq!(run(program), Ok(()));
    }

    #[test]
    fn while_loop() {
        let mut interpreter = Interpreter::new();

        interpreter
            .run(&Program {
                statements: vec![
                    Statement::Let { name: b"x", value: Expr::Number(0) },
                    Statement::While {
                        condition: Expr::Binary {
                            left: Box::new(Expr::Identifier(b"x")),
                            operator: BinaryOp::Less,
                            right: Box::new(Expr::Number(3)),
                        },
                        statement: Box::new(Statement::Assign {
                            name: b"x",
                            value: Expr::Binary {
                                left: Box::new(Expr::Identifier(b"x")),
                                operator: BinaryOp::Add,
                                right: Box::new(Expr::Number(1)),
                            },
                        }),
                    },
                ],
            })
            .unwrap();

        assert_eq!(
            interpreter.eval(&Expr::Identifier(b"x")),
            Ok(Value::Number(3))
        );
    }

    #[test]
    fn while_loop_never_runs() {
        let program = Program {
            statements: vec![Statement::While {
                condition: Expr::Boolean(false),
                statement: Box::new(Statement::Expression(Expr::Number(1))),
            }],
        };

        assert_eq!(run(program), Ok(()));
    }

    #[test]
    fn assignment() {
        let mut interpreter = Interpreter::new();

        interpreter
            .run(&Program {
                statements: vec![
                    Statement::Let { name: b"x", value: Expr::Number(1) },
                    Statement::Assign { name: b"x", value: Expr::Number(2) },
                ],
            })
            .unwrap();

        assert_eq!(
            interpreter.eval(&Expr::Identifier(b"x")),
            Ok(Value::Number(2))
        );
    }

    #[test]
    fn assignment_in_nested_block() {
        let mut interpreter = Interpreter::new();

        interpreter
            .run(&Program {
                statements: vec![
                    Statement::Let { name: b"x", value: Expr::Number(1) },
                    Statement::Block(vec![Statement::Assign {
                        name: b"x",
                        value: Expr::Number(2),
                    }]),
                ],
            })
            .unwrap();

        assert_eq!(
            interpreter.eval(&Expr::Identifier(b"x")),
            Ok(Value::Number(2))
        );
    }
}
