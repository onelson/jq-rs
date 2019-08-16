//! This module takes the unsafe bindings from `jq-sys` then (hopefully)
//! wrapping to present a slightly safer API to use.
//!
//! These are building blocks and not intended for use from the public API.

use crate::errors::{Error, Result};
use jq_sys::{
    jq_compile, jq_get_exit_code, jq_halted, jq_init, jq_next, jq_start, jq_state, jq_teardown, jv,
    jv_copy, jv_dump_string, jv_free, jv_get_kind, jv_invalid_get_msg, jv_invalid_has_msg,
    jv_kind_JV_KIND_INVALID, jv_kind_JV_KIND_NUMBER, jv_kind_JV_KIND_STRING, jv_number_value,
    jv_parser, jv_parser_free, jv_parser_new, jv_parser_next, jv_parser_set_buf, jv_string_value,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

pub struct Jq {
    state: *mut jq_state,
}

impl Jq {
    pub fn compile_program(program: CString) -> Result<Self> {
        let jq = Jq {
            state: {
                // jq's master branch shows this can be a null pointer, in
                // which case the binary will exit with a `Error::System`.
                let ptr = unsafe { jq_init() };
                if ptr.is_null() {
                    return Err(Error::System {
                        reason: Some("Failed to init".into()),
                    });
                } else {
                    ptr
                }
            },
        };

        if unsafe { jq_compile(jq.state, program.as_ptr()) } == 0 {
            Err(Error::InvalidProgram)
        } else {
            Ok(jq)
        }
    }

    fn is_halted(&self) -> bool {
        unsafe { jq_halted(self.state) != 0 }
    }

    fn get_exit_code(&self) -> ExitCode {
        let exit_code = JV {
            ptr: unsafe { jq_get_exit_code(self.state) },
        };

        // The rules for this seem odd, but I'm trying to model this after the
        // similar block in the jq `main.c`s `process()` function.

        if exit_code.is_valid() {
            ExitCode::JQ_OK
        } else {
            exit_code
                .as_number()
                .map(|i| (i as isize).into())
                .unwrap_or(ExitCode::JQ_ERROR_UNKNOWN)
        }
    }

    /// Run the jq program against an input.
    pub fn execute(&mut self, input: CString) -> Result<String> {
        let mut parser = Parser::new();
        self.process(parser.parse(input)?)
    }

    /// Unwind the parser and return the rendered result.
    ///
    /// When this results in `Err`, the String value should contain a message about
    /// what failed.
    fn process(&mut self, initial_value: JV) -> Result<String> {
        let mut buf = String::new();

        unsafe {
            // `jq_start` seems to be a consuming call.
            // In order to avoid a double-free, when `initial_value` is dropped,
            // we have to use `jv_copy` on the inner `jv`.
            jq_start(self.state, jv_copy(initial_value.ptr), 0);
            // After, we can manually free the `initial_value` with `drop` since
            // it is no longer needed.
            drop(initial_value);

            dump(self, &mut buf)?;
        }

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
    pub fn as_dump_string(&self) -> Result<String> {
        let dump = JV {
            ptr: unsafe { jv_dump_string(jv_copy(self.ptr), 0) },
        };
        unsafe { get_string_value(jv_string_value(dump.ptr)) }
    }

    /// Attempts to extract feedback from jq if the JV is invalid.
    pub fn get_msg(&self) -> Option<String> {
        if self.invalid_has_msg() {
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

                format!(
                    "JQ: Parse error: {}",
                    msg.as_string().unwrap_or_else(|_| "unknown".into())
                )
            };
            Some(reason)
        } else {
            None
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        unsafe {
            if jv_get_kind(self.ptr) == jv_kind_JV_KIND_NUMBER {
                Some(jv_number_value(self.ptr))
            } else {
                None
            }
        }
    }

    pub fn as_string(&self) -> Result<String> {
        unsafe {
            if jv_get_kind(self.ptr) == jv_kind_JV_KIND_STRING {
                get_string_value(jv_string_value(self.ptr))
            } else {
                Err(Error::Unknown)
            }
        }
    }

    pub fn is_valid(&self) -> bool {
        unsafe { jv_get_kind(self.ptr) != jv_kind_JV_KIND_INVALID }
    }

    pub fn invalid_has_msg(&self) -> bool {
        unsafe { jv_invalid_has_msg(jv_copy(self.ptr)) == 1 }
    }
}

impl Drop for JV {
    fn drop(&mut self) {
        unsafe { jv_free(self.ptr) };
    }
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

    pub fn parse(&mut self, input: CString) -> Result<JV> {
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
        if value.is_valid() {
            Ok(value)
        } else {
            Err(Error::System {
                reason: Some(
                    value
                        .get_msg()
                        .unwrap_or_else(|| "JQ: Parser error".to_string()),
                ),
            })
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
unsafe fn get_string_value(value: *const c_char) -> Result<String> {
    let s = CStr::from_ptr(value).to_str()?;
    Ok(s.to_owned())
}

/// Renders the data from the parser and pushes it into the buffer.
unsafe fn dump(jq: &Jq, buf: &mut String) -> Result<()> {
    // Looks a lot like an iterator...

    let mut value = JV {
        ptr: jq_next(jq.state),
    };

    while value.is_valid() {
        let s = value.as_dump_string()?;
        buf.push_str(&s);
        buf.push('\n');

        value = JV {
            ptr: jq_next(jq.state),
        };
    }

    if jq.is_halted() {
        use ExitCode::*;
        match jq.get_exit_code() {
            JQ_ERROR_SYSTEM => Err(Error::System {
                reason: value.get_msg(),
            }),
            // As far as I know, we should not be able to see a compile error
            // this deep into the execution of a jq program (it would need to be
            // compiled already, right?)
            // Still, compile failure is represented by an exit code, so in
            // order to be exhaustive we have to check for it.
            JQ_ERROR_COMPILE => Err(Error::InvalidProgram),
            // Any of these `OK_` variants are "success" cases.
            // I suppose the jq program can halt successfully, or not, or not at
            // all and still terminate some other way?
            JQ_OK | JQ_OK_NULL_KIND | JQ_OK_NO_OUTPUT => Ok(()),
            JQ_ERROR_UNKNOWN => Err(Error::Unknown),
        }
    } else if let Some(reason) = value.get_msg() {
        Err(Error::System {
            reason: Some(reason),
        })
    } else {
        Ok(())
    }
}

/// Various exit codes jq checks for during the `if (jq_halted(jq))` branch of
/// their processing loop.
///
/// Adapted from the enum seen in jq's master branch right now.
/// The numbers seem to line up with the magic numbers seen in
/// the 1.6 release, though there's no enum that I saw at that point in the git
/// history.
#[allow(non_camel_case_types, dead_code)]
enum ExitCode {
    JQ_OK = 0,
    JQ_OK_NULL_KIND = -1,
    JQ_ERROR_SYSTEM = 2,
    JQ_ERROR_COMPILE = 3,
    JQ_OK_NO_OUTPUT = -4,
    JQ_ERROR_UNKNOWN = 5,
}

impl From<isize> for ExitCode {
    fn from(number: isize) -> Self {
        use ExitCode::*;
        match number {
            0 => JQ_OK,
            -1 => JQ_OK_NULL_KIND,
            2 => JQ_ERROR_SYSTEM,
            3 => JQ_ERROR_COMPILE,
            -4 => JQ_OK_NO_OUTPUT,
            // `5` is called out explicitly in the jq source, but also "unknown"
            // seems to make good sense for other unexpected number.
            5 | _ => JQ_ERROR_UNKNOWN,
        }
    }
}
