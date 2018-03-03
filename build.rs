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

    // Change to one of the directory that contains moz.build
    // (they’re on the same nesting level)
    // so that paths like `../../checkout/src/…` line up.
    let repo = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let translator = repo.join("gfx").join("angle").join("targets").join("translator");
    env::set_current_dir(translator).unwrap();

    let mut build = cc::Build::new();

    for file in data.includes {
        build.include(file);
    }

    for &(k, v) in data.defines {
        build.define(k, v);
    }

    // Hard-code lines like `if CONFIG['OS_ARCH'] == 'Darwin':` in moz.build files
    for &(os, source) in &[
        ("darwin", "../../checkout/src/common/system_utils_mac.cpp"),
        ("linux", "../../checkout/src/common/system_utils_linux.cpp"),
        ("windows", "../../checkout/src/common/system_utils_win.cpp"),
    ] {
        if target.contains(os) {
            build.file(source);
            break
        }
    }

    build
        .files(data.sources)
        .cpp(true)
        .warnings(false)
        .flag_if_supported("-msse2")  // GNU
        .flag_if_supported("-arch:SSE2")  // MSVC
        .compile("angle");
}
