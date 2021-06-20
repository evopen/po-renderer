use std::time::Duration;

use bytemuck::{Pod, Zeroable};
use maligog::vk;
use maligog::Device;
use maligog::ShaderBindingTables;
use maplit::btreemap;

use crate::Vec3;

use crate::engine::util;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct CameraInfo {
    view_inv: glam::Mat4,
    proj_inv: glam::Mat4,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GeometryInfo {
    pub index_offset: u64,
    pub vertex_offset: u64,
    pub index_count: u32,
    pub vertex_count: u32,
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
    skymap_descriptor_set_layout: maligog::DescriptorSetLayout,
    skymap_descriptor_set: maligog::DescriptorSet,
    descriptor_helper: crate::engine::DescriptorHelper,
    scene: Option<maligog_gltf::Scene>,
    geometry_infos: Vec<GeometryInfo>,
    geometry_info_offsets: Vec<usize>,
    geometry_info_offsets_buffer: maligog::Buffer,
    geometry_infos_buffer: maligog::Buffer,
}

impl RayTracing {
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        let descriptor_helper = crate::engine::DescriptorHelper::new(device);
        let sky_sampler = device.create_sampler(
            Some("sky"),
            maligog::Filter::LINEAR,
            maligog::Filter::LINEAR,
            maligog::SamplerAddressMode::CLAMP_TO_EDGE,
            maligog::SamplerAddressMode::CLAMP_TO_EDGE,
        );
        log::debug!("creating image descriptor set layout");

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
        let builder = util::spirv_builder("./shaders/ray-tracing");
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
            maligog::ImageUsageFlags::STORAGE
                | maligog::ImageUsageFlags::TRANSFER_DST
                | maligog::ImageUsageFlags::TRANSFER_SRC,
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

        log::debug!("creating image descriptor set");
        let image_descriptor_set = device.create_descriptor_set(
            Some("image descriptor set"),
            &descriptor_pool,
            &image_descriptor_set_layout,
            btreemap! {
                0 => maligog::DescriptorUpdate::Image(vec![color_image.create_view()]),
                1 => maligog::DescriptorUpdate::Image(vec![ao_image.create_view()]),
            },
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
            as_descriptor_set,
            skymap_descriptor_set_layout,
            skymap_descriptor_set,
            descriptor_helper,
            scene: None,
            geometry_infos: Vec::new(),
            geometry_info_offsets: Vec::new(),
            geometry_info_offsets_buffer: device.create_buffer(
                Some("placeholder"),
                1,
                maligog::BufferUsageFlags::empty(),
                maligog::MemoryLocation::GpuOnly,
            ),
            geometry_infos_buffer: device.create_buffer(
                Some("placeholder"),
                1,
                maligog::BufferUsageFlags::empty(),
                maligog::MemoryLocation::GpuOnly,
            ),
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
        image_view: &maligog::ImageView,
        camera: &super::super::Camera,
        clear_color: Option<maligog::ClearColorValue>,
        skymap: &maligog::ImageView,
    ) {
        let scene = self.scene.as_ref().unwrap();
        self.as_descriptor_set.update(btreemap! {
            0 => maligog::DescriptorUpdate::AccelerationStructure(vec![scene.tlas().clone()]),
            1 => maligog::DescriptorUpdate::Buffer(vec![scene.index_buffer().clone()]),
            2 => maligog::DescriptorUpdate::Buffer(vec![scene.vertex_buffer().clone()]),
            3 => maligog::DescriptorUpdate::Buffer(vec![maligog::BufferView { buffer: self.geometry_infos_buffer.clone(), offset: 0}]),
            4 => maligog::DescriptorUpdate::Buffer(vec![maligog::BufferView { buffer: self.geometry_info_offsets_buffer.clone(), offset: 0}]),
        });
        self.skymap_descriptor_set.update(btreemap! {
            0 => maligog::DescriptorUpdate::Image(vec![skymap.clone()]),
        });

        let mut hit_groups: Vec<u32> = Vec::new();
        for i in 0..12345 {
            hit_groups.push(0);
        }
        let shader_binding_tables = self.pipeline.create_shader_binding_tables(&hit_groups);

        let mut camera_info = CameraInfo {
            view_inv: glam::Mat4::look_at_lh(
                camera.location,
                camera.location + camera.front,
                camera.up,
            )
            .inverse(),
            proj_inv: glam::Mat4::perspective_lh(camera.fov, camera.aspect_ratio, 0.001, 10000.0)
                .inverse(),
        };

        recorder.clear_color_image(
            &self.color_image,
            &vk::ClearColorValue {
                float32: [1.0, 1.0, 1.0, 1.0],
            },
        );
        recorder.bind_ray_tracing_pipeline(&self.pipeline, |rec| {
            rec.bind_descriptor_sets(
                vec![
                    &self.as_descriptor_set,
                    &self.image_descriptor_set,
                    &self.skymap_descriptor_set,
                ],
                0,
            );
            rec.push_constants(
                maligog::ShaderStageFlags::RAYGEN_KHR | maligog::ShaderStageFlags::CLOSEST_HIT_KHR,
                &bytemuck::cast_slice(&[camera_info]),
            );
            rec.trace_ray(
                &shader_binding_tables.ray_gen_table(),
                &shader_binding_tables.miss_table(),
                &shader_binding_tables.hit_table(),
                &shader_binding_tables.callable_table(),
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
        }
    }

    fn prepare_scene(&mut self, scene: &maligog_gltf::Scene) {
        let need_reload = self.scene.is_none() || self.scene.as_ref().unwrap() != scene;
        if need_reload {
            log::info!("reloading scene");
            self.scene = Some(scene.clone());

            self.geometry_info_offsets.push(0);
            for (i, mesh) in scene.mesh_infos().iter().enumerate() {
                let convert = mesh.primitive_infos.iter().map(|i| {
                    GeometryInfo {
                        index_offset: i.index_offset,
                        vertex_offset: i.vertex_offset,
                        index_count: i.index_count,
                        vertex_count: i.vertex_count,
                    }
                });
                self.geometry_infos.extend(convert);

                // how many geometries in every mesh
                self.geometry_info_offsets
                    .push(self.geometry_info_offsets[i] + mesh.primitive_infos.len());
            }
            self.geometry_infos_buffer = self.device.create_buffer_init(
                Some("geometry infos"),
                bytemuck::cast_slice(&self.geometry_infos),
                maligog::BufferUsageFlags::STORAGE_BUFFER,
                maligog::MemoryLocation::GpuOnly,
            );
            self.geometry_info_offsets_buffer = self.device.create_buffer_init(
                Some("geometry info offsets"),
                bytemuck::cast_slice(&self.geometry_info_offsets),
                maligog::BufferUsageFlags::STORAGE_BUFFER,
                maligog::MemoryLocation::GpuOnly,
            );
        }
    }
}
