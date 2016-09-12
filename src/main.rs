#[macro_use]
extern crate clap;
extern crate gitjournal;

use clap::App;
use gitjournal::GitJournal;

fn main() {
    // Load the CLI parameters from the yaml file
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();
    let path = matches.value_of("path").expect("Could not parse 'path' parameter");

    if matches.is_present("verify") {
        // Verify a commit message and panic! if verification failed.
        match GitJournal::verify(matches.value_of("verify").expect("Could not parse 'verify' value.")) {
            Ok(()) => println!("[ OKAY ] Commit message valid."),
            Err(error) => panic!("Commit message invalid. ({})", error),
        }
    } else if matches.is_present("prepare") {
        // Prepare a commit message before editing by the user
        match GitJournal::prepare(matches.value_of("prepare").expect("Could not parse 'prepare' value.")) {
            Ok(()) => println!("[ OKAY ] Commit message prepared."),
            Err(error) => panic!("Commit message preparation failed. ({})", error),
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
