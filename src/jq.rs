//! This module takes the unsafe bindings from `jq-sys` then (hopefully)
//! wrapping to present a slightly safer API to use.
//!
//! These are building blocks and not intended for use from the public API.

use jq_sys::{
    self, jq_next, jv_copy, jv_dump_string, jv_invalid_get_msg, jv_parser_next, jv_string_value,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

pub type JqValue = ::jq_sys::jv;
pub type JqState = ::jq_sys::jq_state;

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
            Ok(s) => {
                buf.push_str(&s);
                buf.push('\n')
            }
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

    // remove last trailing newline
    let len = buf.trim_end().len();
    buf.truncate(len);

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
