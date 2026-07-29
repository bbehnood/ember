use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, Program, Statement};

pub struct Interpreter<'a> {
    variables: HashMap<&'a [u8], Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    Number(i64),
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
        }
    }
}

impl<'a> Interpreter<'a> {
    #[must_use]
    pub fn new() -> Self {
        Self { variables: HashMap::new() }
    }

    pub fn run(
        &mut self,
        program: &Program<'a>,
    ) -> Result<Option<Value>, RuntimeError> {
        let mut last = None;
        for stmt in &program.statements {
            last = self.execute_statement(stmt)?;
        }

        Ok(last)
    }

    fn execute_statement(
        &mut self,
        stmt: &Statement<'a>,
    ) -> Result<Option<Value>, RuntimeError> {
        match stmt {
            Statement::Let { name, value } => {
                let value = self.eval(value)?;
                self.variables.insert(name, value);

                Ok(None)
            }

            Statement::Expression(expr) => Ok(Some(self.eval(expr)?)),
        }
    }

    fn eval(&mut self, expr: &Expr<'a>) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),

            Expr::Identifier(name) => Ok(self
                .variables
                .get(name)
                .copied()
                .expect("Undefined variables should be caught at sema")),

            Expr::Binary { left, operator, right } => {
                let left = self.eval(left)?;
                let right = self.eval(right)?;

                match (left, right) {
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
                        };

                        result
                            .map(Value::Number)
                            .ok_or(RuntimeError::ArithmeticOverflow)
                    }
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
