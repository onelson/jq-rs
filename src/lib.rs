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
use std::ffi::CString;

mod jq {
    use jq_sys::{
        self, jq_next, jv_copy, jv_dump_string, jv_invalid_get_msg, jv_parser_next, jv_string_value,
    };
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    pub type JqValue = jq_sys::jv;
    pub type JqState = jq_sys::jq_state;

    mod jv {
        use super::JqValue;
        use jq_sys;

        pub fn value_is_valid(value: JqValue) -> bool {
            unsafe { jq_sys::jv_get_kind(value) != jq_sys::jv_kind_JV_KIND_INVALID }
        }

        pub fn invalid_has_msg(value: JqValue) -> bool {
            unsafe { jq_sys::jv_invalid_has_msg(value) == 1 }
        }
    }

    pub fn init() -> *mut JqState {
        unsafe { jq_sys::jq_init() }
    }

    pub fn teardown(state: *mut *mut JqState) {
        unsafe { jq_sys::jq_teardown(state) }
    }

    pub fn load_string(state: *mut *mut JqState, buf: CString) -> Result<String, String> {
        unsafe {
            let parser = jq_sys::jv_parser_new(0);
            let len = buf.as_bytes().len() as i32;
            jq_sys::jv_parser_set_buf(parser, buf.as_ptr(), len, 0);
            let res = parse(state, parser);
            jq_sys::jv_parser_free(parser);
            res
        }
    }

    /// Takes a pointer to a nul term string, and attempts to convert it to a String.
    unsafe fn get_string_value(value: *const c_char) -> Result<String, ::std::str::Utf8Error> {
        let s = CStr::from_ptr(value).to_str()?;
        Ok(s.to_owned())
    }

    /// Frees a jv and extracts any error info as it does so.
    ///
    /// `Err` with the reason if the thing was trash.
    unsafe fn check(value: JqValue) -> Result<(), String> {
        if jv::invalid_has_msg(jv_copy(value)) {
            let msg = jv_invalid_get_msg(value);
            let reason = format!(
                "parse error: {}",
                get_string_value(jv_string_value(msg)).unwrap_or_else(|_| "unknown".into())
            );
            let ret = Err(reason);
            jq_sys::jv_free(msg);
            return ret;
        } else {
            jq_sys::jv_free(value);
        }
        Ok(())
    }

    /// Renders the data from the parser and pushes it into the buffer.
    unsafe fn dump(
        state: *mut *mut JqState,
        initial_value: JqValue,
        buf: &mut String,
    ) -> Result<(), String> {
        let mut value = initial_value;
        while jv::value_is_valid(value) {
            let dumped = jv_dump_string(value, 0);
            match get_string_value(jv_string_value(dumped)) {
                Ok(s) => buf.push_str(&s),
                Err(e) => return Err(format!("parse error: {}", e)),
            };
            value = jq_next(*state);
        }
        check(value)
    }

    /// Unwind the parser and return the rendered result.
    ///
    /// When this results in `Err`, the String value should contain a message about
    /// what failed.
    unsafe fn parse(
        state: *mut *mut JqState,
        parser: *mut jq_sys::jv_parser,
    ) -> Result<String, String> {
        let mut buf = String::new();
        let mut value = jv_parser_next(parser);
        while jv::value_is_valid(value) {
            jq_sys::jq_start(*state, value, 0);
            if let Err(reason) = dump(state, jq_next(*state), &mut buf) {
                // outer loop item needs freeing during early return
                jq_sys::jv_free(value);
                return Err(reason);
            }
            value = jv_parser_next(parser);
        }
        check(value)?;
        Ok(buf)
    }

    pub fn compile_program(state: *mut *mut JqState, program: CString) -> Result<(), String> {
        unsafe {
            if jq_sys::jq_compile(*state, program.as_ptr()) == 0 {
                Err("syntax error: JQ Program failed to compile.".into())
            } else {
                Ok(())
            }
        }
    }
}


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

#[cfg(test)]
mod test {
    use super::run;
    use serde_json::{self, json};

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
}
