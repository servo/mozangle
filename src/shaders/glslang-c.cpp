#include "GLSLANG/ShaderLang.h"
#include "common/utilities.h"

extern "C" int GLSLangInitialize()
{
    if (sh::Initialize())
        return 1;
    return 0;
}

extern "C" int GLSLangFinalize()
{
    if (sh::Finalize())
        return 1;
    return 0;
}

extern "C" void GLSLangInitBuiltInResources(ShBuiltInResources *resources)
{
    sh::InitBuiltInResources(resources);
}

extern "C" const char *GLSLangGetBuiltInResourcesString(const ShHandle handle)
{
    return sh::GetBuiltInResourcesString(handle).c_str();
}

extern "C" ShHandle GLSLangConstructCompiler(unsigned int type,
                                             unsigned int spec,
                                             unsigned int output,
                                             const ShBuiltInResources *resources)
{
    return sh::ConstructCompiler(static_cast<sh::GLenum>(type),
                                 static_cast<ShShaderSpec>(spec),
                                 static_cast<ShShaderOutput>(output),
                                 resources);
}

extern "C" void GLSLangDestructCompiler(ShHandle handle)
{
    sh::Destruct(handle);
}

extern "C" int GLSLangCompile(const ShHandle handle,
                              const char *const shaderStrings[],
                              size_t numStrings,
                              const ShCompileOptions &compileOptions)
{
    if (sh::Compile(handle, shaderStrings, numStrings, compileOptions))
        return 1;

    return 0;
}

extern "C" void GLSLangClearResults(const ShHandle handle)
{
    sh::ClearResults(handle);
}

extern "C" int GLSLangGetShaderVersion(const ShHandle handle)
{
    return sh::GetShaderVersion(handle);
}

extern "C" int GLSLangGetShaderOutputType(const ShHandle handle)
{
    return sh::GetShaderOutputType(handle);
}

extern "C" const char *GLSLangGetInfoLog(const ShHandle handle)
{
    return sh::GetInfoLog(handle).c_str();
}

// Returns null-terminated object code for a compiled shader.
// Parameters:
// handle: Specifies the compiler
extern "C" const char *GLSLangGetObjectCode(const ShHandle handle)
{
    return sh::GetObjectCode(handle).c_str();
}

using StrPairFunction = void (*)(void *, const char *, size_t, const char *, size_t);

extern "C" void GLSLangIterUniformNameMapping(const ShHandle handle, StrPairFunction each, void *closure_each)
{
    for (auto &uniform : *sh::GetUniforms(handle))
    {
        each(
            closure_each,
            uniform.name.data(), uniform.name.length(),
            uniform.mappedName.data(), uniform.mappedName.length());
    }
}

// Returns the number of vectors that the shader's active varyings fit
// in to without additional packing. Can be used to test whether a
// shader will compile on drivers that do not perform spec-compliant
// packing. This contrasts with sh::CheckVariablesWithinPackingLimits
// which does pack the varyings in accordance with the spec.
extern "C" int GLSLangGetNumUnpackedVaryingVectors(const ShHandle handle)
{
    int total_rows = 0;
    const std::vector<sh::Varying> *varyings = sh::GetVaryings(handle);

    if (varyings)
    {
        for (const auto &varying : *varyings)
        {
            if (varying.active)
            {
                total_rows += gl::VariableRowCount(varying.type) * varying.getArraySizeProduct();
            }
        }
    }

    return total_rows;
}
