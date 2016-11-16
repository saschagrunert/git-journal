//! The logger implementation
use log::{Log, LogRecord, LogLevel, LogMetadata};

use term::stderr;
use term::color::{BRIGHT_BLUE, GREEN, BRIGHT_YELLOW, RED};

use errors::{GitJournalResult, error};

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
    fn log_result(&self, record: &LogRecord) -> GitJournalResult<()> {
        // We have to create a new terminal on each log because
        // `term::Terminal<Output=std::io::Stderr> + Send + 'static` cannot be shared between
        // threads safely'
        let mut t = stderr().ok_or(error("Term", "Could not create terminal"))?;
        t.fg(BRIGHT_BLUE)?;
        write!(t, "[git-journal] ")?;
        match record.level() {
            LogLevel::Info => {
                t.fg(GREEN)?;
                write!(t, "[OKAY] ")?;
                t.reset()?;
                writeln!(t, "{}", record.args())?;
            }
            LogLevel::Warn => {
                t.fg(BRIGHT_YELLOW)?;
                write!(t, "[WARN] ")?;
                t.reset()?;
                writeln!(t, "{}", record.args())?;
            }
            LogLevel::Error => {
                t.fg(RED)?;
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
