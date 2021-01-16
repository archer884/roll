use std::num::ParseIntError;

#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    #[error("Bad expression: {0}")]
    BadExpression(String),

    #[error("{0}")]
    ParseInt(ParseIntError),
}

impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Self {
        Error::ParseInt(e)
    }
}
