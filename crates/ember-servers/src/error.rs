//! Coarse server error type. Games and engine crates see this; backends
//! flatten their specific errors into these variants.

/// A coarse error from a server subsystem.
///
/// Per `02-principles.md §5.1`, L2 servers return `Result<_, ServerError>`.
/// Backends implement `From<TheirError>` and choose the variant that
/// best describes the failure.
#[non_exhaustive]
#[derive(Debug)]
pub enum ServerError {
    /// The backend rejected the operation (e.g. texture creation failed).
    Backend(String),
    /// The provided handle is unknown to this backend.
    UnknownHandle,
    /// The operation is not supported by this backend.
    NotSupported,
    /// IO failure during a load operation.
    Io(std::io::Error),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::Backend(s) => write!(f, "backend error: {s}"),
            ServerError::UnknownHandle => write!(f, "unknown handle"),
            ServerError::NotSupported => write!(f, "operation not supported"),
            ServerError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for ServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ServerError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        ServerError::Io(e)
    }
}
