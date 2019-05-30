extern crate cc;
#[cfg(feature = "egl")] extern crate gl_generator;

use std::env;
use std::path::{Path, PathBuf};

mod build_data;

fn main() {
    build_angle();
    generate_bindings();
}

fn build_egl(target: &str) {
    if !target.contains("windows") {
        return;
    }

    let mut build = cc::Build::new();

    let data = build_data::EGL;
    for &(k, v) in data.defines {
        build.define(k, v);
    }

    for file in data.includes {
        build.include(fixup_path(file));
    }

    let mut cmd = build.get_compiler().to_command();
    let out = env::var("OUT_DIR").unwrap();
    let out = Path::new(&out);
    cmd.arg(out.join("angle.lib"));

    for lib in data.os_libs {
        cmd.arg(&format!("{}.lib", lib));
    }

    for file in data.sources {
        cmd.arg(fixup_path(file));
    }

    cmd.arg("/wd4100");
    cmd.arg("/wd4127");
    cmd.arg("/LD");
    cmd.arg(&format!("/Fe{}", out.join("libEGL").display()));
    cmd.arg("/link");
    cmd.arg("/DEF:gfx/angle/checkout/src/libEGL/libEGL.def");
    let status = cmd.status();
    assert!(status.unwrap().success());
}

fn build_angle() {
    let target = env::var("TARGET").unwrap();
    let egl = env::var("CARGO_FEATURE_EGL").is_ok() && target.contains("windows");

    let data = if egl { build_data::ANGLE } else { build_data::TRANSLATOR };

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
    for &(os, sources) in &[
        ("darwin", &[
            "gfx/angle/checkout/src/common/system_utils_mac.cpp",
            "gfx/angle/checkout/src/common/system_utils_posix.cpp",
        ][..]),
        ("linux", &[
            "gfx/angle/checkout/src/common/system_utils_linux.cpp",
            "gfx/angle/checkout/src/common/system_utils_posix.cpp",
        ][..]),
        ("windows", &["gfx/angle/checkout/src/common/system_utils_win.cpp"][..]),
    ] {
        if target.contains(os) {
            for source in sources {
                build.file(source);
            }
            break
        }
    }

    build
        .file("src/shaders/glslang-c.cpp")
        .cpp(true)
        .warnings(false)
        .flag("-std=c++14")
        .flag_if_supported("/wd4100")
        .flag_if_supported("/wd4127")
        .flag_if_supported("/wd9002");

    if target.contains("x86_64") || target.contains("i686") {
        build
            .flag_if_supported("-msse2")  // GNU
            .flag_if_supported("-arch:SSE2");  // MSVC
    }

    build.compile("angle");

    if egl {
        build_egl(&target);
    }

    for lib in data.os_libs {
        println!("cargo:rustc-link-lib={}", lib);
    }
    println!("cargo:rerun-if-changed=src/shaders/glslang-c.cpp");
    println!("cargo:rerun-if-changed=gfx");
}

fn fixup_path(path: &str) -> String {
    let prefix = "../../";
    assert!(path.starts_with(prefix));
    format!("gfx/angle/{}", &path[prefix.len()..])
}

#[cfg(not(feature = "egl"))]
fn generate_bindings() {}

#[cfg(feature = "egl")]
fn generate_bindings() {
    use gl_generator::{Registry, Api, Profile, Fallbacks};
    use std::fs::File;

    let target = env::var("TARGET").unwrap();
    if !target.contains("windows") {
        return
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let mut file = File::create(&out_dir.join("egl_bindings.rs")).unwrap();
    Registry::new(Api::Egl, (1, 5), Profile::Core, Fallbacks::All, [
        "EGL_ANGLE_device_d3d",
        "EGL_EXT_platform_base",
        "EGL_EXT_platform_device",
        "EGL_KHR_create_context",
        "EGL_EXT_create_context_robustness",
        "EGL_KHR_create_context_no_error",
    ])
        .write_bindings(gl_generator::StaticGenerator, &mut file)
        .unwrap();

    let mut file = File::create(&out_dir.join("gles_bindings.rs")).unwrap();
    Registry::new(Api::Gles2, (2, 0), Profile::Core, Fallbacks::None, [])
        .write_bindings(gl_generator::StaticGenerator, &mut file)
        .unwrap();
}
