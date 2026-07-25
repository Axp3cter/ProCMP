//! The `pcmp` command line.

#![forbid(unsafe_code)]
#![warn(clippy::all)]

mod cli;

use clap::Parser;
use procmp::ExitCode;

fn main() -> std::process::ExitCode {
    let code = match cli::run(&cli::Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::Config
        }
    };

    std::process::ExitCode::from(code as u8)
}
