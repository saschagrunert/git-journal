#![doc(html_root_url = "https://saschagrunert.github.io/git-journal/")]
#![deny(missing_docs)]

//! # The Git Commit Message Framework
//!
//! Target of the project is to provide a Rust based framework to write more sensible commit
//! messages. Single commit messages should contain one logical change of the project which is
//! described in a standardized way. This results in a much cleaner git history and provides
//! contributors more information about the actual change.
//!
//! To gain very clean commit message history it is necessary to use git rebase, squashed and
//! amended commits. git-journal will simplify these development approaches by providing sensible
//! conventions and strong defaults.
//!
//! ### Example usage
//!
//! ```
//! use gitjournal::GitJournal;
//! let mut journal = GitJournal::new(".").unwrap();
//! journal.parse_log("HEAD", "rc", &1, &false, &true);
//! journal.print_log(true, None).expect("Could not print short log.");
//! ```
//!
//! Simply create a new git-journal struct from a given path (`.` in this example). Then parse the
//! log between a given commit range or a single commit. In this example we want to retrieve
//! everything included in the last git tag, which does not represent a release candidate (contains
//! `"rc"`). After that parsing the log will be printed in the shortest possible format.
//!

extern crate chrono;
extern crate git2;
extern crate regex;
extern crate rustc_serialize;
extern crate term;
extern crate toml;

#[macro_use]
extern crate nom;

#[macro_use]
extern crate lazy_static;

use git2::{ObjectType, Oid, Repository};
use chrono::{UTC, TimeZone};

use parser::{Parser, ParsedCommit, ParsedTag};
pub use config::Config;

use std::{fmt, fs};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::io::prelude::*;
use std::os::unix::prelude::PermissionsExt;

#[macro_use]
mod macros;
mod parser;
pub mod config;

/// An enumeration of possible errors that can happen when working with git-journal.
#[derive(Debug)]
pub enum Error {
    /// Erros related to the git repository.
    Git(git2::Error),

    /// Erros related to the system IO, like parsing of the configuration file.
    Io(std::io::Error),

    /// Errors related to the terminal emulation, which is used for colored output.
    Term(term::Error),

    /// Errors related to the setup process.
    Setup(config::Error),

    /// Errors related to the parsing and printing of the log.
    Parser(parser::Error),
}

