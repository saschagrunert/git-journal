//! Basic error handling mechanisms

use std::{io, num};
use {git2, log, term, toml};

error_chain! {
    foreign_links {
         Git(git2::Error) #[doc="A git error."];
         Io(io::Error) #[doc="An I/O error."];
         Log(log::ShutdownLoggerError) #[doc="A logger error error."];
         Term(term::Error) #[doc="A terminal error."];
         TomlDeser(toml::de::Error) #[doc="A toml deserialization error."];
         ParseInt(num::ParseIntError) #[doc="A integer parsing error."];
         TomlSer(toml::ser::Error) #[doc="A toml serialization error."];
    }
}
