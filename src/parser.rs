use nom::{IResult, eof, alpha, digit, newline, space, rest};
use super::GitJournalError;
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

        // Parse the summary line
        let summary_line = try!(message.lines().nth(0).ok_or(ParserError::CommitMessageLength));
        named!(summary_parser<SummaryElement>,
            chain!(
                separated_pair!(alpha, char!('-'), digit)? ~
                space? ~
                tag!("[")? ~
                cat: map_res!(
                    alt!(
                        tag!("Added") |
                        tag!("Changed") |
                        tag!("Fixed") |
                        tag!("Improved") |
                        tag!("Removed") |
                        tag!("Changed")),
                str::from_utf8) ~
                tag!("]")? ~
                txt: map_res!(
                    rest,
                    str::from_utf8
                ),
            || SummaryElement {
                prefix: "bla".to_string(),
                category: cat.to_string(),
                text: txt.to_string(),
                tags: vec![],
            })
        );

        let parsed = match summary_parser(summary_line.as_bytes()) {
            IResult::Done(_, parsed) => parsed,
            _ => return Err(ParserError::SummaryParsing(format!("Could not parse commit summary: {}", summary_line))),
        };

        println!("{:?}", parsed);

        // Ok(format!("- [{}]{}", parsed.0, parsed.1))
        Ok("-".to_string())
    }
}
