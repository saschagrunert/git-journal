extern crate git2;
extern crate chrono;

#[macro_use]
extern crate nom;

use git2::{ObjectType, Oid, Repository};
use chrono::{UTC, TimeZone, Datelike};
use nom::{IResult, alpha, alphanumeric, digit, newline, rest, space};

use std::fmt;
use std::str;

#[derive(Debug)]
pub enum GitJournalError {
    GitError(git2::Error),
    ParserError,
    CommitMessageLengthError,
}

pub struct GitJournal {
    repo: Repository,
    tags: Vec<(Oid, String)>,
}

impl From<git2::Error> for GitJournalError {
    fn from(err: git2::Error) -> GitJournalError {
        GitJournalError::GitError(err)
    }
}

impl fmt::Display for GitJournalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GitJournalError::GitError(ref err) => write!(f, "Git error: {}", err),
            GitJournalError::ParserError => write!(f, "Parser error."),
            GitJournalError::CommitMessageLengthError => write!(f, "Commit message length too small."),
        }
    }
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
        })
    }

    /// Parses a revision range for a `GitJournal`.
    pub fn parse_log(&self,
                     revision_range: &str,
                     tag_skip_pattern: &str,
                     max_tags_count: &u32,
                     all: &bool)
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
        let mut result: Vec<(String, Vec<String>)> = vec![];
        let mut current_entries: Vec<String> = vec![];
        let today = UTC::today();
        let mut parsed_tags: u32 = 1;
        let mut current_tag = format!("Unreleased ({}-{}-{})",
                                      today.year(),
                                      today.month(),
                                      today.day());
        'revloop: for (index, id) in revwalk.enumerate() {
            let oid = try!(id);
            let commit = try!(self.repo.find_commit(oid));
            for tag in self.tags
                .iter()
                .filter(|tag| tag.0.as_bytes() == oid.as_bytes() && !tag.1.contains(tag_skip_pattern)) {

                // Parsing entries of the last tag done
                if !current_entries.is_empty() {
                    result.push((current_tag.clone(), current_entries.clone()));
                    current_entries.clear();
                }

                // If a single revision is given stop at the first seen tag
                if !all && index > 0 && parsed_tags > *max_tags_count {
                    break 'revloop;
                }

                // Format the tag and set as current
                parsed_tags += 1;
                let date = UTC.timestamp(commit.time().seconds(), 0).date();
                current_tag = format!("{} ({}-{}-{})",
                                      tag.1,
                                      date.year(),
                                      date.month(),
                                      date.day());
            }
            // Add the commit message to the current entries of the tag
            let message = try!(commit.message().ok_or(git2::Error::from_str("Could not parse commit message")));
            match self.parse_commit_message(message) {
                Ok(parsed_message) => current_entries.push(parsed_message),
                Err(e) => println!("Skip commit: {}", e),
            }
        }
        // Add the last processed items as well
        if !current_entries.is_empty() {
            result.push((current_tag, current_entries));
        }

        // Print for testing purposes
        for (tag, mut commits) in result {
            println!("\n{}:", tag);
            commits.sort();
            for commit in commits {
                println!("{}", commit);
            }
        }
        Ok(())
    }

    /// Parses a single commit message and returns a changelog ready form
    fn parse_commit_message(&self, message: &str) -> Result<String, GitJournalError> {
        named!(commit_parser<(&str, &str, Vec<&str>)>,
            chain!(
                separated_pair!(alpha, char!('-'), digit)? ~
                space? ~
                cat: map_res!(
                    alt!(
                        tag!("Added") |
                        tag!("Changed") |
                        tag!("Fixed") |
                        tag!("Improved") |
                        tag!("Removed") |
                        tag!("Changed")),
                str::from_utf8) ~
                sum: map_res!(
                    take_until!("\n"),
                    str::from_utf8
                ) ~
                many1!(newline) ~
                items: many0!(
                    map_res!(
                        chain!(
                            tag!("-") ~
                            space? ~
                            r: take_until!("\n") ~
                            many1!(newline),
                            || r
                        ),
                        str::from_utf8
                    )
                ) ~
                rest,
            || (cat, sum, items))
        );
        match commit_parser(message.as_bytes()) {
             IResult::Done(_, parsed) => {
                 return Ok(format!("- [{}]{}{}", parsed.0, parsed.1, parsed.2.join("\n    - ")));
             },
             _ => {
                 println!("Could not parse:\n{}\n", message.lines().nth(0).unwrap());
                 return Err(GitJournalError::ParserError);
             },
        };
    }
}
