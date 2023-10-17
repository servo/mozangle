#!/usr/bin/env python3

from os import path, listdir

REPO = path.dirname(__file__)
ANGLE = path.join(REPO, "gfx", "angle")


def libs():
    return listdir(path.join(ANGLE, "targets"))


def lib2const(s: str):
    return s.removeprefix("lib").upper().replace("V2", "v2")


def run():
    data = {}
    for lib in libs():
        data[lib] = {
            "DEFINES": {},
            "LOCAL_INCLUDES": [],
            "SOURCES": [],
            "USE_LIBS": [],
            "OS_LIBS": [],
            "SHARED": False,
        }
        directory = path.join(ANGLE, "targets", lib)
        # include("../../moz.build.common")
        parse_mozbuild(ANGLE, data[lib], ".common")
        # parse the rest
        parse_mozbuild(directory, data[lib])

    with open(path.join(REPO, "build_data.rs"), "wb") as f:
        write(data, f)


def parse_mozbuild(directory, data, suffix=""):
    mozbuild = path.join(directory, "moz.build" + suffix)
    env = {
        "include": noop,
        "Library": noop,
        "GeckoSharedLibrary": noop,
        "AllowCompilerWarnings": noop,
        "SRCDIR": directory,
        "CXXFLAGS": list(),
        "DIRS": [],
        "CONFIG": {
            "SSE2_FLAGS": "",
            "OS_ARCH": "neither",
            "INTEL_ARCHITECTURE": "true",
            "CC_TYPE": "gcc",
            "MOZ_X11_CFLAGS": "",
            "MOZ_WIDGET_TOOLKIT": "",
            "MOZ_WIDGET_GTK": "",
        },
    }
    env.update(data)
    with open(mozbuild) as f:
        readed = f.read()
        code = compile(readed, mozbuild, "exec")
        exec(code, env, env)
        if "GeckoSharedLibrary" in readed:
            data["SHARED"] = True


def noop(*_args, **_kwargs):
    pass


def write(data, f):
    f.write(
        b"// Generated from gfx/angle/**/moz.build by generate_build_data.py\n"
        b"// Do not edit directly. Instead, edit and run generate_build_data.py again.\n"
        b"#![allow(non_camel_case_types)]\n"
        b"\n"
        b"pub struct Data {\n"
        b"    pub lib: &'static str,\n"
        b"    pub sources: &'static [&'static str],\n"
        b"    pub includes: &'static [&'static str],\n"
        b"    pub defines: &'static [(&'static str, Option<&'static str>)],\n"
        b"    pub os_libs: &'static [&'static str],\n"
        b"    pub use_libs: &'static [Libs],\n"
        b"    pub shared: bool,\n"
        b"}\n"
        b"\n"
    )
    enum = "#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]\n"
    enum += "pub enum Libs {\n"
    for lib in libs():
        enum += f"    {lib2const(lib)},\n"
    enum += "}\n"
    enum += """
impl Libs {
    pub fn to_data(&self) -> Data {
        match self {
"""
    for lib in libs():
        enum += f"            Self::{lib2const(lib)} => {lib2const(lib)},\n"
    enum += """        }
    }
}

"""
    f.write(str.encode(enum))
    for lib in libs():
        write_lib(lib, data[lib], f)


def no_platform_sources(source):
    # Filter out any accidental inclusion of platform-specific source files.
    return "system_utils_posix.cpp" not in source and "system_utils_linux.cpp" not in source


def no_zlib(s):
    # Filter out any accidental inclusion of platform-specific source files.
    return "zlib" not in s


def write_lib(lib, data, f):
    name = str.encode(lib2const(lib))
    defines = [
        b"(%s, %s)" % (
            string_literal(k),
            b"None" if v is True else b"Some(%s)" % string_literal(v)
        )
        for k, v in data["DEFINES"].items()
    ]

    f.write(b"pub const %s: Data = Data {\n" % name)
    f.write(b"    lib: %s,\n" % string_literal(lib))
    write_list(b"sources", map(string_literal, filter(no_platform_sources, data["SOURCES"])), f)
    write_list(b"includes", map(string_literal, data["LOCAL_INCLUDES"]), f)
    write_list(b"defines", defines, f)
    write_list(b"os_libs", map(string_literal, data["OS_LIBS"]), f)
    write_list(b"use_libs", map(lib_enum, filter(no_zlib, data["USE_LIBS"])), f)
    if data["SHARED"]:
        f.write(b"    shared: true,\n")
    else:
        f.write(b"    shared: false,\n")
    f.write(b"};\n")


def lib_enum(s: str):
    return b"Libs::%s" % lib2const(s).encode("utf-8")


def string_literal(s):
    prelen = 1
    raw = repr(s).replace('"', '\\"')
    return b"\"%s\"" % raw[prelen:-prelen].encode("utf-8")


def write_list(name, items, f):
    items = sorted(set(items))
    f.write(b"    %s: &[\n" % name)
    for item in items:
        f.write(b"        %s,\n" % item)
    f.write(b"    ],\n")


if __name__ == '__main__':
    run()
