use crate::libyaml::{emitter, error as libyaml};
use crate::path::Path;
use serde::{de, ser};
use std::error;
use std::fmt::{self, Debug, Display};
use std::io;
use std::result;
use std::string;
use std::sync::Arc;

/// An error that happened serializing or deserializing YAML data.
pub struct Error(Box<ErrorImpl>);

/// Alias for a `Result` with the error type `serde_yaml::Error`.
pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub(crate) enum ErrorImpl {
    Message(String, Option<Pos>),

    Libyaml(libyaml::Error),
    Io(io::Error),
    FromUtf8(string::FromUtf8Error),

    EndOfStream,
    MoreThanOneDocument,
    RecursionLimitExceeded(libyaml::Mark),
    UnknownAnchor(libyaml::Mark),

    Shared(Arc<ErrorImpl>),
}

#[derive(Debug)]
pub(crate) struct Pos {
    mark: libyaml::Mark,
    path: String,
}

/// The input location that an error occured.
#[derive(Debug)]
pub struct Location {
    index: usize,
    line: usize,
    column: usize,
}

impl Location {
    /// The byte index of the error
    pub fn index(&self) -> usize {
        self.index
    }

    /// The line of the error
    pub fn line(&self) -> usize {
        self.line
    }

    /// The column of the error
    pub fn column(&self) -> usize {
        self.column
    }

    // This is to keep decoupled with the yaml crate
    #[doc(hidden)]
    fn from_mark(mark: libyaml::Mark) -> Self {
        Location {
            index: mark.index() as usize,
            // `line` and `column` returned from libyaml are 0-indexed but all error messages add +1 to this value
            line: mark.line() as usize + 1,
            column: mark.column() as usize + 1,
        }
    }
}

impl Error {
    /// Returns the Location from the error if one exists.
    ///
    /// Not all types of errors have a location so this can return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_yaml::{Value, Error};
    /// #
    /// // The `@` character as the first character makes this invalid yaml
    /// let invalid_yaml: Result<Value, Error> = serde_yaml::from_str("@invalid_yaml");
    ///
    /// let location = invalid_yaml.unwrap_err().location().unwrap();
    ///
    /// assert_eq!(location.line(), 1);
    /// assert_eq!(location.column(), 1);
    /// ```
    pub fn location(&self) -> Option<Location> {
        self.0.location()
    }

    /// Returns the path from the error if one exists
    pub fn path(&self) -> Option<&String> {
        match self.0.as_ref() {
            ErrorImpl::Message(_, Some(pos)) => Some(&pos.path),
            _ => None,
        }
    }
}

pub(crate) fn end_of_stream() -> Error {
    Error(Box::new(ErrorImpl::EndOfStream))
}

pub(crate) fn more_than_one_document() -> Error {
    Error(Box::new(ErrorImpl::MoreThanOneDocument))
}

pub(crate) fn io(err: io::Error) -> Error {
    Error(Box::new(ErrorImpl::Io(err)))
}

pub(crate) fn string_utf8(err: string::FromUtf8Error) -> Error {
    Error(Box::new(ErrorImpl::FromUtf8(err)))
}

pub(crate) fn recursion_limit_exceeded(mark: libyaml::Mark) -> Error {
    Error(Box::new(ErrorImpl::RecursionLimitExceeded(mark)))
}

pub(crate) fn unknown_anchor(mark: libyaml::Mark) -> Error {
    Error(Box::new(ErrorImpl::UnknownAnchor(mark)))
}

pub(crate) fn shared(shared: Arc<ErrorImpl>) -> Error {
    Error(Box::new(ErrorImpl::Shared(shared)))
}

pub(crate) fn fix_mark(mut error: Error, mark: libyaml::Mark, path: Path) -> Error {
    if let ErrorImpl::Message(_, none @ None) = error.0.as_mut() {
        *none = Some(Pos {
            mark,
            path: path.to_string(),
        });
    }
    error
}

impl Error {
    pub(crate) fn shared(self) -> Arc<ErrorImpl> {
        if let ErrorImpl::Shared(err) = *self.0 {
            err
        } else {
            Arc::from(self.0)
        }
    }
}

impl From<libyaml::Error> for Error {
    fn from(err: libyaml::Error) -> Self {
        Error(Box::new(ErrorImpl::Libyaml(err)))
    }
}

impl From<emitter::Error> for Error {
    fn from(err: emitter::Error) -> Self {
        match err {
            emitter::Error::Libyaml(err) => Self::from(err),
            emitter::Error::Io(err) => io(err),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        self.0.source()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.display(f)
    }
}

// Remove two layers of verbosity from the debug representation. Humans often
// end up seeing this representation because it is what unwrap() shows.
impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.debug(f)
    }
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error(Box::new(ErrorImpl::Message(msg.to_string(), None)))
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error(Box::new(ErrorImpl::Message(msg.to_string(), None)))
    }
}

impl ErrorImpl {
    fn location(&self) -> Option<Location> {
        match self {
            ErrorImpl::Message(_, Some(pos)) => Some(Location::from_mark(pos.mark)),
            ErrorImpl::Libyaml(err) => Some(Location::from_mark(err.mark())),
            ErrorImpl::Shared(err) => err.location(),
            _ => None,
        }
    }

    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            ErrorImpl::Io(err) => Some(err),
            ErrorImpl::FromUtf8(err) => Some(err),
            ErrorImpl::Shared(err) => err.source(),
            _ => None,
        }
    }

    fn display(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorImpl::Message(msg, None) => Display::fmt(msg, f),
            ErrorImpl::Message(msg, Some(Pos { mark, path })) => {
                if path == "." {
                    write!(f, "{} at {}", msg, mark)
                } else {
                    write!(f, "{}: {} at {}", path, msg, mark)
                }
            }
            ErrorImpl::Libyaml(err) => Display::fmt(err, f),
            ErrorImpl::Io(err) => Display::fmt(err, f),
            ErrorImpl::FromUtf8(err) => Display::fmt(err, f),
            ErrorImpl::EndOfStream => f.write_str("EOF while parsing a value"),
            ErrorImpl::MoreThanOneDocument => f.write_str(
                "deserializing from YAML containing more than one document is not supported",
            ),
            ErrorImpl::RecursionLimitExceeded(mark) => {
                write!(f, "recursion limit exceeded at {}", mark)
            }
            ErrorImpl::UnknownAnchor(mark) => write!(f, "unknown anchor at {}", mark),
            ErrorImpl::Shared(err) => err.display(f),
        }
    }

    fn debug(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorImpl::Message(msg, pos) => f.debug_tuple("Message").field(msg).field(pos).finish(),
            ErrorImpl::Libyaml(err) => f.debug_tuple("Libyaml").field(err).finish(),
            ErrorImpl::Io(io) => f.debug_tuple("Io").field(io).finish(),
            ErrorImpl::FromUtf8(from_utf8) => f.debug_tuple("FromUtf8").field(from_utf8).finish(),
            ErrorImpl::EndOfStream => f.debug_tuple("EndOfStream").finish(),
            ErrorImpl::MoreThanOneDocument => f.debug_tuple("MoreThanOneDocument").finish(),
            ErrorImpl::RecursionLimitExceeded(mark) => {
                f.debug_tuple("RecursionLimitExceeded").field(mark).finish()
            }
            ErrorImpl::UnknownAnchor(mark) => f.debug_tuple("UnknownAnchor").field(mark).finish(),
            ErrorImpl::Shared(err) => err.debug(f),
        }
    }
}
