//! QRD core engine scaffold.
//!
//! This crate provides the base module layout for Phase 1 work.

pub mod columnar;
pub mod compression;
pub mod ecc;
pub mod encoding;
pub mod encryption;
pub mod error;
pub mod file;
pub mod footer;
pub mod integrity;
pub mod memory;
pub mod parser;
pub mod reader;
pub mod row_group;
pub mod schema;
pub mod writer;

pub use error::{QrdError, Result};

/// Returns the core engine version string.
///
/// # Examples
///
/// ```
/// let version = qrd_core::version();
/// assert!(version.starts_with("0.1."));
/// ```
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
