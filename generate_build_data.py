#!/usr/bin/env python3

from os import path

REPO = path.dirname(__file__)
ANGLE = path.join(REPO, "gfx", "angle")

def run():
    data = {}
    for lib in ["translator", "libEGL", "libGLESv2"]:
        data[lib] = {
            "DEFINES": {},
            "LOCAL_INCLUDES": [],
            "SOURCES": [],
            "USE_LIBS": [],
            "OS_LIBS": [],
        }
        directory = path.join(ANGLE, "targets", lib)
        parse_lib(directory, data[lib])
        parse_lib(ANGLE, data[lib], ".common")

    with open(path.join(REPO, "build_data.rs"), "wb") as f:
        write(data, f)

def parse_lib(directory, data, suffix=""):
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
        code = compile(f.read(), mozbuild, "exec")
        exec(code, env, env)
    for dir in env["DIRS"]:
        prefix = "../"
        assert dir.startswith(prefix)
        directory = path.join(ANGLE, "targets", dir[len(prefix):])
        parse_lib(directory, data)

def noop(*_args, **_kwargs):
    pass

def write(data, f):
    f.write(
        b"// Generated from gfx/angle/**/moz.build by generate_build_data.py\n"
        b"// Do not edit directly. Instead, edit and run generate_build_data.py again.\n"
        b"\n"
        b"pub struct Data {\n"
        b"     pub sources: &'static [&'static str],\n"
        b"     pub includes: &'static [&'static str],\n"
        b"     pub defines: &'static [(&'static str, Option<&'static str>)],\n"
        b"     pub os_libs: &'static [&'static str],\n"
        b"}\n"
        b"\n"
    )
    write_lib(b"TRANSLATOR", data["translator"], f)
    write_lib(b"EGL", data["libEGL"], f)
    write_lib(b"GLESv2", data["libGLESv2"], f)

def no_platform_sources(source):
    # Filter out any accidental inclusion of platform-specific source files.
    return "system_utils_posix.cpp" not in source and "system_utils_linux.cpp" not in source

def write_lib(name, data, f):
    defines = [
        b"(%s, %s)" % (
            string_literal(k),
            b"None" if v is True else b"Some(%s)" % string_literal(v)
        )
        for k, v in data["DEFINES"].items()
    ]

    f.write(b"pub const %s: Data = Data {\n" % name)
    write_list(b"sources", map(string_literal, filter(no_platform_sources, data["SOURCES"])), f)
    write_list(b"includes", map(string_literal, data["LOCAL_INCLUDES"]), f)
    write_list(b"defines", defines, f)
    write_list(b"os_libs", map(string_literal, data["OS_LIBS"]), f)
    f.write(b"};\n")

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
