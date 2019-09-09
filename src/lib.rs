#[macro_use] extern crate lazy_static;
#[cfg(test)] extern crate dlopen;

pub mod shaders;
#[cfg(test)] mod tests;

#[cfg(all(windows, feature = "egl"))]
pub mod gles {
    pub mod ffi {
        include!(concat!(env!("OUT_DIR"), "/gles_bindings.rs"));
    }
}

#[cfg(all(windows, feature = "egl"))]
pub mod egl {
    use std::ffi::CString;
    use std::os::raw::c_void;

    pub fn get_proc_address(name: &str) -> *const c_void {
        let name = CString::new(name.as_bytes()).unwrap();
        unsafe {
            ffi::GetProcAddress(name.as_ptr()) as *const _ as _
        }
    }

    pub mod ffi {
        use std::os::raw::{c_void, c_long};

        include!(concat!(env!("OUT_DIR"), "/egl_bindings.rs"));

        // Adapted from https://github.com/tomaka/glutin/blob/1f3b8360cb/src/api/egl/ffi.rs
        #[allow(non_camel_case_types)] pub type khronos_utime_nanoseconds_t = khronos_uint64_t;
        #[allow(non_camel_case_types)] pub type khronos_uint64_t = u64;
        #[allow(non_camel_case_types)] pub type khronos_ssize_t = c_long;
        pub type EGLint = i32;
        pub type EGLNativeDisplayType = *const c_void;
        pub type EGLNativePixmapType = *const c_void;
        pub type EGLNativeWindowType = *const c_void;
        pub type NativeDisplayType = EGLNativeDisplayType;
        pub type NativePixmapType = EGLNativePixmapType;
        pub type NativeWindowType = EGLNativeWindowType;

        // Adapted from https://chromium.googlesource.com/angle/angle/+/master/include/EGL/eglext_angle.h
        pub type EGLDeviceEXT = *mut c_void;
        pub const EXPERIMENTAL_PRESENT_PATH_ANGLE: types::EGLenum = 0x33A4;
        pub const EXPERIMENTAL_PRESENT_PATH_FAST_ANGLE: types::EGLenum = 0x33A9;
        pub const D3D_TEXTURE_ANGLE: types::EGLenum = 0x33A3;
        pub const FLEXIBLE_SURFACE_COMPATIBILITY_SUPPORTED_ANGLE: types::EGLenum = 0x33A6;

        extern "C" {
            pub fn eglCreateDeviceANGLE(
                device_type: types::EGLenum,
                device: *mut c_void,
                attrib_list: *const types::EGLAttrib,
            ) -> EGLDeviceEXT;

            pub fn eglReleaseDeviceANGLE(device: EGLDeviceEXT) -> types::EGLBoolean;
        }
    }
}
