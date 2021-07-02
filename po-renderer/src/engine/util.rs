use maligog::vk;
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
        .capability(spirv_builder::Capability::Linkage)
        .extension("SPV_KHR_ray_tracing")
        .extension("SPV_EXT_descriptor_indexing")
        .name_variables(false)
        .scalar_block_layout(true)
        .print_metadata(spirv_builder::MetadataPrintout::None)
}

pub fn cmd_blit_image(
    recorder: &mut maligog::CommandRecorder,
    src: &maligog::Image,
    dst: &maligog::Image,
) {
    recorder.blit_image(
        &src,
        maligog::ImageLayout::GENERAL,
        &dst,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &[vk::ImageBlit::builder()
            .src_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1)
                    .base_array_layer(0)
                    .mip_level(0)
                    .build(),
            )
            .src_offsets([
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: src.width() as i32,
                    y: src.height() as i32,
                    z: 1,
                },
            ])
            .dst_offsets([
                vk::Offset3D { x: 0, y: 0, z: 0 },
                vk::Offset3D {
                    x: dst.width() as i32,
                    y: dst.height() as i32,
                    z: 1,
                },
            ])
            .dst_subresource(
                vk::ImageSubresourceLayers::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1)
                    .base_array_layer(0)
                    .mip_level(0)
                    .build(),
            )
            .build()],
        vk::Filter::LINEAR,
    );
}
