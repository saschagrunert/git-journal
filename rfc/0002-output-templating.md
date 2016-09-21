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
[[tag]]
tag = "feature"
name = "Feature"

[[tag]]
tag = "doc"
name = "Documentation"

[[tag]]
[[tag.subtag]]
tag = "doc_internal"
name = "Internal documentation"

[[tag]]
[[tag.subtag]]
tag = "doc_cust"
name = "Customer documentation"

[[tag]]
tag = "internal"
name = "Internal"
footers = ["Fixes"]
```

Every tag represents a toml table which can be nested as well. Arrays of tables can be used to keep the order of the
items, whereas the name of the array does not matter at all. The `tag` fields corresponds to the commit message tag and
the `name` field inside the table maps the related tag to a chosen name.

The `default` table can be used to specify every commit item which contains no tag at all. The `footers` array specifies
the to be printed commit footers.
