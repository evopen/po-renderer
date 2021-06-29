use super::util;
use glam::Vec3;
use maligog::{vk, Device};
use maplit::btreemap;

pub struct CameraInfo {
    view_inv: glam::Mat4,
    proj_inv: glam::Mat4,
}

pub struct RenderSettings {
    width: u32,
    height: u32,
    max_bounce: u32,
    camera: super::Camera,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            max_bounce: 5,
            camera: super::Camera::new(
                Vec3::new(0.0, 0.0, 10.0),
                Vec3::splat(0.0),
                16 as f32 / 9 as f32,
                std::f32::consts::FRAC_PI_3,
            ),
        }
    }
}

pub struct RenderResult {
    name: String,
    image: maligog::Image,
}

pub struct Po {
    depth_pipeline: maligog::RayTracingPipeline,
    rx: crossbeam::channel::Receiver<Vec<u8>>,
    device: Device,
    pipeline_layout: maligog::PipelineLayout,
    descriptor_pool: maligog::DescriptorPool,
    as_descriptor_set_layout: maligog::DescriptorSetLayout,
}

impl Po {
    pub fn new(device: &Device) -> Self {
        let sky_sampler = device.create_sampler(
            Some("sky"),
            maligog::Filter::LINEAR,
            maligog::Filter::LINEAR,
            maligog::SamplerAddressMode::CLAMP_TO_EDGE,
            maligog::SamplerAddressMode::CLAMP_TO_EDGE,
        );
        log::debug!("creating image descriptor set layout");

        log::debug!("creating as descriptor set layout");

        // descriptor set 0
        let as_descriptor_set_layout = device.create_descriptor_set_layout(
            Some("ray tracing as"),
            &[
                maligog::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: maligog::DescriptorType::AccelerationStructure,
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 1,
                    variable_count: false,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: maligog::DescriptorType::StorageBuffer,
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 1,
                    variable_count: false,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 2,
                    descriptor_type: maligog::DescriptorType::StorageBuffer,
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 1,
                    variable_count: false,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 3,
                    descriptor_type: maligog::DescriptorType::StorageBuffer,
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 1,
                    variable_count: false,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 4,
                    descriptor_type: maligog::DescriptorType::StorageBuffer,
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 1,
                    variable_count: false,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 5,
                    descriptor_type: maligog::DescriptorType::StorageBuffer,
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 1,
                    variable_count: false,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 6,
                    descriptor_type: maligog::DescriptorType::Sampler(None),
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 500,
                    variable_count: false,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 7,
                    descriptor_type: maligog::DescriptorType::SampledImage,
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 500,
                    variable_count: false,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 8,
                    descriptor_type: maligog::DescriptorType::StorageBuffer,
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 1,
                    variable_count: false,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 9,
                    descriptor_type: maligog::DescriptorType::StorageBuffer,
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 1,
                    variable_count: false,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 10,
                    descriptor_type: maligog::DescriptorType::StorageBuffer,
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 1,
                    variable_count: false,
                },
            ],
        );

