use rustc_serialize::Encodable;
use toml::{Encoder, Value, Parser, encode_str, decode};
use toml;

use std::io;
use std::fmt;
use std::fs::File;
use std::path::PathBuf;
use std::io::prelude::*;

#[derive(Debug)]
pub enum Error {
    Toml(toml::Error),
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
            Error::Toml(ref err) => write!(f, "Toml error: {}", err),
            Error::Io(ref err) => write!(f, "Io error: {}", err),
        }
    }
}
#[derive(Default, Debug, PartialEq, RustcEncodable, RustcDecodable)]
pub struct Config {
    pub show_prefix: bool,
    pub colored_output: bool,
    pub excluded_tags: Vec<String>,
    pub enable_debug: bool,
}

impl Config {
    pub fn new() -> Config {
        Config {
            show_prefix: false,
            colored_output: true,
            excluded_tags: vec![],
            enable_debug: true,
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
        let toml_string = encode_str(&Value::Table(encoder.toml));

        let path_buf = self.get_path_with_filename(path);
        let path_string = try!(path_buf.to_str()
            .ok_or(io::Error::new(io::ErrorKind::Other, "Cannot convert path to string")));

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
        *self = try!(decode(Value::Table(toml)).ok_or(toml::Error::Custom("Decoding error".to_owned())));
        Ok(())
    }

    pub fn is_default_config(&self) -> bool {
        *self == Config::new()
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
