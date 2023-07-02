#![allow(non_upper_case_globals)]

extern crate cc;
#[cfg(feature = "egl")]
extern crate gl_generator;
extern crate walkdir;

use std::env;
#[cfg(feature = "egl")]
use std::path::Path;
use std::path::PathBuf;

mod build_data;

fn main() {
    let target = env::var("TARGET").unwrap();
    let egl = env::var("CARGO_FEATURE_EGL").is_ok() && target.contains("windows");

    if cfg!(feature = "egl") && !target.contains("windows") {
        panic!("Do not know how to build EGL support for a non-Windows platform.");
    }

    if cfg!(feature = "build_dlls") && !target.contains("windows") {
        panic!("Do not know how to build DLLs for a non-Windows platform.");
    }

    build_angle(&target, egl);

    #[cfg(feature = "egl")]
    {
        build_egl(&target);
    }

    #[cfg(feature = "egl")]
    {
        generate_bindings();
    }

    #[cfg(feature = "build_dlls")]
    {
        build_windows_dll(
            &build_data::EGL,
            "libEGL",
            "gfx/angle/checkout/src/libEGL/libEGL.def",
        );
        build_windows_dll(
            &build_data::GLESv2,
            "libGLESv2",
            "gfx/angle/checkout/src/libGLESv2/libGLESv2_autogen.def",
        );
    }
}

#[cfg(feature = "build_dlls")]
fn build_windows_dll(data: &build_data::Data, dll_name: &str, def_file: &str) {
    let mut build = cc::Build::new();
    for &(k, v) in data.defines {
        build.define(k, v);
    }
    build.define("ANGLE_USE_EGL_LOADER", None);
    build
        .flag_if_supported("/wd4100")
        .flag_if_supported("/wd4127")
        .flag_if_supported("/wd9002");

    for file in data.includes {
        build.include(fixup_path(file));
    }

    let mut cmd = build.get_compiler().to_command();
    let out_string = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_string);

    // Always include the base angle code.
    cmd.arg(out_path.join("angle.lib"));

    for lib in data.os_libs {
        cmd.arg(&format!("{}.lib", lib));
    }

    for file in data.sources {
        cmd.arg(fixup_path(file));
    }

    // Enable multiprocessing for faster builds.
    cmd.arg("/MP");
    // Specify the creation of a DLL.
    cmd.arg("/LD"); // Create a DLL.
                    // Specify the name of the DLL.
    cmd.arg(format!("/Fe{}", out_path.join(dll_name).display()));
    // Temporary obj files should go into the output directory. The slash
    // at the end is required for multiple source inputs.
    cmd.arg(format!("/Fo{}\\", out_path.display()));

    // Specify the def file for the linker.
    cmd.arg("/link");
    cmd.arg(format!("/DEF:{def_file}"));

    let status = cmd.status();
    assert!(status.unwrap().success());
}

#[cfg(feature = "egl")]
fn build_egl(target: &str) {
    let mut build = cc::Build::new();

    let data = build_data::EGL;
    for &(k, v) in data.defines {
        build.define(k, v);
    }

    if cfg!(feature = "build_dlls") {
        build.define("ANGLE_USE_EGL_LOADER", None);
    }

    for file in data.includes {
        build.include(fixup_path(file));
    }

    for file in data.sources {
        build.file(fixup_path(file));
    }

    if target.contains("x86_64") || target.contains("i686") {
        build
            .flag_if_supported("-msse2") // GNU
            .flag_if_supported("-arch:SSE2"); // MSVC
    }

    build
        .flag_if_supported("/wd4100")
        .flag_if_supported("/wd4127")
        .flag_if_supported("/wd9002");

    build.link_lib_modifier("-whole-archive");

    // Build lib.
    build.compile("EGL");
}

fn build_angle(target: &String, egl: bool) {
    let data = if egl {
        build_data::ANGLE
    } else {
        build_data::TRANSLATOR
    };

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
        (
            "darwin",
            &[
                "gfx/angle/checkout/src/common/system_utils_mac.cpp",
                "gfx/angle/checkout/src/common/system_utils_posix.cpp",
            ][..],
        ),
        (
            "linux",
            &[
                "gfx/angle/checkout/src/common/system_utils_linux.cpp",
                "gfx/angle/checkout/src/common/system_utils_posix.cpp",
            ][..],
        ),
        (
            "windows",
            &["gfx/angle/checkout/src/common/system_utils_win.cpp"][..],
        ),
    ] {
        if target.contains(os) {
            for source in sources {
                build.file(source);
            }
            break;
        }
    }

    build
        .file("src/shaders/glslang-c.cpp")
        .cpp(true)
        .warnings(false)
        .flag_if_supported("-std=c++14")
        .flag_if_supported("/wd4100")
        .flag_if_supported("/wd4127")
        .flag_if_supported("/wd9002");

    if target.contains("x86_64") || target.contains("i686") {
        build
            .flag_if_supported("-msse2") // GNU
            .flag_if_supported("-arch:SSE2"); // MSVC
    }

    build.link_lib_modifier("-whole-archive");

    build.compile("angle");

    for lib in data.os_libs {
        println!("cargo:rustc-link-lib={}", lib);
    }
    println!("cargo:rerun-if-changed=src/shaders/glslang-c.cpp");
    for entry in walkdir::WalkDir::new("gfx") {
        let entry = entry.unwrap();
        println!(
            "{}",
            format!("cargo:rerun-if-changed={}", entry.path().display())
        );
    }
}

fn fixup_path(path: &str) -> String {
    let prefix = "../../";
    assert!(path.starts_with(prefix));
    format!("gfx/angle/{}", &path[prefix.len()..])
}

#[cfg(feature = "egl")]
fn generate_bindings() {
    use gl_generator::{Api, Fallbacks, Profile, Registry};
    use std::fs::File;

    let target = env::var("TARGET").unwrap();
    if !target.contains("windows") {
        return;
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let mut file = File::create(&out_dir.join("egl_bindings.rs")).unwrap();
    Registry::new(
        Api::Egl,
        (1, 5),
        Profile::Core,
        Fallbacks::All,
        [
            "EGL_ANGLE_device_d3d",
            "EGL_EXT_platform_base",
            "EGL_EXT_platform_device",
            "EGL_KHR_create_context",
            "EGL_EXT_create_context_robustness",
            "EGL_KHR_create_context_no_error",
        ],
    )
    .write_bindings(gl_generator::StaticGenerator, &mut file)
    .unwrap();

    let mut file = File::create(&out_dir.join("gles_bindings.rs")).unwrap();
    Registry::new(Api::Gles2, (2, 0), Profile::Core, Fallbacks::None, [])
        .write_bindings(gl_generator::StaticGenerator, &mut file)
        .unwrap();
}
