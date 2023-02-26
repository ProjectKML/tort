use spirv_builder::{MetadataPrintout, SpirvBuilder};

fn main() {
    let _result = SpirvBuilder::new("../tort_shaders", "spirv-unknown-spv1.4")
        .print_metadata(MetadataPrintout::DependencyOnly)
        .multimodule(true)
        .build()
        .unwrap();
}
