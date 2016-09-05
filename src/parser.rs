use nom::{IResult, alpha, digit, space, rest};
use regex::Regex;
use std::str;
use std::fmt;

#[derive(Debug)]
pub enum ParserError {
    SummaryParsing(String),
    CommitMessageLength,
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParserError::SummaryParsing(ref err) => write!(f, "Could not parse commit message summary: {}", err),
            ParserError::CommitMessageLength => write!(f, "Commit message length too small."),
        }
    }
}

#[derive(Debug)]
pub struct SummaryElement {
    prefix: String,
    category: String,
    text: String,
    tags: Vec<String>,
}

impl fmt::Display for SummaryElement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "- [{}]{} (Prefix: '{}', Tags: '{:?}')",
               self.category,
               self.text,
               self.prefix,
               self.tags.join(", "))
    }
}

pub struct ListElement {
    category: String,
    text: String,
    tags: Vec<String>,
}

pub enum BodyElement {
    List(Vec<ListElement>),
    Paragraph(String),
}

pub struct FooterElement {
    key: String,
    value: String,
}

pub struct ParsedCommit {
    summary: SummaryElement,
    body: Vec<BodyElement>,
    footer: Vec<FooterElement>,
}

pub struct Parser {
}

impl Parser {
    /// Constructs a new `Parser`.
    pub fn new() -> Parser {
        Parser {}
    }

    /// Parses a single commit message and returns a changelog ready form
    pub fn parse_commit_message(&self, message: &str) -> Result<String, ParserError> {

        /// Parses for tags and returns them with the resulting string
        fn parse_tags(i: &[u8]) -> (Vec<String>, String) {
            let string = str::from_utf8(i).unwrap_or("");
            let re_tags = Regex::new(r" :(.*?):").unwrap();
            let mut tags = vec![];
            for cap in re_tags.captures_iter(string) {
                tags.extend(cap.at(1).unwrap_or("").split(",").map(|x| x.trim().to_string()).collect::<Vec<String>>());
            }
            (tags, re_tags.replace_all(string, ""))
        }

        // Parse the summary line
        let summary_line = try!(message.lines().nth(0).ok_or(ParserError::CommitMessageLength));
        named!(summary_parser<SummaryElement>,
            chain!(
                p_prefix: separated_pair!(alpha, char!('-'), digit)? ~
                space? ~
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
                tag!("]")? ~
                p_tags_rest: map!(rest, parse_tags),
            || SummaryElement {
                prefix: p_prefix.map_or("".to_string(), |p| {
                    format!("{}-{}", str::from_utf8(p.0).unwrap_or(""), str::from_utf8(p.1).unwrap_or(""))
                }),
                category: p_category.to_string(),
                tags: p_tags_rest.0.clone(),
                text: p_tags_rest.1.clone(),
            })
        );
        let parsed_summary = match summary_parser(summary_line.as_bytes()) {
            IResult::Done(_, parsed) => parsed,
            _ => return Err(ParserError::SummaryParsing(format!("Could not parse commit summary: {}", summary_line))),
        };

        Ok(format!("{}", parsed_summary))
    }
}
