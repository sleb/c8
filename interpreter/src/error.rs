use std::{io, result};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum C8Error {
    #[error("Failed to load the program: {0}")]
    ProgramLoadFailure(#[from] io::Error),
}

pub type Result<T> = result::Result<T, C8Error>;
