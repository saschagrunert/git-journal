#[macro_use]
extern crate clap;
extern crate git2;

use git2::{Error, ObjectType, Oid, Repository};
use clap::App;

struct GitJournal {
    repo: Repository,
    tags: Vec<(Oid, String)>,
}

impl GitJournal {
    fn new(path: &str) -> Result<GitJournal, Error> {
        // Open the repository
        let new_repo = try!(Repository::open(path));

        // Get all available tags
        let mut new_tags = vec![];
        for name in try!(new_repo.tag_names(None)).iter() {
            let name = try!(name.ok_or(Error::from_str("Could not receive tag name")));
            let obj = try!(new_repo.revparse_single(name));
            if let Ok(tag) = obj.into_tag() {
                let tag_name = try!(tag.name().ok_or(Error::from_str("Could not parse tag name"))).to_owned();
                new_tags.push((tag.target_id(), tag_name));
            }
        }
        Ok(GitJournal {
            repo: new_repo,
            tags: new_tags,
        })
    }

    fn parse_log(&self, revision_range: &str, tag_skip_pattern: &str) -> Result<(), Error> {
        let mut revwalk = try!(self.repo.revwalk());
        let mut stop_at_first_tag = false;
        revwalk.set_sorting(git2::SORT_TIME);

        // Fill the revwalk with the selected revisions.
        let revspec = try!(self.repo.revparse(&revision_range));
        if revspec.mode().contains(git2::REVPARSE_SINGLE) {
            // A single commit was given
            let from = try!(revspec.from().ok_or(Error::from_str("Could not set revision range start")));
            try!(revwalk.push(from.id()));
            stop_at_first_tag = true;
        } else {
            // A specific commit range was given
            let from = try!(revspec.from().ok_or(Error::from_str("Could not set revision range start")));
            let to = try!(revspec.to().ok_or(Error::from_str("Could not set revision range end")));
            try!(revwalk.push(to.id()));
            if revspec.mode().contains(git2::REVPARSE_MERGE_BASE) {
                let base = try!(self.repo.merge_base(from.id(), to.id()));
                let o = try!(self.repo.find_object(base, Some(ObjectType::Commit)));
                try!(revwalk.push(o.id()));
            }
            try!(revwalk.hide(from.id()));
        }

        // Iterate over the git objects and process them.
        'revloop: for (index, id) in revwalk.enumerate() {
            let oid = try!(id);
            let mut commit = try!(self.repo.find_commit(oid));
            for tag in self.tags
                .iter()
                .filter(|tag| tag.0.as_bytes() == oid.as_bytes() && !tag.1.contains(tag_skip_pattern)) {
                println!("TAG: {} ({})", tag.1, tag.0);
                if stop_at_first_tag && index > 0 {
                    break 'revloop;
                }
            }
            println!("{}\t{}: {}", index, oid, commit.summary().unwrap());
        }
        Ok(())
    }
}

fn main() {
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from_yaml(yaml).get_matches();

    let path = matches.value_of("path").expect("Could not parse 'path' parameter");
    let revision_range = matches.value_of("revision_range").expect("Could not parse 'revision range' parameter");
    let tag_skip_pattern = matches.value_of("tag_skip_pattern").expect("Could not parse 'skip pattern' parameter");

    match GitJournal::new(path) {
        Ok(journal) => journal.parse_log(revision_range, tag_skip_pattern).expect("Log parsing error"),
        Err(e) => println!("{}", e),
    }
}
