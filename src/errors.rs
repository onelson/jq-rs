use std::error;
use std::fmt;
use std::result;

const ERR_UNKNOWN: &str = "JQ: Unknown error";
const ERR_COMPILE: &str = "JQ: Program failed to compile";
const ERR_STRING_CONV: &str = "JQ: Failed to convert string";

/// This is the common Result type for the crate. Fallible operations will
/// return this.
pub type Result<T> = result::Result<T, Error>;

/// There are many potential causes for failure when running jq programs.
/// This enum attempts to unify them all under a single type.
#[derive(Debug)]
pub enum Error {
    /// The jq program failed to compile.
    InvalidProgram {
        /// JQ's explanation of the compilation error
        reason: String,
    },
    /// System errors are raised by the internal jq state machine. These can
    /// indicate problems parsing input, or even failures while initializing
    /// the state machine itself.
    System {
        /// Feedback from jq about what went wrong, when available.
        reason: Option<String>,
    },
    /// Errors encountered during conversion between CString/String or vice
    /// versa.
    StringConvert {
        /// The original error which lead to this.
        err: Box<dyn error::Error + 'static>,
    },
    /// Something bad happened, but it was unexpected.
    Unknown,
}

unsafe impl Send for Error {}

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::StringConvert { .. } => ERR_STRING_CONV,
            Error::InvalidProgram { reason } => reason,
            Error::System { reason } => reason
                .as_ref()
                .map(|x| x.as_str())
                .unwrap_or_else(|| ERR_UNKNOWN),
            Error::Unknown => ERR_UNKNOWN,
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::StringConvert { err } => {
                if let Some(err) = err.downcast_ref::<std::ffi::NulError>() {
                    Some(err)
                } else if let Some(err) = err.downcast_ref::<std::str::Utf8Error>() {
                    Some(err)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl From<std::ffi::NulError> for Error {
    fn from(err: std::ffi::NulError) -> Self {
        Error::StringConvert { err: Box::new(err) }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::StringConvert { err: Box::new(err) }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let detail: String = match self {
            Error::InvalidProgram { reason } => format!("{}: {}", ERR_COMPILE, reason),
            Error::System { reason } => reason
                .as_ref()
                .cloned()
                .unwrap_or_else(|| ERR_UNKNOWN.into()),
            Error::StringConvert { err } => format!("{} - `{}`", ERR_STRING_CONV, err),
            Error::Unknown => ERR_UNKNOWN.into(),
        };
        write!(f, "{}", detail)
    }
}
