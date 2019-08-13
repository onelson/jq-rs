extern crate jq_rs;
#[macro_use]
extern crate error_chain;
use error_chain::ChainedError;

mod errors {
    error_chain! {
        foreign_links {
            Jq(jq_rs::Error);
        }
    }
}

use self::errors::{Error, ErrorKind, ResultExt};

#[test]
fn test_match_errorkind() {
    match jq_rs::run(".", "[[[{}}").unwrap_err().into() {
        Error(ErrorKind::Jq(e), _s) => {
            // Proving that jq_rs::Error does in fact implement `std::error::Error`.
            // Sort of redundant considering error_chain requires this, but it
            // can't hurt to say it explicitly here.
            use std::error::Error as _;
            let _ = e.source();
        }
        _ => unreachable!("error-chain should be converting."),
    }
}

#[test]
fn test_chain_err() {
    let chain = jq_rs::run(".", "[[[{}}")
        .chain_err(|| "custom message")
        .unwrap_err()
        .display_chain()
        .to_string();

    // the chain is a multi-line string mentioning each error in the chain.
    assert!(chain.contains("custom message"));
    assert!(chain.contains("Parse error"))
}