impl From<git2::Error> for Error {
    fn from(err: git2::Error) -> Error {
        Error::Git(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<term::Error> for Error {
    fn from(err: term::Error) -> Error {
        Error::Term(err)
    }
}

impl From<config::Error> for Error {
    fn from(err: config::Error) -> Error {
        Error::Setup(err)
    }
}

impl From<parser::Error> for Error {
    fn from(err: parser::Error) -> Error {
        Error::Parser(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Git(ref err) => write!(f, "Git: {}", err),
            Error::Io(ref err) => write!(f, "Io: {}", err),
            Error::Term(ref err) => write!(f, "Term: {}", err),
            Error::Setup(ref err) => write!(f, "Setup: {}", err),
            Error::Parser(ref err) => write!(f, "Parser: {}", err),
        }
    }
}

/// The main structure of git-journal.
pub struct GitJournal {
    /// The configuration structure
    pub config: Config,
    parse_result: Vec<(ParsedTag, Vec<ParsedCommit>)>,
    path: String,
    repo: Repository,
    tags: Vec<(Oid, String)>,
}

impl GitJournal {
    /// Constructs a new `GitJournal<Result<GitJournal, Error>>`. Searches upwards if the given
    /// path does not contain the `.git` directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitjournal::GitJournal;
    ///
    /// let journal = GitJournal::new(".").unwrap();
    /// ```
    ///
    /// # Errors
    /// When not providing a path with a valid git repository ('.git' folder or the initial parsing
    /// of the git tags failed.
    ///
    pub fn new(path: &str) -> Result<Self, Error> {
        // Search upwards for the .git directory
        let mut path_buf = if path != "." {
            PathBuf::from(path)
        } else {
            try!(std::env::current_dir())
        };
        'git_search: loop {
            for dir in try!(fs::read_dir(path_buf.clone())) {
                let dir_path = try!(dir).path();
                if dir_path.ends_with(".git") {
                    break 'git_search;
                }
            }
            if !path_buf.pop() {
                break;
            }
        }

        // Open the repository
        let new_repo = try!(Repository::open(path_buf.clone()));

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

        // Search for config in path and load
        let mut new_config = Config::new();
        new_config.load(path).is_ok();

        // Return the git journal object
        Ok(GitJournal {
            config: new_config,
            parse_result: vec![],
            path: path_buf.to_str().unwrap_or("").to_owned(),
            repo: new_repo,
            tags: new_tags,
        })
    }

    /// Does the setup on the target git repository.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitjournal::GitJournal;
    ///
    /// let journal = GitJournal::new(".").unwrap();
    /// journal.setup().expect("Setup error");
    /// ```
    ///
    /// Creates a `.gitjournal` file with the default values inside the given path, which looks
    /// like:
    ///
    /// ```toml
    /// # Set to false if the output should not be colored
    /// colored_output = true
    ///
    /// # The default template for the changelog printing
    /// default_template = "changelog_template.toml"
    ///
    /// # Show or hide the debug messages like `[OKAY] ...` or `[INFO] ...`
    /// enable_debug = true
    ///
    /// # Excluded tags in an array, e.g. "internal"
    /// excluded_tags = []
    ///
    /// # The output file where the changelog should be written to
    /// output_file = "CHANGELOG.md"
    ///
    /// # Show or hide the commit message prefix, e.g. JIRA-1234
    /// show_prefix = false
    ///
    /// # Commit message template prefix which will be added during commit preparation
    /// template_prefix = "JIRA-1234"
    /// ```
    ///
    /// It also creates a symlinks for the commit message validation and preparation hook inside
    /// the given git repository.
    ///
    /// # Errors
    /// - When the writing of the default configuration fails.
    /// - When installation of the commit message (preparation) hook fails.
    ///
    pub fn setup(&self) -> Result<(), Error> {
        // Save the default config
        let output_file = try!(Config::new().save_default_config(&self.path));
        if self.config.enable_debug {
            println_ok!("Defaults written to '{}' file.", output_file);
        }

        // Install commit message hook
        try!(self.install_git_hook("commit-msg", "git journal v $1\n"));

        // Install the prepare commit message hook
        try!(self.install_git_hook("prepare-commit-msg", "git journal p $1\n"));

        Ok(())
    }

    fn install_git_hook(&self, name: &str, content: &str) -> Result<(), Error> {
        let mut hook_path = PathBuf::from(&self.path);
        hook_path.push(".git/hooks");
        hook_path.push(name);
        let mut hook_file: File;
        if hook_path.exists() {
            if self.config.enable_debug {
                println_warn!("There is already a hook available in '{}'. Please verifiy \
                               the hook by hand after the installation.", hook_path.display());
            }
            hook_file = try!(OpenOptions::new().read(true).append(true).open(hook_path.clone()));
            let mut hook_content = String::new();
            try!(hook_file.read_to_string(&mut hook_content));
            if hook_content.contains(content) {
                if self.config.enable_debug {
                    println_ok!("Hook already installed, nothing changed in existing hook.");
                }
                return Ok(());
            }
        } else {
            hook_file = try!(File::create(hook_path.clone()));
            try!(hook_file.write_all("#!/usr/bin/env sh\n".as_bytes()));
        }
        try!(hook_file.write_all(content.as_bytes()));
        try!(std::fs::set_permissions(hook_path.clone(), std::fs::Permissions::from_mode(0o755)));

        if self.config.enable_debug {
            println_ok!("Git hook installed to '{}'.", hook_path.display());
        }
        Ok(())
    }

    /// Prepare a commit message before the user edits it. This includes also a verification of the
    /// commit message, e.g. for amended commits.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitjournal::GitJournal;
    ///
    /// let journal = GitJournal::new(".").unwrap();
    /// journal.prepare("./tests/commit_messages/success_1").expect("Commit message preparation error");
    /// ```
    ///
    /// # Errors
    /// When the path is not available or writing the commit message fails.
    ///
    pub fn prepare(&self, path: &str) -> Result<(), Error> {
        // If the message is not valid, assume a new commit and provide the template
        if let Err(_) = self.verify(path) {
            // Read the file contents to get the actual commit message string
            let mut read_file = try!(File::open(path));
            let mut commit_message = String::new();
            try!(read_file.read_to_string(&mut commit_message));

            // Write the new generated content to the file
            let mut file = try!(OpenOptions::new().write(true).open(path));
            let mut old_msg_vec = commit_message.lines()
                .filter_map(|line| {
                    if !line.is_empty() {
                        if line.starts_with('#') {
                            Some(line.to_owned())
                        } else {
                            Some("# ".to_owned() + line)
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            if !old_msg_vec.is_empty() {
                old_msg_vec.insert(0, "# The provided commit message:".to_owned());
            }
            let new_content = self.get_default_commit_template() + &old_msg_vec.join("\n");
            try!(file.write_all(&new_content.as_bytes()));
        }
        Ok(())
    }

    fn get_default_commit_template(&self) -> String {
        self.config.template_prefix.clone() +
        " Added ...\n\n# Add a more detailed description if needed\n\n# - Added ...\n# - Changed ...\n# - Fixed \
         ...\n# - Improved ...\n# - Removed ...\n\n"
    }

    /// Verify a given commit message against the parsing rules of
    /// [RFC0001](https://github.com/saschagrunert/git-journal/blob/master/rfc/0001-commit-msg.md)
    ///
    /// # Examples
    ///
    /// ```
    /// use gitjournal::GitJournal;
    ///
    /// let journal = GitJournal::new(".").unwrap();
    /// journal.verify("tests/commit_messages/success_1").expect("Commit message verification error");
    /// ```
    ///
    /// # Errors
    /// When the commit message is not valid due to RFC0001 or opening of the given file failed.
    ///
    pub fn verify(&self, path: &str) -> Result<(), Error> {
        let mut file = try!(File::open(path));
        let mut commit_message = String::new();
        try!(file.read_to_string(&mut commit_message));
        try!(Parser::parse_commit_message(&commit_message));
        Ok(())
    }

    /// Parses a revision range for a `GitJournal`.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitjournal::GitJournal;
    ///
    /// let mut journal = GitJournal::new(".").unwrap();
    /// journal.parse_log("HEAD", "rc", &1, &false, &false);
    /// ```
    ///
    /// # Errors
    /// When something during the parsing fails, for example if the revision range is invalid.
    ///
    pub fn parse_log(&mut self,
                     revision_range: &str,
                     tag_skip_pattern: &str,
                     max_tags_count: &u32,
                     all: &bool,
                     skip_unreleased: &bool)
                     -> Result<(), Error> {

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

            match Parser::parse_commit_message(message) {
                Ok(parsed_message) => current_entries.push(parsed_message),
                Err(e) => {
                    if self.config.enable_debug {
                        println_info!("Skipping commit: {}", e);
                    }
                }
            }
        }
        // Add the last processed items as well
        if !current_entries.is_empty() {
            self.parse_result.push((current_tag, current_entries));
        }

        if self.config.enable_debug {
            println_ok!("Parsing done.");
        }
        Ok(())
    }

    /// Prints the resulting log in a short or detailed variant. Will use the template as an output
    /// formatter if provided.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitjournal::GitJournal;
    ///
    /// let mut journal = GitJournal::new(".").unwrap();
    /// journal.parse_log("HEAD", "rc", &1, &false, &false);
    /// journal.print_log(true, None).expect("Could not print short log.");
    /// journal.print_log(false, None).expect("Could not print detailed log.");
    /// ```
    ///
    /// # Errors
    /// If some commit message could not be print.
    ///
    pub fn print_log(&self, compact: bool, template: Option<&str>) -> Result<(), Error> {
        if let Some(template) = template {
            try!(Parser::parse_template_and_print(template, &self.parse_result, &self.config, &compact));
        } else {
            try!(Parser::print(&self.parse_result, &self.config, &compact));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        assert!(GitJournal::new(".").is_ok());
        let res = GitJournal::new("/dev/null");
        assert!(res.is_err());
        if let Err(e) = res {
            println!("{}", e);
        }
    }

    #[test]
    fn setup() {
        let path = ".";
        let journal = GitJournal::new(path);
        assert!(journal.is_ok());
        assert!(journal.unwrap().setup().is_ok());
        assert!(GitJournal::new(path).is_ok());
    }

    #[test]
    fn setup_failed() {
        let journal = GitJournal::new("./tests/test_repo");
        assert!(journal.is_ok());
        let res = journal.unwrap().setup();
        assert!(res.is_err());
        if let Err(e) = res {
            println!("{}", e);
        }
    }

    #[test]
    fn verify_commit_msg_summary_success_1() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.verify("./tests/commit_messages/success_1").is_ok());
    }

    #[test]
    fn verify_commit_msg_summary_success_2() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.verify("./tests/commit_messages/success_2").is_ok());
    }

    #[test]
    fn verify_commit_msg_summary_success_3() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.verify("./tests/commit_messages/success_3").is_ok());
    }

