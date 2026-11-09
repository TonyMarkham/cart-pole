//! Sends one PNG to the Orin's webserver, which shows it on every open viewer
//! page. The Orin has no monitor, so this is how a picture made on it is seen.

mod arguments;
mod constants;
mod error;

// ---------------------------------------------------------------------------------------------- //

use crate::{
    arguments::Arguments,
    constants::{IMAGE_ROUTE, PNG_CONTENT_TYPE, PNG_SIGNATURE},
    error::{app_error::AppError, app_result::AppResult},
};

use clap::Parser;
use std::{fs, process::ExitCode};

/// Prints why it failed, and exits with a failure code, so a script can tell.
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Reads the file, refuses anything that isn't a PNG, and posts it.
fn run() -> AppResult<()> {
    let args = Arguments::parse();

    let image = fs::read(&args.png)?;

    if !image.starts_with(PNG_SIGNATURE) {
        return Err(AppError::not_png(&args.png));
    }

    ureq::post(format!("{}{IMAGE_ROUTE}", args.server))
        .content_type(PNG_CONTENT_TYPE)
        .send(&image[..])?;

    println!("Pushed {}", args.png.display());

    Ok(())
}
