use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let target = env::var("CARGO_CFG_TARGET_OS").unwrap();

    println!("cargo::rerun-if-env-changed=CARGO_CFG_FEATURE");

    if target != "windows" || !cfg!(feature = "pdn-sys") {
        return;
    }

    println!("cargo:rerun-if-changed=bridge/libs");
    println!("cargo:rerun-if-changed=bridge/Library.cs");
    println!("cargo:rerun-if-changed=bridge/PdnBridge.csproj");

    let mut dotnet_source_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    dotnet_source_dir.push("bridge");

    let mut command = Command::new("dotnet");

    command
        .arg("publish")
        .arg(&dotnet_source_dir)
        .arg("-c")
        .arg("Release")
        .arg("--os")
        .arg("win");

    let mut out = dotnet_source_dir;
    out.push("bin");
    out.push("included");

    command.arg("-o").arg(&out);

    command.output().expect("tried to run dotnet publish");
}
