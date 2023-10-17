#![allow(non_upper_case_globals)]

extern crate cc;
#[cfg(feature = "egl")]
extern crate gl_generator;
extern crate walkdir;

extern crate bindgen;

use std::collections::HashSet;
use std::env;
#[cfg(feature = "egl")]
use std::path::Path;
use std::path::PathBuf;

use bindgen::Formatter;

use crate::build_data::Libs;

mod build_data;

fn main() {
    let target = env::var("TARGET").unwrap();

    if cfg!(feature = "egl") && !target.contains("windows") {
        panic!("Do not know how to build EGL support for a non-Windows platform.");
    }

    if cfg!(feature = "build_dlls") && !target.contains("windows") {
        panic!("Do not know how to build DLLs for a non-Windows platform.");
    }

    // Contains compiled libs
    let mut libs: HashSet<Libs> = HashSet::new();

    build_translator(&mut libs, &target);

    #[cfg(feature = "build_dlls")]
    {
        for lib in build_data::GLESv2.use_libs {
            build_lib(&mut libs, &target, *lib);
        }
        build_windows_dll(
            &build_data::GLESv2,
            "libGLESv2",
            "gfx/angle/checkout/src/libGLESv2/libGLESv2_autogen.def",
        );
        build_windows_dll(
            &build_data::EGL,
            "libEGL",
            "gfx/angle/checkout/src/libEGL/libEGL_autogen.def",
        );

        let out = env::var("OUT_DIR").unwrap();
        println!("cargo:rustc-link-search={out}");
        println!("cargo:rustc-link-lib=libEGL");
    }

    #[cfg(feature = "egl")]
    {
        if !cfg!(feature = "build_dlls") {
            build_lib(&mut libs, &target, Libs::EGL);
        }
        generate_gl_bindings();
    }

    for entry in walkdir::WalkDir::new("gfx") {
        let entry = entry.unwrap();
        println!(
            "{}",
            format!("cargo:rerun-if-changed={}", entry.path().display())
        );
    }
}

#[cfg(feature = "build_dlls")]
fn build_windows_dll(data: &build_data::Data, dll_name: &str, def_file: &str) {
    println!("build_windows_dll: {dll_name}");
    let mut build = cc::Build::new();
    build.cpp(true);
    build.std("c++17");
    for &(k, v) in data.defines {
        build.define(k, v);
    }
    // add zlib from libz-sys to include path
    let zlib_link_arg = if let Ok(zlib_include_dir) = env::var("DEP_Z_INCLUDE") {
        build.include(zlib_include_dir.replace("\\", "/"));
        PathBuf::from(zlib_include_dir)
            .parent()
            .unwrap()
            .join("lib")
            .join("z.lib")
            .as_path()
            .display()
            .to_string()
    } else {
        String::from("z.lib")
    };
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

    for lib in data.use_libs {
        cmd.arg(out_path.join(format!("{}.lib", lib.to_data().lib)));
    }

    for lib in data.os_libs {
        cmd.arg(&format!("{}.lib", lib));
    }
    // also need to link zlib
    cmd.arg(&zlib_link_arg);

    if dll_name == "libGLESv2" {
        //std::fs::rename(out_path.join("libGLESv2.lib"), out_path.join("libGLESv2_static.lib")).unwrap();
        //cmd.arg(out_path.join("libGLESv2_static.lib"));
        // transitive lib (that's the only case)
        cmd.arg(out_path.join("preprocessor.lib"));
        for file in data.sources {
            //if !file.contains("libANGLE") {
            cmd.arg(fixup_path(file));
            //}
        }
    } else {
        for file in data.sources {
            cmd.arg(fixup_path(file));
        }
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

fn build_lib(libs: &mut HashSet<Libs>, target: &String, lib: Libs) {
    // Check if we already built it do not rebuild
    if libs.contains(&lib) {
        return;
    }

    println!("build_lib: {lib:?}");
    let data = lib.to_data();
    for dep_lib in data.use_libs {
        build_lib(libs, target, *dep_lib);
    }
    let repo = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    env::set_current_dir(repo).unwrap();

    // Change to one of the directory that contains moz.build
    let mut build = cc::Build::new();

    build.cpp(true).std("c++17").warnings(false);

    for &(k, v) in data.defines {
        build.define(k, v);
    }

    if cfg!(feature = "build_dlls") {
        build.define("ANGLE_USE_EGL_LOADER", None);
    }

    for file in data.includes {
        build.include(fixup_path(file));
    }

    //if matches!(lib, Libs::COMPRESSION_UTILS_PORTABLE) {
    // add zlib from libz-sys to include path
    if let Ok(zlib_include_dir) = env::var("DEP_Z_INCLUDE") {
        build.include(zlib_include_dir.replace("\\", "/"));
    }
    //}

    for file in data.sources {
        build.file(fixup_path(file));
    }

    if matches!(lib, Libs::ANGLE_COMMON) {
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
                &[
                    "gfx/angle/checkout/src/common/system_utils_win.cpp",
                    "gfx/angle/checkout/src/common/system_utils_win32.cpp",
                ][..],
            ),
        ] {
            if target.contains(os) {
                for source in sources {
                    build.file(source);
                }
                break;
            }
        }
    }

    build
        .flag_if_supported("/wd4100")
        .flag_if_supported("/wd4127")
        .flag_if_supported("/wd9002");

    if target.contains("x86_64") || target.contains("i686") {
        build
            .flag_if_supported("-msse2") // GNU
            .flag_if_supported("-arch:SSE2"); // MSVC
    }

    // Enable multiprocessing for faster builds.
    build.flag_if_supported("/MP");

    //build.link_lib_modifier("-whole-archive");

    build.compile(data.lib);

    for lib in data.os_libs {
        println!("cargo:rustc-link-lib={}", lib);
    }

    libs.insert(lib);
}

