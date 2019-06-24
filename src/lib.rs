//! ## Overview
//!
//! [jq] is a command line tool which allows users to write small filter/transform
//! programs with a special DSL to extract data from json.
//!
//! This crate provides bindings to the C API internals to give us programmatic
//! access to this tool.
//!
//! For example, given a blob of json data, we can extract the values from
//! the `id` field of a series of objects.
//!
//! ```
//! let data = r#"{
//!     "colors": [
//!         {"id": 12, "name": "cyan"},
//!         {"id": 34, "name": "magenta"},
//!         {"id": 56, "name": "yellow"},
//!         {"id": 78, "name": "black"}
//!     ]
//! }"#;
//!
//! let output = json_query::run("[.colors[].id]", data).unwrap();
//! assert_eq!("[12,34,56,78]", &output);
//! ```
//!
//! For times where you want to run the same jq program against multiple inputs, `compile()`
//! returns a handle to the compiled jq program.
//!
//! ```
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
//! let mut program = json_query::compile("[.[].title] | sort").unwrap();
//!
//! assert_eq!(
//!     r#"["The Outer Limits","Twilight Zone","X-Files"]"#,
//!     &program.run(tv_shows).unwrap()
//! );
//!
//! assert_eq!(
//!     r#"["Amityville Horror","The Omen","The Thing"]"#,
//!     &program.run(movies).unwrap()
//! );
//! ```
//!
//! The output from these jq programs are returned as a string (just as is
//! the case if you were using [jq] from the command-line), so be prepared to
//! parse the output as needed after this step.
//!
//! Pairing this crate with something like [serde_json] might make a lot of
//! sense.
//!
//! See the [jq site][jq] for details on the jq program syntax.
//!
//! ## Linking to `libjq`
//!
//! When the `bundled` feature is enabled (on by default) `libjq` is provided and
//! linked statically by [jq-sys] and [jq-src]
//! which require having autotools and gcc in `PATH` to build.
//!
//! If you disable the `bundled` feature, you will need to ensure your crate
//! links to `libjq` in order for the bindings to work.
//! For this you may need to add a `build.rs` script if you don't have one already.
//!
//! [jq]: https://stedolan.github.io/jq/
//! [serde_json]: https://github.com/serde-rs/json
//! [jq-sys]: https://github.com/onelson/jq-sys
//! [jq-src]: https://github.com/onelson/jq-src
//!
extern crate jq_sys;
#[cfg(test)]
#[macro_use]
extern crate serde_json;

mod jq;

use jq::JqState;
use std::ffi::CString;

/// Run a jq program on a blob of json data.
///
/// In the case of failure to run the program, feedback from the jq api will be
/// available in the supplied `String` value.
/// Failures can occur for a variety of reasons, but mostly you'll see them as
/// a result of bad jq program syntax, or invalid json data.
pub fn run(program: &str, data: &str) -> Result<String, String> {
    let mut state = jq::init();
    let buf = CString::new(data).map_err(|_| "unable to convert data to c string.".to_string())?;
    let prog =
        CString::new(program).map_err(|_| "unable to convert data to c string.".to_string())?;

    jq::compile_program(&mut state, prog)?;
    let res = jq::load_string(&mut state, buf);
    jq::teardown(&mut state);
    res
}

/// A pre-compiled jq program which can be run against different inputs.
pub struct JqProgram {
    state: *mut JqState,
}

impl JqProgram {
    /// Runs a json string input against a pre-compiled jq program.
    pub fn run(&mut self, data: &str) -> Result<String, String> {
        let buf =
            CString::new(data).map_err(|_| "unable to convert data to c string.".to_string())?;
        let res = jq::load_string(&mut self.state, buf);
        res
    }
}

impl Drop for JqProgram {
    fn drop(&mut self) {
        jq::teardown(&mut self.state);
    }
}

/// Compile a jq program then reuse it, running several inputs against it.
pub fn compile(program: &str) -> Result<JqProgram, String> {
    let mut state = jq::init();
    let prog =
        CString::new(program).map_err(|_| "unable to convert data to c string.".to_string())?;

    jq::compile_program(&mut state, prog)?;
    Ok(JqProgram { state })
}

#[cfg(test)]
mod test {
    use super::{compile, run};
    use serde_json;

    #[test]
    fn reuse_compiled_program() {
        let query = r#"if . == 0 then "zero" elif . == 1 then "one" else "many" end"#;
        let mut prog = compile(&query).unwrap();
        assert_eq!(prog.run("2").unwrap(), r#""many""#);
        assert_eq!(prog.run("1").unwrap(), r#""one""#);
        assert_eq!(prog.run("0").unwrap(), r#""zero""#);
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

        assert_eq!(prog1.run(input).unwrap(), r#""foo""#);
        assert_eq!(prog2.run(input).unwrap(), r#"123"#);
        assert_eq!(prog1.run(input).unwrap(), r#""foo""#);
        assert_eq!(prog2.run(input).unwrap(), r#"123"#);
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
        assert_eq!(run(".", ""), Ok("".to_string()));
    }

    #[test]
    fn identity_empty() {
        assert_eq!(run(".", "{}"), Ok("{}".to_string()));
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
        assert_eq!(res, Ok(r#""test""#.to_string()));
    }

    #[test]
    fn unpack_array() {
        let res = run(".[]", "[1,2,3]");
        assert_eq!(res, Ok("1\n2\n3".to_string()));
    }

    #[test]
    fn compile_error() {
        let res = run(". aa12312me  dsaafsdfsd", "{\"name\": \"test\"}");
        assert!(res.is_err());
    }

    #[test]
    fn parse_error() {
        let res = run(".", "{1233 invalid json ahoy : est\"}");
        assert!(res.is_err());
    }

    #[test]
    fn just_open_brace() {
        let res = run(".", "{");
        assert!(res.is_err());
    }

    #[test]
    fn just_close_brace() {
        let res = run(".", "}");
        assert!(res.is_err());
    }

    #[test]
    fn total_garbage() {
        let data = r#"
        {
            moreLike: "an object literal but also bad"
            loveToDangleComma: true,
        }"#;

        let res = run(".", data);
        assert!(res.is_err());
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
            assert!(run(prog, data).is_err());
        }

        #[test]
        #[ignore]
        fn missing_field_access_compiled() {
            let mut prog = compile(".[] | .hello").unwrap();
            let data = "[1,2,3]";
            assert!(prog.run(data).is_err());
        }
    }
}
