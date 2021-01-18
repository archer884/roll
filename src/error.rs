use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unable to parse expression: {0}")]
    Expr(expr::Error),
    #[error("IO error: {0}")]
    IO(io::Error),
}

impl From<expr::Error> for Error {
    fn from(e: expr::Error) -> Self {
        Error::Expr(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(e)
    }
}
