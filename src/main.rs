extern crate gitjournal;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;

use std::process::exit;
use std::{env, fmt, fs};

use clap::{App, Shell};
use gitjournal::GitJournal;

fn error_and_exit(string: &str, error: Error) {
    error!("{}: {}", string, error);
    exit(1);
}

enum Error {
    Cli,
    ParseInt(std::num::ParseIntError),
    GitJournal(gitjournal::Error),
}

impl From<gitjournal::Error> for Error {
    fn from(err: gitjournal::Error) -> Error {
        Error::GitJournal(err)
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
            Error::GitJournal(ref err) => write!(f, "GitJournal: {}", err),
        }
    }
}

fn is_program_in_path(program: &str) -> bool {
    if let Ok(path) = env::var("PATH") {
        for p in path.split(':') {
            let p_str = format!("{}/{}", p, program);
            if fs::metadata(p_str).is_ok() {
                return true;
            }
        }
    }
    false
}

fn main() {
    if let Err(error) = run() {
        error_and_exit("Main", error);
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
                match journal.prepare(try!(sub_matches.value_of("message").ok_or(Error::Cli)),
                                      sub_matches.value_of("type")) {
                    Ok(()) => info!("Commit message prepared."),
                    Err(error) => {
                        error_and_exit("Commit message preparation failed",
                                       Error::GitJournal(error))
                    }
                }
            }
        }
        Some("setup") => {
            // Do the setup procedure
            try!(journal.setup());

            // Generate completions if necessary
            if is_program_in_path("bash") {
                app.gen_completions("git-journal", Shell::Bash, path);
                info!("Installed bash completions to the current path.");
            }
            if is_program_in_path("fish") {
                app.gen_completions("git-journal", Shell::Fish, path);
                info!("Installed fish completions to the current path.");
            }
            if is_program_in_path("zsh") {
                app.gen_completions("git-journal", Shell::Zsh, path);
                info!("Installed zsh completions to the current path.");
            }
        }
        Some("verify") => {
            // Verify a commit message
            if let Some(sub_matches) = matches.subcommand_matches("verify") {
                match journal.verify(try!(sub_matches.value_of("message").ok_or(Error::Cli))) {
                    Ok(()) => info!("Commit message valid."),
                    Err(error) => error_and_exit("Commit message invalid", Error::GitJournal(error)),
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
                error_and_exit("Log parsing error", Error::GitJournal(error));
            }

            // Generate the template or print the log
            if matches.is_present("generate") {
                try!(journal.generate_template());
            } else {
                try!(journal.print_log(matches.is_present("short"),
                                       matches.value_of("template"),
                                       matches.value_of("output")));
            }
        }
    };
    Ok(())
}
