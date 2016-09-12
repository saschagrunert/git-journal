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
