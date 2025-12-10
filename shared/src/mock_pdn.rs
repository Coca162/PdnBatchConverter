#![allow(clippy::pedantic, clippy::nursery)]

use std::{
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use crate::ConversionFormat;

#[derive(Debug)]
pub struct MockPdnHoster;

#[allow(unused)]
impl MockPdnHoster {
    pub fn new(paintdotnet_dll: PathBuf) -> Result<Self, HosterError> {
        Ok(Self)
    }

    pub fn file_from_pdn(
        &self,
        format: ConversionFormat,
        input: &Path,
        output: &Path,
    ) -> eyre::Result<()> {
        thread::sleep(Duration::from_secs(1));

        // match format {
        //     ConversionFormat::Jpeg { .. } => std::fs::File::create(output.with_extension(".jpeg"))?,
        //     ConversionFormat::Png => std::fs::File::create(output.with_extension(".png"))?,
        //     ConversionFormat::Ora => std::fs::File::create(output.with_extension(".ora"))?,
        // };

        Ok(())
    }
}

#[derive(Debug)]
pub struct HosterError;

impl Display for HosterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Could not set Paint.net host!")
    }
}

impl Error for HosterError {}
