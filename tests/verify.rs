extern crate git_journal;

use git_journal::GitJournal;

#[test]
fn commit_msg_summary_success_1() {
    GitJournal::verify("./tests/commit_messages/success_1").unwrap();
}

#[test]
fn commit_msg_summary_success_2() {
    GitJournal::verify("./tests/commit_messages/success_2").unwrap();
}

#[test]
fn commit_msg_summary_success_3() {
    GitJournal::verify("./tests/commit_messages/success_3").unwrap();
}

#[test]
fn commit_msg_summary_success_4() {
    GitJournal::verify("./tests/commit_messages/success_4").unwrap();
}

#[test]
#[should_panic]
fn commit_msg_summary_failure_1() {
    GitJournal::verify("./tests/commit_messages/failure_1").unwrap();
}

#[test]
#[should_panic]
fn commit_msg_summary_failure_2() {
    GitJournal::verify("./tests/commit_messages/failure_2").unwrap();
}

#[test]
#[should_panic]
fn commit_msg_summary_failure_3() {
    GitJournal::verify("./tests/commit_messages/failure_3").unwrap();
}

#[test]
#[should_panic]
fn commit_msg_paragraph_failure_1() {
    GitJournal::verify("./tests/commit_messages/failure_4").unwrap();
}

#[test]
#[should_panic]
fn commit_msg_paragraph_failure_2() {
    GitJournal::verify("./tests/commit_messages/failure_5").unwrap();
}

#[test]
#[should_panic]
fn commit_msg_paragraph_failure_3() {
    GitJournal::verify("./tests/commit_messages/failure_6").unwrap();
}

