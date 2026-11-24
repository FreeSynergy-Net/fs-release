//! Pattern: none (one error value; every failure of the packer is named, nothing is swallowed).

use std::fmt;

/// Why packing failed. `Check` carries every problem found, not only the first.
#[derive(Debug)]
pub enum PackError {
    /// The source folder or the output folder could not be read or written.
    Io(String),
    /// The sources are not sound (a missing pair, a message that does not parse, variables that differ ...).
    Check(Vec<String>),
    /// The `mf2` library refused the data (catalog writer, locale data, date data).
    Library(String),
    /// A language was asked for that has no source file.
    NoSuchLanguage(String),
}

impl fmt::Display for PackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(why) | Self::Library(why) => f.write_str(why),
            Self::Check(list) => {
                write!(f, "{} problem(s) in the texts:", list.len())?;
                list.iter().try_for_each(|p| write!(f, "\n  {p}"))
            }
            Self::NoSuchLanguage(l) => write!(f, "language {l}: no source file"),
        }
    }
}

impl std::error::Error for PackError {}
