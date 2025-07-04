use std::{
    env,
    fs::{copy, write},
    io,
    path::PathBuf,
};

fn main() -> io::Result<()> {
    if env::var_os("CARGO_CFG_WINDOWS").is_none() {
        return Ok(());
    }

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    copy(manifest_dir.join("icon.ico"), out_dir.join("icon.ico")).unwrap();
    write(out_dir.join("icon.rc"), "1 ICON icon.ico").unwrap();

    println!("cargo:rerun-if-changed=icon.ico");
    embed_resource::compile(out_dir.join("icon.rc"), embed_resource::NONE)
        .manifest_optional()
        .unwrap();

    Ok(())
}
