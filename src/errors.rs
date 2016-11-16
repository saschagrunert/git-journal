//! Basic error handling mechanisms

use std::error::Error;
use std::{fmt, io, num};

use git2;
use log;
use term;
use toml;

/// The result type for GitJournal
pub type GitJournalResult<T> = Result<T, Box<GitJournalError>>;

/// GitJournal error trait
pub trait GitJournalError: Error + Send + 'static {}

/// Concrete errors
struct ConcreteGitJournalError {
    description: String,
    detail: Option<String>,
    cause: Option<Box<Error + Send>>,
}

impl fmt::Display for ConcreteGitJournalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)?;
        if let Some(ref s) = self.detail {
            write!(f, ": {}", s)?;
        }
        Ok(())
    }
}

impl fmt::Debug for ConcreteGitJournalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for ConcreteGitJournalError {
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

/// Various error implementors
macro_rules! from_error {
    ($($p:ty,)*) => (
        $(impl From<$p> for Box<GitJournalError> {
            fn from(t: $p) -> Box<GitJournalError> { Box::new(t) }
        })*
    )
}

from_error! {
    git2::Error,
    io::Error,
    log::ShutdownLoggerError,
    num::ParseIntError,
    term::Error,
    toml::Error,
}

impl GitJournalError for git2::Error {}
impl GitJournalError for io::Error {}
impl GitJournalError for log::ShutdownLoggerError {}
impl GitJournalError for num::ParseIntError {}
impl GitJournalError for term::Error {}
impl GitJournalError for toml::Error {}
impl GitJournalError for ConcreteGitJournalError {}

/// Raise and internal error
pub fn internal_error(error: &str, detail: &str) -> Box<GitJournalError> {
    Box::new(ConcreteGitJournalError {
        description: error.to_string(),
        detail: Some(detail.to_string()),
        cause: None,
    })
}

pub fn internal(error: &fmt::Display) -> Box<GitJournalError> {
    Box::new(ConcreteGitJournalError {
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
