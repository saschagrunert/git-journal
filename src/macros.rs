macro_rules! println_color_category(
    ($color:expr, $text:tt, $($arg:tt)*) => {{
        let mut t = try!(term::stderr().ok_or(term::Error::NotSupported));
        try!(t.fg(term::color::YELLOW));
        try!(write!(t, "[git-journal] "));
        try!(t.fg($color));
        try!(write!(t, "[{}] ", $text));
        try!(t.reset());
        try!(writeln!(t, $($arg)*));
    }}
);

macro_rules! println_ok(
    ($($arg:tt)*) => {{
        println_color_category!(term::color::BRIGHT_GREEN, "OKAY", $($arg)*);
    }}
);

macro_rules! println_info(
    ($($arg:tt)*) => {{
        println_color_category!(term::color::BRIGHT_BLUE, "INFO", $($arg)*);
    }}
);

macro_rules! println_warn(
    ($($arg:tt)*) => {{
        println_color_category!(term::color::BRIGHT_YELLOW, "WARN", $($arg)*);
    }}
);

macro_rules! trywln(
    ($($arg:tt)*) => {{
        try!(writeln!($($arg)*));
    }}
);

macro_rules! tryw(
    ($($arg:tt)*) => {{
        try!(write!($($arg)*));
    }}
);
