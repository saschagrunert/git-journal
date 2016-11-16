//! Basic error handling mechanisms

use std::error::Error;
use std::fmt;

/// The result type for GitJournal
pub type GitJournalResult<T> = Result<T, Box<Error>>;

/// Concrete errors
struct GitJournalError {
    description: String,
    detail: Option<String>,
    cause: Option<Box<Error + Send>>,
}

impl fmt::Display for GitJournalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)?;
        if let Some(ref s) = self.detail {
            write!(f, ": {}", s)?;
        }
        Ok(())
    }
}

impl fmt::Debug for GitJournalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for GitJournalError {
    fn description(&self) -> &str {
        &self.description
    }

    fn cause(&self) -> Option<&Error> {
        self.cause.as_ref().map(|c| {
            let e: &Error = &**c;
            e
        })
    }
}

/// Raise an internal error
pub fn internal_error(error: &str, detail: &str) -> Box<Error> {
    Box::new(GitJournalError {
        description: error.to_string(),
        detail: Some(detail.to_string()),
        cause: None,
    })
}

pub fn internal(error: &fmt::Display) -> Box<Error> {
    Box::new(GitJournalError {
        description: error.to_string(),
        detail: None,
        cause: None,
    })
}

macro_rules! bail {
    ($($fmt:tt)*) => (
        return Err(::errors::internal(&format_args!($($fmt)*)))
    )
}
