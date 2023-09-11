//! ## Overview
//!
//! > Prior to v0.4.0 this crate was named [json-query].
//!
//! This rust crate provides access to [jq] 1.6 via the `libjq` C API (rather than
//! "shelling out").
//!
//! By leveraging [jq] we can extract data from json strings using `jq`'s dsl.
//!
//! This crate requires Rust **1.32** or above.
//!
//! ## Usage
//!
//! The interface provided by this crate is very basic. You supply a jq program
//! string and a string to run the program over.
//!
//! ```rust
//! use jq_rs;
//! // ...
//!
//! let res = jq_rs::run(".name", r#"{"name": "test"}"#);
//! assert_eq!(res.unwrap(), "\"test\"\n".to_string());
//! ```
//!
//! In addition to running one-off programs with `jq_rs::run()`, you can also
//! use `jq_rs::compile()` to compile a jq program and reuse it with
//! different inputs.
//!
//! ```rust
//! use jq_rs;
//!
//! let tv_shows = r#"[
//!     {"title": "Twilight Zone"},
//!     {"title": "X-Files"},
//!     {"title": "The Outer Limits"}
//! ]"#;
//!
//! let movies = r#"[
//!     {"title": "The Omen"},
//!     {"title": "Amityville Horror"},
//!     {"title": "The Thing"}
//! ]"#;
//!
//! let mut program = jq_rs::compile("[.[].title] | sort").unwrap();
//!
//! assert_eq!(
//!     &program.run(tv_shows).unwrap(),
//!     "[\"The Outer Limits\",\"Twilight Zone\",\"X-Files\"]\n"
//! );
//!
//! assert_eq!(
//!     &program.run(movies).unwrap(),
//!     "[\"Amityville Horror\",\"The Omen\",\"The Thing\"]\n",
//! );
//! ```
//!
//! ## A Note on Performance
//!
//! While the benchmarks are far from exhaustive, they indicate that much of the
//! runtime of a simple jq program goes to the compilation. In fact, the compilation
//! is _quite expensive_.
//!
//! ```text
//! run one off             time:   [48.594 ms 48.689 ms 48.800 ms]
//! Found 6 outliers among 100 measurements (6.00%)
//!   3 (3.00%) high mild
//!   3 (3.00%) high severe
//!
//! run pre-compiled        time:   [4.0351 us 4.0708 us 4.1223 us]
//! Found 15 outliers among 100 measurements (15.00%)
//!   6 (6.00%) high mild
//!   9 (9.00%) high severe
//! ```
//!
//! If you have a need to run the same jq program multiple times it is
//! _highly recommended_ to retain a pre-compiled `JqProgram` and reuse it.
//!
//! ## Handling Output
//!
//! The return values from jq are _strings_ since there is no certainty that the
//! output will be valid json. As such the output will need to be parsed if you want
//! to work with the actual data types being represented.
//!
//! In such cases you may want to pair this crate with [serde_json] or similar.
//!
//! For example, here we want to extract the numbers from a set of objects:
//!
//! ```rust
//! use jq_rs;
//! use serde_json::{self, json};
//!
//! // ...
//!
//! let data = json!({
//!     "movies": [
//!         { "title": "Coraline", "year": 2009 },
//!         { "title": "ParaNorman", "year": 2012 },
//!         { "title": "Boxtrolls", "year": 2014 },
//!         { "title": "Kubo and the Two Strings", "year": 2016 },
//!         { "title": "Missing Link", "year": 2019 }
//!     ]
//! });
//!
//! let query = "[.movies[].year]";
//! // program output as a json string...
//! let output = jq_rs::run(query, &data.to_string()).unwrap();
//! // ... parse via serde
//! let parsed: Vec<i64> = serde_json::from_str(&output).unwrap();
//!
//! assert_eq!(vec![2009, 2012, 2014, 2016, 2019], parsed);
//! ```
//!
//! Barely any of the options or flags available from the [jq] cli are exposed
//! currently.
//! Literally all that is provided is the ability to execute a _jq program_ on a blob
//! of json.
//! Please pardon my dust as I sort out the details.
//!
//! ## Linking to libjq
//!
//! This crate requires access to `libjq` at build and/or runtime depending on the
//! your choice.
//!
//! When the `bundled` feature is enabled (**off by default**) `libjq` is provided
//! and linked statically to your crate by [jq-sys] and [jq-src]. Using this feature
//! requires having autotools and gcc in `PATH` in order for the to build to work.
//!
//! Without the `bundled` feature, _you_ will need to ensure your crate
//! can link to `libjq` in order for the bindings to work.
//!
//! You can choose to compile `libjq` yourself, or perhaps install it via your
//! system's package manager.
//! See the [jq-sys building docs][jq-sys-building] for details on how to share
//! hints with the [jq-sys] crate on how to link.
//!
//! [jq]: https://github.com/stedolan/jq
//! [serde_json]: https://github.com/serde-rs/json
//! [jq-rs]: https://crates.io/crates/jq-rs
//! [json-query]: https://crates.io/crates/json-query
//! [jq-sys]: https://github.com/onelson/jq-sys
//! [jq-sys-building]: https://github.com/onelson/jq-sys#building
//! [jq-src]: https://github.com/onelson/jq-src

#![deny(missing_docs)]

extern crate jq_sys;
#[cfg(test)]
#[macro_use]
extern crate serde_json;

mod errors;
mod jq;

use std::ffi::CString;

pub use errors::{Error, Result};

