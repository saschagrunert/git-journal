use chrono::{Date, UTC, Datelike};
use nom::{IResult, alpha, digit, space, rest};
use regex::{Regex, RegexBuilder};
use term;
use toml;

use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::io;
use std::iter;
use std::str;

use config::Config;

static TOML_DEFAULT_KEY: &'static str = "default";
static TOML_FOOTER_KEY: &'static str = "footers";
static TOML_NAME_KEY: &'static str = "name";
static TOML_TAG: &'static str = "tag";

#[derive(Debug)]
pub enum Error {
    CommitMessageLength,
    FooterParsing(String),
    Io(io::Error),
    ParagraphParsing(String),
    SummaryParsing(String),
    Terminal,
    Toml(toml::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::CommitMessageLength => write!(f, "Commit message length too small."),
            Error::FooterParsing(ref line) => write!(f, "Footer parsing: '{}'", line),
            Error::Io(ref e) => write!(f, "Io: {}", e),
            Error::ParagraphParsing(ref line) => write!(f, "Paragraph parsing: '{}'", line),
            Error::SummaryParsing(ref line) => write!(f, "Summary parsing: '{}'", line),
            Error::Terminal => write!(f, "Could not print to terminal."),
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

pub trait Print {
    fn print<T: Write, F, G, H>(&self,
                                t: &mut T,
                                config: &Config,
                                tag: Option<&str>,
                                c1: &F,
                                c2: &G,
                                c3: &H)
                                -> Result<Printed, Error>
        where F: Fn(&mut T) -> Result<(), Error>,
              G: Fn(&mut T) -> Result<(), Error>,
              H: Fn(&mut T) -> Result<(), Error>;

    fn print_default<T: Write>(&self, t: &mut T, config: &Config, tag: Option<&str>) -> Result<(), Error> {
        try!(self.print(t, config, tag, &|_| Ok(()), &|_| Ok(()), &|_| Ok(())));
        Ok(())
    }

    fn print_default_term(&self,
                          mut t: &mut Box<term::StdoutTerminal>,
                          config: &Config,
                          tag: Option<&str>)
                          -> Result<(), Error> {
        try!(self.print(&mut t,
                        config,
                        tag,
                        &|t| {
                            try!(t.fg(term::color::BRIGHT_BLUE));
                            Ok(())
                        },
                        &|t| {
                            try!(t.fg(term::color::WHITE));
                            Ok(())
                        },
                        &|t| {
                            try!(t.reset());
                            Ok(())
                        }));
        Ok(())
    }

    fn contains_tag(&self, tag: Option<&str>) -> bool;

    fn contains_untagged_elements(&self) -> bool;

    fn matches_default_tag(&self, tag: Option<&str>) -> bool {
        match tag {
            Some(tag) => tag == TOML_DEFAULT_KEY && self.contains_untagged_elements(),
            None => false,
        }
    }

    fn should_be_printed(&self, tag: Option<&str>) -> bool {
        self.contains_tag(tag) || self.matches_default_tag(tag)
    }

    fn print_to_term_and_write_to_vector(&self,
                                         mut term: &mut Box<term::StdoutTerminal>,
                                         mut vec: &mut Vec<u8>,
                                         config: &Config,
                                         tag: Option<&str>)
                                         -> Result<(), Error> {
        try!(self.print_default_term(&mut term, config, tag));
        try!(self.print_default(&mut vec, config, tag));
        Ok(())
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ParsedTag {
    pub name: String,
    pub date: Date<UTC>,
    pub commits: Vec<ParsedCommit>,
}

impl ParsedTag {
    fn print<T: Write, F, G, H>(&self, t: &mut T, config: &Config, c1: &F, c2: &G, c3: &H) -> Result<Printed, Error>
        where F: Fn(&mut T) -> Result<(), Error>,
              G: Fn(&mut T) -> Result<(), Error>,
              H: Fn(&mut T) -> Result<(), Error>
    {
        if config.colored_output {
            try!(c1(t));
        }
        tryw!(t, "\n# {} ", self.name);
        if config.colored_output {
            try!(c2(t));
        }
        tryw!(t,
              "({}-{:02}-{:02}):",
              self.date.year(),
              self.date.month(),
              self.date.day());
        if config.colored_output {
            try!(c3(t));
        }
        Ok(Printed::Something)
    }

    fn print_default<T: Write>(&self, t: &mut T, config: &Config) -> Result<(), Error> {
        try!(self.print(t, config, &|_| Ok(()), &|_| Ok(()), &|_| Ok(())));
        Ok(())
    }

    fn print_default_term(&self, mut t: &mut Box<term::StdoutTerminal>, config: &Config) -> Result<(), Error> {
        try!(self.print(&mut t,
                        config,
                        &|t| {
                            try!(t.fg(term::color::GREEN));
                            Ok(())
                        },
                        &|t| {
                            try!(t.fg(term::color::YELLOW));
                            Ok(())
                        },
                        &|t| {
                            try!(t.reset());
                            Ok(())
                        }));
        Ok(())
    }

    fn print_to_term_and_write_to_vector(&self,
                                         mut term: &mut Box<term::StdoutTerminal>,
                                         mut vec: &mut Vec<u8>,
                                         compact: &bool,
                                         config: &Config,
                                         template: Option<&str>)
                                         -> Result<(), Error> {
        try!(self.print_default_term(&mut term, config));
        try!(self.print_default(&mut vec, config));

        match template {
            Some(template) => {
                let mut file = try!(File::open(template));
                let mut toml_string = String::new();
                try!(file.read_to_string(&mut toml_string));
                let toml = try!(toml::Parser::new(&toml_string)
                    .parse()
                    .ok_or(toml::Error::Custom("Could not parse toml template.".to_owned())));
                try!(self.print_commits_in_table(&mut term, &mut vec, &toml, &mut 1, config, &compact));
            }
            None => {
                for commit in &self.commits {
                    if *compact {
                        try!(commit.summary.print_to_term_and_write_to_vector(&mut term, &mut vec, config, None));
                    } else {
                        try!(commit.print_to_term_and_write_to_vector(&mut term, &mut vec, config, None));
                    }
                }
                trywln!(term, "");
                trywln!(vec, "");
                if !*compact && config.enable_footers {
                    try!(self.print_footers(&mut term, &mut vec, None, &config));
                }
            }
        }

        Ok(())
    }

    fn print_commits_in_table(&self,
                              mut term: &mut Box<term::StdoutTerminal>,
                              mut vec: &mut Vec<u8>,
                              table: &toml::Table,
                              level: &mut usize,
                              config: &Config,
                              compact: &bool)
                              -> Result<(), Error> {
        for value in table {
            if let toml::Value::Array(ref array) = *value.1 {
                for item in array {
                    if let toml::Value::Table(ref table) = *item {
                        *level += 1;
                        try!(self.print_commits_in_table(term, vec, table, level, config, compact));
                        *level -= 1;
                    }
                }
            }
        }

        let header_lvl: String = iter::repeat('#').take(*level).collect();
        let tag = match table.get(TOML_TAG) {
            Some(t) => t.as_str().unwrap_or(""),
            None => return Ok(()),
        };
        let name = match table.get(TOML_NAME_KEY) {
            Some(name_value) => name_value.as_str().unwrap_or(tag),
            None => tag,
        };

        if (*compact &&
            ((self.commits.iter().filter(|c| c.summary.contains_tag(Some(tag))).count() > 0 &&
              !config.excluded_commit_tags.contains(&tag.to_owned())) ||
             (tag == TOML_DEFAULT_KEY &&
              self.commits.iter().filter(|c| c.summary.contains_untagged_elements()).count() > 0))) ||
           (!*compact &&
            ((self.commits.iter().filter(|c| c.contains_tag(Some(tag))).count() > 0 &&
              !config.excluded_commit_tags.contains(&tag.to_owned())) ||
             (tag == TOML_DEFAULT_KEY && self.commits.iter().filter(|c| c.contains_untagged_elements()).count() > 0))) {


            if config.colored_output {
                try!(term.fg(term::color::BRIGHT_RED));
            }
            tryw!(term, "\n{} {}", header_lvl, name);
            tryw!(vec, "\n{} {}", header_lvl, name);

            try!(term.reset());

            // Print commits for this tag
            for commit in &self.commits {
                if *compact {
                    try!(commit.summary
                        .print_to_term_and_write_to_vector(&mut term, &mut vec, config, Some(tag)));
                } else {
                    try!(commit.print_to_term_and_write_to_vector(&mut term, &mut vec, config, Some(tag)));
                }
            }

            trywln!(term, "");
            trywln!(vec, "");
        }

        // Print footers is specified in template
        if let Some(footers) = table.get(TOML_FOOTER_KEY) {
            if let toml::Value::Array(ref array) = *footers {
                try!(self.print_footers(term, vec, Some(array), config));
            }
        }

        Ok(())
    }

    fn print_footers(&self,
                     mut term: &mut Box<term::StdoutTerminal>,
                     mut vec: &mut Vec<u8>,
                     footer_keys: Option<&[toml::Value]>,
                     config: &Config)
                     -> Result<(), Error> {

        let mut footer_tree: BTreeMap<String, Vec<String>> = BTreeMap::new();

        // Collect valid footer keys into one vector
        let valid_footer_keys = match footer_keys {
            Some(keys) => {
                let mut vec = vec![];
                for key in keys {
                    if let toml::Value::String(ref footer_key) = *key {
                        vec.push(footer_key.clone());
                    }
                }
                vec
            }
            None => vec![],
        };

        // Map the parsed results into a BTreeMap
        for footer in self.commits
            .iter()
            .flat_map(|commit| commit.footer.clone())
            .collect::<Vec<FooterElement>>() {
            if valid_footer_keys.is_empty() || valid_footer_keys.contains(&footer.key) {
                footer_tree.entry(footer.key).or_insert_with(|| vec![]).push(footer.value);
            }
        }

        // Sort the values by the containing strings
        for value in footer_tree.values_mut() {
            value.sort();
        }

        // Print the mapped footers
        for (key, values) in &footer_tree {
            if config.colored_output {
                try!(term.fg(term::color::BRIGHT_RED));
            }
            trywln!(term, "\n{}:", key);
            trywln!(vec, "\n{}:", key);
            try!(term.reset());
            let footer_string = values.join(", ");
            let mut char_count = 0;
            let mut footer_lines = String::new();
            for cur_char in footer_string.chars() {
                if char_count > 80 && cur_char == ' ' {
                    footer_lines.push('\n');
                    char_count = 0;
                } else {
                    footer_lines.push(cur_char);
                    char_count += 1;
                }
            }
            trywln!(term, "{}", footer_lines);
            trywln!(vec, "{}", footer_lines);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ParsedCommit {
    pub summary: SummaryElement,
    pub body: Vec<BodyElement>,
    pub footer: Vec<FooterElement>,
}

impl Print for ParsedCommit {
    fn print<T: Write, F, G, H>(&self,
                                t: &mut T,
                                config: &Config,
                                tag: Option<&str>,
                                c1: &F,
                                c2: &G,
                                c3: &H)
                                -> Result<Printed, Error>
        where F: Fn(&mut T) -> Result<(), Error>,
              G: Fn(&mut T) -> Result<(), Error>,
              H: Fn(&mut T) -> Result<(), Error>
    {
        // If summary is already filtered out then do not print at all
        if try!(self.summary.print(t, config, tag, c1, c2, c3)) == Printed::Nothing {
            return Ok(Printed::Nothing);
        }
        for item in &self.body {
            try!(item.print(t, config, tag, c1, c2, c3));
        }
        Ok(Printed::Something)
    }

    fn contains_tag(&self, tag: Option<&str>) -> bool {
        self.summary.contains_tag(tag) || self.body.iter().filter(|x| x.contains_tag(tag)).count() > 0
    }

    fn contains_untagged_elements(&self) -> bool {
        self.summary.contains_untagged_elements() ||
        self.body.iter().filter(|x| x.contains_untagged_elements()).count() > 0
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct SummaryElement {
    pub prefix: String,
    pub category: String,
    pub text: String,
    pub tags: Vec<String>,
}

impl Print for SummaryElement {
    fn print<T: Write, F, G, H>(&self,
                                t: &mut T,
                                config: &Config,
                                tag: Option<&str>,
                                c1: &F,
                                c2: &G,
                                c3: &H)
                                -> Result<Printed, Error>
        where F: Fn(&mut T) -> Result<(), Error>,
              G: Fn(&mut T) -> Result<(), Error>,
              H: Fn(&mut T) -> Result<(), Error>
    {
        // Filter out excluded tags
        if self.tags.iter().filter(|x| config.excluded_commit_tags.contains(x)).count() > 0usize {
            return Ok(Printed::Nothing);
        }

        if self.should_be_printed(tag) {
            tryw!(t, "\n- ");
            if config.show_prefix && !self.prefix.is_empty() {
                tryw!(t, "{} ", self.prefix);
            }
            if config.colored_output {
                try!(c1(t));
            }
            tryw!(t, "[{}] ", self.category);
            if config.colored_output {
                try!(c2(t));
            }
            tryw!(t, "{}", self.text);
            try!(c3(t));
        }
        Ok(Printed::Something)
    }

    fn contains_tag(&self, tag: Option<&str>) -> bool {
        match tag {
            Some(tag) => self.tags.contains(&tag.to_owned()),
            None => true,
        }
    }

    fn contains_untagged_elements(&self) -> bool {
        self.tags.is_empty()
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


impl Print for BodyElement {
    fn print<T: Write, F, G, H>(&self,
                                t: &mut T,
                                config: &Config,
                                tag: Option<&str>,
                                c1: &F,
                                c2: &G,
                                c3: &H)
                                -> Result<Printed, Error>
        where F: Fn(&mut T) -> Result<(), Error>,
              G: Fn(&mut T) -> Result<(), Error>,
              H: Fn(&mut T) -> Result<(), Error>
    {
        match *self {
            BodyElement::List(ref vec) => {
                for list_item in vec {
                    try!(list_item.print(t, config, tag, c1, c2, c3));
                }
            }
            BodyElement::Paragraph(ref paragraph) => {
                try!(paragraph.print(t, config, tag, c1, c2, c3));
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

    fn contains_untagged_elements(&self) -> bool {
        match *self {
            BodyElement::List(ref vec) => vec.iter().filter(|x| x.contains_untagged_elements()).count() > 0,
            BodyElement::Paragraph(ref paragraph) => paragraph.contains_untagged_elements(),
        }
    }
}

impl Print for ListElement {
    fn print<T: Write, F, G, H>(&self,
                                t: &mut T,
                                config: &Config,
                                tag: Option<&str>,
                                c1: &F,
                                c2: &G,
                                c3: &H)
                                -> Result<Printed, Error>
        where F: Fn(&mut T) -> Result<(), Error>,
              G: Fn(&mut T) -> Result<(), Error>,
              H: Fn(&mut T) -> Result<(), Error>
    {
        // Check if list item contains excluded tag
        if self.tags.iter().filter(|x| config.excluded_commit_tags.contains(x)).count() > 0usize {
            return Ok(Printed::Nothing);
        }

        if self.should_be_printed(tag) {
            tryw!(t, "\n{}- ", {
                if tag.is_none() {
                    iter::repeat(' ').take(4).collect::<String>()
                } else {
                    String::new()
                }
            });
            if !self.category.is_empty() {
                if config.colored_output {
                    try!(c1(t));
                }
                tryw!(t, "[{}] ", self.category);
                if config.colored_output {
                    try!(c2(t));
                }
            }
            tryw!(t, "{}", self.text);
            try!(c3(t));
        }

        Ok(Printed::Something)
    }

    fn contains_tag(&self, tag: Option<&str>) -> bool {
        match tag {
            Some(tag) => self.tags.contains(&tag.to_owned()),
            None => true,
        }
    }

    fn contains_untagged_elements(&self) -> bool {
        self.tags.is_empty()
    }
}

impl Print for ParagraphElement {
    #[allow(unused_variables)]
    fn print<T: Write, F, G, H>(&self,
                                t: &mut T,
                                config: &Config,
                                tag: Option<&str>,
                                c1: &F,
                                c2: &G,
                                c3: &H)
                                -> Result<Printed, Error>
        where F: Fn(&mut T) -> Result<(), Error>,
              G: Fn(&mut T) -> Result<(), Error>,
              H: Fn(&mut T) -> Result<(), Error>
    {
        // Check if paragraph contains excluded tag
        if self.tags.iter().filter(|x| config.excluded_commit_tags.contains(x)).count() > 0usize {
            return Ok(Printed::Nothing);
        }

        if self.should_be_printed(tag) {
            for (index, line) in self.text
                .lines()
                .map(|x| {
                    let indent = if tag.is_none() { 4 } else { 2 };
                    iter::repeat(' ').take(indent).collect::<String>()
                } + x)
                .collect::<Vec<String>>()
                .iter()
                .enumerate() {
                if tag.is_some() && index == 0 {
                    // Paragraphs will be transformed into lists when using templates
                    tryw!(t, "\n{}", line.replace("  ", "- "));
                } else {
                    tryw!(t, "\n{}", line);
                }
            }
        }
        Ok(Printed::Something)
    }

    fn contains_tag(&self, tag: Option<&str>) -> bool {
        match tag {
            Some(tag) => self.tags.contains(&tag.to_owned()),
            None => true,
        }
    }

    fn contains_untagged_elements(&self) -> bool {
        self.tags.is_empty()
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

#[derive(Clone)]
pub struct Parser {
    pub categories: Vec<String>,
    pub result: Vec<ParsedTag>,
}

impl Parser {
    method!(parse_category<Self, &[u8], &str>, self,
        chain!(
            tag!("[")? ~
            p_category: map_res!(
                re_bytes_find!(&self.categories.join("|")),
                str::from_utf8
            ) ~
            tag!("]")? ,
            || p_category
    ));

    method!(parse_list_item<Self, &[u8], ListElement>, mut self,
        chain!(
            many0!(space) ~
            tag!("-") ~
            space? ~
            p_category: call_m!(self.parse_category)? ~
            space? ~
            p_tags_rest: map!(rest, Self::parse_and_consume_tags),
            || ListElement {
                category: p_category.unwrap_or("").to_owned(),
                tags: p_tags_rest.0.clone(),
                text: p_tags_rest.1.clone(),
            }
        )
    );

    method!(parse_summary<Self, &[u8], SummaryElement>, mut self,
        chain!(
            p_prefix: separated_pair!(alpha, char!('-'), digit)? ~
            space? ~
            p_category: call_m!(self.parse_category) ~
            space ~
            p_tags_rest: map!(rest, Self::parse_and_consume_tags),
        || SummaryElement {
            prefix: p_prefix.map_or("".to_owned(), |p| {
                format!("{}-{}", str::from_utf8(p.0).unwrap_or(""), str::from_utf8(p.1).unwrap_or(""))
            }),
            category: p_category.to_owned(),
            tags: p_tags_rest.0.clone(),
            text: p_tags_rest.1.clone(),
        })
    );

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

    /// Parses a single commit message and returns a changelog ready form
    pub fn parse_commit_message(&self, message: &str) -> Result<ParsedCommit, Error> {
        // Every block is split by two newlines
        let mut commit_parts = message.split("\n\n");

        // Parse the summary line
        let summary_line = try!(commit_parts.nth(0).ok_or(Error::CommitMessageLength)).trim();
        let parsed_summary = match self.clone().parse_summary(summary_line.as_bytes()) {
            (_, IResult::Done(_, parsed)) => parsed,
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
                    if let (_, IResult::Done(_, result)) = self.clone().parse_list_item(list_item.as_bytes()) {
                        list.push(result);
                    };
                }
                parsed_body.push(BodyElement::List(list));
            } else {
                // Assume paragraph, test for a valid paragraph
                if !RE_PARAGRAPH.is_match(part) {
                    return Err(Error::ParagraphParsing(part.to_owned()));
                }
                let (parsed_tags, parsed_text) = Self::parse_and_consume_tags(part.as_bytes());
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

    /// Prints the commits without any template
    pub fn print(&self,
                 config: &Config,
                 compact: &bool,
                 template: Option<&str>)
                 -> Result<Vec<u8>, Error> {

        let mut term = try!(term::stdout().ok_or(Error::Terminal));
        let mut vec = vec![];

        for tag in &self.result {
            try!(tag.print_to_term_and_write_to_vector(&mut term, &mut vec, compact, config, template));
        }

        trywln!(term, "");
        Ok(vec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use term;
    use config;
    use config::Config;

    fn get_parser() -> Parser {
        let config = Config::new();
        Parser {
            categories: config.categories,
            result: vec![],
        }
    }

    fn parse_and_print_error(message: &str) {
        let ret = get_parser().parse_commit_message(message);
        assert!(ret.is_err());
        if let Err(e) = ret {
            println!("{}", e);
        }
    }

    #[test]
    fn parse_commit_ok_1() {
        let commit = get_parser()
            .parse_commit_message("JIRA-1234 [Changed] my commit summary\n\nSome paragraph\n\n# A comment\n# Another \
                                   comment");
        assert!(commit.is_ok());
        if let Ok(commit) = commit {
            assert_eq!(commit.body.len(), 1);
            assert_eq!(commit.body[0],
                       BodyElement::Paragraph(ParagraphElement {text: "Some paragraph".to_owned(), tags: vec![]}));
            assert_eq!(commit.footer.len(), 0);
            assert_eq!(commit.summary.prefix, "JIRA-1234");
            assert_eq!(commit.summary.category, "Changed");
            assert_eq!(commit.summary.text, "my commit summary");
            assert_eq!(commit.summary.tags.len(), 0);
            let mut t = term::stdout().unwrap();
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![],
                                                             &config::Config::new(), None).is_ok());
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![],
                                                             &config::Config::new(), Some("tag")).is_ok());
        }
    }

    #[test]
    fn parse_commit_ok_2() {
        let commit = get_parser()
            .parse_commit_message("Changed my commit summary\n\n- List item 1\n- List item 2\n- List item 3");
        assert!(commit.is_ok());
        if let Ok(commit) = commit {
            assert_eq!(commit.body.len(), 1);
            assert_eq!(commit.footer.len(), 0);
            assert_eq!(commit.summary.prefix, "");
            assert_eq!(commit.summary.category, "Changed");
            assert_eq!(commit.summary.text, "my commit summary");
            assert_eq!(commit.summary.tags.len(), 0);
            let mut t = term::stdout().unwrap();
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![],
                                                             &config::Config::new(), None).is_ok());
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![],
                                                             &config::Config::new(), Some("tag")).is_ok());
        }
    }

    #[test]
    fn parse_commit_ok_3() {
        let commit = get_parser()
            .parse_commit_message("PREFIX-666 Fixed some ____ commit :tag1: :tag2: :tag3:\n\nSome: Footer\nAnother: \
                                   Footer\nMy-ID: IDVALUE");
        assert!(commit.is_ok());
        if let Ok(commit) = commit {
            assert_eq!(commit.body.len(), 0);
            assert_eq!(commit.footer.len(), 3);
            assert_eq!(commit.summary.prefix, "PREFIX-666");
            assert_eq!(commit.summary.category, "Fixed");
            assert_eq!(commit.summary.text, "some ____ commit");
            assert_eq!(commit.summary.tags, vec!["tag1".to_owned(), "tag2".to_owned(), "tag3".to_owned()]);
            let mut t = term::stdout().unwrap();
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![],
                                                             &config::Config::new(), None).is_ok());
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![],
                                                             &config::Config::new(), Some("tag3")).is_ok());
        }
    }


    #[test]
    fn parse_commit_ok_4() {
        let commit = get_parser()
            .parse_commit_message("Added my :1234: commit 💖 summary :some tag:\n\nParagraph\n\n- List \
                                   Item\n\nReviewed-by: Me");
        assert!(commit.is_ok());
        if let Ok(commit) = commit {
            assert_eq!(commit.body.len(), 2);
            assert_eq!(commit.footer.len(), 1);
            assert_eq!(commit.summary.prefix, "");
            assert_eq!(commit.summary.category, "Added");
            assert_eq!(commit.summary.text, "my commit 💖 summary");
            assert_eq!(commit.summary.tags, vec!["1234".to_owned(), "some tag".to_owned()]);
            let mut t = term::stdout().unwrap();
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![],
                                                             &config::Config::new(), None).is_ok());
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![],
                                                             &config::Config::new(), Some("some tag")).is_ok());
        }
    }

    #[test]
    fn parse_commit_failure_1() {
        parse_and_print_error("None");
    }

    #[test]
    fn parse_commit_failure_2() {
        parse_and_print_error("PREFIX+1234 Changing some stuff");
    }

    #[test]
    fn parse_commit_failure_3() {
        parse_and_print_error("Fix some stuff");
    }
}
