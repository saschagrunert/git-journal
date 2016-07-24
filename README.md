# git-journal
The Git Commit Message Framework

## Targets
Target of the project is to provide a Python based framework to write more sensible commit messages. Single commit messages should contain one logical change of the project which is described in a standardized way. This results in a much cleaner git history and provides contributors more information about the actual change.

To gain very clean commit message history it is neccesary to use git rebase, squashed and amended commits. git-journal will simplify these development approaches by providing sensible conventions and strong defaults.

## Features
The base project will be developed as a python based git extension which can be installed via pip. These features are planned for the initial release:

* Commit Message templating and validation
    * Provide a meaningful template for a commit message
    * Validate the commit message via git hooks
* TODO list support within commit messages
    * Keep track of the feature directly in the commit message
* Changelog generation with filtering support
    * Support for Changelogs containing distinct sections
    * Create Changelogs between git tags automatically
    * Filter out selected commits (e.g. merges)
    * Support multiple output formats

## Development progress
The development has not yet started.
