use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-env-changed=WINDRES");
    println!("cargo:rerun-if-env-changed=RC");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let resource = out.join(if msvc { "icon.res" } else { "icon.o" });
    let mut command = if msvc {
        let mut cmd = Command::new(env::var_os("RC").unwrap_or_else(|| "rc.exe".into()));
        cmd.arg("/nologo")
            .arg("/fo")
            .arg(&resource)
            .arg("assets/app.rc");
        cmd
    } else {
        let mut cmd = Command::new(env::var_os("WINDRES").unwrap_or_else(|| "windres".into()));
        cmd.args([
            "--input",
            "assets/app.rc",
            "--output-format=coff",
            "--output",
        ])
        .arg(&resource);
        cmd
    };
    let status = command
        .current_dir(env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .status()
        .expect("Windows icon compiler unavailable: install windres (GNU) or rc.exe (MSVC)");
    assert!(status.success(), "Windows icon resource compilation failed");
    println!(
        "cargo:rustc-link-arg-bin=omoi-desktop={}",
        resource.display()
    );
}
