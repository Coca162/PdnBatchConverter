use std::{
    error::Error,
    fmt::{self, Debug, Display},
    fs::{self, File},
    io::{self, Write},
    os::windows::io::IntoRawHandle,
    path::{Path, PathBuf},
};

use netcorehost::{
    error::HostingError,
    hostfxr::{GetManagedFunctionError, ManagedFunction},
    nethost::{LoadHostfxrError, load_hostfxr_with_assembly_path},
    pdcstr,
    pdcstring::{ContainsNul, PdCString},
};
use std::os::windows::io::RawHandle;

use crate::DEFAULT_LOCATION;

pub struct PdnHoster {
    // Doesn't need to be kept for some reason
    // How do the functions still work work if the context has closed?
    // #[allow(dead_code)]
    // context: HostfxrContext<InitializedForCommandLine>,
    pdn_to_ora: ManagedFunction<unsafe extern "system" fn(*const PdnToOraIO)>,
}

impl Debug for PdnHoster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str("PdnHoster")
    }
}

#[repr(C)]
struct PdnToOraIO {
    input: RawHandle,
    output: RawHandle,
}

impl PdnHoster {
    /// Creates hostfxr context for the paintdotnet application located at the absolute path
    pub fn new(paintdotnet_dll: PathBuf) -> Result<Self, HosterError> {
        let app_path = PdCString::from_os_str(paintdotnet_dll.as_os_str())?;

        let hostfxr = load_hostfxr_with_assembly_path(&app_path)?;

        let context = hostfxr.initialize_for_dotnet_command_line(app_path)?;

        let ora_dll = {
            let mut path = paintdotnet_dll;
            path.pop();
            path.push("FileTypes");
            path.push("OpenRasterFileType.dll");
            path
        };

        match File::create_new(&ora_dll) {
            Ok(mut f) => f.write_all(include_bytes!("../libs/OpenRasterFileType.dll"))?,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e.into()),
        }

        let file_types = {
            let mut path = ora_dll;
            path.pop();
            path
        };

        let mut dir = fs::read_dir(&file_types)?;

        while let Some(entry) = dir.next().transpose()? {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "dll") {
                context.load_assembly_from_path(PdCString::from_os_str(path.as_os_str())?)?
            }
        }

        context.load_assembly_from_bytes(
            include_bytes!("../libs/PdnBridge.dll"),
            include_bytes!("../libs/PdnBridge.pdb"),
        )?;

        let pdn_to_ora = context
            .get_delegate_loader()?
            .get_function_with_unmanaged_callers_only::<unsafe fn(*const PdnToOraIO)>(
                pdcstr!("PdnBridge.Library, PdnBridge"),
                pdcstr!("PdnToOra"),
            )?;

        Ok(Self { pdn_to_ora })
    }

    pub fn ora_file_from_pdn(
        &self,
        input: impl AsRef<Path>,
        output: impl AsRef<Path>,
    ) -> io::Result<()> {
        let input = fs::File::open(input)?;
        let output = fs::File::create_new(output)?;

        let handles = PdnToOraIO {
            input: input.into_raw_handle(),
            output: output.into_raw_handle(),
        };

        let args = (&handles) as *const PdnToOraIO;

        // SAFETY: the C# side should ensure that these handles will be closed
        // We are also making sure we won't drop these ourselves later
        unsafe { (*self.pdn_to_ora)(args) };

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HosterError {
    #[error("Could not load dotnet host")]
    LoadHostfxrError(#[from] LoadHostfxrError),
    #[error("Could not initialize either paint.net, its plugins or bridge in dotnet host")]
    InitAssembliesError(#[from] HostingError),
    #[error("Could not get function needed for bridging paint.net")]
    BridgeFunctionError(#[from] GetManagedFunctionError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("Path to paintdotnet.dll contains a nul character where they shouldn't be")]
    ContainsNul(#[from] ContainsNul),
}
