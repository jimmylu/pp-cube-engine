//! Prints the execution manifest of one dialect: `execution-cases doris`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let (Some(dialect), None) = (arguments.next(), arguments.next()) else {
        eprintln!("usage: execution-cases <dialect>");
        return ExitCode::from(2);
    };
    match pp_cube_conformance::manifest(&dialect) {
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
