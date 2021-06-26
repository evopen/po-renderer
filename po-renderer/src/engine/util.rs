use spirv_builder::CompileResult;

pub fn handle_shader_compile(
    compile_result: CompileResult,
    tx: &crossbeam::channel::Sender<Vec<u8>>,
) {
    log::info!("shader incoming");
    let module = std::fs::read(compile_result.module.unwrap_single()).unwrap();
    tx.send(module).unwrap();
}

pub fn spirv_builder(path_to_crate: &str) -> spirv_builder::SpirvBuilder {
    spirv_builder::SpirvBuilder::new(path_to_crate, "spirv-unknown-vulkan1.2")
        .capability(spirv_builder::Capability::RayTracingKHR)
        .capability(spirv_builder::Capability::ImageQuery)
        .capability(spirv_builder::Capability::Int8)
        .capability(spirv_builder::Capability::Int16)
        .capability(spirv_builder::Capability::Int64)
        .capability(spirv_builder::Capability::RuntimeDescriptorArray)
        .extension("SPV_KHR_ray_tracing")
        .extension("SPV_EXT_descriptor_indexing")
        .name_variables(false)
        .print_metadata(spirv_builder::MetadataPrintout::None)
}
