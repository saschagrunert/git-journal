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
[header]
text = "Some header Markdown/HTML."
once = true

[footer]
text = "Some footer Markdown/HTML."
once = true

[[tags]]
tag = "default"
name = "Default"

[[tags]]
tag = "feature"
name = "Feature"

[[tags]]
tag = "doc"
name = "Documentation"

[[tags]]
[[tags.subtags]]
tag = "doc_internal"
name = "Internal documentation"

[[tags]]
[[tags.subtags]]
tag = "doc_cust"
name = "Customer documentation"

[[tags]]
tag = "internal"
name = "Internal"
footers = ["Fixes"]
```

Every tag represents a toml table which can be nested as well. Arrays of tables (`[[tags]]`) can be used to keep the
order of the items, whereas the name of the array does not matter at all. The `tag` fields corresponds to the commit
message tag and the `name` field inside the table map the related tag to a chosen name. The tables `header` and `footer`
are optional and give the possibility to add additional header or footer text for every git tag. The field `once`
specifies if the header/footer should be print for every git tag or only once per run.

The `default` tag can be used to specify every commit item which contains no tag at all. The `footers` array specifies
the to be printed commit footers.
