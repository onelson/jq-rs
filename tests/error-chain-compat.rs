extern crate jq_rs;
#[macro_use]
extern crate error_chain;

mod errors {
    error_chain! {
        foreign_links {
            Jq(jq_rs::Error);
        }
    }
}

use self::errors::{Error, ErrorKind};

#[test]
fn test_error_chain_compat() {
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
