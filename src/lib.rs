//! deadgate. Whether the required checks of a branch are
//! produced by jobs that can fire for that branch and have run.
//!
//! This crate is the Rust port of the Python package of the
//! same name in this repository. The Python package stands as
//! the behavioral specification of this one, and the
//! conformance suite under tests holds the two readings to the
//! recorded behavior of the Python reading.

pub mod dates;
pub mod errors;
pub mod ingest;
pub mod jsonvalue;
pub mod printable;
pub mod pytext;
pub mod reconcile;
pub mod report;
pub mod yaml;
pub mod yamlvalue;

pub use errors::DeadgateError;
