//! The logger implementation
use log::{Log, LogRecord, LogLevel, LogMetadata};
use term;

/// The logging structure
pub struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LogLevel::Info
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            if let Err(e) = self.log_result(record) {
                println!("Error printing to log: {}", e);
            }
        }
    }
}

impl Logger {
    fn log_result(&self, record: &LogRecord) -> Result<(), term::Error> {
        // We have to create a new terminal on each log because
        // `term::Terminal<Output=std::io::Stderr> + Send + 'static` cannot be shared between
        // threads safely'
        let mut t = try!(term::stderr().ok_or(term::Error::NotSupported));
        try!(t.fg(term::color::BRIGHT_BLUE));
        try!(write!(t, "[git-journal] "));
        match record.level() {
            LogLevel::Info => {
                try!(t.fg(term::color::GREEN));
                try!(write!(t, "[OKAY] "));
                try!(t.reset());
                try!(writeln!(t, "{}", record.args()));
            }
            LogLevel::Warn => {
                try!(t.fg(term::color::BRIGHT_YELLOW));
                try!(write!(t, "[WARN] "));
                try!(t.reset());
                try!(writeln!(t, "{}", record.args()));
            }
            LogLevel::Error => {
                try!(t.fg(term::color::RED));
                try!(write!(t, "[ERROR] "));
                try!(t.reset());
                try!(writeln!(t, "{}", record.args()));
            }
            _ => {
                try!(writeln!(t, "[{}] {}", record.level(), record.args()));
            }
        }
        Ok(())
    }
}
