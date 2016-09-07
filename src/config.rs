use rustc_serialize::Encodable;
use toml::{Encoder, Value, Parser};
use toml;

use std;
use std::fmt;
use std::fs::File;
use std::path::PathBuf;
use std::io::prelude::*;

#[derive(Debug)]
pub enum Error {
    Toml(toml::Error),
    Io(std::io::Error),
}

impl From<toml::Error> for Error {
    fn from(err: toml::Error) -> Error {
        Error::Toml(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Toml(ref err) => write!(f, "Toml error: {}", err),
            Error::Io(ref err) => write!(f, "Io error: {}", err),
        }
    }
}
#[derive(Debug, RustcEncodable, RustcDecodable)]
pub struct Config {
    pub show_prefix: bool,
    pub excluded_tags: Vec<String>,
    pub categories: Vec<String>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            show_prefix: false,
            excluded_tags: vec![],
            categories: vec!["Added".to_owned(),
                             "Changed".to_owned(),
                             "Fixed".to_owned(),
                             "Improved".to_owned(),
                             "Removed".to_owned()],
        }
    }

    fn get_path_with_filename(&self, path: &str) -> PathBuf {
        let mut path_buf = PathBuf::from(path);
        path_buf.push(".gitjournal.toml");
        path_buf
    }

    pub fn save_default_config(&self, path: &str) -> Result<String, Error> {
        let mut encoder = Encoder::new();
        try!(self.encode(&mut encoder));
        let toml_string = toml::encode_str(&Value::Table(encoder.toml));

        let path_buf = self.get_path_with_filename(path);
        let path_string = try!(path_buf.to_str()
            .ok_or(std::io::Error::new(std::io::ErrorKind::Other, "Cannot convert path to string")));

        let mut file = try!(File::create(path_buf.clone()));
        try!(file.write_all(toml_string.as_bytes()));
        Ok(path_string.to_owned())
    }

    pub fn load(&mut self, path: &str) -> Result<(), Error> {
        let path_buf = self.get_path_with_filename(path);
        let mut file = try!(File::open(path_buf));
        let mut toml_string = String::new();
        try!(file.read_to_string(&mut toml_string));

        let toml = try!(Parser::new(&toml_string).parse().ok_or(toml::Error::Custom("Parsing error".to_owned())));
        *self = try!(toml::decode(Value::Table(toml)).ok_or(toml::Error::Custom("Decoding error".to_owned())));
        Ok(())
    }
}
