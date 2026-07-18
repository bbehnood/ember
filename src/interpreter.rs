use std::{collections::HashMap, fmt::Display};

use crate::ast::{BinaryOp, Expr, Program, Statement};

pub struct Interpreter<'a> {
    variables: HashMap<&'a [u8], Value>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Number(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
pub enum RuntimeError {
    #[error("attempt to divide by zero")]
    DivideByZero,

    #[error("arithmetic overflow")]
    ArithmeticOverflow,
}

impl Display for Value {
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
                .expect("undefined variables should be caught by sema")),

            Expr::Binary { left, operator, right } => {
                let l = self.eval(left)?;
                let r = self.eval(right)?;

                match (l, r) {
                    (Value::Number(a), Value::Number(b)) => {
                        let result = match operator {
                            BinaryOp::Add => a.checked_add(b),
                            BinaryOp::Sub => a.checked_sub(b),
                            BinaryOp::Mul => a.checked_mul(b),
                            BinaryOp::Div => {
                                if b == 0 {
                                    return Err(RuntimeError::DivideByZero);
                                }

                                a.checked_div(b)
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
