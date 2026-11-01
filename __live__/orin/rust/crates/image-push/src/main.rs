mod arguments;
mod constants;
mod error;

// ---------------------------------------------------------------------------------------------- //

pub(crate) use crate::constants::{DEFAULT_SERVER, IMAGE_ROUTE, PNG_CONTENT_TYPE, PNG_SIGNATURE};

// ---------------------------------------------------------------------------------------------- //

use crate::{
    arguments::Args,
    error::{AppError, result::AppResult},
};

use clap::Parser;
use std::{fs, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> AppResult<()> {
    let args = Args::parse();

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
