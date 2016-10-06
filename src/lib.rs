#![doc(html_root_url = "https://saschagrunert.github.io/git-journal/")]
#![deny(missing_docs)]

//! # The Git Commit Message and Changelog Generation Framework
//!
//! This crate contains the library for the
//! [`git-journal`](https://github.com/saschagrunert/git-journal) executable. It handles all the
//! parsing and commit message modification stuff which is provided by the executable.
//!
//! ### Example usage
//!
//! ```
//! use gitjournal::GitJournal;
//! let mut journal = GitJournal::new(".").unwrap();
//! journal.parse_log("HEAD", "rc", &1, &false, &true);
//! journal.print_log(true, None, None).expect("Could not print short log.");
//! ```
//!
//! Simply create a new git-journal struct from a given path (`.` in this example). Then parse the
//! log between a given commit range or a single commit. In this example we want to retrieve
//! everything included in the last git tag, which does not represent a release candidate (contains
//! `"rc"`). After that parsing the log will be printed in the shortest possible format.
//!

extern crate chrono;
extern crate git2;
extern crate rayon;
extern crate regex;
extern crate rustc_serialize;
extern crate term;
extern crate toml;

#[macro_use]
extern crate nom;

#[macro_use]
extern crate lazy_static;

use chrono::{UTC, TimeZone};
use git2::{ObjectType, Oid, Repository};
use rayon::prelude::*;
use toml::Value;

use parser::{Parser, ParsedTag, Tags};
pub use config::Config;

use std::{fmt, fs};
use std::collections::BTreeMap;
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

    /// Errors related to the parsing and printing of the log.
    Parser(parser::Error),

    /// Errors related to the setup process.
    Setup(config::Error),

    /// Errors related to the template generation.
    Template(String),

    /// Errors related to the terminal emulation, which is used for colored output.
    Term(term::Error),
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
            Error::Parser(ref err) => write!(f, "Parser: {}", err),
            Error::Setup(ref err) => write!(f, "Setup: {}", err),
            Error::Template(ref err) => write!(f, "Template: {}", err),
            Error::Term(ref err) => write!(f, "Term: {}", err),
        }
    }
}

