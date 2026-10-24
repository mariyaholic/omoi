//! Omoi: local browser-history ingestion.
//!
//! U1 establishes the crate root, the shared error type and the reader
//! contracts. Later units add the remaining modules.

pub mod error;
pub mod readers;

pub use error::{Error, Result};
