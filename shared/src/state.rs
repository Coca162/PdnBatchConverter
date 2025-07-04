use std::{
    fs::{self, File},
    io::{self, Write},
    num::{NonZero, NonZeroUsize},
    ops::Not,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};

use crate::{DEFAULT_LOCATION, HosterError, PdnHoster};
use directories::ProjectDirs;
use eyre::{Context, OptionExt};
use toml_edit::DocumentMut;

#[derive(Debug)]
pub struct State {
    hoster: Arc<PdnHoster>,
    parallelism: NonZeroUsize,
    total_parallelism: NonZeroUsize,
}

impl State {
    pub fn init() -> Result<Self, StateInitError> {
        match Self::from_config() {
            Ok(Some(s)) => return Ok(s),
            Ok(None) => (),
            Err(e) => return Err(e.into()),
        };

        eprintln!("Attempting to use system paint.net installation...");
        if Path::new(DEFAULT_LOCATION).exists().not() {
            return Err(StateInitError::DefaultNotFound);
        }

        Self::from_pdn_location(Path::new(DEFAULT_LOCATION))
            .map_err(StateInitError::DefaultInitError)
    }

    pub fn from_config() -> Result<Option<Self>, ConfigError> {
        let config_file = match fs::read_to_string(get_config_path()?) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let doc = config_file.parse::<DocumentMut>()?;

        let pdn_location = if let Some(v) = doc.get("pdn_location") {
            v.as_str()
                .ok_or_eyre("Expected pdn_location in config file to be a string!")?
        } else {
            return Ok(None);
        };

        let total_parallelism = thread::available_parallelism()?;
        Ok(Some(Self {
            hoster: Arc::new(PdnHoster::new(pdn_location.into()).map_err(|e| {
                ConfigError::CreationError(e, pdn_location.replace('\\', "\\\u{200B}"))
            })?),
            parallelism: parse_parallelism(&doc)?.unwrap_or(default_parallelism(total_parallelism)),
            total_parallelism,
        }))
    }

    pub fn from_pdn_location(path: &Path) -> eyre::Result<Self> {
        let pdn = Arc::new(PdnHoster::new(path.to_path_buf())?);

        let config_path = get_config_path()?;
        let mut doc = get_or_create_config(&config_path)?;

        doc.insert(
            "pdn_location",
            path.to_str()
                .ok_or_eyre("Non Unicode paths for the pdn location are not supported")?
                .into(),
        );

        fs::create_dir_all(config_path.parent().expect("Config to have a parent!"))?;

        File::create(config_path)?.write_fmt(format_args!("{doc}"))?;

        let total_parallelism = thread::available_parallelism()?;
        Ok(Self {
            hoster: pdn,
            parallelism: parse_parallelism(&doc)?.unwrap_or(default_parallelism(total_parallelism)),
            total_parallelism,
        })
    }

    pub fn pdn_hoster(&self) -> &Arc<PdnHoster> {
        &self.hoster
    }

    pub fn max_parallelism(&self) -> NonZeroUsize {
        // SAFETY: ceiled divides are guaranteed to be non-zero
        unsafe { NonZero::new_unchecked(self.total_parallelism.get().div_ceil(3)) }
    }

    pub fn parallelism(&self) -> NonZeroUsize {
        self.parallelism
    }

    pub fn set_parallelism(&mut self, new: NonZeroUsize) -> eyre::Result<()> {
        self.parallelism = new;

        let config_path = get_config_path()?;
        let mut doc = get_or_create_config(&config_path)?;

        doc.insert(
            "parallelism",
            i64::try_from(new.get())
                .context("parallelism is set too high")?
                .into(),
        );

        File::create(config_path)?.write_fmt(format_args!("{doc}"))?;

        Ok(())
    }
}

fn get_or_create_config(path: &Path) -> Result<DocumentMut, eyre::Error> {
    let doc = match fs::read_to_string(path) {
        Ok(f) => f.parse::<DocumentMut>()?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => DocumentMut::new(),
        Err(e) => return Err(e.into()),
    };
    Ok(doc)
}

fn get_config_path() -> Result<PathBuf, eyre::Error> {
    let proj_dirs = ProjectDirs::from("", "", "PdnBatchConverter")
        .ok_or_eyre("Could not find where OS stores application's configs.")?;
    Ok(proj_dirs.config_dir().join("config.toml"))
}

pub fn default_parallelism(parallelism: NonZeroUsize) -> NonZeroUsize {
    // We assume each paint.net export uses roughly 6 threads
    // SAFETY: ceiled divides are guaranteed to be non-zero
    unsafe { NonZero::new_unchecked(parallelism.get().div_ceil(6)) }
}

fn parse_parallelism(doc: &DocumentMut) -> eyre::Result<Option<NonZero<usize>>> {
    doc.get("parallelism")
        .map(|i| {
            i.as_integer()
                .ok_or_eyre("Expected parallelism in config file to be a integer!")
                .and_then(|x| usize::try_from(x).wrap_err("parallelism cannot be negative!"))
                .and_then(|x| NonZeroUsize::try_from(x).wrap_err("parallelism cannot be zero!"))
        })
        .transpose()
}

#[derive(Debug, thiserror::Error)]
pub enum StateInitError {
    #[error("Failed to setup from config. Paint.net might have been removed or corrupted.")]
    ConfigError(#[from] ConfigError),
    #[error(
        "No system paint.net installation found, please either install Paint.net and retry or select a portable Paint.net version."
    )]
    DefaultNotFound,
    #[error("Failed to initialize from default Paint.net, it is likely corrupted.")]
    DefaultInitError(#[source] eyre::Report),
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Could not create dotnet host from {1}")]
    CreationError(#[source] HosterError, String),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("Could not parse the program's config")]
    TomlError(#[from] toml_edit::TomlError),
    #[error(transparent)]
    Generic(#[from] eyre::Error),
}
