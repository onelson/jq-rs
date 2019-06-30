//! This module takes the unsafe bindings from `jq-sys` then (hopefully)
//! wrapping to present a slightly safer API to use.
//!
//! These are building blocks and not intended for use from the public API.

use jq::jv::value_is_valid;
use jq_sys::{self, jq_next, jv_copy, jv_dump_string, jv_invalid_get_msg, jv_string_value};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

pub type JqValue = ::jq_sys::jv;
pub type JqState = ::jq_sys::jq_state;

mod jv {
    use super::JqValue;
    use jq_sys::{jv_get_kind, jv_invalid_has_msg, jv_kind_JV_KIND_INVALID};

    pub fn value_is_valid(value: JqValue) -> bool {
        unsafe { jv_get_kind(value) != jv_kind_JV_KIND_INVALID }
    }

    // Will eventually free the value it receives.
    // You will may want to `jv_copy()` before handing your value in.
    pub fn invalid_has_msg(value: JqValue) -> bool {
        unsafe { jv_invalid_has_msg(value) == 1 }
    }
}

pub fn is_halted(state: *mut *mut JqState) -> bool {
    unsafe { jq_sys::jq_halted(*state) != 0 }
}

pub fn init() -> *mut JqState {
    unsafe { jq_sys::jq_init() }
}

pub fn teardown(state: *mut *mut JqState) {
    unsafe { jq_sys::jq_teardown(state) }
}

pub fn load_string(state: *mut *mut JqState, input: CString) -> Result<String, String> {
    let len = input.as_bytes().len() as i32;
    let ptr = input.as_ptr();

    // For a single run, we could set this to `1` (aka `true`) but this will
    // break the repeated `JqProgram` usage.
    // It may be worth exposing this to the caller so they can set it for each
    // use case, but for now we'll just "leave it open."
    let is_last = 0;

    unsafe {
        let parser = jq_sys::jv_parser_new(0);
        jq_sys::jv_parser_set_buf(parser, ptr, len, is_last);

        // the parser produces the initial value to process?
        let value = jq_sys::jv_parser_next(parser);

        let ret = if value_is_valid(value) {
            process(state, value)
        } else if is_halted(state) {
            Err("halted".to_string())
        } else {
            Err("invalid parser next".to_string())
        };
        jq_sys::jv_parser_free(parser);
        ret
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
unsafe fn check(state: *mut *mut JqState, value: JqValue) -> Result<(), String> {
    let ret = if is_halted(state) {
        jq_sys::jv_free(value);
        Err("halted".into())
    } else if jv::invalid_has_msg(jv_copy(value)) {
        // `value` is consumed here, converted into `msg`
        let msg = jv_invalid_get_msg(value);
        let reason = format!(
            "parse error: {}",
            get_string_value(jv_string_value(msg)).unwrap_or_else(|_| "unknown".into())
        );
        let ret = Err(reason);
        jq_sys::jv_free(msg);
        ret
    } else {
        jq_sys::jv_free(value);
        Ok(())
    };
    ret
}

/// Renders the data from the parser and pushes it into the buffer.
unsafe fn dump(state: *mut *mut JqState, buf: &mut String) -> Result<(), String> {
    let mut value = jq_next(*state);

    while jv::value_is_valid(value) {
        let dumped = jv_dump_string(value, 0);
        match get_string_value(jv_string_value(dumped)) {
            Ok(s) => {
                buf.push_str(&s);
                buf.push('\n');
                jq_sys::jv_free(dumped);
            }
            Err(e) => {
                jq_sys::jv_free(dumped);
                jq_sys::jv_free(value);
                return Err(format!("parse error: {}", e));
            }
        };

        value = jq_next(*state);
    }

    check(state, value)
}

/// Unwind the parser and return the rendered result.
///
/// When this results in `Err`, the String value should contain a message about
/// what failed.
unsafe fn process(state: *mut *mut JqState, initial_value: JqValue) -> Result<String, String> {
    let mut buf = String::new();
    jq_sys::jq_start(*state, initial_value, 0);
    if let Err(reason) = dump(state, &mut buf) {
        return Err(reason);
    }
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
