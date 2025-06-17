use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo::rerun-if-env-changed=CARGO_CFG_FEATURE");

    if !cfg!(feature = "pdn-sys") {
        return;
    }

    let target = env::var("CARGO_CFG_TARGET_OS").unwrap();
    if target != "windows" {
        println!("cargo::error=You cannot use Paint.net on non-windows targets")
    }

    match env::var("SKIP_DOTNET_BUILDING") {
        Ok(x) if x.eq_ignore_ascii_case("yes") | x.eq_ignore_ascii_case("y") | x.eq_ignore_ascii_case("true") => return,
        Ok(_) | Err(env::VarError::NotPresent) => (),
        Err(e) => panic!("{e}"),
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
    out.pop();
    out.push("libs");

    command.arg("-o").arg(&out);

    command.output().expect("tried to run dotnet publish");
}
