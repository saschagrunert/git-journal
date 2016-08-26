#![feature(plugin)]
#![plugin(clippy)]

#[macro_use]
extern crate clap;
extern crate git2;

use git2::{Error, ObjectType, Repository, Commit};
use clap::App;

fn main() {
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();
    let path = matches.value_of("path").unwrap();
    let revision_range = matches.value_of("revision_range").unwrap();
    match get_log_vector(path, revision_range) {
        Ok(()) => println!("Done."),
        Err(e) => panic!("Can't parse git log: {}", e),
    };
}

fn get_log_vector(path: &str, revision_range: &str) -> Result<(), Error> {
    let repo = try!(Repository::open(path));
    let mut revwalk = try!(repo.revwalk());
    revwalk.set_sorting(git2::SORT_TIME);

    // Fill the revwalk with the selected revisions.
    let revspec = try!(repo.revparse(&revision_range));
    if revspec.mode().contains(git2::REVPARSE_SINGLE) {
        try!(revwalk.push(revspec.from().unwrap().id()));
    } else {
        let from = revspec.from().unwrap().id();
        let to = revspec.to().unwrap().id();
        try!(revwalk.push(to));
        if revspec.mode().contains(git2::REVPARSE_MERGE_BASE) {
            let base = try!(repo.merge_base(from, to));
            let o = try!(repo.find_object(base, Some(ObjectType::Commit)));
            try!(revwalk.push(o.id()));
        }
        try!(revwalk.hide(from));
    }

    // Iterate over the git objects and process them.
    for id in revwalk {
        let commit = try!(repo.find_commit(try!(id)));
        print_commit(commit);
    }
    Ok(())
}

fn print_commit(commit: Commit) {
    println!("Commit: {}", commit.id());

    if commit.parents().len() > 1 {
        print!("Merge:");
        for id in commit.parent_ids() {
            print!(" {:.8}", id);
        }
        println!("");
    }

    let author = commit.author();
    println!("Author: {}\n", author);

    for line in String::from_utf8_lossy(commit.message_bytes()).lines() {
        println!("\t{}", line);
    }
    println!("\n");
}
