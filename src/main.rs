#[macro_use]
extern crate clap;
extern crate gitjournal;
extern crate term;

use std::fmt;
use std::process::exit;

use clap::{App, Shell};
use gitjournal::GitJournal;

fn print_colored(string: &str, prefix: &str, color: term::color::Color) -> Result<(), term::Error> {
    let mut t = try!(term::stderr().ok_or(term::Error::NotSupported));
    try!(t.fg(term::color::YELLOW));
    try!(write!(t, "[git-journal] "));
    try!(t.fg(color));
    try!(write!(t, "[{}] ", prefix));
    try!(t.reset());
    try!(writeln!(t, "{}", string));
    Ok(())
}

fn print_ok(string: &str) -> Result<(), term::Error> {
    try!(print_colored(string, "OKAY", term::color::GREEN));
    Ok(())
}

fn print_err_exit(string: &str, error: Error) -> Result<(), term::Error> {
    let format_string = format!("{}: {}", string, error);
    try!(print_colored(&format_string, "ERROR", term::color::RED));
    exit(1);
}

enum Error {
    Cli,
    ParseInt(std::num::ParseIntError),
    Term(term::Error),
    GitJournal(gitjournal::Error),
}

impl From<gitjournal::Error> for Error {
    fn from(err: gitjournal::Error) -> Error {
        Error::GitJournal(err)
    }
}

impl From<term::Error> for Error {
    fn from(err: term::Error) -> Error {
        Error::Term(err)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Error {
        Error::ParseInt(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Cli => write!(f, "Cli argument parsing"),
            Error::ParseInt(ref err) => write!(f, "ParseInt: {}", err),
            Error::Term(ref err) => write!(f, "Term: {}", err),
            Error::GitJournal(ref err) => write!(f, "GitJournal: {}", err),
        }
    }
}

fn main() {
    if let Err(error) = run() {
        print_err_exit("Main", error).expect("Cannot print error message");
    }
}

fn run() -> Result<(), Error> {
    // Load the CLI parameters from the yaml file
    let yaml = load_yaml!("cli.yaml");
    let mut app = App::from_yaml(yaml).version(crate_version!());
    let matches = app.clone().get_matches();
    let path = try!(matches.value_of("path").ok_or(Error::Cli));

    // Create the journal
    let mut journal = try!(GitJournal::new(path));

    // Check for the subcommand
    match matches.subcommand_name() {
        Some("prepare") => {
            // Prepare a commit message before editing by the user
            if let Some(sub_matches) = matches.subcommand_matches("prepare") {
                match journal.prepare(try!(sub_matches.value_of("message").ok_or(Error::Cli))) {
                    Ok(()) => try!(print_ok("Commit message prepared.")),
                    Err(error) => {
                        try!(print_err_exit("Commit message preparation failed",
                                            Error::GitJournal(error)))
                    }
                }
            }
        }
        Some("setup") => {
            // Do the setup procedure
            try!(journal.setup());

            // Generate completions
            app.gen_completions("git-journal", Shell::Bash, env!("PWD"));
            app.gen_completions("git-journal", Shell::Fish, env!("PWD"));
        }
        Some("verify") => {
            // Verify a commit message
            if let Some(sub_matches) = matches.subcommand_matches("verify") {
                match journal.verify(try!(sub_matches.value_of("message").ok_or(Error::Cli))) {
                    Ok(()) => try!(print_ok("Commit message valid.")),
                    Err(error) => try!(print_err_exit("Commit message invalid", Error::GitJournal(error))),
                }
            }
        }
        _ => {
            // Get all values of the given CLI parameters with default values
            let revision_range = try!(matches.value_of("revision_range").ok_or(Error::Cli));
            let tag_skip_pattern = try!(matches.value_of("tag_skip_pattern").ok_or(Error::Cli));
            let tags_count = try!(matches.value_of("tags_count").ok_or(Error::Cli));
            let max_tags = try!(tags_count.parse::<u32>());

            // Parse the log
            if let Err(error) = journal.parse_log(revision_range,
                                                  tag_skip_pattern,
                                                  &max_tags,
                                                  &matches.is_present("all"),
                                                  &matches.is_present("skip_unreleased")) {
                try!(print_err_exit("Log parsing error", Error::GitJournal(error)));
            }
            try!(journal.print_log(matches.is_present("short"),
                                   matches.value_of("template"),
                                   matches.value_of("output")));
        }
    };
    Ok(())
}
