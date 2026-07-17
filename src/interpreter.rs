use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, Program, Statement};

pub struct Interpreter<'a> {
    variables: HashMap<&'a [u8], Value>,
}

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Number(i64),
}

impl<'a> Interpreter<'a> {
    pub fn new() -> Self {
        Self { variables: HashMap::new() }
    }

    pub fn run(&mut self, program: &Program<'a>) -> Option<Value> {
        let mut last = None;

        for stmt in &program.statements {
            last = self.execute_statement(&stmt);
        }

        last
    }

    fn execute_statement(&mut self, stmt: &Statement<'a>) -> Option<Value> {
        match stmt {
            Statement::Let { name, value } => {
                let value = self.eval(value);
                self.variables.insert(name, value);

                None
            }

            Statement::Expression(expr) => Some(self.eval(expr)),
        }
    }

    fn eval(&mut self, expr: &Expr<'a>) -> Value {
        match expr {
            Expr::Number(n) => Value::Number(*n),

            Expr::Identifier(name) => self
                .variables
                .get(name)
                .copied()
                .expect("Undefined variables are caught at sema"),

            Expr::Binary { left, operator, right } => {
                let l = self.eval(left);
                let r = self.eval(right);

                match (l, r) {
                    (Value::Number(a), Value::Number(b)) => match operator {
                        BinaryOp::Add => Value::Number(a + b),
                        BinaryOp::Sub => Value::Number(a - b),
                        BinaryOp::Mul => Value::Number(a * b),
                        BinaryOp::Div => Value::Number(a / b),
                    },
                }
            }
        }
    }
}
