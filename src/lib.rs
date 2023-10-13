#[macro_use]
extern crate lazy_static;
// This extern crates are needed for linking
#[cfg(feature = "egl")]
extern crate libz_sys;
#[cfg(test)]
extern crate dlopen;

pub mod shaders;
#[cfg(test)]
mod tests;

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
        unsafe { ffi::GetProcAddress(name.as_ptr()) as *const _ as _ }
    }

    pub mod ffi {
        use std::os::raw::{c_long, c_void};

        include!(concat!(env!("OUT_DIR"), "/egl_bindings.rs"));

        // Adapted from https://github.com/tomaka/glutin/blob/1f3b8360cb/src/api/egl/ffi.rs
        #[allow(non_camel_case_types)]
        pub type khronos_utime_nanoseconds_t = khronos_uint64_t;
        #[allow(non_camel_case_types)]
        pub type khronos_uint64_t = u64;
        #[allow(non_camel_case_types)]
        pub type khronos_ssize_t = c_long;
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

        // EGL_ANGLE_platform_angle
        pub const PLATFORM_ANGLE_ANGLE: types::EGLenum = 0x3202;
        pub const PLATFORM_ANGLE_TYPE_ANGLE: types::EGLenum = 0x3203;
        pub const PLATFORM_ANGLE_MAX_VERSION_MAJOR_ANGLE: types::EGLenum = 0x3204;
        pub const PLATFORM_ANGLE_MAX_VERSION_MINOR_ANGLE: types::EGLenum = 0x3205;
        pub const PLATFORM_ANGLE_TYPE_DEFAULT_ANGLE: types::EGLenum = 0x3206;
        pub const PLATFORM_ANGLE_DEBUG_LAYERS_ENABLED_ANGLE: types::EGLenum = 0x3451;
        pub const PLATFORM_ANGLE_DEVICE_TYPE_ANGLE: types::EGLenum = 0x3209;
        pub const PLATFORM_ANGLE_DEVICE_TYPE_HARDWARE_ANGLE: types::EGLenum = 0x320A;
        pub const PLATFORM_ANGLE_DEVICE_TYPE_NULL_ANGLE: types::EGLenum = 0x345E;
        pub const PLATFORM_ANGLE_NATIVE_PLATFORM_TYPE_ANGLE: types::EGLenum = 0x348F;

        // EGL_ANGLE_feature_control
        pub const FEATURE_NAME_ANGLE: types::EGLenum = 0x3460;
        pub const FEATURE_CATEGORY_ANGLE: types::EGLenum = 0x3461;
        pub const FEATURE_DESCRIPTION_ANGLE: types::EGLenum = 0x3462;
        pub const FEATURE_BUG_ANGLE: types::EGLenum = 0x3463;
        pub const FEATURE_STATUS_ANGLE: types::EGLenum = 0x3464;
        pub const FEATURE_COUNT_ANGLE: types::EGLenum = 0x3465;
        pub const FEATURE_OVERRIDES_ENABLED_ANGLE: types::EGLenum = 0x3466;
        pub const FEATURE_OVERRIDES_DISABLED_ANGLE: types::EGLenum = 0x3467;
        pub const FEATURE_CONDITION_ANGLE: types::EGLenum = 0x3468;
        pub const FEATURE_ALL_DISABLED_ANGLE: types::EGLenum = 0x3469;

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
