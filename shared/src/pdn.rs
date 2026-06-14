use core::slice;
use std::{
    ffi::{CString, c_char},
    fmt::{self, Debug},
    fs::{self, File},
    io::{self, Write},
    iter,
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

use crate::ConversionFormat;

pub struct PdnHoster {
    // Doesn't need to be kept for some reason
    // How do the functions still work work if the context has closed?
    // #[allow(dead_code)]
    // context: HostfxrContext<InitializedForCommandLine>,
    ora: ManagedFunction<unsafe extern "system" fn(*const PdnToImageIO) -> *mut c_char>,
    png: ManagedFunction<unsafe extern "system" fn(*const PdnToImageIO) -> *mut c_char>,
    jpeg: ManagedFunction<unsafe extern "system" fn(*const JpegIO) -> *mut c_char>,
}

impl Debug for PdnHoster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str("PdnHoster")
    }
}

#[repr(C)]
struct PdnToImageIO {
    input: RawHandle,
    output: RawHandle,
}

#[repr(C)]
struct JpegIO {
    quality: i64,
    io: PdnToImageIO,
}

impl PdnToImageIO {
    pub fn new(input: &Path, output: &Path) -> io::Result<Self> {
        let input = fs::File::open(input)?;
        let output = fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(output)?;

        Ok(Self {
            input: input.into_raw_handle(),
            output: output.into_raw_handle(),
        })
    }
}

#[repr(C)]
struct Version {
    major: i32,
    minor: i32,
    build: i32,
    revision: i32,
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
                context.load_assembly_from_path(PdCString::from_os_str(path.as_os_str())?)?;
            }
        }

        context.load_assembly_from_bytes(
            include_bytes!("../libs/PdnBridge.dll"),
            include_bytes!("../libs/PdnBridge.pdb"),
        )?;

        let set_copy_to_c_string = context.get_delegate_loader()?
        .get_function_with_unmanaged_callers_only::<fn(f: unsafe extern "system" fn(*const u16, i32) -> *mut c_char)>(
            pdcstr!("PdnBridge.Library, PdnBridge"),
            pdcstr!("SetCopyToCStringFunctionPtr"),
        )?;
        set_copy_to_c_string(copy_to_c_string);

        let set_pdn_version = context
            .get_delegate_loader()?
            .get_function_with_unmanaged_callers_only::<fn(*const Version)>(
                pdcstr!("PdnBridge.Library, PdnBridge"),
                pdcstr!("SetPdnVersion"),
            )?;

        let deps_path = {
            let mut path = file_types;
            path.pop();
            path.push("paintdotnet.deps.json");
            path
        };

        let dep_file = fs::read_to_string(deps_path)?;
        let mut version_iter = dep_file
            .split_once(r#""PaintDotNet.Data.Reference": ""#)
            .ok_or(HosterError::DepParser)?
            .1
            .split_once('"')
            .ok_or(HosterError::DepParser)?
            .0
            .split('.')
            .map(str::parse::<i32>)
            .chain(iter::repeat(Ok(-1)))
            .map(|r| r.map_err(|_| HosterError::DepParser));
        let version = Version {
            major: version_iter.next().ok_or(HosterError::DepParser)??,
            minor: version_iter.next().ok_or(HosterError::DepParser)??,
            build: version_iter.next().ok_or(HosterError::DepParser)??,
            revision: version_iter.next().ok_or(HosterError::DepParser)??,
        };

        set_pdn_version(&raw const version);

        let pdn_to_ora = context
            .get_delegate_loader()?
            .get_function_with_unmanaged_callers_only::<unsafe fn(*const PdnToImageIO) -> *mut c_char>(
                pdcstr!("PdnBridge.Library, PdnBridge"),
                pdcstr!("PdnToOra"),
            )?;

        let png_to_ora = context
            .get_delegate_loader()?
            .get_function_with_unmanaged_callers_only::<unsafe fn(*const PdnToImageIO) -> *mut c_char>(
                pdcstr!("PdnBridge.Library, PdnBridge"),
                pdcstr!("PdnToPng"),
            )?;

        let jpeg_to_ora = context
            .get_delegate_loader()?
            .get_function_with_unmanaged_callers_only::<unsafe fn(*const JpegIO) -> *mut c_char>(
                pdcstr!("PdnBridge.Library, PdnBridge"),
                pdcstr!("PdnToJpeg"),
            )?;

        Ok(Self {
            ora: pdn_to_ora,
            png: png_to_ora,
            jpeg: jpeg_to_ora,
        })
    }

    pub fn file_from_pdn(
        &self,
        format: ConversionFormat,
        input: &Path,
        output: &Path,
    ) -> eyre::Result<()> {
        let io = PdnToImageIO::new(input, output)?;
        match format {
            ConversionFormat::Jpeg { quality } => call_converter(
                &self.jpeg,
                JpegIO {
                    quality: quality.into(),
                    io: PdnToImageIO::new(input, output)?,
                },
            ),
            ConversionFormat::Png => call_converter(&self.png, io),
            ConversionFormat::Ora => call_converter(&self.ora, io),
        }
    }
}

fn call_converter<T: 'static>(
    fun: &ManagedFunction<unsafe extern "system" fn(*const T) -> *mut c_char>,
    value: T,
) -> eyre::Result<()> {
    let args = &raw const value;

    // SAFETY: the C# side should ensure that things like file handles will be closed
    // We are also making sure we won't drop these ourselves later
    let error = unsafe { fun(args) };
    drop(value);

    if !error.is_null() {
        // SAFETY: This should be only getting created by our own allocator
        let error = unsafe { CString::from_raw(error) };
        eyre::bail!("Exception caught in C# code: {}", error.to_string_lossy());
    }

    Ok(())
}

// This is only really used for exceptions for now, name should probably be changed
unsafe extern "system" fn copy_to_c_string(ptr: *const u16, length: i32) -> *mut c_char {
    // SAFETY: This should only be called with a length from String.Length, which is only a Int32 for complaince or something
    let length = unsafe { length.try_into().unwrap_unchecked() };

    let wide_chars = unsafe { slice::from_raw_parts(ptr, length) };
    // I haven't seen a nul in errors but just in case
    // Honestly this entire thing sucks but not using CString needs lots of work
    let string = String::from_utf16_lossy(wide_chars).replace('\0', "\u{FFFD}");
    // This should never fail but lets prevent the nasty edge case
    CString::new(string)
        .map(CString::into_raw)
        .unwrap_or_else(|_| {
            c"Found a nul where all should've been replaced in the string for a error"
                .to_owned()
                .into_raw()
        })
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
    #[error("Failed at parsing paintdotnet.deps.json in json")]
    DepParser,
    #[error("Path to paintdotnet.dll contains a nul character where they shouldn't be")]
    ContainsNul(#[from] ContainsNul),
}
