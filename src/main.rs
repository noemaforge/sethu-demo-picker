//! Minimal photo picker demo against the search API.

use demo_picker::{Error, ImmichClient, Picker};
use std::env;
use std::process::ExitCode;

/// Find the base URL after a `--base-url` flag.
fn base_url_flag(args: &[String]) -> Result<Option<String>, Error> {
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--base-url" {
            match args.get(index + 1) {
                Some(value) => return Ok(Some(value.clone())),
                None => {
                    return Err(Error::Config(
                        "pass a URL after the base URL flag".to_string(),
                    ));
                }
            }
        }
        index += 1;
    }
    Ok(None)
}

/// Pick one random page and print its asset IDs.
fn run() -> Result<(), Error> {
    let args: Vec<String> = env::args().skip(1).collect();
    let client = match base_url_flag(&args)? {
        Some(url) => ImmichClient::new(&url),
        None => ImmichClient::from_env()?,
    };
    let mut picker = Picker::new();
    picker.more(&client)?;
    for id in picker.selection() {
        println!("{id}");
    }
    Ok(())
}

/// Run the demo and report failures on stderr.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("demo-picker: {error}");
            ExitCode::FAILURE
        }
    }
}
