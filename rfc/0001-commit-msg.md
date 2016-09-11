# Summary
[summary]: #summary

Extending the git commit message syntax.

# Motivation
[motivation]: #motivation

The current commit message syntax is very limited and will automatically lead into a lots of different commit message
styles in the git history.

# Detailed design
[design]: #detailed-design

Git-journal tries to solve this problem by implementing small syntax additions. This syntax can be used to create strong
conventions and templates as a foundation for a nice looking changelog.

## Commit message layout

A git commit message follows a basic layout, it has a subject and a message body. An extension to this will make an easier
parsing of the commit message possible. This means that an overall commit message can be split into three parts:

- *Summary line*: The first line of the commit
- *Body*: Can hold lists or paragraphs of text
- *Footer*: Add additional information to the commit as `Key: Value` pairs, e.g. `Reviewed-by: John Doe`, multiplication
  is also possible since there could be multiple `Reviewed-by` keys.

An empty line separates the commit message parts, whereas newlines between lists and paragraphs within the body are
possible as well. The separation between the body and the footer itself is implicit by their different syntax.

## Syntax elements

The following new syntax elements will be defined:

- Commit Prefix
- Categories
- Tags

### Commit prefix
A commit message prefix is a single appearing prefix for the summary line of the commit. For example `JIRA-1234` is a valid commit
prefix, which appears directly in the commit summary. Commit prefixes are used to keep a direct connection to a issue tracking
system and are not mandatory.

### Categories
A category is used to indicate with one word what has been done in the commit. Valid categories are: `[Added]`,
`[Changed]`, `[Fixed]`, `[Improved]` and `[Removed]`. Rules for categories are:

- A verb always in simple past form
- Wrapped in square brackets
- Appear at the beginning of a list item or a paragraph
- Only once per list item, but multiple times in a commit
- Only mandatory for the commit summary line

### Tags
Tags help to generate a sectioned changelog and filter out certain messages which are only for a special purpose. Valid
examples for tags are `:internal:`, `:API:`,·`:Documentation:` or `:Feature A:`. The following rules apply to tags:

- Not mandatory
- Wrapped in colons
- Apply to the current line/paragraph
- Can appear anywhere in the line

Tags enable the possibility of sectioned changelogs with filtering the content. For example it is possible to skip
everything which is tagged as `:internal:`.

### Example commit message
```
JIRA-1234 [Added] the fancy thing everyone looks for        | Summary line
                                                            |
Now I describe what I did in a detailed way.                | Body
This detail message will be handeled as a certain           | - Paragraph
paragraph. There is no need for a tag or a category.        |
                                                            |
- [Fixed] some very bas thing                               | - List
- [Added] detailed documentation about that thing :doc:     |
- [Changed] A to look now as B :internal:                   |
                                                            |
Reviewed-by: John Doe                                       | Footer
```

## Syntax validation
Some adaptions to an already existing git repository are needed to use these newly introduced changes:

- Apply the commit message template via `commit.template` or a `prepare-commit-msg` hook
- Use a different comment char via `core.commentchar` if necessary
- Use a `commit-msg` hook for commit message validation

# Drawbacks
[drawbacks]: #drawbacks

The already existing git functionality provides a lots of freedom in writing commit messages. This freedom should not be
limited in a negative way.
