# [git-journal](https://saschagrunert.github.io/git-journal) [![License MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/saschagrunert/git-journal/blob/master/LICENSE) [![Build Status](https://travis-ci.org/saschagrunert/git-journal.svg?branch=master)](https://travis-ci.org/saschagrunert/git-journal)  [![Coverage Status](https://coveralls.io/repos/github/saschagrunert/git-journal/badge.svg?branch=master)](https://coveralls.io/github/saschagrunert/git-journal?branch=master)
The Git Commit Message Framework

## Targets
Target of the project is to provide a Rust based framework to write more sensible commit messages. Single commit
messages should contain one logical change of the project which is described in a standardized way. This results in a
much cleaner git history and provides contributors more information about the actual change.

To gain very clean commit message history it is necessary to use git rebase, squashed and amended commits. git-journal
will simplify these development approaches by providing sensible conventions and strong defaults.

## Development progress
The development is actually ongoing which means that the library and binary is currently not available on
[crates.io](http://crates.io). The binary `git-journal` depends on the Rust library `gitjournal`, which also can be used
independently from the binary application.

## ToDo
The base project will be developed as a git extension written in Rust. These features are planned for the initial
release:

* [x] Automatic setup (`--setup/-s`) in a certain git repository.
    * [x] Install git hooks
    * [x] Provide an initial configuration file with default values.
* [x] Commit Message validation based on config
* [x] Commit Message preparation based on config
* [ ] Changelog generation
    * [x] Parse until the last tag or in a commit range.
    * [x] Add the possibility to parse the last n Releases.
    * [x] Filter out excluded tags
    * [ ] Generate output regarding template (tags mapping and order)
    * [ ] Output formats:
        * [ ] Markdown
        * [ ] HTML
        * [ ] PDF

