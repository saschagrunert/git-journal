extern crate git2;
extern crate chrono;
extern crate regex;

use git2::{ObjectType, Oid, Repository};
use chrono::{UTC, TimeZone, Datelike};
use regex::Regex;
use std::fmt;

#[derive(Debug)]
pub enum GitJournalError {
    GitError(git2::Error),
    RegexError(regex::Error),
    EntryNotCategorized(String),
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

impl From<regex::Error> for GitJournalError {
    fn from(err: regex::Error) -> GitJournalError {
        GitJournalError::RegexError(err)
    }
}

impl fmt::Display for GitJournalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GitJournalError::GitError(ref err) => write!(f, "Git error: {}", err),
            GitJournalError::RegexError(ref err) => write!(f, "Regex error: {}", err),
            GitJournalError::EntryNotCategorized(ref sum) => write!(f, "No valid category: {}", sum),
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
    pub fn parse_log(&self, revision_range: &str, tag_skip_pattern: &str, all: bool) -> Result<(), GitJournalError> {
        let mut revwalk = try!(self.repo.revwalk());
        let mut stop_at_first_tag = false;
        revwalk.set_sorting(git2::SORT_TIME);

        // Fill the revwalk with the selected revisions.
        let revspec = try!(self.repo.revparse(&revision_range));
        if revspec.mode().contains(git2::REVPARSE_SINGLE) {
            // A single commit was given
            let from = try!(revspec.from().ok_or(git2::Error::from_str("Could not set revision range start")));
            try!(revwalk.push(from.id()));
            stop_at_first_tag = if !all { true } else { false };
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
                if stop_at_first_tag && index > 0 {
                    break 'revloop;
                }

                // Format the tag and set as current
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

        for (tag, commits) in result {
            println!("\n{}:", tag);
            for commit in commits {
                println!("{}", commit);
            }
        }
        Ok(())
    }

    /// Parses a single commit message and returns a changelog ready form
    fn parse_commit_message(&self, message: &str) -> Result<String, GitJournalError> {
        let summary = try!(message.lines().nth(0).ok_or(GitJournalError::CommitMessageLengthError)).trim();
        let body = message.lines().skip(1).collect::<Vec<&str>>();
        let list_char = '-';

        // Skip this entry if not in any category
        let categories = ["Critical", "Added", "Changed", "Fixed", "Improved", "Removed"];
        if categories.iter().filter(|&cat| summary.contains(cat)).count() == 0 {
            return Err(GitJournalError::EntryNotCategorized(summary.to_owned()));
        }

        // Trim commits by pattern and highlight category
        let trim_re = try!(Regex::new(r"^[A-Z]+-[0-9]+\s(?P<c>\S+)\s(?P<m>.*)"));
        let summary_message = trim_re.replace(summary, "[$c] $m");

        // Remove unnecessary information from the body
        let mut body_message = body.iter()
            .filter(|&s| !s.is_empty() && s.chars().nth(0).unwrap() == list_char)
            .map(|s| format!("    {}", s))
            .collect::<Vec<String>>()
            .join("\n");
        if !body_message.is_empty() {
            body_message.insert(0, '\n');
        }

        Ok(format!("{} {}{}", list_char, summary_message, body_message))
    }
}
