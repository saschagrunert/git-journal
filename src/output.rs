use failure::Error;
use term::color::Color;

/// An abstraction over all outputs
pub enum Output {
    /// Buffer that is used for file output
    Buffer(Vec<u8>),
    /// Stdout Terminal
    Terminal(Box<term::StdoutTerminal>),
    /// Stderr as fallback if a terminal cannot be instantiated
    TerminalFallback(std::io::Stdout),
}

impl Output {
    /// Creates an output that writes into a buffer
    pub fn buffered() -> Self {
        Output::Buffer(Vec::new())
    }

    /// Creates an output that writes into the terminal
    pub fn terminal() -> Self {
        if let Some(terminal) = term::stdout() {
            Self::Terminal(terminal)
        } else {
            Self::TerminalFallback(std::io::stdout())
        }
    }

    /// Tests if the Output is to a buffer
    pub fn is_buffered(&self) -> bool {
        if let Self::Buffer(_) = self {
            true
        } else {
            false
        }
    }

    /// Sets the foreground color for the terminal
    pub fn fg(&mut self, color: Color) -> Result<(), Error> {
        if let Self::Terminal(t) = self {
            t.fg(color)?;
        }
        Ok(())
    }

    /// Resets the colors for the terminal
    pub fn reset(&mut self) -> Result<(), Error> {
        if let Self::Terminal(t) = self {
            t.reset()?;
        }
        Ok(())
    }
}

/// Implement Write for `Output` by forwarding to the underlying Writers
impl std::io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Buffer(b) => b.write(buf),
            Self::Terminal(t) => t.write(buf),
            Self::TerminalFallback(e) => e.write(buf),
        }
    }

    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        match self {
            Self::Buffer(b) => b.write_vectored(bufs),
            Self::Terminal(t) => t.write_vectored(bufs),
            Self::TerminalFallback(e) => e.write_vectored(bufs),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Buffer(b) => b.flush(),
            Self::Terminal(t) => t.flush(),
            Self::TerminalFallback(e) => e.flush(),
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self {
            Self::Buffer(b) => b.write_all(buf),
            Self::Terminal(t) => t.write_all(buf),
            Self::TerminalFallback(e) => e.write_all(buf),
        }
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::io::Result<()> {
        match self {
            Self::Buffer(b) => b.write_fmt(args),
            Self::Terminal(t) => t.write_fmt(args),
            Self::TerminalFallback(e) => e.write_fmt(args),
        }
    }
}
