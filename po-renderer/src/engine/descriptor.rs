use maligog::vk;

pub struct DescriptorHelper {
    descriptor_pool: maligog::DescriptorPool,
    sampled_image_set_layout: maligog::DescriptorSetLayout,
    storage_image_set_layout: maligog::DescriptorSetLayout,
    storage_buffer_set_layout: maligog::DescriptorSetLayout,
    as_set_layout: maligog::DescriptorSetLayout,
    sampled_image_set: maligog::DescriptorSet,
    storage_image_set: maligog::DescriptorSet,
    storage_buffer_set: maligog::DescriptorSet,
    as_set: maligog::DescriptorSet,
}

impl DescriptorHelper {
    pub fn new(device: &maligog::Device) -> Self {
        let descriptor_pool = device.create_descriptor_pool(
            &[
                maligog::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(100)
                    .build(),
                maligog::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::STORAGE_BUFFER)
                    .descriptor_count(100)
                    .build(),
                maligog::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(100)
                    .build(),
                maligog::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::SAMPLER)
                    .descriptor_count(100)
                    .build(),
                maligog::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                    .descriptor_count(100)
                    .build(),
            ],
            1000,
        );
        let storage_image_set_layout = device.create_descriptor_set_layout(
            Some("storage image"),
            &[maligog::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: maligog::DescriptorType::StorageImage,
                stage_flags: maligog::ShaderStageFlags::ALL,
                descriptor_count: 100,
                variable_count: true,
            }],
        );

        let as_set_layout = device.create_descriptor_set_layout(
            Some("acceleration structure"),
            &[maligog::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: maligog::DescriptorType::AccelerationStructure,
                stage_flags: maligog::ShaderStageFlags::ALL,
                descriptor_count: 100,
                variable_count: true,
            }],
        );
        let sampled_image_set_layout = device.create_descriptor_set_layout(
            Some("sampled image"),
            &[maligog::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: maligog::DescriptorType::SampledImage,
                stage_flags: maligog::ShaderStageFlags::MISS_KHR,
                descriptor_count: 100,
                variable_count: true,
            }],
        );
        let storage_buffer_set_layout = device.create_descriptor_set_layout(
            Some("stroage buffer"),
            &[maligog::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: maligog::DescriptorType::StorageBuffer,
                stage_flags: maligog::ShaderStageFlags::ALL,
                descriptor_count: 100,
                variable_count: true,
            }],
        );

        let storage_image_set = device.allocate_descriptor_set(
            Some("storage image"),
            &&descriptor_pool,
            &storage_image_set_layout,
        );
        let as_set = device.allocate_descriptor_set(
            Some("storage image"),
            &&descriptor_pool,
            &as_set_layout,
        );
        let sampled_image_set = device.allocate_descriptor_set(
            Some("storage image"),
            &&descriptor_pool,
            &sampled_image_set_layout,
        );
        let storage_buffer_set = device.allocate_descriptor_set(
            Some("storage image"),
            &&descriptor_pool,
            &storage_buffer_set_layout,
        );

        Self {
            descriptor_pool,
            sampled_image_set_layout,
            storage_image_set_layout,
            storage_buffer_set_layout,
            as_set_layout,
            sampled_image_set,
            storage_image_set,
            storage_buffer_set,
            as_set,
        }
    }
}
