//! Static semantic analysis: variable resolution and type checking.
//!
//! [`Sema`] walks the AST once, before the interpreter runs, verifying that
//! every variable is declared before use and that operand types line up
//! with what each operator expects. This lets the interpreter itself skip
//! those checks entirely (see the `expect`-based unreachables in
//! [`crate::interpreter`]) since a program that reaches execution has
//! already been proven well-typed.

use std::collections::HashMap;

use thiserror::Error;

use crate::ast::{BinaryOp, Expr, Program, Statement};

/// Performs a single static-analysis pass over a [`Program`].
///
/// Like the interpreter, `Sema` tracks variables using a stack of scopes -
/// one [`HashMap`] per lexical block - so that shadowing and scoping rules
/// can be checked without actually running the program.
pub struct Sema<'a> {
    scopes: Vec<HashMap<&'a [u8], Type>>,
}

/// The type of an Ember value, as determined statically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Number,
    Boolean,
}

/// Errors that can occur during semantic analysis.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SemaError {
    /// A variable was referenced (in an expression or an assignment) that
    /// hasn't been declared in the current or any enclosing scope.
    #[error("undefined variable '{0}'")]
    UndefinedVariable(String),

    /// A `let` declared a variable that already exists in the *same*
    /// scope. Shadowing across scopes is fine; only same-scope redeclaration
    /// is rejected.
    #[error("variable '{0}' is already defined")]
    DuplicateVariable(String),

    /// An operand (or condition) had a type other than the one required,
    /// e.g. using a boolean where a number was expected.
    #[error("expected {expected}, found {found}")]
    UnexpectedType { expected: Type, found: Type },

    /// The two operands of `==`/`!=` had different types, e.g. comparing a
    /// number to a boolean.
    #[error("mismatched types: '{left}' and '{right}'")]
    MismatchedTypes { left: Type, right: Type },
}

impl BinaryOp {
    /// Checks that `left` and `right` are valid operand types for this
    /// operator and returns the resulting type if so.
    ///
    /// The operators fall into three families with different rules:
    /// - Arithmetic (`+ - * /`): both operands must be `Number`, result is
    ///   `Number`.
    /// - Ordering comparisons (`< <= > >=`): both operands must be
    ///   `Number`, result is `Boolean`.
    /// - Equality (`== !=`): operands may be any type as long as they
    ///   *match each other*, result is `Boolean`.
    /// - Logical (`&& ||`): both operands must be `Boolean`, result is
    ///   `Boolean`.
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

impl<'a> Sema<'a> {
    #[must_use]
    pub fn new() -> Self {
        Self { scopes: vec![HashMap::new()] }
    }

    /// Type-checks an entire program, statement by statement, starting in
    /// the single top-level (global) scope.
    pub fn check(&mut self, program: &Program<'a>) -> Result<(), SemaError> {
        for stmt in &program.statements {
            self.check_statement(stmt)?;
        }

        Ok(())
    }

    /// Type-checks a single statement, updating scope state as needed
    /// (e.g. `let` inserts into the current scope, blocks push/pop a new
    /// scope).
    fn check_statement(
        &mut self,
        stmt: &Statement<'a>,
    ) -> Result<(), SemaError> {
        match stmt {
            Statement::Let { name, value } => {
                let value_type = self.check_expr(value)?;

                // Redeclaring a name within the *same* scope is an error,
                // but shadowing a name from an outer scope is allowed - so
                // this only checks the innermost (`.last()`) scope.
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

                // Unlike `let`, assignment doesn't care which scope the
                // variable lives in - it just needs to exist *somewhere* on
                // the scope stack, searched innermost-first.
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
                // Blocks introduce a new lexical scope: variables declared
                // inside are dropped once the block ends, so later
                // references to them correctly fail as undefined (see the
                // `variable_out_of_scope_after_block` test).
                self.scopes.push(HashMap::new());
                for stmt in statements {
                    self.check_statement(stmt)?;
                }

                self.scopes.pop();

                Ok(())
            }

            // NOTE: only the `condition` is type-checked here - the
            // `statement`/`else_clause` bodies (`..`) are never recursed
            // into. In practice this means type errors inside an `if`/
            // `while` body (e.g. `if true { 1 + true; }`) are not caught
            // by Sema and will only surface as a panic/`unreachable!()` at
            // runtime in `Interpreter::eval`. This looks like a real gap
            // rather than intentional behavior; flagging it here rather
            // than silently "fixing" it, since that's a semantic change
            // beyond documentation.
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

            // NOTE: same gap as `Statement::If` above - the loop body isn't
            // type-checked, only the condition.
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

    /// Type-checks an expression and returns its resulting [`Type`].
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

/// Formats a type for use in diagnostic messages, e.g.
/// `expected number, found boolean`.
impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Number => write!(f, "number"),
            Type::Boolean => write!(f, "boolean"),
        }
    }
}

impl Default for Sema<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts a borrowed identifier byte slice into an owned `String` for
/// embedding in [`SemaError`] variants.
///
/// Panics if `name` isn't valid UTF-8, which should never happen: the
/// lexer only ever produces identifiers from ASCII alphanumeric characters
/// and `_` (see [`crate::lexer::Lexer::read_identifier`]).
fn name_as_string(name: &[u8]) -> String {
    std::str::from_utf8(name)
        .expect("The lexer only consumes valid ASCII")
        .to_owned()
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
