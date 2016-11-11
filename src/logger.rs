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
        let mut t = term::stderr().ok_or(term::Error::NotSupported)?;
        t.fg(term::color::BRIGHT_BLUE)?;
        write!(t, "[git-journal] ")?;
        match record.level() {
            LogLevel::Info => {
                t.fg(term::color::GREEN)?;
                write!(t, "[OKAY] ")?;
                t.reset()?;
                writeln!(t, "{}", record.args())?;
            }
            LogLevel::Warn => {
                t.fg(term::color::BRIGHT_YELLOW)?;
                write!(t, "[WARN] ")?;
                t.reset()?;
                writeln!(t, "{}", record.args())?;
            }
            LogLevel::Error => {
                t.fg(term::color::RED)?;
                write!(t, "[ERROR] ")?;
                t.reset()?;
                writeln!(t, "{}", record.args())?;
            }
            _ => {
                writeln!(t, "[{}] {}", record.level(), record.args())?;
            }
        }
        Ok(())
    }
}
