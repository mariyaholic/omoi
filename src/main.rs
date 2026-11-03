//! Console entry point.
//!
//! Command-line collection, export and database operations.

use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(omoi::cli::run())
}
