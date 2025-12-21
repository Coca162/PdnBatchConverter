use core::fmt;

#[cfg(not(feature = "pdn-sys"))]
pub use self::mock_pdn::{HosterError, MockPdnHoster as PdnHoster};
#[cfg(all(windows, feature = "pdn-sys"))]
pub use self::pdn::{HosterError, PdnHoster};
#[cfg(all(unix, feature = "pdn-sys"))]
pub use self::pdn_unix::{HosterError, PdnHoster};

mod mock_pdn;
#[cfg(all(windows, feature = "pdn-sys"))]
mod pdn;
#[cfg(all(unix, feature = "pdn-sys"))]
mod pdn_unix;

pub use self::state::*;
mod state;

pub const DEFAULT_PDN_DIR: &str = r"C:\Program Files\Paint.NET";
pub const DEFAULT_PDN_DLL: &str = r"C:\Program Files\Paint.NET\paintdotnet.dll";

pub const VERSION: &str = match (COMMIT_SHORT_SHA, VERSION_NAME) {
    (_, Some(version)) => version,
    (Some(hash), _) => hash.split_at(7).0,
    (None, None) => "Dev Version",
};

const VERSION_NAME: Option<&str> = match option_env!("VERSION_NAME") {
    Some(v) if !v.is_empty() => Some(v),
    Some(_) | None => None,
};

const COMMIT_SHORT_SHA: Option<&str> = option_env!("GITHUB_SHA");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversionFormat {
    Jpeg { quality: u8 },
    Png,
    Ora,
}

impl ConversionFormat {
    #[must_use]
    pub const fn ext(self) -> &'static str {
        match self {
            Self::Ora => "ora",
            Self::Png => "png",
            Self::Jpeg { .. } => "jpeg",
        }
    }
}

impl fmt::Display for ConversionFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.ext())
    }
}
