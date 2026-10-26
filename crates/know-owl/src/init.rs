use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

/// A setup failure while constructing a rustdl reasoner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RustdlInitError {
    /// Input cannot be represented by the approved OWL/rustdl conformance slice.
    Unsupported(String),
    /// Parsing or backend setup failed independently of input semantics.
    InternalError(String),
}

impl Display for RustdlInitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Unsupported(message) => write!(formatter, "unsupported OWL input: {message}"),
            Self::InternalError(message) => write!(formatter, "OWL backend initialization failed: {message}"),
        }
    }
}

impl Error for RustdlInitError {}
