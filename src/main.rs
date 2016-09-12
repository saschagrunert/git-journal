#[macro_use]
extern crate clap;
extern crate gitjournal;

use std::process::exit;
use clap::App;
use gitjournal::GitJournal;

fn print(string: &str) {
    println!("[Git-Journal] {}", string)
}

fn main() {
    // Load the CLI parameters from the yaml file
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();
    let path = matches.value_of("path").expect("Could not parse 'path' parameter");

    if matches.is_present("verify") {
        // Verify a commit message and panic! if verification failed.
        match GitJournal::verify(matches.value_of("verify").expect("Could not parse 'verify' value.")) {
            Ok(()) => print("Commit message valid."),
            Err(error) => {
                print(&format!("Commit message invalid: {}", error));
                exit(1);
            }
        }
    } else if matches.is_present("prepare") {
        // Prepare a commit message before editing by the user
        match GitJournal::prepare(matches.value_of("prepare").expect("Could not parse 'prepare' value.")) {
            Ok(()) => print("Commit message prepared."),
            Err(error) => {
                print(&format!("Commit message preparation failed: {}", error));
                exit(1);
            }
        }
    } else {
        // Create the journal
        let mut journal = GitJournal::new(path).expect("Could not initialize journal");

        if matches.is_present("setup") {
            journal.setup().expect("Setup error");
        } else {
            // Get all values of the given CLI parameters
            let revision_range = matches.value_of("revision_range")
                .expect("Could not parse 'revision range' parameter");
            let tag_skip_pattern = matches.value_of("tag_skip_pattern")
                .expect("Could not parse 'skip pattern' parameter");
            let tags_count = matches.value_of("tags_count").expect("Could not parse 'tags count' parameter");

            let max_tags = tags_count.parse::<u32>().expect("Could not parse tags count to integer");

            // Parse the log
            journal.parse_log(revision_range,
                           tag_skip_pattern,
                           &max_tags,
                           &matches.is_present("all"),
                           &matches.is_present("skip_unreleased"))
                .expect("Log parsing error");
            journal.print_log(matches.is_present("short")).expect("Could not print log.");
        }
    }
}