/// Run a jq program on a blob of json data.
///
/// In the case of failure to run the program, feedback from the jq api will be
/// available in the supplied `String` value.
/// Failures can occur for a variety of reasons, but mostly you'll see them as
/// a result of bad jq program syntax, or invalid json data.
pub fn run(program: &str, data: &str) -> Result<String> {
    compile(program)?.run(data)
}

/// A pre-compiled jq program which can be run against different inputs.
pub struct JqProgram {
    jq: jq::Jq,
}

impl JqProgram {
    /// Runs a json string input against a pre-compiled jq program.
    pub fn run(&mut self, data: &str) -> Result<String> {
        if data.trim().is_empty() {
            // During work on #4, #7, the parser test which allows us to avoid a memory
            // error shows that an empty input just yields an empty response BUT our
            // implementation would yield a parse error.
            return Ok("".into());
        }
        let input = CString::new(data)?;
        self.jq.execute(input)
    }
}

/// Compile a jq program then reuse it, running several inputs against it.
pub fn compile(program: &str) -> Result<JqProgram> {
    let prog = CString::new(program)?;
    Ok(JqProgram {
        jq: jq::Jq::compile_program(prog)?,
    })
}

#[cfg(test)]
mod test {

    use super::{compile, run, Error};
    use matches::assert_matches;
    use serde_json;

    #[test]
    fn reuse_compiled_program() {
        let query = r#"if . == 0 then "zero" elif . == 1 then "one" else "many" end"#;
        let mut prog = compile(&query).unwrap();
        assert_eq!(prog.run("2").unwrap(), "\"many\"\n");
        assert_eq!(prog.run("1").unwrap(), "\"one\"\n");
        assert_eq!(prog.run("0").unwrap(), "\"zero\"\n");
    }

    #[test]
    fn jq_state_is_not_global() {
        let input = r#"{"id": 123, "name": "foo"}"#;
        let query1 = r#".name"#;
        let query2 = r#".id"#;

        // Basically this test is just to check that the state pointers returned by
        // `jq::init()` are completely independent and don't share any global state.
        let mut prog1 = compile(&query1).unwrap();
        let mut prog2 = compile(&query2).unwrap();

        assert_eq!(prog1.run(input).unwrap(), "\"foo\"\n");
        assert_eq!(prog2.run(input).unwrap(), "123\n");
        assert_eq!(prog1.run(input).unwrap(), "\"foo\"\n");
        assert_eq!(prog2.run(input).unwrap(), "123\n");
    }

    fn get_movies() -> serde_json::Value {
        json!({
            "movies": [
                { "title": "Coraline", "year": 2009 },
                { "title": "ParaNorman", "year": 2012 },
                { "title": "Boxtrolls", "year": 2014 },
                { "title": "Kubo and the Two Strings", "year": 2016 },
                { "title": "Missing Link", "year": 2019 }
            ]
        })
    }

    #[test]
    fn identity_nothing() {
        assert_eq!(run(".", "").unwrap(), "".to_string());
    }

    #[test]
    fn identity_empty() {
        assert_eq!(run(".", "{}").unwrap(), "{}\n".to_string());
    }

    #[test]
    fn extract_dates() {
        let data = get_movies();
        let query = "[.movies[].year]";
        let output = run(query, &data.to_string()).unwrap();
        let parsed: Vec<i64> = serde_json::from_str(&output).unwrap();
        assert_eq!(vec![2009, 2012, 2014, 2016, 2019], parsed);
    }

    #[test]
    fn extract_name() {
        let res = run(".name", r#"{"name": "test"}"#);
        assert_eq!(res.unwrap(), "\"test\"\n".to_string());
    }

    #[test]
    fn unpack_array() {
        let res = run(".[]", "[1,2,3]");
        assert_eq!(res.unwrap(), "1\n2\n3\n".to_string());
    }

    #[test]
    fn compile_error() {
        let res = run(". aa12312me  dsaafsdfsd", "{\"name\": \"test\"}");
        assert_matches!(res, Err(Error::InvalidProgram { .. }));
    }

    #[test]
    fn parse_error() {
        let res = run(".", "{1233 invalid json ahoy : est\"}");
        assert_matches!(res, Err(Error::System { .. }));
    }

    #[test]
    fn just_open_brace() {
        let res = run(".", "{");
        assert_matches!(res, Err(Error::System { .. }));
    }

    #[test]
    fn just_close_brace() {
        let res = run(".", "}");
        assert_matches!(res, Err(Error::System { .. }));
    }

    #[test]
    fn total_garbage() {
        let data = r#"
        {
            moreLike: "an object literal but also bad"
            loveToDangleComma: true,
        }"#;

        let res = run(".", data);
        assert_matches!(res, Err(Error::System { .. }));
    }

    pub mod mem_errors {
        //! Attempting run a program resulting in bad field access has been
        //! shown to sometimes trigger a use after free or double free memory
        //! error.
        //!
        //! Technically the program and inputs are both valid, but the
        //! evaluation of the program causes bad memory access to happen.
        //!
        //! https://github.com/onelson/json-query/issues/4

        use super::*;

        #[test]
        fn missing_field_access() {
            let prog = ".[] | .hello";
            let data = "[1,2,3]";
            let res = run(prog, data);
            assert_matches!(res, Err(Error::System { .. }));
        }

        #[test]
        fn missing_field_access_compiled() {
            let mut prog = compile(".[] | .hello").unwrap();
            let data = "[1,2,3]";
            let res = prog.run(data);
            assert_matches!(res, Err(Error::System { .. }));
        }
    }
}
