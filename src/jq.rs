//! This module takes the unsafe bindings from `jq-sys` then (hopefully)
//! wrapping to present a slightly safer API to use.
//!
//! These are building blocks and not intended for use from the public API.

// Yeah, it's a lot.
use jq_sys::{
    jq_compile, jq_halted, jq_init, jq_next, jq_start, jq_state, jq_teardown, jv, jv_copy,
    jv_dump_string, jv_free, jv_get_kind, jv_invalid_get_msg, jv_invalid_has_msg,
    jv_kind_JV_KIND_INVALID, jv_parser, jv_parser_free, jv_parser_new, jv_parser_next,
    jv_parser_set_buf, jv_string_value,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

pub struct Jq {
    state: *mut jq_state,
}

impl Jq {
    pub fn compile_program(program: CString) -> Result<Self, String> {
        let jq = Jq {
            state: unsafe { jq_init() },
        };
        unsafe {
            if jq_compile(jq.state, program.as_ptr()) == 0 {
                Err("syntax error: JQ Program failed to compile.".into())
            } else {
                Ok(jq)
            }
        }
    }

    fn is_halted(&self) -> bool {
        unsafe { jq_halted(self.state) != 0 }
    }

    /// Evaluate the program against an input.
    pub fn load_string(&mut self, input: CString) -> Result<String, String> {
        let mut parser = Parser::new();
        if self.is_halted() {
            Err("halted".into())
        } else {
            self.process(parser.parse(input)?)
        }
    }

    /// Unwind the parser and return the rendered result.
    ///
    /// When this results in `Err`, the String value should contain a message about
    /// what failed.
    fn process(&mut self, initial_value: JV) -> Result<String, String> {
        let mut buf = String::new();

        unsafe {
            jq_start(self.state, initial_value.ptr, 0);
        }
        if let Err(reason) = unsafe { dump(self, &mut buf) } {
            return Err(reason);
        }
        // remove last trailing newline
        let len = buf.trim_end().len();
        buf.truncate(len);

        Ok(buf)
    }
}

impl Drop for Jq {
    fn drop(&mut self) {
        unsafe { jq_teardown(&mut self.state) }
    }
}

struct JV {
    ptr: jv,
}

impl JV {
    /// Convert the current `JV` into the "dump string" rendering of itself.
    pub fn dump_string(&self) -> Result<String, std::str::Utf8Error> {
        let dump = JV {
            ptr: unsafe { jv_dump_string(self.ptr, 0) },
        };
        unsafe { get_string_value(jv_string_value(dump.ptr)) }
    }

    /// Attempts to extract feedback from jq if the JV is invalid.
    pub fn get_msg(&self) -> Option<String> {
        if invalid_has_msg(&self) {
            let reason = {
                let msg = JV {
                    ptr: unsafe {
                        // This call is gross since we're dipping outside of the
                        // safe/drop-enabled wrapper to get a copy which will be freed
                        // by jq. If we wrap it in a `JV`, we'll run into a double-free
                        // situation.
                        jv_invalid_get_msg(jv_copy(self.ptr))
                    },
                };

                let s = unsafe { get_string_value(jv_string_value(msg.ptr)) };

                format!("Parse error: {}", s.unwrap_or_else(|_| "unknown".into()))
            };
            Some(reason)
        } else {
            None
        }
    }
}

impl Drop for JV {
    fn drop(&mut self) {
        unsafe { jv_free(self.ptr) };
    }
}

fn value_is_valid(value: &JV) -> bool {
    unsafe {
        // FIXME: looks like this copy should not be needed (but it is?)
        //   Test suite shows a memory error if this value is freed after being passed to
        //   `jv_get_kind()`, so I guess this is a consuming call.
        let x = jv_copy(value.ptr);
        jv_get_kind(x) != jv_kind_JV_KIND_INVALID
    }
}

fn invalid_has_msg(value: &JV) -> bool {
    // XXX: the C lib suggests the jv passed in here will eventually be freed.
    //   I had a a `jv_copy()` to side-step this, but removing it removes one
    //   leak warning in valgrind, so I don't know what the deal is.
    unsafe { jv_invalid_has_msg(value.ptr) == 1 }
}

struct Parser {
    ptr: *mut jv_parser,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            ptr: unsafe { jv_parser_new(0) },
        }
    }

    pub fn parse(&mut self, input: CString) -> Result<JV, String> {
        // For a single run, we could set this to `1` (aka `true`) but this will
        // break the repeated `JqProgram` usage.
        // It may be worth exposing this to the caller so they can set it for each
        // use case, but for now we'll just "leave it open."
        let is_last = 0;

        // Originally I planned to have a separate "set_buf" method, but it looks like
        // the C api really wants you to set the buffer, then call `jv_parser_next()` in
        // the same logical block.
        // Mainly I think the important thing is to ensure the `input` outlives both the
        // set_buf and next calls.
        unsafe {
            jv_parser_set_buf(
                self.ptr,
                input.as_ptr(),
                input.as_bytes().len() as i32,
                is_last,
            )
        };

        let value = JV {
            ptr: unsafe { jv_parser_next(self.ptr) },
        };
        if value_is_valid(&value) {
            Ok(value)
        } else {
            Err(value
                .get_msg()
                .unwrap_or_else(|| "Parser error".to_string()))
        }
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        unsafe {
            jv_parser_free(self.ptr);
        }
    }
}

/// Takes a pointer to a nul term string, and attempts to convert it to a String.
unsafe fn get_string_value(value: *const c_char) -> Result<String, std::str::Utf8Error> {
    let s = CStr::from_ptr(value).to_str()?;
    Ok(s.to_owned())
}

/// Renders the data from the parser and pushes it into the buffer.
unsafe fn dump(jq: &Jq, buf: &mut String) -> Result<(), String> {
    let mut value = JV {
        ptr: jq_next(jq.state),
    };

    while value_is_valid(&value) {
        match value.dump_string() {
            Ok(s) => {
                buf.push_str(&s);
                buf.push('\n');
            }
            Err(e) => {
                return Err(format!("String Decode error: {}", e));
            }
        };

        value = JV {
            ptr: jq_next(jq.state),
        };
    }

    // formerly extracted as `check()`.
    if jq.is_halted() {
        return Err("halted".into());
    } else if let Some(reason) = value.get_msg() {
        return Err(reason);
    } else {
        return Ok(());
    }
}
