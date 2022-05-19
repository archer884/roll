use std::io;
use std::num::ParseIntError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unable to parse expression: {0}")]
    BadExpression(String),
    #[error("Bad integer: {0}; {1}")]
    BadInteger(String, ParseIntError),
    #[error(transparent)]
    IoError(#[from] io::Error),
}
