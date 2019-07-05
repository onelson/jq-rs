//! This module takes the unsafe bindings from `jq-sys` then (hopefully)
//! wrapping to present a slightly safer API to use.
//!
//! These are building blocks and not intended for use from the public API.

use super::Error;
// Yeah, it's a lot.
use jq_sys::{
    jq_compile, jq_get_exit_code, jq_halted, jq_init, jq_next, jq_start, jq_state, jq_teardown, jv,
    jv_copy, jv_dump_string, jv_free, jv_get_kind, jv_invalid_get_msg, jv_invalid_has_msg,
    jv_kind_JV_KIND_INVALID, jv_kind_JV_KIND_NUMBER, jv_number_value, jv_parser, jv_parser_free,
    jv_parser_new, jv_parser_next, jv_parser_set_buf, jv_string_value,
};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

pub struct Jq {
    state: *mut jq_state,
}

impl Jq {
    pub fn compile_program(program: CString) -> Result<Self, Error> {
        let jq = Jq {
            state: unsafe {
                // jq's master branch shows this can be a null pointer, in
                // which case the binary will exit with a `Error::System`.
                let ptr = jq_init();
                if ptr.is_null() {
                    return Err(Error::System {
                        msg: Some("Failed to init".into()),
                    });
                } else {
                    ptr
                }
            },
        };
        unsafe {
            if jq_compile(jq.state, program.as_ptr()) == 0 {
                Err(Error::Compile)
            } else {
                Ok(jq)
            }
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
    pub fn execute(&mut self, input: CString) -> Result<String, Error> {
        let mut parser = Parser::new();
        self.process(parser.parse(input)?)
    }

    /// Unwind the parser and return the rendered result.
    ///
    /// When this results in `Err`, the String value should contain a message about
    /// what failed.
    fn process(&mut self, initial_value: JV) -> Result<String, Error> {
        let mut buf = String::new();

        unsafe {
            jq_start(self.state, initial_value.ptr, 0);

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
    pub fn as_dump_string(&self) -> Result<String, std::str::Utf8Error> {
        let dump = JV {
            ptr: unsafe { jv_dump_string(self.ptr, 0) },
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

                let s = unsafe { get_string_value(jv_string_value(msg.ptr)) };

                format!("Parse error: {}", s.unwrap_or_else(|_| "unknown".into()))
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

    pub fn is_valid(&self) -> bool {
        unsafe {
            // FIXME: looks like this copy should not be needed (but it is?)
            //  Test suite shows a memory error if this value is freed after
            //  being passed to `jv_get_kind()`, so I guess this is a
            //  consuming call?

            jv_get_kind(jv_copy(self.ptr)) != jv_kind_JV_KIND_INVALID
        }
    }

    pub fn invalid_has_msg(&self) -> bool {
        // FIXME: the C lib suggests the jv passed in here will eventually be freed.
        //  I had a a `jv_copy()` to side-step this, but removing it removes one
        //  leak warning in valgrind, so I don't know what the deal is.
        unsafe { jv_invalid_has_msg(self.ptr) == 1 }
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

    pub fn parse(&mut self, input: CString) -> Result<JV, Error> {
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
                msg: Some(
                    value
                        .get_msg()
                        .unwrap_or_else(|| "Parser error".to_string()),
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
unsafe fn get_string_value(value: *const c_char) -> Result<String, std::str::Utf8Error> {
    let s = CStr::from_ptr(value).to_str()?;
    Ok(s.to_owned())
}

/// Renders the data from the parser and pushes it into the buffer.
unsafe fn dump(jq: &Jq, buf: &mut String) -> Result<(), Error> {
    // Looks a lot like an iterator...

    let mut value = JV {
        ptr: jq_next(jq.state),
    };

    while value.is_valid() {
        match value.as_dump_string() {
            Ok(s) => {
                buf.push_str(&s);
                buf.push('\n');
            }
            Err(e) => {
                return Err(Error::System {
                    msg: Some(format!("String Decode error: {}", e)),
                });
            }
        };

        value = JV {
            ptr: jq_next(jq.state),
        };
    }

    if jq.is_halted() {
        use self::ExitCode::*;
        match jq.get_exit_code() {
            JQ_ERROR_SYSTEM => Err(Error::System {
                msg: value.get_msg(),
            }),
            // As far as I know, we should not be able to see a compile error
            // this deep into the execution of a jq program (it would need to be
            // compiled already, right?)
            // Still, compile failure is represented by an exit code, so in
            // order to be exhaustive we have to check for it.
            JQ_ERROR_COMPILE => Err(Error::Compile),
            // Any of these `OK_` variants are "success" cases.
            // I suppose the jq program can halt successfully, or not, or not at
            // all and still terminate some other way?
            JQ_OK | JQ_OK_NULL_KIND | JQ_OK_NO_OUTPUT => Ok(()),
            JQ_ERROR_UNKNOWN => Err(Error::Unknown),
        }
    } else if let Some(reason) = value.get_msg() {
        Err(Error::System { msg: Some(reason) })
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
        use self::ExitCode::*;
        match number as isize {
            n if n == JQ_OK as isize => JQ_OK,
            n if n == JQ_OK_NULL_KIND as isize => JQ_OK_NULL_KIND,
            n if n == JQ_ERROR_SYSTEM as isize => JQ_ERROR_SYSTEM,
            n if n == JQ_ERROR_COMPILE as isize => JQ_ERROR_COMPILE,
            n if n == JQ_OK_NO_OUTPUT as isize => JQ_OK_NO_OUTPUT,
            n if n == JQ_ERROR_UNKNOWN as isize => JQ_ERROR_UNKNOWN,
            // `5` is called out explicitly in the jq source, but also "unknown"
            // seems to make good sense for other unexpected number.
            _ => JQ_ERROR_UNKNOWN,
        }
    }
}
