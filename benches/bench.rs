#![feature(test)]
extern crate test;

use gitjournal::GitJournal;
use test::Bencher;

#[bench]
fn verify_huge_message(b: &mut Bencher) {
    let journal = GitJournal::new(".").unwrap();
    b.iter(|| {
        journal
            .verify("./tests/commit_messages/success_huge")
            .is_ok();
    });
}

#[bench]
fn parse(b: &mut Bencher) {
    let mut journal = GitJournal::new(".").unwrap();
    journal.config.enable_debug = false;
    b.iter(|| {
        journal
            .parse_log("HEAD", "rc", 0, true, false, None)
            .is_ok();
    });
}
