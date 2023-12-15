use std::{io, num};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unable to parse expression: {0}")]
    Expr(#[from] ExpressionError),

    #[error(transparent)]
    IO(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ExpressionError {
    #[error("Unable to parse expression: {0}")]
    BadExpression(String),

    #[error("Bad integer: {0}; {1}")]
    BadInteger(String, num::ParseIntError),

    #[error(transparent)]
    Io(#[from] io::Error),
}
