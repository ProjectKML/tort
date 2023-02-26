use std::{fs, path::Path};

use spirv_builder::{Capability, MetadataPrintout, SpirvBuilder};

fn compile_shaders() {
    let result = SpirvBuilder::new("../tort_shaders", "spirv-unknown-spv1.4")
        .print_metadata(MetadataPrintout::DependencyOnly)
        .multimodule(true)
        .capability(Capability::MeshShadingEXT)
        .extension("SPV_EXT_mesh_shader")
        .build()
        .unwrap();

    let modules = result.module.unwrap_multi();
    for path in modules.values() {
        let target_path = Path::new("../../assets/shaders/").join(
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace('-', "_"),
        );

        fs::copy(path, target_path).unwrap();
    }
}

fn main() {
    compile_shaders();
}
