extern crate jq_rs;
#[macro_use]
extern crate error_chain;

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain! {}
}

use errors::*;

#[test]
fn test_error_chain_compat() {
    assert_eq!(
        jq_rs::run(".", "[[[{}}")
            .chain_err(|| "custom error message")
            .map_err(|e| format!("{}", e)),
        Err("custom error message".to_string())
    );
}
