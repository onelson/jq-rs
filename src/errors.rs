use std::error;
use std::ffi;
use std::fmt;
use std::result;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    /// The jq program failed to compile.
    InvalidProgram,
    /// System errors are raised by the internal jq state machine. These can
    /// indicate problems parsing input, or even failures while initializing
    /// the state machine itself.
    ///
    /// `reason` will be feedback from jq about what went wrong, when available.
    System { reason: Option<String> },
    /// Errors encountered during conversion between CString/String or vice
    /// versa.
    ///
    /// `err` will be the original error which lead to this.
    StringConvert { err: Box<error::Error> },
    /// Something bad happened, but it was unexpected.
    Unknown,
}

impl From<ffi::NulError> for Error {
    fn from(err: ffi::NulError) -> Self {
        Error::StringConvert { err: Box::new(err) }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::StringConvert { err: Box::new(err) }
    }
}

const UNKNOWN: &'static str = "Unknown JQ Error";

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let detail: String = match self {
            Error::InvalidProgram => "JQ Program failed to compile.".into(),
            Error::System { reason } => reason.as_ref().cloned().unwrap_or_else(|| UNKNOWN.into()),
            Error::StringConvert { err } => format!("Failed to convert string: `{}`", err),
            Error::Unknown => UNKNOWN.into(),
        };
        write!(f, "{}", detail)
    }
}

impl error::Error for Error {}
