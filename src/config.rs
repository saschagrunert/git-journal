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
#[derive(Default, Debug, Clone, PartialEq, RustcEncodable, RustcDecodable)]
pub struct Config {
    /// Specifies the available categories for the commit message
    pub categories: Vec<String>,

    /// Set the characters where the categories are wrapped in
    pub category_delimiters: (String, String),

    /// Set to false if the output should not be colored
    pub colored_output: bool,

    /// Specifies the default template. Will be used for tag validation and printing.
    pub default_template: Option<String>,

    /// Show or hide the debug messages like `[OKAY] ...` or `[INFO] ...`
    pub enable_debug: bool,

    /// Excluded tags in an array, e.g. "internal"
    pub excluded_commit_tags: Vec<String>,

    /// Enable or disable the output and accumulation of commit footers
    pub enable_footers: bool,

    /// Show or hide the commit hash for every entry
    pub show_commit_hash: bool,

    /// Show or hide the commit message prefix, e.g. JIRA-1234
    pub show_prefix: bool,

    /// Sort the commits during the output by "date" (default) or "name"
    pub sort_by: String,

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
            categories: Self::get_default_categories(),
            category_delimiters: ("[".to_owned(), "]".to_owned()),
            colored_output: true,
            default_template: None,
            enable_debug: true,
            excluded_commit_tags: vec![],
            enable_footers: false,
            show_commit_hash: false,
            show_prefix: false,
            sort_by: "date".to_owned(),
            template_prefix: "JIRA-1234".to_owned(),
        }
    }

    fn get_default_categories() -> Vec<String> {
        vec!["Added".to_owned(), "Changed".to_owned(), "Fixed".to_owned(), "Improved".to_owned(), "Removed".to_owned()]
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
        self.encode(&mut encoder)?;
        let toml_string = encode_str(&Value::Table(encoder.toml));

        let path_buf = self.get_path_with_filename(path);
        let path_string = path_buf.to_str()
            .ok_or(io::Error::new(io::ErrorKind::Other, "Cannot convert path to string"))?;

        let mut file = File::create(&path_buf)?;
        file.write_all(toml_string.as_bytes())?;
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
        let mut file = File::open(path_buf)?;
        let mut toml_string = String::new();
        file.read_to_string(&mut toml_string)?;

        let toml = Parser::new(&toml_string).parse()
            .ok_or(toml::Error::Custom("Could not parse toml configuration.".to_owned()))?;
        *self =
            decode(Value::Table(toml)).ok_or(toml::Error::Custom("Could not decode toml configuration.".to_owned()))?;

        // If the categories are not found within the toml it will return an empty array
        // which will break the parser. So use the default ones instead.
        if self.categories.is_empty() {
            self.categories = Self::get_default_categories();
        }
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
