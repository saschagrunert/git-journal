use chrono::{Date, UTC, Datelike};
use git2::Oid;
use nom::{IResult, alpha, digit, space, rest};
use regex::{Regex, RegexBuilder};
use term;
use toml::{self, Value};

use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use std::{iter, str};

use config::Config;
use errors::{GitJournalResult, error};

pub static TOML_DEFAULT_KEY: &'static str = "default";
pub static TOML_FOOTERS_KEY: &'static str = "footers";
pub static TOML_NAME_KEY: &'static str = "name";
pub static TOML_TAG: &'static str = "tag";

pub static TOML_TEXT_KEY: &'static str = "text";
pub static TOML_ONCE_KEY: &'static str = "once";
pub static TOML_HEADER_KEY: &'static str = "header";
pub static TOML_FOOTER_KEY: &'static str = "footer";

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
                                -> GitJournalResult<Printed>
        where F: Fn(&mut T) -> GitJournalResult<()>,
              G: Fn(&mut T) -> GitJournalResult<()>,
              H: Fn(&mut T) -> GitJournalResult<()>;

    fn print_default<T: Write>(&self, t: &mut T, config: &Config, tag: Option<&str>) -> GitJournalResult<()> {
        self.print(t, config, tag, &|_| Ok(()), &|_| Ok(()), &|_| Ok(()))?;
        Ok(())
    }

    fn print_default_term(&self,
                          mut t: &mut Box<term::StdoutTerminal>,
                          config: &Config,
                          tag: Option<&str>)
                          -> GitJournalResult<()> {
        self.print(&mut t,
                   config,
                   tag,
                   &|t| {
                       t.fg(term::color::BRIGHT_BLUE)?;
                       Ok(())
                   },
                   &|t| {
                       t.fg(term::color::WHITE)?;
                       Ok(())
                   },
                   &|t| {
                       t.reset()?;
                       Ok(())
                   })?;
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
                                         -> GitJournalResult<()> {
        self.print_default_term(&mut term, config, tag)?;
        self.print_default(&mut vec, config, tag)?;
        Ok(())
    }
}

pub trait Tags {
    /// Just extends a given vector with all found tags, unsorted.
    /// Transfers ownership of the vector back if done.
    fn get_tags(&self, vec: Vec<String>) -> Vec<String>;

