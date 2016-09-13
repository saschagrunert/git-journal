#[macro_use]
extern crate clap;
extern crate gitjournal;
extern crate term;

use std::process::exit;
use clap::App;
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

fn print_err(string: &str, error: gitjournal::Error) -> Result<(), term::Error> {
    let format_string = format!("{}: {}", string, error);
    try!(print_colored(&format_string, "ERROR", term::color::RED));
    exit(1);
}

static TERM_ERR: &'static str = "Could not print to terminal";
static PATH_ERR: &'static str = "Could not parse 'path' parameter";
static VERI_ERR: &'static str = "Could not parse 'verify' value";
static TAGS_ERR: &'static str = "Could not parse 'tags count' parameter";
static CONV_ERR: &'static str = "Could not parse 'tags count' to integer";
static REVR_ERR: &'static str = "Could not parse 'revision range' parameter";
static SKIP_ERR: &'static str = "Could not parse 'skip pattern' parameter";
static PREP_ERR: &'static str = "Could not parse 'prepare' parameter";
static JOUR_ERR: &'static str = "Could not initialize git-journal";
static PRNT_ERR: &'static str = "Could not print log";
static SETU_ERR: &'static str = "Setup failed.";

fn main() {
    // Load the CLI parameters from the yaml file
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();
    let path = matches.value_of("path").expect(PATH_ERR);

    // Create the journal
    let mut journal = GitJournal::new(path).expect(JOUR_ERR);

    if matches.is_present("verify") {
        // Verify a commit message
        match journal.verify(matches.value_of("verify").expect(VERI_ERR)) {
            Ok(()) => print_ok("Commit message valid.").expect(TERM_ERR),
            Err(error) => print_err("Commit message invalid", error).expect(TERM_ERR),
        }

    } else if matches.is_present("prepare") {
        // Prepare a commit message before editing by the user
        match journal.prepare(matches.value_of("prepare").expect(PREP_ERR)) {
            Ok(()) => print_ok("Commit message prepared.").expect(TERM_ERR),
            Err(error) => print_err("Commit message preparation failed", error).expect(TERM_ERR),
        }

    } else if matches.is_present("setup") {
        // Do the setup procedure
        journal.setup().expect(SETU_ERR);

    } else {
        // Get all values of the given CLI parameters
        let revision_range = matches.value_of("revision_range").expect(REVR_ERR);
        let tag_skip_pattern = matches.value_of("tag_skip_pattern").expect(SKIP_ERR);
        let tags_count = matches.value_of("tags_count").expect(TAGS_ERR);
        let max_tags = tags_count.parse::<u32>().expect(CONV_ERR);

        // Parse the log
        if let Err(error) = journal.parse_log(revision_range,
                                              tag_skip_pattern,
                                              &max_tags,
                                              &matches.is_present("all"),
                                              &matches.is_present("skip_unreleased")) {
            print_err("Log parsing error", error).expect(TERM_ERR);
        }
        journal.print_log(matches.is_present("short")).expect(PRNT_ERR);
    }
}
