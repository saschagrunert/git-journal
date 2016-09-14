# Summary
[summary]: #summary

Providing the possibility to output the parsed git log via custom templates.

# Motivation
[motivation]: #motivation

Since everyone has a different way in creating Changelogs a more flexible approach is needed. A dedicated
templating engine for the output of git-journal will enable these flexibility.

# Detailed design
[design]: #detailed-design

Templates are stored as regular toml files and provide additional information to git-journal regarding the output of the
Changelog. The format of an output template consists of
[tags](https://github.com/saschagrunert/git-journal/blob/master/rfc/0001-commit-msg.md#tags) and their mapping. An
example for such a template looks like this:

```toml
[default]

[tag1]
name = "Section 1"
[tag1.subtag1]
[tag1.subtag2]

[tag2]
```

Every tag represents a toml table which can be nested as well. The `name` field inside the table maps the related tag to
a chosen name. The `default` table can be used to specify every commit item which contains no tag at all.
