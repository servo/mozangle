use shaders::*;
use std::sync::{Once, ONCE_INIT};

static GLSLANG_INITIALIZATION: Once = ONCE_INIT;

fn init() {
    GLSLANG_INITIALIZATION.call_once(|| initialize().unwrap());
}

#[test]
fn test_linkage() {
    init();
}

#[cfg(all(windows, feature = "egl"))]
#[test]
fn test_egl_dll_linkage() {
    use dlopen::symbor::Library;
    use egl::ffi;
    let lib = Library::open("libEGL.dll").unwrap();
    let GetError = unsafe { lib.symbol::<unsafe extern "C" fn() -> u32>("eglGetError") }.unwrap();
    assert_eq!(unsafe { GetError() }, ffi::SUCCESS);
}

#[cfg(all(windows, feature = "egl"))]
#[test]
fn test_egl_linkage() {
    use egl::ffi;
    assert_eq!(unsafe { ffi::GetError() } as u32, ffi::SUCCESS);
}

#[test]
fn test_translation_complex() {
    init();
    const FRAGMENT_SHADER: u32 = 0x8B30;
    let source = "
precision mediump float;
varying vec2 vTextureCoord;
uniform sampler2D uSampler;
void main() {
  gl_FragColor = texture2D(uSampler, vTextureCoord);
}
";
    let resources = BuiltInResources::default();
    let compiler = ShaderValidator::for_webgl(FRAGMENT_SHADER, Output::Glsl, &resources).unwrap();

    assert!(compiler.compile_and_translate(&[source]).is_ok());

    let map = compiler.uniform_name_map();
    let keys = map.keys().collect::<Vec<_>>();
    assert_eq!(keys, &["uSampler"], "name hashing map: {:?}", map)
}

#[test]
fn test_translation() {
    const SHADER: &'static str = "void main() {
gl_FragColor = vec4(0, 1, 0, 1);  // green
}";
    const EXPECTED: &'static str = "void main(){
(gl_FragColor = vec4(0.0, 1.0, 0.0, 1.0));
}\n";
    const FRAGMENT_SHADER: u32 = 0x8B30;

    init();

    let resources = BuiltInResources::default();
    let compiler = ShaderValidator::for_webgl(FRAGMENT_SHADER, Output::Glsl, &resources).unwrap();

    let result = compiler.compile_and_translate(&[SHADER]).unwrap();
    println!("{:?}", result);
    // Use result.contains instead of equal because Angle may add some extensions such as
    // "#extension GL_ARB_gpu_shader5 : enable" on some platorms and compilation options.
    // See TranslatorGLSL.cpp for more details.
    assert!(result.contains(EXPECTED));
}

#[test]
fn test_translation_essl() {
    const SHADER: &'static str = "void main() {
gl_FragColor = vec4(0, 1, 0, 1);  // green
}";
    const EXPECTED: &'static str = "void main(){
(gl_FragColor = vec4(0.0, 1.0, 0.0, 1.0));
}\n";
    const FRAGMENT_SHADER: u32 = 0x8B30;

    init();

    let compiler =
        ShaderValidator::for_webgl(FRAGMENT_SHADER, Output::Essl, &BuiltInResources::default())
            .expect("Failed to create a validator for essl");

    let result = compiler.compile_and_translate(&[SHADER]).unwrap();
    println!("{:?}", result);
    assert!(result.contains(EXPECTED));
}
