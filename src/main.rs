#![feature(plugin)]
#![plugin(clippy)]

#[macro_use]
extern crate clap;
extern crate git2;

use git2::{Error, ObjectType, Repository, Commit, Oid};
use clap::App;
use std::collections::HashMap;

fn main() {
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let path = matches.value_of("path").expect("Could not parse 'path' parameter");
    let revision_range = matches.value_of("revision_range").expect("Could not parse 'revision range' parameter");

    let repo = Repository::open(path).expect("Could not open repository");
    let tags = get_tags(&repo).expect("Could not retrieve tags from repo");
    parse_log(&repo, revision_range, &tags).expect("Could not parse log");
}

type TagHashMap = HashMap<Oid, String>;
fn get_tags(repo: &Repository) -> Result<TagHashMap, Error> {
    let mut tags = HashMap::new();
    for name in try!(repo.tag_names(None)).iter() {
        let name = name.expect("Could not retrieve tag name");
        let obj = try!(repo.revparse_single(name));
        if let Ok(tag) = obj.into_tag() {
            let tag_name = tag.name().expect("Could not parse tag name").to_owned();
            tags.insert(tag.target_id(), tag_name);
        }
    }
    Ok(tags)
}

fn parse_log(repo: &Repository, revision_range: &str, tags: &TagHashMap) -> Result<(), Error> {
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
        let oid = try!(id);
        let commit = try!(repo.find_commit(oid));
        if let Some(tag) = tags.get(&oid) {
            println!("Found tag {} for commit {}", tag, oid);
        }
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
