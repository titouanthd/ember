//! `SceneError`: everything a structural `NodeTree` operation can reject.

use crate::NodeId;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
    /// The id is stale, null, or refers to a node that is not live yet.
    NotFound(NodeId),
    /// `attach(child, parent)` with `child == parent`.
    SelfParent(NodeId),
    /// The root cannot be attached under another node.
    IsRoot(NodeId),
    /// The child already has a parent; `detach` it first.
    AlreadyAttached(NodeId),
    /// `detach` on a node that has no parent.
    NotAttached(NodeId),
    /// The new parent is a descendant of the child.
    Cycle { child: NodeId, parent: NodeId },
    /// Malformed `.escene` text or structure.
    Parse(String),
    /// `.escene` `format` newer than this engine understands.
    UnsupportedFormat(u32),
    /// A node's `class` is not in the `ClassDB`.
    ClassNotRegistered(String),
    /// A node name is empty, contains `/`, or is `..`.
    InvalidName(String),
    /// Reading a scene file failed.
    Io(String),
    /// A nested `instance:` could not be loaded.
    Asset(String),
    /// A scene instances itself, directly or through other scenes.
    InstanceCycle(String),
}

impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "node {id:?} not found"),
            Self::SelfParent(id) => write!(f, "node {id:?} cannot be its own parent"),
            Self::IsRoot(id) => write!(f, "node {id:?} is the root and cannot be attached"),
            Self::AlreadyAttached(id) => write!(f, "node {id:?} already has a parent"),
            Self::NotAttached(id) => write!(f, "node {id:?} has no parent"),
            Self::Parse(m) => write!(f, "scene parse error: {m}"),
            Self::UnsupportedFormat(v) => write!(f, "unsupported .escene format {v}"),
            Self::ClassNotRegistered(c) => write!(f, "class `{c}` is not registered"),
            Self::InvalidName(n) => write!(f, "invalid node name `{n}`"),
            Self::Io(m) => write!(f, "scene io error: {m}"),
            Self::Asset(m) => write!(f, "nested scene load failed: {m}"),
            Self::InstanceCycle(c) => write!(f, "cycle detected instancing {c}"),
            Self::Cycle { child, parent } => {
                write!(
                    f,
                    "attaching {child:?} under {parent:?} would create a cycle"
                )
            }
        }
    }
}

impl std::error::Error for SceneError {}
