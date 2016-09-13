#![feature(test)]
extern crate test;
extern crate gitjournal;

use test::Bencher;
use gitjournal::GitJournal;

#[bench]
fn verify_huge_message(b: &mut Bencher) {
    let journal = GitJournal::new(".").unwrap();
    b.iter(|| {
        assert!(journal.verify("./tests/commit_messages/success_huge").is_ok());
    });
}

#[bench]
fn parse(b: &mut Bencher) {
    let mut journal = GitJournal::new(".").unwrap();
    journal.config.enable_debug = false;
    b.iter(|| {
        assert!(journal.parse_log("HEAD", "rc", &0, &true, &false).is_ok());
    });
}
