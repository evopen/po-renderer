use bytemuck::{Pod, Zeroable};
use maligog::vk;
use maligog::Device;
use maplit::btreemap;

use crate::Vec3;

use crate::engine::util;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct CameraInfo {
    origin: Vec3,
    direction: Vec3,
    vfov: f32,
}

pub struct RayTracing {
    pipeline: maligog::RayTracingPipeline,
    rx: crossbeam::channel::Receiver<Vec<u8>>,
    device: Device,
    pipeline_layout: maligog::PipelineLayout,
    color_image: maligog::Image,
    ao_image: maligog::Image,
    descriptor_pool: maligog::DescriptorPool,
    as_descriptor_set_layout: maligog::DescriptorSetLayout,
    image_descriptor_set: maligog::DescriptorSet,
    as_descriptor_set: maligog::DescriptorSet,
    shader_binding_tables: maligog::PipelineShaderBindingTables,
}

impl RayTracing {
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        let image_descriptor_set_layout = device.create_descriptor_set_layout(
            Some("ray tracing image"),
            &[
                maligog::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: maligog::DescriptorType::StorageImage,
                    stage_flags: maligog::ShaderStageFlags::RAYGEN_KHR,
                    descriptor_count: 1,
                },
                maligog::DescriptorSetLayoutBinding {
                    binding: 1,
                    descriptor_type: maligog::DescriptorType::StorageImage,
                    stage_flags: maligog::ShaderStageFlags::RAYGEN_KHR,
                    descriptor_count: 1,
                },
            ],
        );
        let as_descriptor_set_layout = device.create_descriptor_set_layout(
            Some("ray tracing as"),
            &[maligog::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: maligog::DescriptorType::AccelerationStructure,
                stage_flags: maligog::ShaderStageFlags::RAYGEN_KHR,
                descriptor_count: 1,
            }],
        );
        let pipeline_layout = device.create_pipeline_layout(
            Some("ray tracing"),
            &[&as_descriptor_set_layout, &image_descriptor_set_layout],
            &[maligog::PushConstantRange::builder()
                .offset(0)
                .size(std::mem::size_of::<CameraInfo>() as u32)
                .stage_flags(maligog::ShaderStageFlags::RAYGEN_KHR)
                .build()],
        );
        let (tx, rx) = crossbeam::channel::bounded(1);
        let watcher = util::spirv_builder("./shaders/ray-tracing");
        let h = std::thread::spawn(|| {
            log::info!("watching ray tracing shader");
            watcher
                .watch(move |result| {
                    crate::engine::util::handle_shader_compile(result, &tx);
                })
                .unwrap();
            log::info!("watch has ended");
        });
        std::mem::forget(h);

        let spirv = rx.recv().unwrap();

        let module = device.create_shader_module(spirv);

        let pipeline = Self::build_pipeline(
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
                    "closest_hit",
                ),
                None,
            )],
        );

        let color_image = device.create_image(
            Some("color image"),
            maligog::Format::R32G32B32A32_SFLOAT,
            width,
            height,
            maligog::ImageUsageFlags::STORAGE | maligog::ImageUsageFlags::TRANSFER_DST,
            maligog::MemoryLocation::GpuOnly,
        );
        let ao_image = device.create_image(
            Some("ao image"),
            maligog::Format::R32_SFLOAT,
            width,
            height,
            maligog::ImageUsageFlags::STORAGE,
            maligog::MemoryLocation::GpuOnly,
        );

        let descriptor_pool = device.create_descriptor_pool(
            &[
                maligog::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::STORAGE_IMAGE)
                    .descriptor_count(2)
                    .build(),
                maligog::DescriptorPoolSize::builder()
                    .ty(vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                    .descriptor_count(1)
                    .build(),
            ],
            2,
        );

        let image_descriptor_set = device.create_descriptor_set(
            Some("image descriptor set"),
            &descriptor_pool,
            &image_descriptor_set_layout,
            btreemap! {
                0 => maligog::DescriptorUpdate::Image(vec![color_image.create_view()]),
                1 => maligog::DescriptorUpdate::Image(vec![ao_image.create_view()]),
            },
        );
        let as_descriptor_set = device.allocate_descriptor_set(
            Some("temp descriptor set"),
            &descriptor_pool,
            &as_descriptor_set_layout,
        );

        let shader_binding_tables = maligog::PipelineShaderBindingTables::new(&device, &pipeline);

        Self {
            pipeline,
            rx,
            device: device.clone(),
            pipeline_layout,
            color_image,
            ao_image,
            descriptor_pool,
            as_descriptor_set_layout,
            image_descriptor_set,
            shader_binding_tables,
            as_descriptor_set,
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
}

impl super::ScenePass for RayTracing {
    fn execute(
        &self,
        recorder: &mut maligog::CommandRecorder,
        scene: &maligog_gltf::Scene,
        image_view: &maligog::ImageView,
        camera: &super::super::Camera,
        clear_color: Option<maligog::ClearColorValue>,
    ) {
        self.as_descriptor_set.update(btreemap! {
            0 => maligog::DescriptorUpdate::AccelerationStructure(vec![scene.tlas().clone()]),
        });
        let mut camera_info = CameraInfo {
            origin: camera.location,
            direction: camera.front,
            vfov: camera.fov,
        };

        recorder.clear_color_image(
            &self.color_image,
            &vk::ClearColorValue {
                float32: [1.0, 1.0, 1.0, 1.0],
            },
        );
        recorder.bind_ray_tracing_pipeline(&self.pipeline, |rec| {
            rec.bind_descriptor_sets(vec![&self.as_descriptor_set, &self.image_descriptor_set], 0);
            rec.push_constants(
                maligog::ShaderStageFlags::RAYGEN_KHR,
                &bytemuck::cast_slice(&[camera_info]),
            );
            rec.trace_ray(
                &self.shader_binding_tables.get_raygen_table(),
                &self.shader_binding_tables.get_miss_table(),
                &self.shader_binding_tables.get_hit_table(),
                &self.shader_binding_tables.get_callable_table(),
                self.color_image.width(),
                self.color_image.height(),
                31,
            );
        });
        recorder.blit_image(
            &self.color_image,
            maligog::ImageLayout::GENERAL,
            &image_view.image(),
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
                        x: self.color_image.width() as i32,
                        y: self.color_image.height() as i32,
                        z: 1,
                    },
                ])
                .dst_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: image_view.width() as i32,
                        y: image_view.height() as i32,
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
            vk::Filter::NEAREST,
        );
    }

    fn update(&mut self) {
        if let Ok(spirv) = self.rx.try_recv() {
            log::info!("updating shader");
            let module = self.device.create_shader_module(spirv);

            self.pipeline = Self::build_pipeline(
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
                        "closest_hit",
                    ),
                    None,
                )],
            );
            self.shader_binding_tables =
                maligog::PipelineShaderBindingTables::new(&self.device, &self.pipeline);
        }
    }
}