        // descriptor set 1
        let image_descriptor_set_layout = device.create_descriptor_set_layout(
            Some("ray tracing image"),
            &[
                maligog::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: maligog::DescriptorType::StorageImage,
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 1,
                    variable_count: false,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: maligog::DescriptorType::StorageImage,
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 1,
                    variable_count: false,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 2,
                    descriptor_type: maligog::DescriptorType::Sampler(Some(sky_sampler)),
                    stage_flags: maligog::ShaderStageFlags::ALL,
                    descriptor_count: 1,
                    variable_count: false,
                },
            ],
        );

        log::debug!("creating skymap descriptor set layout");
        let skymap_descriptor_set_layout = device.create_descriptor_set_layout(
            Some("ray tracing skymap"),
            &[maligog::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: maligog::DescriptorType::SampledImage,
                stage_flags: maligog::ShaderStageFlags::MISS_KHR,
                descriptor_count: 1,
                variable_count: false,
            }],
        );
        let pipeline_layout = device.create_pipeline_layout(
            Some("ray tracing"),
            &[
                &as_descriptor_set_layout,
                &image_descriptor_set_layout,
                &skymap_descriptor_set_layout,
            ],
            &[maligog::PushConstantRange::builder()
                .offset(0)
                .size(std::mem::size_of::<CameraInfo>() as u32)
                .stage_flags(
                    maligog::ShaderStageFlags::RAYGEN_KHR
                        | maligog::ShaderStageFlags::CLOSEST_HIT_KHR,
                )
                .build()],
        );
        let (tx, rx) = crossbeam::channel::bounded(1);
        let builder = util::spirv_builder("./shaders/po");
        let tx1 = tx.clone();
        let result = builder
            .watch(move |result| {
                crate::engine::util::handle_shader_compile(result, &tx1);
            })
            .unwrap();
        crate::engine::util::handle_shader_compile(result, &tx);

        let spirv = rx.recv().unwrap();

        log::debug!("creating shader module");
        let module = device.create_shader_module(spirv);

        log::debug!("creating pipeline");
        let depth_pipeline = Self::build_pipeline(
            device,
            &pipeline_layout,
            &maligog::ShaderStage::new(&module, maligog::ShaderStageFlags::RAYGEN_KHR, "main"),
            &[&maligog::ShaderStage::new(
                &module,
                maligog::ShaderStageFlags::MISS_KHR,
                "miss",
            )],
            &[&maligog::TrianglesHitGroup::new(
                &maligog::ShaderStage::new(
                    &module,
                    maligog::ShaderStageFlags::CLOSEST_HIT_KHR,
                    "depth_closest_hit",
                ),
                None,
            )],
        );

        let descriptor_pool = device.create_descriptor_pool(
            &[
                maligog::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(2)
                    .build(),
                maligog::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::SAMPLED_IMAGE)
                    .descriptor_count(2)
                    .build(),
                maligog::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::SAMPLER)
                    .descriptor_count(2)
                    .build(),
                maligog::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                    .descriptor_count(1)
                    .build(),
            ],
            10,
        );

        log::debug!("allocating as descriptor set");
        let as_descriptor_set = device.allocate_descriptor_set(
            Some("as descriptor set"),
            &descriptor_pool,
            &as_descriptor_set_layout,
        );
        log::debug!("allocating skymap descriptor set");
        let skymap_descriptor_set = device.allocate_descriptor_set(
            Some("skymap descriptor set"),
            &descriptor_pool,
            &skymap_descriptor_set_layout,
        );

        let default_sampler = device.create_sampler(
            Some("rt default sampler"),
            maligog::Filter::NEAREST,
            maligog::Filter::NEAREST,
            maligog::SamplerAddressMode::CLAMP_TO_EDGE,
            maligog::SamplerAddressMode::CLAMP_TO_EDGE,
        );
        Self {
            depth_pipeline,
            rx,
            device: device.clone(),
            pipeline_layout,
            descriptor_pool,
            as_descriptor_set_layout,
        }
    }

    fn build_pipeline(
        device: &Device,
        pipeline_layout: &maligog::PipelineLayout,
        ray_gen_shader: &maligog::ShaderStage,
        miss_shaders: &[&maligog::ShaderStage],
        hit_groups: &[&dyn maligog::HitGroup],
    ) -> maligog::RayTracingPipeline {
        let pipeline = device.create_ray_tracing_pipeline(
            Some("ray tracing"),
            pipeline_layout,
            ray_gen_shader,
            miss_shaders,
            hit_groups,
            31,
        );
        pipeline
    }

    pub fn render(
        &mut self,
        settings: &RenderSettings,
        scene: &maligog_gltf::Scene,
    ) -> Vec<RenderResult> {
        let depth_image = self.device.create_image(
            Some("depth"),
            maligog::Format::R32_SFLOAT,
            settings.width,
            settings.height,
            maligog::ImageUsageFlags::STORAGE,
            maligog::MemoryLocation::GpuOnly,
        );

        let mut hit_groups: Vec<u32> = Vec::new();
        for i in 0..12345 {
            hit_groups.push(0);
        }
        let shader_binding_tables = self
            .depth_pipeline
            .create_shader_binding_tables(&hit_groups);

        Vec::new()
    }

    fn update(&mut self) {
        if let Ok(spirv) = self.rx.try_recv() {
            log::info!("updating shader");
            let module = self.device.create_shader_module(spirv);

            self.depth_pipeline = Self::build_pipeline(
                &self.device,
                &self.pipeline_layout,
                &maligog::ShaderStage::new(&module, maligog::ShaderStageFlags::RAYGEN_KHR, "main"),
                &[&maligog::ShaderStage::new(
                    &module,
                    maligog::ShaderStageFlags::MISS_KHR,
                    "miss",
                )],
                &[&maligog::TrianglesHitGroup::new(
                    &maligog::ShaderStage::new(
                        &module,
                        maligog::ShaderStageFlags::CLOSEST_HIT_KHR,
                        "depth_closest_hit",
                    ),
                    None,
                )],
            );
        }
    }
}
