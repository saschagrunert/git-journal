# [git-journal](https://saschagrunert.github.io/gitjournal) ![License MIT](https://img.shields.io/badge/license-MIT-blue.svg) [![Build Status](https://travis-ci.org/saschagrunert/git-journal.svg?branch=master)](https://travis-ci.org/saschagrunert/git-journal)  [![Coverage Status](https://coveralls.io/repos/github/saschagrunert/git-journal/badge.svg?branch=master)](https://coveralls.io/github/saschagrunert/git-journal?branch=master)
The Git Commit Message Framework

## Targets
Target of the project is to provide a Rust based framework to write more sensible commit messages. Single commit
messages should contain one logical change of the project which is described in a standardized way. This results in a
much cleaner git history and provides contributors more information about the actual change.

To gain very clean commit message history it is necessary to use git rebase, squashed and amended commits. git-journal
will simplify these development approaches by providing sensible conventions and strong defaults.

## Development progress
The development is actually ongoing which means that the library and binary is currently not available on
[crates.io](http://crates.io).

## ToDo
The base project will be developed as a git extension written in Rust. These features are planned for the initial
release:

* [ ] Automatic setup (`--setup/-s`) in a certain git repository.
    * [ ] Install git hooks as symlinks to the binary.
    * [x] Provide an initial configuration file with default values.
* [ ] Changelog generation
    * [x] Parse until the last tag or in a commit range.
    * [x] Add the possibility to parse the last n Releases.
    * [x] Filter out excluded tags
    * [ ] Output formats:
        * [ ] Markdown
        * [ ] HTML
        * [ ] PDF
  * [ ] Commit Message templating and validation
    * [ ] Verify commit message based on config via git hook (which is the executable itself)

