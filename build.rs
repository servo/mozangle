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
    panic!("{}", std::env::vars().into_iter().map(|(k, v)| format!("{k}: {v}")).fold(String::new(), |a, b| a + &b + "\n"));
    let target = env::var("TARGET").unwrap();

    if cfg!(feature = "egl") && !target.contains("windows") {
        panic!("Do not know how to build EGL support for a non-Windows platform.");
    }

    if cfg!(feature = "build_dlls") && !target.contains("windows") {
        panic!("Do not know how to build DLLs for a non-Windows platform.");
    }

    let mut compiled_libraries: HashSet<Libs> = HashSet::new();

    build_translator(&mut compiled_libraries, &target);

    #[cfg(feature = "build_dlls")]
    {
        for lib in build_data::GLESv2.use_libs {
            build_lib(&mut compiled_libraries, &target, *lib);
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
            build_lib(&mut compiled_libraries, &target, Libs::EGL);
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
    // See: https://doc.rust-lang.org/cargo/reference/build-script-examples.html#using-another-sys-crate
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

    // compile source files to object files (workaround for clang-cl not supporting /MP)
    for file in data.sources {
        build.file(fixup_path(file));
    }
    let obj_paths = build.compile_intermediates();

    let build = cc::Build::new();
    let mut cmd = build.get_compiler().to_command();
    let out_string = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_string);

    // link with object files compiled previously
    for obj_path in &obj_paths {
        cmd.arg(obj_path);
    }

    for lib in data.use_libs {
        cmd.arg(out_path.join(format!("{}.lib", lib.to_data().lib)));
    }

    for lib in data.os_libs {
        cmd.arg(&format!("{}.lib", lib));
    }
    // also need to link zlib
    cmd.arg(&zlib_link_arg);

    if dll_name == "libGLESv2" {
        // transitive lib (that's the only case)
        cmd.arg(out_path.join("preprocessor.lib"));
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

fn build_lib(compiled_libraries: &mut HashSet<Libs>, target: &String, lib: Libs) {
    // Do not rebuild this library if it is already built.
    if compiled_libraries.contains(&lib) {
        return;
    }

    println!("build_lib: {lib:?}");
    let data = lib.to_data();
    for dep_lib in data.use_libs {
        build_lib(compiled_libraries, target, *dep_lib);
    }
    let repo = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    env::set_current_dir(repo).unwrap();

    let mut build = cc::Build::new();

    build.cpp(true).std("c++17").warnings(false);

    if let Ok(android_api) = env::var("ANDROID_API_LEVEL").as_deref() {
        build.define("__ANDROID_MIN_SDK_VERSION__", android_api);
    }

    for &(k, v) in data.defines {
        build.define(k, v);
    }

    if cfg!(feature = "build_dlls") {
        build.define("ANGLE_USE_EGL_LOADER", None);
    }

    for file in data.includes {
        build.include(fixup_path(file));
    }

    // add zlib from libz-sys to include path
    // See: https://doc.rust-lang.org/cargo/reference/build-script-examples.html#using-another-sys-crate
    if let Ok(zlib_include_dir) = env::var("DEP_Z_INCLUDE") {
        build.include(zlib_include_dir.replace("\\", "/"));
    }

    for file in data.sources {
        build.file(fixup_path(file));
    }

    if matches!(lib, Libs::ANGLE_COMMON) {
        // These platform-specific files are added conditionally in moz.build files
        // `if CONFIG['OS_ARCH'] == 'Darwin':`
        for &(os, sources) in &[
            (
                "darwin",
                &[
                    "gfx/angle/checkout/src/common/system_utils_mac.cpp",
                    "gfx/angle/checkout/src/common/system_utils_apple.cpp",
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
        .flag_if_supported("/wd9002")
        .flag_if_supported("-Wno-unused-command-line-argument");

    if target.contains("x86_64") || target.contains("i686") {
        if build.get_compiler().is_like_msvc() {
            build.flag_if_supported("-arch:SSE2");
        } else {
            build.flag_if_supported("-msse2");
        }
    }

    // Enable multiprocessing for faster builds.
    build.flag_if_supported("/MP");

    // we want all symbols as they are for consumers (are shared libs)
    if data.shared {
        build.link_lib_modifier("-bundle");
        build.link_lib_modifier("+whole-archive");
    }

    build.compile(data.lib);

    for lib in data.os_libs {
        println!("cargo:rustc-link-lib={}", lib);
    }

    compiled_libraries.insert(lib);
}

fn build_translator(compiled_libraries: &mut HashSet<Libs>, target: &String) {
    println!("build_translator");
    build_lib(compiled_libraries, target, Libs::TRANSLATOR);
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

    if let Ok(android_api) = env::var("ANDROID_API_LEVEL").as_deref() {
        clang_args.push(format!("-D__ANDROID_MIN_SDK_VERSION__={}", android_api));
    }

    let mut build = cc::Build::new();

    for flag in &clang_args {
        build.flag(flag);
    }

    build
        .cpp(true)
        .std("c++17")
        .warnings(false)
        .flag_if_supported("/wd4100")
        .flag_if_supported("/wd4127")
        .flag_if_supported("/wd9002");

    if target.contains("x86_64") || target.contains("i686") {
        if build.get_compiler().is_like_msvc() {
            build.flag_if_supported("-arch:SSE2");
        } else {
            build.flag_if_supported("-msse2");
        }
    }

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    if cfg!(feature = "dynamic_lib") {
        build
        .flag_if_supported("-shared")
        .flag_if_supported("-dynamiclib");

        let mut cmd = build.get_compiler().to_command();
        cmd.arg(out_dir.join(format!("lib{}.a", data.lib)));
        for lib in data.use_libs {
            cmd.arg(out_dir.join(format!("lib{}.a", lib.to_data().lib)));
        }

        cmd.arg("src/shaders/glslang-c.cpp");
        println!("cargo:rustc-link-lib=dylib={}", "glslang_glue");
        let target_os = std::env::var("CARGO_CFG_TARGET_OS").expect("Cargo error?");
        if target_os == "macos" {
            let file = out_dir.join("libglslang_glue.dylib");
            cmd.arg("-o").arg(&file);
        } else if target_os == "linux" {
            let file = out_dir.join(format!("libglslang_glue.so"));
            cmd.arg("-o").arg(&file);
        }

        let status = cmd.status().expect("Failed to link the dynamic library");
        assert!(status.success(), "Linking failed");

        println!("cargo:rustc-link-search=native={}", out_dir.display());
    } else {
        build.file("src/shaders/glslang-c.cpp");
        build.compile("glslang_glue");
    }

    let Ok(rust_target) = bindgen::RustTarget::stable(80, 0) else {
        // `InvalidRustTarget` doesn't implement debug so we manually panic.
        panic!("Invalid rust target specified");
    };

    // now generate bindings
    let mut builder = bindgen::builder()
        .rust_target(rust_target)
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

/// Make a path relative to the working directory that is used for the build.
fn fixup_path(path: &str) -> String {
    let prefix = "../../";
    assert!(path.starts_with(prefix));
    format!("gfx/angle/{}", &path[prefix.len()..])
}

#[cfg(feature = "egl")]
fn generate_gl_bindings() {
    println!("generate_gl_bindings");
    use gl_generator::{Api, Fallbacks, Profile, Registry};
    use std::{fs::File, io::Write};

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let mut file = File::create(&out_dir.join("egl_bindings.rs")).unwrap();
    file.write_all(b"#[allow(unused_imports)]\n").unwrap();
    Registry::new(
        Api::Egl,
        (1, 5),
        Profile::Core,
        Fallbacks::All,
        [
            "EGL_ANGLE_device_d3d",
            "EGL_EXT_platform_base",
            "EGL_EXT_platform_device",
            "EGL_EXT_device_query",
            "EGL_KHR_create_context",
            "EGL_EXT_create_context_robustness",
            "EGL_KHR_create_context_no_error",
        ],
    )
    .write_bindings(gl_generator::StaticGenerator, &mut file)
    .unwrap();

    let mut file = File::create(&out_dir.join("gles_bindings.rs")).unwrap();
    file.write_all(b"#[allow(unused_imports)]\n").unwrap();
    Registry::new(Api::Gles2, (2, 0), Profile::Core, Fallbacks::None, [])
        .write_bindings(gl_generator::StaticGenerator, &mut file)
        .unwrap();
}
