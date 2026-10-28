use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=../../../icon/RetinaCare.ico");
    println!("cargo:rerun-if-env-changed=RETINACARE_RC");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo OUT_DIR"));
    let icon = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest"))
        .join("../../../icon/RetinaCare.ico")
        .canonicalize()
        .expect("RetinaCare icon asset");
    let rc = out.join("retinacare.rc");
    fs::write(
        &rc,
        format!(
            "1 ICON \"{}\"\n",
            icon.display().to_string().replace('\\', "/")
        ),
    )
    .expect("write icon resource definition");
    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let resource = out.join(if msvc {
        "retinacare.res"
    } else {
        "retinacare.o"
    });
    let compiler = env::var("RETINACARE_RC").unwrap_or_else(|_| {
        if msvc {
            "rc.exe"
        } else {
            "x86_64-w64-mingw32-windres"
        }
        .into()
    });
    let mut command = Command::new(compiler);
    if msvc {
        command
            .arg("/nologo")
            .arg(format!("/fo{}", resource.display()))
            .arg(&rc);
    } else {
        command.arg(&rc).arg(&resource);
    }
    let status = command
        .status()
        .expect("Windows resource compiler required to embed EXE icon");
    assert!(status.success(), "Windows icon compilation failed");
    println!("cargo:rustc-link-arg={}", resource.display());
}
