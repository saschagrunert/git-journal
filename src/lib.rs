extern crate git2;
extern crate chrono;
extern crate regex;

#[macro_use]
extern crate nom;

#[macro_use]
extern crate lazy_static;

use git2::{ObjectType, Oid, Repository};
use chrono::{UTC, TimeZone};
use std::fmt;
use parser::{ParsedCommit, ParsedTag};

mod parser;

#[derive(Debug)]
pub enum GitJournalError {
    Git(git2::Error),
    Parser(String),
}

impl From<git2::Error> for GitJournalError {
    fn from(err: git2::Error) -> GitJournalError {
        GitJournalError::Git(err)
    }
}

impl fmt::Display for GitJournalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GitJournalError::Git(ref err) => write!(f, "Git error: {}", err),
            GitJournalError::Parser(ref err) => write!(f, "Parser error: {}", err),
        }
    }
}

pub struct GitJournal {
    repo: Repository,
    tags: Vec<(Oid, String)>,
    parse_result: Vec<(ParsedTag, Vec<ParsedCommit>)>,
}

impl GitJournal {
    /// Constructs a new `GitJournal`.
    pub fn new(path: &str) -> Result<GitJournal, GitJournalError> {
        // Open the repository
        let new_repo = try!(Repository::open(path));

        // Get all available tags in some vector of tuples
        let mut new_tags = vec![];
        for name in try!(new_repo.tag_names(None)).iter() {
            let name = try!(name.ok_or(git2::Error::from_str("Could not receive tag name")));
            let obj = try!(new_repo.revparse_single(name));
            if let Ok(tag) = obj.into_tag() {
                let tag_name = try!(tag.name().ok_or(git2::Error::from_str("Could not parse tag name"))).to_owned();
                new_tags.push((tag.target_id(), tag_name));
            }
        }

        // Return the git journal object
        Ok(GitJournal {
            repo: new_repo,
            tags: new_tags,
            parse_result: vec![],
        })
    }

    /// Parses a revision range for a `GitJournal`.
    pub fn parse_log(&mut self,
                     revision_range: &str,
                     tag_skip_pattern: &str,
                     max_tags_count: &u32,
                     all: &bool,
                     skip_unreleased: &bool)
                     -> Result<(), GitJournalError> {

        let mut revwalk = try!(self.repo.revwalk());
        revwalk.set_sorting(git2::SORT_TIME);

        // Fill the revwalk with the selected revisions.
        let revspec = try!(self.repo.revparse(&revision_range));
        if revspec.mode().contains(git2::REVPARSE_SINGLE) {
            // A single commit was given
            let from = try!(revspec.from().ok_or(git2::Error::from_str("Could not set revision range start")));
            try!(revwalk.push(from.id()));
        } else {
            // A specific commit range was given
            let from = try!(revspec.from().ok_or(git2::Error::from_str("Could not set revision range start")));
            let to = try!(revspec.to().ok_or(git2::Error::from_str("Could not set revision range end")));
            try!(revwalk.push(to.id()));
            if revspec.mode().contains(git2::REVPARSE_MERGE_BASE) {
                let base = try!(self.repo.merge_base(from.id(), to.id()));
                let o = try!(self.repo.find_object(base, Some(ObjectType::Commit)));
                try!(revwalk.push(o.id()));
            }
            try!(revwalk.hide(from.id()));
        }

        // Iterate over the git objects and collect them in a vector of tuples
        let mut current_entries: Vec<ParsedCommit> = vec![];
        let mut parsed_tags: u32 = 1;
        let unreleased_str = "Unreleased";
        let mut current_tag = ParsedTag {
            name: unreleased_str.to_owned(),
            date: UTC::today(),
        };
        let parser = parser::Parser;
        'revloop: for (index, id) in revwalk.enumerate() {
            let oid = try!(id);
            let commit = try!(self.repo.find_commit(oid));
            for tag in self.tags
                .iter()
                .filter(|tag| tag.0.as_bytes() == oid.as_bytes() && !tag.1.contains(tag_skip_pattern)) {

                // Parsing entries of the last tag done
                if !current_entries.is_empty() {
                    self.parse_result.push((current_tag.clone(), current_entries.clone()));
                    current_entries.clear();
                }

                // If a single revision is given stop at the first seen tag
                if !all && index > 0 && parsed_tags > *max_tags_count {
                    break 'revloop;
                }

                // Format the tag and set as current
                parsed_tags += 1;
                let date = UTC.timestamp(commit.time().seconds(), 0).date();
                current_tag = ParsedTag {
                    name: tag.1.clone(),
                    date: date,
                };
            }

            // Do not parse if we want to skip commits which do not belong to any release
            if *skip_unreleased && current_tag.name == unreleased_str {
                continue;
            }

            // Add the commit message to the current entries of the tag
            let message = try!(commit.message().ok_or(git2::Error::from_str("Parsing error:")));

            match parser.parse_commit_message(message) {
                Ok(parsed_message) => current_entries.push(parsed_message),
                Err(e) => println!("Skiping commit: {}", e),
            }
        }
        // Add the last processed items as well
        if !current_entries.is_empty() {
            self.parse_result.push((current_tag, current_entries));
        }

        Ok(())
    }

    pub fn print_log(&self, short: bool) {
        for &(ref tag, ref commits) in &self.parse_result {
            println!("\n{}:", tag);
            let mut c = commits.clone();
            c.sort_by(|a, b| a.summary.category.cmp(&b.summary.category));
            for commit in c {
                if short {
                    println!("{}", commit.summary);
                } else {
                    println!("{}", commit);
                }
            }
        }
    }
}
