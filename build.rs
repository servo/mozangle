extern crate cc;

use std::env;
use std::path::PathBuf;

mod build_data;

fn main() {
    let egl = env::var("CARGO_FEATURE_EGL").is_ok();
    let target = env::var("TARGET").unwrap();
    if egl && !target.contains("windows") {
        println!("The `egl` feature is only supported on Windows.");
        std::process::exit(1)
    }

    let data = if egl { build_data::EGL } else { build_data::TRANSLATOR };

    let repo = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    env::set_current_dir(repo).unwrap();

    // Change to one of the directory that contains moz.build
    let mut build = cc::Build::new();

    for &(k, v) in data.defines {
        build.define(k, v);
    }

    for file in data.includes {
        build.include(fixup_path(file));
    }

    for file in data.sources {
        build.file(fixup_path(file));
    }

    // Hard-code lines like `if CONFIG['OS_ARCH'] == 'Darwin':` in moz.build files
    for &(os, source) in &[
        ("darwin", "gfx/angle/checkout/src/common/system_utils_mac.cpp"),
        ("linux", "gfx/angle/checkout/src/common/system_utils_linux.cpp"),
        ("windows", "gfx/angle/checkout/src/common/system_utils_win.cpp"),
    ] {
        if target.contains(os) {
            build.file(source);
            break
        }
    }

    build
        .file("src/shaders/glslang-c.cpp")
        .cpp(true)
        .warnings(false)
        .flag_if_supported("-msse2")  // GNU
        .flag_if_supported("-arch:SSE2")  // MSVC
        .compile("angle");

    println!("cargo:rerun-if-changed=src/shaders/glslang-c.cpp");
    println!("cargo:rerun-if-changed=gfx");
}

fn fixup_path(path: &str) -> String {
    let prefix = "../../";
    assert!(path.starts_with(prefix));
    format!("gfx/angle/{}", &path[prefix.len()..])
}
