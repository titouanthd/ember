//! `AssetError`: everything loading or registering an asset can reject.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AssetError {
    /// The file does not exist under the asset root.
    NotFound(String),
    /// Reading or writing a file failed.
    Io(String),
    /// Absolute path, `.`/`..` component, or empty path.
    InvalidPath(String),
    /// The path's extension or cache entry belongs to another asset type.
    WrongType {
        path: String,
        expected: &'static str,
    },
    /// The bytes could not be decoded.
    Parse(String),
    /// Not of the form `uid://<name>`.
    InvalidUid(String),
    /// No registry entry for this UID.
    UnknownUid(String),
    /// The asset needs a `RenderingServer`; use `load_with`.
    NeedsRenderer(String),
    /// A server call failed.
    Server(String),
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(p) => write!(f, "asset `{p}` not found"),
            Self::Io(m) => write!(f, "asset io error: {m}"),
            Self::InvalidPath(p) => write!(f, "invalid asset path `{p}`"),
            Self::WrongType { path, expected } => {
                write!(f, "asset `{path}` cannot be loaded as {expected}")
            }
            Self::Parse(m) => write!(f, "asset parse error: {m}"),
            Self::InvalidUid(u) => write!(f, "invalid uid `{u}`"),
            Self::UnknownUid(u) => write!(f, "unknown uid `{u}`"),
            Self::NeedsRenderer(p) => write!(f, "asset `{p}` needs a RenderingServer"),
            Self::Server(m) => write!(f, "server error: {m}"),
        }
    }
}

impl std::error::Error for AssetError {}
