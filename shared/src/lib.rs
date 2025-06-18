#[cfg(not(all(windows, feature = "pdn-sys")))]
pub use self::mock_pdn::{HosterError, MockPdnHoster as PdnHoster};
#[cfg(all(windows, feature = "pdn-sys"))]
pub use self::pdn::{HosterError, PdnHoster};

#[cfg(all(not(windows), feature = "pdn-sys"))]
compile_error!("Paint.net cannot be run on non-windows targets");

mod mock_pdn;
#[cfg(all(windows, feature = "pdn-sys"))]
mod pdn;

pub use self::state::*;
mod state;

pub const DEFAULT_LOCATION: &str = r#"C:\Program Files\Paint.NET\paintdotnet.dll"#;

pub const VERSION: &str = "Version 1";
