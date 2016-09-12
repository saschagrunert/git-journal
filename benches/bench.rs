#![feature(test)]
extern crate test;
extern crate gitjournal;

use test::Bencher;
use gitjournal::GitJournal;

#[bench]
fn verify_huge_message(b: &mut Bencher) {
    b.iter(|| {
        GitJournal::verify("./tests/success_huge");
    });
}

#[bench]
fn parse(b: &mut Bencher) {
    let mut journal = GitJournal::new("./tests/test_repo").unwrap();
    b.iter(|| {
        assert!(journal.parse_log("HEAD", "rc", &0, &true, &false).is_ok());
    });
}