fn build_translator(libs: &mut HashSet<Libs>, target: &String) {
    println!("build_translator");
    build_lib(libs, target, Libs::TRANSLATOR);
    let data = build_data::TRANSLATOR;

    let repo = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    env::set_current_dir(repo).unwrap();

    // common clang args
    let mut clang_args = vec![];

    for &(k, v) in data.defines {
        if let Some(v) = v {
            clang_args.push(format!("-D{}={}", k, v));
        } else {
            clang_args.push(format!("-D{}", k));
        }
    }

    for file in data.includes {
        clang_args.push(String::from("-I"));
        clang_args.push(fixup_path(file));
    }

    // Change to one of the directory that contains moz.build
    let mut build = cc::Build::new();

    for flag in &clang_args {
        build.flag(flag);
    }

    build
        .file("src/shaders/glslang-c.cpp")
        .cpp(true)
        .std("c++17")
        .warnings(false)
        .flag_if_supported("/wd4100")
        .flag_if_supported("/wd4127")
        .flag_if_supported("/wd9002");

    if target.contains("x86_64") || target.contains("i686") {
        build
            .flag_if_supported("-msse2") // GNU
            .flag_if_supported("-arch:SSE2"); // MSVC
    }

    build.link_lib_modifier("-whole-archive");

    build.compile("glslang_glue");

    // now generate bindings
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let mut builder = bindgen::builder()
        .rust_target(bindgen::RustTarget::Stable_1_59)
        .header("./src/shaders/glslang-c.cpp")
        .opaque_type("std.*")
        .allowlist_type("Sh.*")
        .allowlist_var("SH.*")
        .rustified_enum("Sh.*")
        .formatter(Formatter::Rustfmt)
        .clang_args(["-I", "gfx/angle/checkout/include"])
        .clang_args(clang_args)
        // ensure cxx
        .clang_arg("-x")
        .clang_arg("c++")
        .clang_arg("-std=c++17");

    if target.contains("x86_64") || target.contains("i686") {
        builder = builder.clang_arg("-msse2")
    }

    for func in ALLOWLIST_FN {
        builder = builder.allowlist_function(func)
    }

    builder
        .generate()
        .expect("Should generate shader bindings")
        .write_to_file(out_dir.join("glslang_glue_bindings.rs"))
        .expect("Should write bindings to file");

    println!("cargo:rerun-if-changed=src/shaders/glslang-c.cpp");
}

const ALLOWLIST_FN: &'static [&'static str] = &[
    "GLSLangInitialize",
    "GLSLangFinalize",
    "GLSLangInitBuiltInResources",
    "GLSLangConstructCompiler",
    "GLSLangDestructCompiler",
    "GLSLangCompile",
    "GLSLangClearResults",
    "GLSLangGetShaderVersion",
    "GLSLangGetShaderOutputType",
    "GLSLangGetObjectCode",
    "GLSLangGetInfoLog",
    "GLSLangIterUniformNameMapping",
    "GLSLangGetNumUnpackedVaryingVectors",
];

fn fixup_path(path: &str) -> String {
    let prefix = "../../";
    assert!(path.starts_with(prefix));
    format!("gfx/angle/{}", &path[prefix.len()..])
}

#[cfg(feature = "egl")]
fn generate_gl_bindings() {
    println!("generate_gl_bindings");
    use gl_generator::{Api, Fallbacks, Profile, Registry};
    use std::fs::File;

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
