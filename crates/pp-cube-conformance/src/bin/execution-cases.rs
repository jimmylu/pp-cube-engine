//! Prints the execution manifest of one dialect: `execution-cases doris`.
//!
//! `execution-cases presto --sources` adds the model text and each case's Cube
//! query, timezone and security filters, for comparison against Cube Core.

use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = match arguments.as_slice() {
        [dialect] => pp_cube_conformance::manifest(dialect),
        [dialect, flag] if flag == "--sources" => {
            pp_cube_conformance::manifest_with_sources(dialect)
        }
        _ => {
            eprintln!("usage: execution-cases <dialect> [--sources]");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(document) => {
            print!("{document}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("execution-cases: {error}");
            ExitCode::FAILURE
        }
    }
}
