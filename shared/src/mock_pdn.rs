use std::{
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

#[derive(Debug)]
pub struct MockPdnHoster;

#[allow(unused)]
impl MockPdnHoster {
    pub fn new(paintdotnet_dll: PathBuf) -> Result<Self, HosterError> {
        Ok(Self)
    }

    pub fn ora_file_from_pdn(
        &self,
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> eyre::Result<()> {
        thread::sleep(Duration::from_secs(1));

        // File::create(output)?;

        Ok(())
    }

    pub fn png_file_from_pdn(
        &self,
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> eyre::Result<()> {
        thread::sleep(Duration::from_secs(1));

        // File::create(output)?;

        Ok(())
    }

    pub fn jpeg_file_from_pdn(
        &self,
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
        quality: u8,
    ) -> eyre::Result<()> {
        thread::sleep(Duration::from_secs(1));

        // File::create(output)?;

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