/// The main structure of git-journal.
pub struct GitJournal {
    /// The configuration structure
    pub config: Config,
    parser: Parser,
    path: String,
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
            for dir in try!(fs::read_dir(&path_buf)) {
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
        let repo = try!(Repository::open(&path_buf));

        // Get all available tags in some vector of tuples
        let mut new_tags = vec![];
        for name in try!(repo.tag_names(None)).iter() {
            let name = try!(name.ok_or(git2::Error::from_str("Could not receive tag name")));
            let obj = try!(repo.revparse_single(name));
            if let Ok(tag) = obj.into_tag() {
                let tag_name = try!(tag.name().ok_or(git2::Error::from_str("Could not parse tag name"))).to_owned();
                new_tags.push((tag.target_id(), tag_name));
            }
        }

        // Search for config in path and load
        let mut new_config = Config::new();
        if let Err(e) = new_config.load(path) {
            println_warn!("Can't load configuration file, using default one: {}", e);
        }

        // Create a new parser with empty results
        let new_parser = Parser {
            categories: new_config.categories.clone(),
            result: vec![],
        };

        // Return the git journal object
        Ok(GitJournal {
            config: new_config,
            parser: new_parser,
            path: path_buf.to_str().unwrap_or("").to_owned(),
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
    /// # Specifies the available categories for the commit message, allowd regular expressions.
    /// categories = ["Added", "Changed", "Fixed", "Improved", "Removed"]
    ///
    /// # Set to false if the output should not be colored
    /// colored_output = true
    ///
    /// # Specifies the default template. Will be used for tag validation and printing. Can be
    /// removed from the configuration file as well.
    /// default_template = "CHANGELOG.toml"
    ///
    /// # Show or hide the debug messages like `[OKAY] ...` or `[INFO] ...`
    /// enable_debug = true
    ///
    /// # Excluded tags in an array, e.g. "internal"
    /// excluded_commit_tags = []
    ///
    /// Enable or disable the output and accumulation of commit footers.
    /// pub enable_footers: bool,
    ///
    /// # Show or hide the commit message prefix, e.g. JIRA-1234
    /// show_prefix = false
    ///
    /// # Sort the commits during the output by "date" (default) or "name"
    /// sort_by = "date"
    ///
    /// # Commit message template prefix which will be added during commit preparation.
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
        try!(self.install_git_hook("prepare-commit-msg", "git journal p $1 $2\n"));

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
                               the hook by hand after the installation.",
                              hook_path.display());
            }
            hook_file = try!(OpenOptions::new().read(true).append(true).open(&hook_path));
            let mut hook_content = String::new();
            try!(hook_file.read_to_string(&mut hook_content));
            if hook_content.contains(content) {
                if self.config.enable_debug {
                    println_ok!("Hook already installed, nothing changed in existing hook.");
                }
                return Ok(());
            }
        } else {
            hook_file = try!(File::create(&hook_path));
            try!(hook_file.write_all("#!/usr/bin/env sh\n".as_bytes()));
        }
        try!(hook_file.write_all(content.as_bytes()));
        try!(std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755)));

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
    /// journal.prepare("./tests/commit_messages/success_1", None)
    ///        .expect("Commit message preparation error");
    /// ```
    ///
    /// # Errors
    /// When the path is not available or writing the commit message fails.
    ///
    pub fn prepare(&self, path: &str, commit_type: Option<&str>) -> Result<(), Error> {
        // If the message is not valid, assume a new commit and provide the template.
        if let Err(error) = self.verify(path) {
            // But if the message is provided via the cli with `-m`, then abort since
            // the user can not edit this message any more.
            if let Some(commit_type) = commit_type {
                if commit_type == "message" {
                    return Err(error);
                }
            }

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
            let new_content = self.config.template_prefix.clone() + " " + &self.config.categories[0] +
                              " ...\n\n# Add a more detailed description if needed\n\n# - " +
                              &self.config.categories.join("\n# - ") + "\n\n" +
                              &old_msg_vec.join("\n");
            try!(file.write_all(&new_content.as_bytes()));
        }
        Ok(())
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
        try!(self.parser.parse_commit_message(&commit_message));
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

        let repo = try!(Repository::open(&self.path));
        let mut revwalk = try!(repo.revwalk());
        revwalk.set_sorting(git2::SORT_TIME);

        // Fill the revwalk with the selected revisions.
        let revspec = try!(repo.revparse(&revision_range));
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
                let base = try!(repo.merge_base(from.id(), to.id()));
                let o = try!(repo.find_object(base, Some(ObjectType::Commit)));
                try!(revwalk.push(o.id()));
            }
            try!(revwalk.hide(from.id()));
        }

        // Iterate over the git objects and collect them in a vector of tuples
        let mut num_parsed_tags: u32 = 1;
        let unreleased_str = "Unreleased";
        let mut current_tag = ParsedTag {
            name: unreleased_str.to_owned(),
            date: UTC::today(),
            commits: vec![],
            message_ids: vec![],
        };
        let mut worker_vec = vec![];
        'revloop: for (index, id) in revwalk.enumerate() {
            let oid = try!(id);
            let commit = try!(repo.find_commit(oid));
            for tag in self.tags
                .iter()
                .filter(|tag| tag.0.as_bytes() == oid.as_bytes() && !tag.1.contains(tag_skip_pattern)) {

                // Parsing entries of the last tag done
                if !current_tag.message_ids.is_empty() {
                    self.parser.result.push(current_tag.clone());
                }

                // If a single revision is given stop at the first seen tag
                if !all && index > 0 && num_parsed_tags > *max_tags_count {
                    break 'revloop;
                }

                // Format the tag and set as current
                num_parsed_tags += 1;
                let date = UTC.timestamp(commit.time().seconds(), 0).date();
                current_tag = ParsedTag {
                    name: tag.1.clone(),
                    date: date,
                    commits: vec![],
                    message_ids: vec![],
                };
            }

            // Do not parse if we want to skip commits which do not belong to any release
            if *skip_unreleased && current_tag.name == unreleased_str {
                continue;
            }

            // Add the commit message to the parser work to be done, the `id` represents the index
            // within the worker vector
            let message = try!(commit.message().ok_or(git2::Error::from_str("Commit message error.")));
            let id = worker_vec.len();

            // The worker_vec contains the commit message and the parsed commit (currently none)
            worker_vec.push((message.to_owned(), None));
            current_tag.message_ids.push(id);
        }

        // Add the last element as well if needed
        if !current_tag.message_ids.is_empty() && !self.parser.result.contains(&current_tag) {
            self.parser.result.push(current_tag);
        }

        // Process with the full CPU power
        worker_vec.par_iter_mut().for_each(|&mut (ref message, ref mut result)| {
            match self.parser.parse_commit_message(message) {
                Ok(parsed_message) => {
                    *result = Some(parsed_message);
                }
                Err(e) => {
                    if self.config.enable_debug {
                        if let Some(mut t) = term::stderr() {
                            // Since this part is not important for production we
                            // skip the error handling here.
                            t.fg(term::color::YELLOW).is_ok();
                            write!(t, "[git-journal] ").is_ok();
                            t.fg(term::color::BRIGHT_BLUE).is_ok();
                            write!(t, "[INFO] ").is_ok();
                            t.reset().is_ok();
                            writeln!(t, "Skipping commit: {}.", e).is_ok();
                        }
                    }
                }
            }
        });

        // Assemble results together via the message_id
        self.parser.result = self.parser
            .result
            .clone()
            .into_iter()
            .filter_map(|mut parsed_tag| {
                for id in &parsed_tag.message_ids {
                    if let Some(parsed_commit) = worker_vec[*id].1.clone() {
                        parsed_tag.commits.push(parsed_commit);
                    }
                }
                if parsed_tag.commits.is_empty() {
                    None
                } else {
                    if self.config.sort_by == "name" {
                        parsed_tag.commits.sort_by(|l, r| l.summary.category.cmp(&r.summary.category));
                    }
                    Some(parsed_tag)
                }
            })
            .collect::<Vec<ParsedTag>>();

        if self.config.enable_debug {
            println_ok!("Parsing done. Processed {} commit messages.",
                        worker_vec.len());
        }

        Ok(())
    }

    /// Generates an output template from the current parsing results.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitjournal::GitJournal;
    ///
    /// let mut journal = GitJournal::new(".").unwrap();
    /// journal.parse_log("HEAD", "rc", &1, &false, &false);
    /// journal.generate_template().expect("Template generation failed.");
    /// ```
    ///
    /// # Errors
    /// If the generation of the template was impossible.
    ///
    pub fn generate_template(&self) -> Result<(), Error> {
        let mut tags = vec![parser::TOML_DEFAULT_KEY.to_owned()];

        // Get all the tags
        for parsed_tag in &self.parser.result {
            parsed_tag.get_tags(&mut tags);
        }

        // Sort and dedpup since get_tags just extends the vector
        tags.sort();
        tags.dedup();

        if tags.is_empty() {
            // This path should not be possible since "default" will always be in.
            return Err(Error::Template("No tags found.".to_owned()));
        }

        if self.config.enable_debug {
            println_ok!("Found tags: '{}'.", tags[1..].join(", "))
        }

        // Create the toml representation
        let mut toml_map = BTreeMap::new();
        let toml_tags = tags.iter()
            .map(|tag| {
                let mut map = BTreeMap::new();
                map.insert(parser::TOML_TAG.to_owned(), Value::String(tag.to_owned()));
                map.insert(parser::TOML_NAME_KEY.to_owned(),
                           Value::String(tag.to_owned()));
                map.insert(parser::TOML_FOOTERS_KEY.to_owned(), Value::Array(vec![]));
                Value::Table(map)
            })
            .collect::<Vec<Value>>();
        toml_map.insert("tags".to_owned(), Value::Array(toml_tags));

        let mut header_footer_map = BTreeMap::new();
        header_footer_map.insert(parser::TOML_ONCE_KEY.to_owned(), Value::Boolean(false));
        header_footer_map.insert(parser::TOML_TEXT_KEY.to_owned(),
                                 Value::String(String::new()));
        toml_map.insert(parser::TOML_HEADER_KEY.to_owned(),
                        Value::Table(header_footer_map.clone()));
        toml_map.insert(parser::TOML_FOOTER_KEY.to_owned(),
                        Value::Table(header_footer_map));

        let toml = Value::Table(toml_map);

        // Write toml to file
        let mut path_buf = PathBuf::from(&self.path);
        path_buf.push("template.toml");
        let toml_string = toml::encode_str(&toml);
        let mut toml_file = try!(File::create(&path_buf));
        try!(toml_file.write_all(toml_string.as_bytes()));

        if self.config.enable_debug {
            println_ok!("Template written to '{}'", path_buf.display());
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
    /// journal.print_log(true, None, None).expect("Could not print short log.");
    /// journal.print_log(false, None, None).expect("Could not print detailed log.");
    /// ```
    ///
    /// # Errors
    /// If some commit message could not be print.
    ///
    pub fn print_log(&self, compact: bool, template: Option<&str>, output: Option<&str>) -> Result<(), Error> {

        // Choose the template
        let mut default_template = PathBuf::from(&self.path);
        let used_template = match self.config.default_template {
            Some(ref default_template_file) => {
                default_template.push(default_template_file);

                match template {
                    None => {
                        if default_template.exists() {
                            if self.config.enable_debug {
                                println_ok!("Using default template '{}'.", default_template.display());
                            }
                            default_template.to_str()
                        } else {
                            if self.config.enable_debug {
                                println_warn!("The default template '{}' does not exist.",
                                              default_template.display());
                            }
                            None
                        }
                    }
                    Some(t) => Some(t),
                }
            }
            None => template,
        };

        // Print the log
        let output_vec = try!(self.parser.print(&self.config, &compact, used_template));

        // Print the log to the file if necessary
        if let Some(output) = output {
            let mut output_file = try!(OpenOptions::new().create(true).append(true).open(output));
            try!(output_file.write_all(&output_vec));
            if self.config.enable_debug {
                println_ok!("Output written to '{}'.", output);
            }
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
        assert_eq!(journal.parser.result.len(), 0);
        assert_eq!(journal.config.show_prefix, false);
        assert_eq!(journal.config.colored_output, true);
        assert_eq!(journal.config.excluded_commit_tags.len(), 0);
        assert!(journal.parse_log("HEAD", "rc", &0, &true, &false).is_ok());
        assert_eq!(journal.parser.result.len(), journal.tags.len() + 1);
        assert_eq!(journal.parser.result[0].commits.len(), 13);
        assert_eq!(journal.parser.result[1].commits.len(), 1);
        assert_eq!(journal.parser.result[2].commits.len(), 2);
        assert!(journal.print_log(false, None, Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(true, None, Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(false, Some("./tests/template.toml"), Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(true, Some("./tests/template.toml"), Some("CHANGELOG.md")).is_ok());
    }

    #[test]
    fn parse_and_print_log_2() {
        let mut journal = GitJournal::new("./tests/test_repo").unwrap();
        assert!(journal.parse_log("HEAD", "rc", &1, &false, &false).is_ok());
        assert_eq!(journal.parser.result.len(), 2);
        assert_eq!(journal.parser.result[0].name, "Unreleased");
        assert_eq!(journal.parser.result[1].name, "v2");
        assert!(journal.print_log(false, None, Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(true, None, Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(false, Some("./tests/template.toml"), Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(true, Some("./tests/template.toml"), Some("CHANGELOG.md")).is_ok());
    }

    #[test]
    fn parse_and_print_log_3() {
        let mut journal = GitJournal::new("./tests/test_repo").unwrap();
        assert!(journal.parse_log("HEAD", "rc", &1, &false, &true).is_ok());
        assert_eq!(journal.parser.result.len(), 1);
        assert_eq!(journal.parser.result[0].name, "v2");
        assert!(journal.print_log(false, None, Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(true, None, Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(false, Some("./tests/template.toml"), Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(true, Some("./tests/template.toml"), Some("CHANGELOG.md")).is_ok());
    }

    #[test]
    fn parse_and_print_log_4() {
        let mut journal = GitJournal::new("./tests/test_repo").unwrap();
        assert!(journal.parse_log("HEAD", "rc", &2, &false, &true).is_ok());
        assert_eq!(journal.parser.result.len(), 2);
        assert_eq!(journal.parser.result[0].name, "v2");
        assert_eq!(journal.parser.result[1].name, "v1");
        assert!(journal.print_log(false, None, Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(true, None, Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(false, Some("./tests/template.toml"), Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(true, Some("./tests/template.toml"), Some("CHANGELOG.md")).is_ok());
    }

    #[test]
    fn parse_and_print_log_5() {
        let mut journal = GitJournal::new("./tests/test_repo").unwrap();
        assert!(journal.parse_log("v1..v2", "rc", &0, &false, &true).is_ok());
        assert_eq!(journal.parser.result.len(), 1);
        assert_eq!(journal.parser.result[0].name, "v2");
        assert!(journal.print_log(false, None, Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(true, None, Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(false, Some("./tests/template.toml"), Some("CHANGELOG.md")).is_ok());
        assert!(journal.print_log(true, Some("./tests/template.toml"), Some("CHANGELOG.md")).is_ok());
    }

    #[test]
    fn prepare_message_success_1() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.prepare("./tests/COMMIT_EDITMSG", None).is_ok());
    }

    #[test]
    fn prepare_message_success_2() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.prepare("./tests/commit_messages/prepare_1", None).is_ok());
    }

    #[test]
    fn prepare_message_success_3() {
        let journal = GitJournal::new("./tests/commit_messages").unwrap();
        assert!(journal.prepare("./tests/commit_messages/prepare_2", None).is_ok());
    }

    #[test]
    fn prepare_message_failure_1() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.prepare("TEST", None).is_err());
        assert!(journal.prepare("TEST", Some("message")).is_err());
    }

    #[test]
    fn prepare_message_failure_2() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.prepare("./tests/commit_messages/prepare_3", Some("message")).is_err());
    }

    #[test]
    fn install_git_hook() {
        let journal = GitJournal::new(".").unwrap();
        assert!(journal.install_git_hook("test", "echo 1\n").is_ok());
        assert!(journal.install_git_hook("test", "echo 1\n").is_ok());
        assert!(journal.install_git_hook("test", "echo 2\n").is_ok());
    }
}
