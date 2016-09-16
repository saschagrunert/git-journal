use nom::{IResult, alpha, digit, space, rest};
use regex::{Regex, RegexBuilder};
use chrono::{Date, UTC, Datelike};
use term;
use toml;

use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::io;
use std::iter;
use std::str;

use config::Config;

#[derive(Debug)]
pub enum Error {
    SummaryParsing(String),
    ParagraphParsing(String),
    FooterParsing(String),
    CommitMessageLength,
    Terminal,
    Io(io::Error),
    Toml(toml::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::SummaryParsing(ref line) => write!(f, "Summary parsing: {}", line),
            Error::ParagraphParsing(ref line) => write!(f, "Paragraph parsing: {}", line),
            Error::FooterParsing(ref line) => write!(f, "Footer parsing: {}", line),
            Error::CommitMessageLength => write!(f, "Commit message length too small."),
            Error::Terminal => write!(f, "Could not print to terminal."),
            Error::Io(ref e) => write!(f, "Io: {}", e),
            Error::Toml(ref e) => write!(f, "Toml: {}", e),
        }
    }
}

impl From<term::Error> for Error {
    #[allow(unused_variables)]
    fn from(err: term::Error) -> Error {
        Error::Terminal
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<toml::Error> for Error {
    fn from(err: toml::Error) -> Error {
        Error::Toml(err)
    }
}

#[derive(PartialEq)]
pub enum Printed {
    Nothing,
    Something,
}

pub trait PrintWithTag {
    fn print(&self, t: &mut Box<term::StdoutTerminal>, config: &Config, tag: Option<&str>) -> Result<Printed, Error>;
    fn contains_tag(&self, tag: Option<&str>) -> bool;
}

pub trait Print {
    fn print(&self, t: &mut Box<term::StdoutTerminal>, config: &Config) -> Result<Printed, Error>;
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ParsedTag {
    pub name: String,
    pub date: Date<UTC>,
}

impl Print for ParsedTag {
    fn print(&self, t: &mut Box<term::StdoutTerminal>, config: &Config) -> Result<Printed, Error> {
        if config.colored_output {
            try!(t.fg(term::color::GREEN));
        }
        try!(write!(t, "\n{} ", self.name));
        if config.colored_output {
            try!(t.fg(term::color::YELLOW));
        }
        try!(writeln!(t,
                      "({}-{:02}-{:02}):",
                      self.date.year(),
                      self.date.month(),
                      self.date.day()));
        if config.colored_output {
            try!(t.reset());
        }
        Ok(Printed::Something)
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ParsedCommit {
    pub summary: SummaryElement,
    pub body: Vec<BodyElement>,
    pub footer: Vec<FooterElement>,
}

impl PrintWithTag for ParsedCommit {
    fn print(&self, t: &mut Box<term::StdoutTerminal>, config: &Config, tag: Option<&str>) -> Result<Printed, Error> {
        // If summary is already filtered out then do not print at all
        if try!(self.summary.print(t, config, tag)) == Printed::Nothing {
            return Ok(Printed::Nothing);
        }
        for item in &self.body {
            try!(item.print(t, config, tag));
        }
        Ok(Printed::Something)
    }

    fn contains_tag(&self, tag: Option<&str>) -> bool {
        self.summary.contains_tag(tag) || self.body.iter().filter(|x| x.contains_tag(tag)).count() > 0
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct SummaryElement {
    pub prefix: String,
    pub category: String,
    pub text: String,
    pub tags: Vec<String>,
}

impl PrintWithTag for SummaryElement {
    fn print(&self, t: &mut Box<term::StdoutTerminal>, config: &Config, tag: Option<&str>) -> Result<Printed, Error> {
        // Filter out excluded tags
        if self.tags.iter().filter(|x| config.excluded_tags.contains(x)).count() > 0usize {
            return Ok(Printed::Nothing);
        }

        if self.contains_tag(tag) {
            try!(write!(t, "- "));
            if config.show_prefix && !self.prefix.is_empty() {
                try!(write!(t, "{} ", self.prefix));
            }
            if config.colored_output {
                try!(t.fg(term::color::BRIGHT_BLUE));
            }
            print!("[{}]", self.category);
            if config.colored_output {
                try!(t.fg(term::color::WHITE));
            }
            try!(writeln!(t, "{}", self.text));
            try!(t.reset());
        }
        Ok(Printed::Something)
    }

    fn contains_tag(&self, tag: Option<&str>) -> bool {
        if let Some(tag) = tag {
            !(!self.tags.contains(&tag.to_owned()))
        } else {
            true
        }
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum BodyElement {
    List(Vec<ListElement>),
    Paragraph(ParagraphElement),
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ListElement {
    pub category: String,
    pub text: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ParagraphElement {
    pub text: String,
    pub tags: Vec<String>,
}


impl PrintWithTag for BodyElement {
    fn print(&self, t: &mut Box<term::StdoutTerminal>, config: &Config, tag: Option<&str>) -> Result<Printed, Error> {
        match *self {
            BodyElement::List(ref vec) => {
                for list_item in vec {
                    try!(list_item.print(t, config, tag));
                }
            }
            BodyElement::Paragraph(ref paragraph) => {
                try!(paragraph.print(t, config, tag));
            }
        }
        Ok(Printed::Something)
    }

    fn contains_tag(&self, tag: Option<&str>) -> bool {
        match *self {
            BodyElement::List(ref vec) => vec.iter().filter(|x| x.contains_tag(tag)).count() > 0,
            BodyElement::Paragraph(ref paragraph) => paragraph.contains_tag(tag),
        }
    }
}

impl PrintWithTag for ListElement {
    fn print(&self, t: &mut Box<term::StdoutTerminal>, config: &Config, tag: Option<&str>) -> Result<Printed, Error> {
        // Check if list item contains excluded tag
        if self.tags.iter().filter(|x| config.excluded_tags.contains(x)).count() > 0usize {
            return Ok(Printed::Nothing);
        }

        if self.contains_tag(tag) {
            try!(write!(t, "{}- ", {
                if tag.is_none() {
                    iter::repeat(' ').take(4).collect::<String>()
                } else {
                    String::new()
                }
            }));
            if !self.category.is_empty() {
                if config.colored_output {
                    try!(t.fg(term::color::BRIGHT_BLUE));
                }
                try!(write!(t, "[{}]", self.category));
                if config.colored_output {
                    try!(t.fg(term::color::WHITE));
                }
            }
            try!(writeln!(t, "{}", self.text));
            try!(t.reset());
        }

        Ok(Printed::Something)
    }

    fn contains_tag(&self, tag: Option<&str>) -> bool {
        if let Some(tag) = tag {
            !(!self.tags.contains(&tag.to_owned()))
        } else {
            true
        }
    }
}

impl PrintWithTag for ParagraphElement {
    fn print(&self, t: &mut Box<term::StdoutTerminal>, config: &Config, tag: Option<&str>) -> Result<Printed, Error> {
        // Check if paragraph contains excluded tag
        if self.tags.iter().filter(|x| config.excluded_tags.contains(x)).count() > 0usize {
            return Ok(Printed::Nothing);
        }

        if self.contains_tag(tag) {
            for line in self.text
                .lines()
                .map(|x| {
                    if tag.is_none() {
                        iter::repeat(' ').take(4).collect::<String>()
                    } else {
                        String::new()
                    }
                } + x)
                .collect::<Vec<String>>() {
                try!(writeln!(t, "{}", line));
            }
        }
        Ok(Printed::Something)
    }

    fn contains_tag(&self, tag: Option<&str>) -> bool {
        if let Some(tag) = tag {
            !(!self.tags.contains(&tag.to_owned()))
        } else {
            true
        }
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct FooterElement {
    pub key: String,
    pub value: String,
}

lazy_static! {
    static ref RE_TAGS: Regex = Regex::new(r" :(.*?):").unwrap();
    static ref RE_FOOTER: Regex = RegexBuilder::new(r"^([\w-]+):\s(.*)$").multi_line(true).compile().unwrap();
    static ref RE_LIST: Regex = RegexBuilder::new(r"^-\s.*$(\n^\s+-\s.*)*").multi_line(true).compile().unwrap();
    static ref RE_PARAGRAPH: Regex = RegexBuilder::new(r"^\w").multi_line(true).compile().unwrap();
    static ref RE_COMMENT: Regex = RegexBuilder::new(r"^#.*").multi_line(true).compile().unwrap();
}

pub struct Parser;
impl Parser {
    /// Parses a single commit message and returns a changelog ready form
    pub fn parse_commit_message(message: &str) -> Result<ParsedCommit, Error> {

        /// Parses for tags and returns them with the resulting string
        fn parse_and_consume_tags(input: &[u8]) -> (Vec<String>, String) {
            let string = str::from_utf8(input).unwrap_or("");
            let mut tags = vec![];
            for cap in RE_TAGS.captures_iter(string) {
                tags.extend(cap.at(1)
                    .unwrap_or("")
                    .split(',')
                    .filter_map(|x| {
                        // Ignore tags containing dots.
                        if !x.contains('.') {
                            Some(x.trim().to_owned())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<String>>());
            }
            (tags, RE_TAGS.replace_all(string, ""))
        }

        named!(parse_category<&str>,
            chain!(
                tag!("[")? ~
                p_category: map_res!(
                    alt!(
                        tag!("Added") |
                        tag!("Changed") |
                        tag!("Fixed") |
                        tag!("Improved") |
                        tag!("Removed")
                    ),
                    str::from_utf8
                ) ~
                tag!("]")? ,
                || p_category
            )
        );

        named!(parse_list_item<ListElement>,
            chain!(
                many0!(space) ~
                tag!("-") ~
                space? ~
                p_category: parse_category? ~
                p_tags_rest: map!(rest, parse_and_consume_tags),
                || ListElement {
                    category: p_category.unwrap_or("").to_owned(),
                    tags: p_tags_rest.0.clone(),
                    text: p_tags_rest.1.clone(),
                }
            )
        );

        // Every block is split by two newlines
        let mut commit_parts = message.split("\n\n");

        // Parse the summary line
        let summary_line = try!(commit_parts.nth(0).ok_or(Error::CommitMessageLength)).trim();
        named!(parse_summary<SummaryElement>,
            chain!(
                p_prefix: separated_pair!(alpha, char!('-'), digit)? ~
                space? ~
                p_category: parse_category ~
                p_tags_rest: map!(rest, parse_and_consume_tags),
            || SummaryElement {
                prefix: p_prefix.map_or("".to_owned(), |p| {
                    format!("{}-{}", str::from_utf8(p.0).unwrap_or(""), str::from_utf8(p.1).unwrap_or(""))
                }),
                category: p_category.to_owned(),
                tags: p_tags_rest.0.clone(),
                text: p_tags_rest.1.clone(),
            })
        );
        let parsed_summary = match parse_summary(summary_line.as_bytes()) {
            IResult::Done(_, parsed) => parsed,
            _ => return Err(Error::SummaryParsing(summary_line.to_owned())),
        };

        // Parse the body and the footer, the summary is already consumed
        let mut parsed_footer = vec![];
        let mut parsed_body = vec![];
        for part in commit_parts {
            // Do nothing on comments
            if RE_COMMENT.is_match(part) {
                continue;
            } else if RE_FOOTER.is_match(part) {
                // Parse footer
                for cap in RE_FOOTER.captures_iter(part) {
                    parsed_footer.push(FooterElement {
                        key: try!(cap.at(1).ok_or(Error::FooterParsing(part.to_owned()))).to_owned(),
                        value: try!(cap.at(2).ok_or(Error::FooterParsing(part.to_owned()))).to_owned(),
                    });
                }
            } else if RE_LIST.is_match(part) {
                // Parse list items
                let mut list = vec![];
                for list_item in part.lines() {
                    if let IResult::Done(_, result) = parse_list_item(list_item.as_bytes()) {
                        list.push(result);
                    };
                }
                parsed_body.push(BodyElement::List(list));
            } else {
                // Assume paragraph, test for a valid paragraph
                if !RE_PARAGRAPH.is_match(part) {
                    return Err(Error::ParagraphParsing(part.to_owned()));
                }
                let (parsed_tags, parsed_text) = parse_and_consume_tags(part.as_bytes());
                parsed_body.push(BodyElement::Paragraph(ParagraphElement {
                    text: parsed_text.trim().to_owned(),
                    tags: parsed_tags,
                }));
            }
        }

        Ok(ParsedCommit {
            summary: parsed_summary,
            body: parsed_body,
            footer: parsed_footer,
        })
    }

    /// Parses a toml output template and filters it through parsed commits
    pub fn parse_template_and_print(template: &str,
                                    parsed_commits: &[(ParsedTag, Vec<ParsedCommit>)],
                                    config: &Config)
                                    -> Result<(), Error> {
        let mut file = try!(File::open(template));
        let mut toml_string = String::new();
        try!(file.read_to_string(&mut toml_string));
        let toml = try!(toml::Parser::new(&toml_string).parse().ok_or(toml::Error::Custom("Could not parse toml template.".to_owned())));
        let mut t = try!(term::stdout().ok_or(Error::Terminal));
        for &(ref tag, ref commits) in parsed_commits {
            try!(tag.print(&mut t, config));
            try!(Parser::print_commits_in_table(&mut t, &toml, &mut 1, commits, config));
        }
        try!(t.reset());
        Ok(())
    }

    fn print_commits_in_table(t: &mut Box<term::StdoutTerminal>,
                              table: &toml::Table,
                              level: &mut usize,
                              commits: &[ParsedCommit],
                              config: &Config)
                              -> Result<(), Error> {
        for (tag, value) in table {
            if let toml::Value::Table(ref table) = *value {
                let header_lvl: String = iter::repeat('#').take(*level).collect();
                let name = match table.get("name") {
                    Some(name_value) => name_value.as_str().unwrap_or(tag),
                    None => tag,
                };

                // Do not print at all if none of the commits matches to the section
                if commits.iter().filter(|c| c.contains_tag(Some(tag))).count() > 0 {
                    try!(t.fg(term::color::RED));
                    try!(writeln!(t, "{} {}", header_lvl, name));
                    try!(t.reset());

                    // Print commits for this tag
                    for commit in commits {
                        try!(commit.print(t, config, Some(tag)));
                    }

                    try!(writeln!(t, ""));
                }

                *level += 1;
                try!(Parser::print_commits_in_table(t, table, level, commits, config));
                *level -= 1;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config;

    fn parse_and_print_error(message: &str) {
        let ret = Parser::parse_commit_message(message);
        assert!(ret.is_err());
        if let Err(e) = ret {
            println!("{}", e);
        }
    }

    #[test]
    fn parse_commit_ok_1() {
        let commit = Parser::parse_commit_message("JIRA-1234 [Changed] my commit summary\n\nSome paragraph\n\n# A \
                                                   comment\n# Another comment");
        assert!(commit.is_ok());
        if let Ok(commit) = commit {
            assert_eq!(commit.body.len(), 1);
            assert_eq!(commit.body[0],
                       BodyElement::Paragraph(ParagraphElement {text: "Some paragraph".to_owned(), tags: vec![]}));
            assert_eq!(commit.footer.len(), 0);
            assert_eq!(commit.summary.prefix, "JIRA-1234");
            assert_eq!(commit.summary.category, "Changed");
            assert_eq!(commit.summary.text, " my commit summary");
            assert_eq!(commit.summary.tags.len(), 0);
            assert!(commit.print(&config::Config::new(), None).is_ok());
        }
    }

    #[test]
    fn parse_commit_ok_2() {
        let commit = Parser::parse_commit_message("Changed my commit summary\n\n- List item 1\n- List item 2\n- List \
                                                   item 3");
        assert!(commit.is_ok());
        if let Ok(commit) = commit {
            assert_eq!(commit.body.len(), 1);
            assert_eq!(commit.footer.len(), 0);
            assert_eq!(commit.summary.prefix, "");
            assert_eq!(commit.summary.category, "Changed");
            assert_eq!(commit.summary.text, " my commit summary");
            assert_eq!(commit.summary.tags.len(), 0);
            assert!(commit.print(&config::Config::new(), None).is_ok());
        }
    }

    #[test]
    fn parse_commit_ok_3() {
        let commit = Parser::parse_commit_message("PREFIX-666 Fixed some ____ commit :tag1: :tag2: :tag3:\n\nSome: \
                                                   Footer\nAnother: Footer\nMy-ID: IDVALUE");
        assert!(commit.is_ok());
        if let Ok(commit) = commit {
            assert_eq!(commit.body.len(), 0);
            assert_eq!(commit.footer.len(), 3);
            assert_eq!(commit.summary.prefix, "PREFIX-666");
            assert_eq!(commit.summary.category, "Fixed");
            assert_eq!(commit.summary.text, " some ____ commit");
            assert_eq!(commit.summary.tags, vec!["tag1".to_owned(), "tag2".to_owned(), "tag3".to_owned()]);
            assert!(commit.print(&config::Config::new(), None).is_ok());
        }
    }


    #[test]
    fn parse_commit_ok_4() {
        let commit = Parser::parse_commit_message("Added my :1234: commit 💖 summary :some tag:\n\nParagraph\n\n- \
                                                   List Item\n\nReviewed-by: Me");
        assert!(commit.is_ok());
        if let Ok(commit) = commit {
            assert_eq!(commit.body.len(), 2);
            assert_eq!(commit.footer.len(), 1);
            assert_eq!(commit.summary.prefix, "");
            assert_eq!(commit.summary.category, "Added");
            assert_eq!(commit.summary.text, " my commit 💖 summary");
            assert_eq!(commit.summary.tags, vec!["1234".to_owned(), "some tag".to_owned()]);
            assert!(commit.print(&config::Config::new(), None).is_ok());
        }
    }

    #[test]
    fn parse_commit_failure_1() {
        parse_and_print_error("None");
    }

    #[test]
    fn parse_commit_failure_2() {
        parse_and_print_error("PREFIX+1234 Changed some stuff");
    }

    #[test]
    fn parse_commit_failure_3() {
        parse_and_print_error("Fix some stuff");
    }
}
