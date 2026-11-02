//! Omoi: local browser-history ingestion.
//!
//! U1 establishes the crate root, the shared error type and the reader
//! contracts. Later units add the remaining modules.

pub mod cli;
pub mod collect;
pub mod config;
pub mod db;
pub mod dedupe;
pub mod error;
pub mod export;
pub mod normalize;
pub mod paths;
pub mod readers;

pub use error::{Error, Result};
