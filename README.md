mozangle
========

Mozilla’s fork of Google ANGLE, repackaged as a Rust crate.

* [ANGLE] is an implementation of OpenGL ES. Its official build system is `gn`, from Chromium's
  [depot_tools].

* [mozilla/angle] on GitHub is a fork with some Gecko-specific patches.

* [`gfx/angle`] in mozilla-central is generated from that. [`update-angle.py`] runs `gn desc` to
  extract information from the official build system, copies relevant source files, and creates
  `moz.build` files for Gecko's build system.

* This repository imports a copy of the `gfx/angle` directory. The `generate_build_data.py` script
  turns data from `moz.build` files into a Rust source file. (This script supports just enough of
  the `moz.build` format for this specific purpose.) Finally, a Cargo build script drives the C++
  compilation with the [cc] crate based on that data.

[ANGLE]: https://chromium.googlesource.com/angle/angle
[depot_tools]: https://commondatastorage.googleapis.com/chrome-infra-docs/flat/depot_tools/docs/html/depot_tools_tutorial.html
[mozilla/angle]: https://github.com/mozilla/angle
[`gfx/angle`]: https://hg.mozilla.org/mozilla-central/file/tip/gfx/angle
[`update-angle.py`]: https://hg.mozilla.org/mozilla-central/file/tip/gfx/angle/update-angle.py
[cc]: https://crates.io/crates/cc


Feature flags
-------------

By default, this crate only compiles the shader translator.

In Windows, the `egl` Cargo feature enables the EGL and OpenGL ES implementations. Although upstream
ANGLE supports more platforms, this crate only configures the Direct3D 11 rendering backend.

```toml
[dependencies]
mozangle = { version = "0.3", features = ["egl"] }
```


Updating ANGLE
--------------

To update:

* Remove `gfx/angle` entirely
* Copy a new version of it from mozilla-central
* Apply any patches present in the `patches` directory
* Run `python3 generate_build_data.py`
* In the commit message, include the mozilla-central commit hash