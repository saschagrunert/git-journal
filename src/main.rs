#[macro_use]
extern crate clap;
extern crate git_journal;

use clap::App;
use git_journal::GitJournal;

fn main() {
    // Load the CLI parameters from the yaml file
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();


    if matches.is_present("setup") {
        println!("Setup");
    } else {
        // Get all values of the given CLI parameters
        let path = matches.value_of("path").expect("Could not parse 'path' parameter");
        let revision_range = matches.value_of("revision_range").expect("Could not parse 'revision range' parameter");
        let tag_skip_pattern = matches.value_of("tag_skip_pattern").expect("Could not parse 'skip pattern' parameter");
        let tags_count = matches.value_of("tags_count").expect("Could not parse 'tags count' parameter");

        let max_tags = tags_count.parse::<u32>().expect("Could not parse tags count to integer");

        // Create the git journal
        match GitJournal::new(path) {
            Ok(mut journal) => {
                journal.parse_log(revision_range,
                               tag_skip_pattern,
                               &max_tags,
                               &matches.is_present("all"),
                               &matches.is_present("skip_unreleased"))
                    .expect("Log parsing error");
                journal.print_log(true);
                println!("\n---");
                journal.print_log(false);
            }
            Err(e) => println!("{}", e),
        }
    }
}
