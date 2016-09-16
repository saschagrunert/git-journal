//! Everything related to the git-journal configuration. The configuration files are stored in
//! [toml](https://github.com/toml-lang/toml) format with the file name `.gitjournal.toml`.
//!

use rustc_serialize::Encodable;
use toml::{Encoder, Value, Parser, encode_str, decode};
use toml;

use std::io;
use std::fmt;
use std::fs::File;
use std::path::PathBuf;
use std::io::prelude::*;

/// An enumeration of possible errors that can happen when working with the configuration.
#[derive(Debug)]
pub enum Error {
    /// Erros related to the toml parsing.
    Toml(toml::Error),

    /// Erros related to the system IO, like saving the configuration file.
    Io(io::Error),
}

impl From<toml::Error> for Error {
    fn from(err: toml::Error) -> Error {
        Error::Toml(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Toml(ref err) => write!(f, "Toml: {}", err),
            Error::Io(ref err) => write!(f, "Io: {}", err),
        }
    }
}

/// The configuration structure for git-journal.
#[derive(Default, Debug, PartialEq, RustcEncodable, RustcDecodable)]
pub struct Config {
    /// Set to false if the output should not be colored
    pub colored_output: bool,

    /// Show or hide the debug messages like `[OKAY] ...` or `[INFO] ...`
    pub enable_debug: bool,

    /// Excluded tags in an array, e.g. "internal"
    pub excluded_tags: Vec<String>,

    /// Show or hide the commit message prefix, e.g. JIRA-1234
    pub show_prefix: bool,

    /// Commit message template prefix which will be added during commit preparation
    pub template_prefix: String,
}

impl Config {
    /// Constructs a new `Config` with default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitjournal::Config;
    /// let config = Config::new();
    /// ```
    ///
    pub fn new() -> Self {
        Config {
            colored_output: true,
            enable_debug: true,
            excluded_tags: vec![],
            show_prefix: false,
            template_prefix: "JIRA-1234".to_owned(),
        }
    }

    /// Save the default configuration file in a certain path.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitjournal::Config;
    /// Config::new().save_default_config(".").expect("Could not save config.");
    /// ```
    ///
    /// # Errors
    /// When toml encoding or file creation failed.
    ///
    pub fn save_default_config(&self, path: &str) -> Result<String, Error> {
        let mut encoder = Encoder::new();
        try!(self.encode(&mut encoder));
        let toml_string = encode_str(&Value::Table(encoder.toml));

        let path_buf = self.get_path_with_filename(path);
        let path_string = try!(path_buf.to_str()
            .ok_or(io::Error::new(io::ErrorKind::Other, "Cannot convert path to string")));

        let mut file = try!(File::create(path_buf.clone()));
        try!(file.write_all(toml_string.as_bytes()));
        Ok(path_string.to_owned())
    }

    /// Load a configuration file from a certain path.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitjournal::Config;
    /// Config::new().load(".").expect("Could not load config.");
    /// ```
    ///
    /// # Errors
    /// When toml decoding or file opening failed.
    ///
    pub fn load(&mut self, path: &str) -> Result<(), Error> {
        let path_buf = self.get_path_with_filename(path);
        let mut file = try!(File::open(path_buf));
        let mut toml_string = String::new();
        try!(file.read_to_string(&mut toml_string));

        let toml = try!(Parser::new(&toml_string)
            .parse()
            .ok_or(toml::Error::Custom("Could not parse toml configuration.".to_owned())));
        *self = try!(decode(Value::Table(toml))
            .ok_or(toml::Error::Custom("Could not decode toml configuration.".to_owned())));
        Ok(())
    }

    /// Check if the configuration matches with the default one.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitjournal::Config;
    /// assert_eq!(Config::new().is_default_config(), true);
    /// ```
    ///
    pub fn is_default_config(&self) -> bool {
        *self == Config::new()
    }

    fn get_path_with_filename(&self, path: &str) -> PathBuf {
        let mut path_buf = PathBuf::from(path);
        path_buf.push(".gitjournal.toml");
        path_buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_save_and_load_ok() {
        let mut config = Config::new();
        assert!(config.save_default_config(".").is_ok());
        assert!(config.load(".").is_ok());
        assert_eq!(config.is_default_config(), true);
    }


    #[test]
    fn config_save_err() {
        let config = Config::new();
        let res = config.save_default_config("/dev/null");
        assert!(res.is_err());
        if let Err(e) = res {
            println!("{}", e);
        }
    }

    fn load_and_print_failure(path: &str) {
        let mut config = Config::new();
        let res = config.load(path);
        assert!(res.is_err());
        if let Err(e) = res {
            println!("{}", e);
        }
    }

    #[test]
    fn config_load_err() {
        load_and_print_failure("/dev/null");
    }

    #[test]
    fn config_load_invalid_1() {
        load_and_print_failure("tests/invalid_1.toml");
    }

    #[test]
    fn config_load_invalid_2() {
        load_and_print_failure("tests/invalid_2.toml");
    }

    #[test]
    fn config_load_invalid_3() {
        load_and_print_failure("tests/invalid_3.toml");
    }
}