    /// Sort and uniq the tags as well.
    /// Transfers ownership of the vector back if done.
    fn get_tags_unique(&self, mut vec: Vec<String>) -> Vec<String> {
        vec = self.get_tags(vec);
        vec.sort();
        vec.dedup();
        vec
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ParsedTag {
    pub name: String,
    pub date: Date<UTC>,
    pub commits: Vec<ParsedCommit>,
    pub message_ids: Vec<usize>,
}

impl ParsedTag {
    fn print<T: Write, F, G, H>(&self, t: &mut T, config: &Config, c1: &F, c2: &G, c3: &H) -> GitJournalResult<Printed>
        where F: Fn(&mut T) -> GitJournalResult<()>,
              G: Fn(&mut T) -> GitJournalResult<()>,
              H: Fn(&mut T) -> GitJournalResult<()>
    {
        if config.colored_output {
            c1(t)?;
        }
        write!(t, "\n# {} ", self.name)?;
        if config.colored_output {
            c2(t)?;
        }
        write!(t,
               "({}-{:02}-{:02}):",
               self.date.year(),
               self.date.month(),
               self.date.day())?;
        if config.colored_output {
            c3(t)?;
        }
        Ok(Printed::Something)
    }

    fn print_default<T: Write>(&self, t: &mut T, config: &Config) -> GitJournalResult<()> {
        self.print(t, config, &|_| Ok(()), &|_| Ok(()), &|_| Ok(()))?;
        Ok(())
    }

    fn print_default_term(&self, mut t: &mut Box<term::StdoutTerminal>, config: &Config) -> GitJournalResult<()> {
        self.print(&mut t,
                   config,
                   &|t| {
                       t.fg(term::color::GREEN)?;
                       Ok(())
                   },
                   &|t| {
                       t.fg(term::color::YELLOW)?;
                       Ok(())
                   },
                   &|t| {
                       t.reset()?;
                       Ok(())
                   })?;
        Ok(())
    }

    fn print_to_term_and_write_to_vector(&self,
                                         mut term: &mut Box<term::StdoutTerminal>,
                                         mut vec: &mut Vec<u8>,
                                         compact: &bool,
                                         config: &Config,
                                         template: Option<&str>,
                                         index_len: (usize, usize))
                                         -> GitJournalResult<()> {
        match template {
            Some(template) => {
                // Try to parse the template
                let mut file = File::open(template)?;
                let mut toml_string = String::new();
                file.read_to_string(&mut toml_string)?;
                let toml: Value = toml::from_str(&toml_string)?;

                // Print header in template if exists
                if let Some(&Value::Table(ref header_table)) = toml.get(TOML_HEADER_KEY) {
                    let mut print_once = false;
                    if let Some(&Value::Boolean(ref once)) = header_table.get(TOML_ONCE_KEY) {
                        print_once = *once;
                    }
                    if let Some(&Value::String(ref header)) = header_table.get(TOML_TEXT_KEY) {
                        if (index_len.0 == 0 || !print_once) && !header.is_empty() {
                            writeln!(term, "\n{}", header)?;
                            writeln!(vec, "\n{}", header)?;
                        }
                    }
                }

                // Print the tags
                self.print_default_term(&mut term, config)?;
                self.print_default(&mut vec, config)?;

                // Print commits
                if let Some(main_table) = toml.as_table() {
                    self.print_commits_in_table(&mut term, &mut vec, main_table, &mut 1, config, &compact)?;
                }

                // Print footer in template if exists
                if let Some(&Value::Table(ref footer_table)) = toml.get(TOML_FOOTER_KEY) {
                    let mut print_once = false;
                    if let Some(&Value::Boolean(ref once)) = footer_table.get(TOML_ONCE_KEY) {
                        print_once = *once;
                    }
                    if let Some(&Value::String(ref footer)) = footer_table.get(TOML_TEXT_KEY) {
                        if (index_len.0 == index_len.1 - 1 || !print_once) && !footer.is_empty() {
                            writeln!(term, "\n{}", footer)?;
                            writeln!(vec, "\n{}", footer)?;
                        }
                    }
                }
            }
            None => {
                self.print_default_term(&mut term, config)?;
                self.print_default(&mut vec, config)?;

                for commit in &self.commits {
                    if *compact {
                        commit.summary.print_to_term_and_write_to_vector(&mut term, &mut vec, config, None)?;
                    } else {
                        commit.print_to_term_and_write_to_vector(&mut term, &mut vec, config, None)?;
                    }
                }
                writeln!(term, "")?;
                writeln!(vec, "")?;
                if !*compact && config.enable_footers {
                    self.print_footers(&mut term, &mut vec, None, &config)?;
                }
            }
        }

        Ok(())
    }

    fn print_commits_in_table(&self,
                              mut term: &mut Box<term::StdoutTerminal>,
                              mut vec: &mut Vec<u8>,
                              table: &toml::value::Table,
                              level: &mut usize,
                              config: &Config,
                              compact: &bool)
                              -> GitJournalResult<()> {
        for value in table {
            if let Value::Array(ref array) = *value.1 {
                for item in array {
                    if let Value::Table(ref table) = *item {
                        *level += 1;
                        self.print_commits_in_table(term, vec, table, level, config, compact)?;
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
                term.fg(term::color::BRIGHT_RED)?;
            }
            write!(term, "\n{} {}", header_lvl, name)?;
            write!(vec, "\n{} {}", header_lvl, name)?;

            term.reset()?;

            // Print commits for this tag
            for commit in &self.commits {
                if *compact {
                    commit.summary
                        .print_to_term_and_write_to_vector(&mut term, &mut vec, config, Some(tag))?;
                } else {
                    commit.print_to_term_and_write_to_vector(&mut term, &mut vec, config, Some(tag))?;
                }
            }

            writeln!(term, "")?;
            writeln!(vec, "")?;

            // Print footers if specified in the template
            if let Some(footers) = table.get(TOML_FOOTERS_KEY) {
                if let Value::Array(ref array) = *footers {
                    if !array.is_empty() {
                        self.print_footers(term, vec, Some(array), config)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn print_footers(&self,
                     mut term: &mut Box<term::StdoutTerminal>,
                     mut vec: &mut Vec<u8>,
                     footer_keys: Option<&[Value]>,
                     config: &Config)
                     -> GitJournalResult<()> {

        let mut footer_tree: BTreeMap<String, Vec<String>> = BTreeMap::new();

        // Collect valid footer keys into one vector
        let valid_footer_keys = match footer_keys {
            Some(keys) => {
                let mut vec = vec![];
                for key in keys {
                    if let Value::String(ref footer_key) = *key {
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
                let mut value = footer.value;
                if config.show_commit_hash {
                    if let Some(oid) = footer.oid {
                        value = format!("{} ({:.7})", value, oid);
                    }
                }
                footer_tree.entry(footer.key).or_insert_with(|| vec![]).push(value);
            }
        }

        // Sort the values by the containing strings
        for value in footer_tree.values_mut() {
            value.sort();
        }

        // Print the mapped footers
        for (key, values) in &footer_tree {
            if config.colored_output {
                term.fg(term::color::BRIGHT_RED)?;
            }
            writeln!(term, "\n{}:", key)?;
            writeln!(vec, "\n{}:", key)?;
            term.reset()?;
            let footer_string = values.join(", ");
            let mut char_count = 0;
            let mut footer_lines = String::new();
            for cur_char in footer_string.chars() {
                if char_count > 100 && cur_char == ' ' {
                    footer_lines.push('\n');
                    char_count = 0;
                } else {
                    footer_lines.push(cur_char);
                    char_count += 1;
                }
            }
            writeln!(term, "{}", footer_lines)?;
            writeln!(vec, "{}", footer_lines)?;
        }
        Ok(())
    }
}

impl Tags for ParsedTag {
    fn get_tags(&self, mut vec: Vec<String>) -> Vec<String> {
        for commit in &self.commits {
            vec = commit.get_tags(vec);
        }
        vec
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ParsedCommit {
    pub oid: Option<Oid>,
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
                                -> GitJournalResult<Printed>
        where F: Fn(&mut T) -> GitJournalResult<()>,
              G: Fn(&mut T) -> GitJournalResult<()>,
              H: Fn(&mut T) -> GitJournalResult<()>
    {
        // If summary is already filtered out then do not print at all
        if self.summary.print(t, config, tag, c1, c2, c3)? == Printed::Nothing {
            return Ok(Printed::Nothing);
        }
        for item in &self.body {
            item.print(t, config, tag, c1, c2, c3)?;
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

impl Tags for ParsedCommit {
    fn get_tags(&self, mut vec: Vec<String>) -> Vec<String> {
        vec.extend(self.summary.tags.clone());
        for body_element in &self.body {
            vec = body_element.get_tags(vec);
        }
        vec
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct SummaryElement {
    pub oid: Option<Oid>,
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
                                -> GitJournalResult<Printed>
        where F: Fn(&mut T) -> GitJournalResult<()>,
              G: Fn(&mut T) -> GitJournalResult<()>,
              H: Fn(&mut T) -> GitJournalResult<()>
    {
        // Filter out excluded tags
        if self.tags.iter().filter(|x| config.excluded_commit_tags.contains(x)).count() > 0usize {
            return Ok(Printed::Nothing);
        }

        if self.should_be_printed(tag) {
            write!(t, "\n- ")?;
            if config.show_prefix && !self.prefix.is_empty() {
                write!(t, "{} ", self.prefix)?;
            }
            if config.colored_output {
                c1(t)?;
            }
            write!(t, "{}", config.category_delimiters[0])?;
            write!(t, "{}", self.category)?;
            write!(t, "{} ", config.category_delimiters[1])?;
            if config.colored_output {
                c2(t)?;
            }
            write!(t, "{}", self.text)?;

            // Print the oid for the summary element (always)
            if config.show_commit_hash {
                if let Some(oid) = self.oid {
                    write!(t, " ({:.7})", oid)?;
                }
            }
            c3(t)?;
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
    pub oid: Option<Oid>,
    pub category: String,
    pub text: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ParagraphElement {
    pub oid: Option<Oid>,
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
                                -> GitJournalResult<Printed>
        where F: Fn(&mut T) -> GitJournalResult<()>,
              G: Fn(&mut T) -> GitJournalResult<()>,
              H: Fn(&mut T) -> GitJournalResult<()>
    {
        match *self {
            BodyElement::List(ref vec) => {
                for list_item in vec {
                    list_item.print(t, config, tag, c1, c2, c3)?;
                }
            }
            BodyElement::Paragraph(ref paragraph) => {
                paragraph.print(t, config, tag, c1, c2, c3)?;
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

impl Tags for BodyElement {
    fn get_tags(&self, mut vec: Vec<String>) -> Vec<String> {
        match *self {
            BodyElement::List(ref list_vec) => {
                for list_item in list_vec {
                    vec = list_item.get_tags(vec);
                }
            }
            BodyElement::Paragraph(ref paragraph) => vec.extend(paragraph.tags.clone()),
        }
        vec
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
                                -> GitJournalResult<Printed>
        where F: Fn(&mut T) -> GitJournalResult<()>,
              G: Fn(&mut T) -> GitJournalResult<()>,
              H: Fn(&mut T) -> GitJournalResult<()>
    {
        // Check if list item contains excluded tag
        if self.tags.iter().filter(|x| config.excluded_commit_tags.contains(x)).count() > 0usize {
            return Ok(Printed::Nothing);
        }

        if self.should_be_printed(tag) {
            write!(t, "\n{}- ", {
                if tag.is_none() {
                    iter::repeat(' ').take(4).collect::<String>()
                } else {
                    String::new()
                }
            })?;
            if !self.category.is_empty() {
                if config.colored_output {
                    c1(t)?;
                }
                write!(t, "{}", config.category_delimiters[0])?;
                write!(t, "{}", self.category)?;
                write!(t, "{} ", config.category_delimiters[1])?;
                if config.colored_output {
                    c2(t)?;
                }
            }
            write!(t, "{}", self.text)?;
            // Print only in templating mode, otherwise hide unnecessary information
            if config.show_commit_hash && tag.is_some() {
                if let Some(oid) = self.oid {
                    write!(t, " ({:.7})", oid)?;
                }
            }
            c3(t)?;
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

impl Tags for ListElement {
    fn get_tags(&self, mut vec: Vec<String>) -> Vec<String> {
        vec.extend(self.tags.clone());
        vec
    }
}

impl Print for ParagraphElement {
    fn print<T: Write, F, G, H>(&self,
                                t: &mut T,
                                config: &Config,
                                tag: Option<&str>,
                                _c1: &F,
                                _c2: &G,
                                _c3: &H)
                                -> GitJournalResult<Printed>
        where F: Fn(&mut T) -> GitJournalResult<()>,
              G: Fn(&mut T) -> GitJournalResult<()>,
              H: Fn(&mut T) -> GitJournalResult<()>
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
                    write!(t, "\n{}", line.replace("  ", "- "))?;
                } else {
                    write!(t, "\n{}", line)?;
                }
                // Print only in templating mode, otherwise hide unnecessary information
                if config.show_commit_hash && tag.is_some() {
                    if let Some(oid) = self.oid {
                        write!(t, " ({:.7})", oid)?;
                    }
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
    pub oid: Option<Oid>,
    pub key: String,
    pub value: String,
}

lazy_static! {
    static ref RE_TAGS: Regex = Regex::new(r"[ \n]:(.*?):").unwrap();
    static ref RE_FOOTER: Regex = RegexBuilder::new(r"^([\w-]+):\s(.*)$").multi_line(true).compile().unwrap();
    static ref RE_LIST: Regex = RegexBuilder::new(r"^-\s.*$(\n^\s+-\s.*)*").multi_line(true).compile().unwrap();
    static ref RE_PARAGRAPH: Regex = RegexBuilder::new(r"^\w").multi_line(true).compile().unwrap();
    static ref RE_COMMENT: Regex = RegexBuilder::new(r"^#.*").multi_line(true).compile().unwrap();
}

#[derive(Clone)]
pub struct Parser {
    pub config: Config,
    pub result: Vec<ParsedTag>,
}

impl Parser {
    method!(parse_category<Self, &[u8], &str>, self,
        do_parse!(
            opt!(tag!(self.config.category_delimiters[0].as_str())) >>
            p_category: map_res!(
                re_bytes_find!(&self.config.categories.join("|")),
                str::from_utf8
            ) >>
            opt!(tag!(self.config.category_delimiters[1].as_str())) >>

            (p_category)
    ));

    method!(parse_list_item<Self, &[u8], ListElement>, mut self,
        do_parse!(
            many0!(space) >>
            tag!("-") >>
            opt!(space) >>
            p_category: opt!(call_m!(self.parse_category)) >>
            opt!(space) >>
            p_tags_rest: map!(rest, Self::parse_and_consume_tags) >>

            (ListElement {
                oid: None,
                category: p_category.unwrap_or("").to_owned(),
                tags: p_tags_rest.0.clone(),
                text: p_tags_rest.1.clone(),
            })
        )
    );

    method!(parse_summary<Self, &[u8], SummaryElement>, mut self,
        do_parse!(
            p_prefix: opt!(separated_pair!(alpha, char!('-'), digit)) >>
            opt!(space) >>
            p_category: call_m!(self.parse_category) >>
            space >>
            p_tags_rest: map!(rest, Self::parse_and_consume_tags) >>

            (SummaryElement {
                oid: None,
                prefix: p_prefix.map_or("".to_owned(), |p| {
                    format!("{}-{}", str::from_utf8(p.0).unwrap_or(""), str::from_utf8(p.1).unwrap_or(""))
                }),
                category: p_category.to_owned(),
                tags: p_tags_rest.0.clone(),
                text: p_tags_rest.1.clone(),
            })
        )
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
        let mut text = RE_TAGS.replace_all(string, "");
        if let Some('.') = text.chars().rev().nth(0) {
            text.pop();
        }
        (tags, text)
    }

    /// Parses a single commit message and returns a changelog ready form
    pub fn parse_commit_message(&self, message: &str, oid: Option<Oid>) -> GitJournalResult<ParsedCommit> {
        // Every block is split by two newlines
        let mut commit_parts = message.split("\n\n");

        // Parse the summary line
        let summary_line = commit_parts.nth(0).ok_or(error("Summary line", "Commit message length too small."))?.trim();
        let mut parsed_summary = match self.clone().parse_summary(summary_line.as_bytes()) {
            (_, IResult::Done(_, parsed)) => parsed,
            _ => bail!("Summary parsing failed: '{}'", summary_line),
        };
        parsed_summary.oid = oid;

        // Parse the body and the footer, the summary is already consumed
        let mut parsed_footer = vec![];
        let mut parsed_body = vec![];

        // Iterate over all the commit message parts
        for part in commit_parts {

            // Do nothing on comments and empty parts
            if RE_COMMENT.is_match(part) || part.is_empty() {
                continue;

                // Parse the footer
            } else if RE_FOOTER.is_match(part) {
                for cap in RE_FOOTER.captures_iter(part) {
                    parsed_footer.push(FooterElement {
                        oid: oid,
                        key: cap.at(1).ok_or(error("Footer parsing", part))?.to_owned(),
                        value: cap.at(2).ok_or(error("Footer parsing", part))?.to_owned(),
                    });
                }

                // Parse all list items
            } else if RE_LIST.is_match(part) {
                let mut list = vec![];
                for list_item in part.lines() {
                    if let (_, IResult::Done(_, mut result)) = self.clone().parse_list_item(list_item.as_bytes()) {
                        result.oid = oid;
                        list.push(result);
                    };
                }
                parsed_body.push(BodyElement::List(list));

                // Nothing of tbe above items matched, check for a Paragraph element
            } else if RE_PARAGRAPH.is_match(part) {
                let (parsed_tags, parsed_text) = Self::parse_and_consume_tags(part.as_bytes());
                parsed_body.push(BodyElement::Paragraph(ParagraphElement {
                    oid: oid,
                    text: parsed_text.trim().to_owned(),
                    tags: parsed_tags,
                }));

                // Nothing matched, this should not happen at all
            } else {
                bail!("Could not parse commit message part: '{}'", part);
            }
        }

        Ok(ParsedCommit {
            oid: oid,
            summary: parsed_summary,
            body: parsed_body,
            footer: parsed_footer,
        })
    }

    /// Prints the commits without any template
    pub fn print(&self, compact: &bool, template: Option<&str>) -> GitJournalResult<Vec<u8>> {
        let mut term = term::stdout().ok_or(error("Terminal", "Could not print to terminal"))?;
        let mut vec = vec![];

        // Print every tag
        for (index, tag) in self.result.iter().enumerate() {
            tag.print_to_term_and_write_to_vector(&mut term,
                                                   &mut vec,
                                                   compact,
                                                   &self.config,
                                                   template,
                                                   (index, self.result.len()))?;
        }

        writeln!(term, "")?;
        Ok(vec)
    }

    /// Returns all tags recursively from a toml table
    pub fn get_tags_from_toml(&self, table: &toml::value::Table, mut vec: Vec<String>) -> Vec<String> {
        for value in table {
            if let Value::Array(ref array) = *value.1 {
                for item in array {
                    if let Value::Table(ref table) = *item {
                        vec = self.get_tags_from_toml(table, vec);
                    }
                }
            }
        }

        if let Some(element) = table.get(TOML_TAG) {
            if let Value::String(ref tag) = *element {
                vec.push(tag.to_owned());
            }
        }
        vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use term;
    use config;
    use config::Config;

    fn get_parser() -> Parser {
        Parser {
            config: Config::new(),
            result: vec![],
        }
    }

    fn parse_and_print_error(message: &str) {
        let ret = get_parser().parse_commit_message(message, None);
        assert!(ret.is_err());
        if let Err(e) = ret {
            println!("{}", e);
        }
    }

    #[test]
    fn parse_commit_ok_1() {
        let commit = get_parser()
            .parse_commit_message("JIRA-1234 [Changed] my commit summary\n\nSome paragraph\n\n# A comment\n# Another \
                                   comment",
                                  None);
        assert!(commit.is_ok());
        if let Ok(commit) = commit {
            assert_eq!(commit.body.len(), 1);
            assert_eq!(commit.body[0],
                       BodyElement::Paragraph(ParagraphElement {
                           text: "Some paragraph".to_owned(),
                           tags: vec![],
                           oid: None,
                       }));
            assert_eq!(commit.footer.len(), 0);
            assert_eq!(commit.summary.prefix, "JIRA-1234");
            assert_eq!(commit.summary.category, "Changed");
            assert_eq!(commit.summary.text, "my commit summary");
            assert_eq!(commit.summary.tags.len(), 0);
            let mut t = term::stdout().unwrap();
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![], &config::Config::new(), None)
                .is_ok());
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![], &config::Config::new(), Some("tag"))
                .is_ok());
        }
    }

    #[test]
    fn parse_commit_ok_2() {
        let commit = get_parser()
            .parse_commit_message("Changed my commit summary\n\n- List item 1\n- List item 2\n- List item 3",
                                  None);
        assert!(commit.is_ok());
        if let Ok(commit) = commit {
            assert_eq!(commit.body.len(), 1);
            assert_eq!(commit.footer.len(), 0);
            assert_eq!(commit.summary.prefix, "");
            assert_eq!(commit.summary.category, "Changed");
            assert_eq!(commit.summary.text, "my commit summary");
            assert_eq!(commit.summary.tags.len(), 0);
            let mut t = term::stdout().unwrap();
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![], &config::Config::new(), None)
                .is_ok());
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![], &config::Config::new(), Some("tag"))
                .is_ok());
        }
    }

    #[test]
    fn parse_commit_ok_3() {
        let commit = get_parser()
            .parse_commit_message("PREFIX-666 Fixed some ____ commit :tag1: :tag2: :tag3:\n\nSome: Footer\nAnother: \
                                   Footer\nMy-ID: IDVALUE",
                                  None);
        assert!(commit.is_ok());
        if let Ok(commit) = commit {
            assert_eq!(commit.body.len(), 0);
            assert_eq!(commit.footer.len(), 3);
            assert_eq!(commit.summary.prefix, "PREFIX-666");
            assert_eq!(commit.summary.category, "Fixed");
            assert_eq!(commit.summary.text, "some ____ commit");
            assert_eq!(commit.summary.tags,
                       vec!["tag1".to_owned(), "tag2".to_owned(), "tag3".to_owned()]);
            let mut t = term::stdout().unwrap();
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![], &config::Config::new(), None)
                .is_ok());
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![], &config::Config::new(), Some("tag3"))
                .is_ok());
        }
    }


    #[test]
    fn parse_commit_ok_4() {
        let commit = get_parser()
            .parse_commit_message("Added my :1234: commit 💖 summary :some tag:\n\nParagraph\n\n- List \
                                   Item\n\nReviewed-by: Me",
                                  None);
        assert!(commit.is_ok());
        if let Ok(commit) = commit {
            assert_eq!(commit.body.len(), 2);
            assert_eq!(commit.footer.len(), 1);
            assert_eq!(commit.summary.prefix, "");
            assert_eq!(commit.summary.category, "Added");
            assert_eq!(commit.summary.text, "my commit 💖 summary");
            assert_eq!(commit.summary.tags,
                       vec!["1234".to_owned(), "some tag".to_owned()]);
            let mut t = term::stdout().unwrap();
            assert!(commit.print_to_term_and_write_to_vector(&mut t, &mut vec![], &config::Config::new(), None)
                .is_ok());
            assert!(commit.print_to_term_and_write_to_vector(&mut t,
                                                   &mut vec![],
                                                   &config::Config::new(),
                                                   Some("some tag"))
                .is_ok());
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