    #[test]
    fn verify_commit_msg_summary_success_4() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.verify("./tests/commit_messages/success_4").is_ok());
    }

    fn verify_failure(path: &str) {
        let journal = GitJournal::new(".").unwrap();
        let res = journal.verify(path);
        assert!(res.is_err());
        if let Err(e) = res {
            println!("{}", e);
        }
    }

    #[test]
    fn verify_commit_msg_summary_failure_1() {
        verify_failure("./tests/commit_messages/failure_1");
    }

    #[test]
    fn verify_commit_msg_summary_failure_2() {
        verify_failure("./tests/commit_messages/failure_2");
    }

    #[test]
    fn verify_commit_msg_summary_failure_3() {
        verify_failure("./tests/commit_messages/failure_3");
    }

    #[test]
    fn verify_commit_msg_paragraph_failure_1() {
        verify_failure("./tests/commit_messages/failure_4");
    }

    #[test]
    fn verify_commit_msg_paragraph_failure_2() {
        verify_failure("./tests/commit_messages/failure_5");
    }

    #[test]
    fn verify_commit_msg_paragraph_failure_3() {
        verify_failure("./tests/commit_messages/failure_6");
    }

    #[test]
    fn parse_and_print_log_1() {
        let mut journal = GitJournal::new("./tests/test_repo").unwrap();
        assert_eq!(journal.tags.len(), 2);
        assert_eq!(journal.parse_result.len(), 0);
        assert_eq!(journal.config.show_prefix, false);
        assert_eq!(journal.config.colored_output, true);
        assert_eq!(journal.config.excluded_tags.len(), 0);
        assert!(journal.parse_log("HEAD", "rc", &0, &true, &false).is_ok());
        assert_eq!(journal.parse_result.len(), journal.tags.len() + 1);
        assert_eq!(journal.parse_result[0].1.len(), 4);
        assert_eq!(journal.parse_result[1].1.len(), 1);
        assert_eq!(journal.parse_result[2].1.len(), 2);
        assert!(journal.print_log(false, None).is_ok());
        assert!(journal.print_log(true, None).is_ok());
        assert!(journal.print_log(false, Some("./tests/template.toml")).is_ok());
        assert!(journal.print_log(true, Some("./tests/template.toml")).is_ok());
    }

    #[test]
    fn parse_and_print_log_2() {
        let mut journal = GitJournal::new("./tests/test_repo").unwrap();
        assert!(journal.parse_log("HEAD", "rc", &1, &false, &false).is_ok());
        assert_eq!(journal.parse_result.len(), 2);
        assert_eq!(journal.parse_result[0].0.name, "Unreleased");
        assert_eq!(journal.parse_result[1].0.name, "v2");
        assert!(journal.print_log(false, None).is_ok());
        assert!(journal.print_log(true, None).is_ok());
        assert!(journal.print_log(false, Some("./tests/template.toml")).is_ok());
        assert!(journal.print_log(true, Some("./tests/template.toml")).is_ok());
    }

    #[test]
    fn parse_and_print_log_3() {
        let mut journal = GitJournal::new("./tests/test_repo").unwrap();
        assert!(journal.parse_log("HEAD", "rc", &1, &false, &true).is_ok());
        assert_eq!(journal.parse_result.len(), 1);
        assert_eq!(journal.parse_result[0].0.name, "v2");
        assert!(journal.print_log(false, None).is_ok());
        assert!(journal.print_log(true, None).is_ok());
        assert!(journal.print_log(false, Some("./tests/template.toml")).is_ok());
        assert!(journal.print_log(true, Some("./tests/template.toml")).is_ok());
    }

    #[test]
    fn parse_and_print_log_4() {
        let mut journal = GitJournal::new("./tests/test_repo").unwrap();
        assert!(journal.parse_log("HEAD", "rc", &2, &false, &true).is_ok());
        assert_eq!(journal.parse_result.len(), 2);
        assert_eq!(journal.parse_result[0].0.name, "v2");
        assert_eq!(journal.parse_result[1].0.name, "v1");
        assert!(journal.print_log(false, None).is_ok());
        assert!(journal.print_log(true, None).is_ok());
        assert!(journal.print_log(false, Some("./tests/template.toml")).is_ok());
        assert!(journal.print_log(true, Some("./tests/template.toml")).is_ok());
    }

    #[test]
    fn parse_and_print_log_5() {
        let mut journal = GitJournal::new("./tests/test_repo").unwrap();
        assert!(journal.parse_log("v1..v2", "rc", &0, &false, &true).is_ok());
        assert_eq!(journal.parse_result.len(), 1);
        assert_eq!(journal.parse_result[0].0.name, "v2");
        assert!(journal.print_log(false, None).is_ok());
        assert!(journal.print_log(true, None).is_ok());
        assert!(journal.print_log(false, Some("./tests/template.toml")).is_ok());
        assert!(journal.print_log(true, Some("./tests/template.toml")).is_ok());
    }

    #[test]
    fn prepare_message_success_1() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.prepare("./tests/COMMIT_EDITMSG").is_ok());
    }

    #[test]
    fn prepare_message_success_2() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.prepare("./tests/commit_messages/prepare_1").is_ok());
    }

    #[test]
    fn prepare_message_success_3() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.prepare("./tests/commit_messages/prepare_2").is_ok());
    }

    #[test]
    fn prepare_message_success_4() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.prepare("./tests/commit_messages/prepare_3").is_ok());
    }

    #[test]
    fn prepare_message_failure_1() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.prepare("TEST").is_err());
    }

    #[test]
    fn install_git_hook() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.install_git_hook("test", "echo 1\n").is_ok());
        assert!(journal.install_git_hook("test", "echo 1\n").is_ok());
        assert!(journal.install_git_hook("test", "echo 2\n").is_ok());
    }
}
