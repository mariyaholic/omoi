//! Console entry point.
//!
//! The real command surface arrives in U13. Until then every invocation is a
//! clean, non-panicking `NotImplemented` failure.

use std::process::ExitCode;

use omoi::Error;

fn main() -> ExitCode {
    let error = Error::NotImplemented("omoi cli");
    eprintln!("{error}");
    ExitCode::from(error.exit_code())
}
