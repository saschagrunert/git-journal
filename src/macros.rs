macro_rules! println_color_category(
    ($color:expr, $text:tt, $($arg:tt)*) => {{
        let mut t = try!(term::stderr().ok_or(term::Error::NotSupported));
        try!(t.fg($color));
        try!(write!(t, "[ {} ] ", $text));
        try!(writeln!(t, $($arg)*));
        try!(t.reset());
    }}
);

macro_rules! println_ok(
    ($($arg:tt)*) => {{
        println_color_category!(term::color::GREEN, "OKAY", $($arg)*);
    }}
);

macro_rules! println_info(
    ($($arg:tt)*) => {{
        println_color_category!(term::color::YELLOW, "INFO", $($arg)*);
    }}
);

