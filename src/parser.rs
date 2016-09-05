use nom::{IResult, alpha, digit, space, rest};
use regex::{Regex, RegexBuilder};
use std::str;
use std::fmt;

#[derive(Debug)]
pub enum ParserError {
    SummaryParsing(String),
    FooterParsing(String),
    CommitMessageLength,
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParserError::SummaryParsing(ref line) => write!(f, "Could not parse commit summary: {}", line),
            ParserError::FooterParsing(ref line) => write!(f, "Could not parse commit footer: {}", line),
            ParserError::CommitMessageLength => write!(f, "Commit message length too small."),
        }
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ParsedCommit {
    summary: SummaryElement,
    body: Vec<BodyElement>,
    footer: Vec<FooterElement>,
}

impl fmt::Display for ParsedCommit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "{}\n  {:?}\n  {:?}",
               self.summary,
               self.body,
               self.footer)
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct SummaryElement {
    prefix: String,
    category: String,
    text: String,
    tags: Vec<String>,
}

impl fmt::Display for SummaryElement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "- [{}]{} (Prefix: '{}', Tags: {:?})",
               self.category,
               self.text,
               self.prefix,
               self.tags)
    }
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub enum BodyElement {
    List(Vec<ListElement>),
    Paragraph(ParagraphElement),
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ListElement {
    category: String,
    text: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct ParagraphElement {
    text: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq)]
pub struct FooterElement {
    key: String,
    value: String,
}

lazy_static! {
    static ref RE_TAGS: Regex = Regex::new(r" :(.*?):").unwrap();
    static ref RE_FOOTER: Regex = RegexBuilder::new(r"^([\w-]+):\s(.*)$").multi_line(true).compile().unwrap();
    static ref RE_LIST: Regex = RegexBuilder::new(r"^-\s.*$(\n^\s+-\s.*)*").multi_line(true).compile().unwrap();
}

pub struct Parser;
impl Parser {
    /// Parses a single commit message and returns a changelog ready form
    pub fn parse_commit_message(&self, message: &str) -> Result<ParsedCommit, ParserError> {

        /// Parses for tags and returns them with the resulting string
        fn parse_and_consume_tags(i: &[u8]) -> (Vec<String>, String) {
            let string = str::from_utf8(i).unwrap_or("");
            let mut tags = vec![];
            for cap in RE_TAGS.captures_iter(string) {
                tags.extend(cap.at(1).unwrap_or("").split(',').map(|x| x.trim().to_owned()).collect::<Vec<String>>());
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

        named!(parse_list_category<&str>,
            chain!(
                tag!("- ") ~
                p_category: parse_category ,
                || p_category
            )
        );

        // Every block is split by two newlines
        let mut commit_parts = message.split("\n\n");

        // Parse the summary line
        let summary_line = try!(commit_parts.nth(0).ok_or(ParserError::CommitMessageLength));
        named!(parse_summary<SummaryElement>,
            chain!(
                p_prefix: separated_pair!(alpha, char!('-'), digit)? ~
                space? ~
                p_category: parse_category ~
                tag!("]")? ~
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
            _ => return Err(ParserError::SummaryParsing(summary_line.to_owned())),
        };

        // Parse the body and the footer, the summary is already consumed
        let mut parsed_footer = vec![];
        let mut parsed_body = vec![];
        for part in commit_parts {
            // Parse footer
            if RE_FOOTER.is_match(part) {
                for cap in RE_FOOTER.captures_iter(part) {
                    parsed_footer.push(FooterElement {
                        key: try!(cap.at(1).ok_or(ParserError::FooterParsing(part.to_owned()))).to_owned(),
                        value: try!(cap.at(2).ok_or(ParserError::FooterParsing(part.to_owned()))).to_owned(),
                    });
                }
            } else if RE_LIST.is_match(part) {
                // Parse list items
                let mut list = vec![];
                for list_item in part.lines() {
                    let parsed_category = match parse_list_category(list_item.as_bytes()) {
                        IResult::Done(_, cat) => cat,
                        _ => "",

                    };
                    let (parsed_tags, parsed_text) = parse_and_consume_tags(list_item.as_bytes());
                    list.push(ListElement {
                        category: parsed_category.to_owned(),
                        text: parsed_text,
                        tags: parsed_tags,
                    });
                }
                parsed_body.push(BodyElement::List(list));
            } else {
                // Assume paragraph
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
}
