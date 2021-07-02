use super::util;
use glam::Vec3;
use maligog::{vk, Device};
use maplit::btreemap;

use bytemuck::{Pod, Zeroable};

pub struct CameraInfo {
    view_inv: glam::Mat4,
    proj_inv: glam::Mat4,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GeometryInfo {
    pub index_offset: u64,
    pub vertex_offset: u64,
    pub index_count: u64,
    pub vertex_count: u64,
    pub material_index: u64,
    pub color_offset: u64,
    pub tex_coord_offset: u64,
    pub has_color: u32,
    pub has_tex_coord: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct MaterialInfo {
    base_color_factor: glam::Vec4,
    has_base_color_texture: u32,
    base_color_sampler_index: u32,
    base_color_image_index: u32,
    has_metallic_roughness_texture: u32,
    metallic_roughness_sampler_index: u32,
    metallic_roughness_image_index: u32,
    padding: u64,
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
    pub name: String,
    pub image: maligog::Image,
}

pub struct Po {
    depth_pipeline: maligog::RayTracingPipeline,
    rx: crossbeam::channel::Receiver<Vec<u8>>,
    device: Device,
    pipeline_layout: maligog::PipelineLayout,
    descriptor_pool: maligog::DescriptorPool,
    as_descriptor_set_layout: maligog::DescriptorSetLayout,
    skymap_descriptor_set_layout: maligog::DescriptorSetLayout,
    image_descriptor_set_layout: maligog::DescriptorSetLayout,
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

        let default_sampler = device.create_sampler(
            Some("rt default sampler"),
            maligog::Filter::NEAREST,
            maligog::Filter::NEAREST,
            maligog::SamplerAddressMode::CLAMP_TO_EDGE,
            maligog::SamplerAddressMode::CLAMP_TO_EDGE,
        );
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
        Self {
            depth_pipeline,
            rx,
            device: device.clone(),
            pipeline_layout,
            descriptor_pool,
            as_descriptor_set_layout,
            skymap_descriptor_set_layout,
            image_descriptor_set_layout,
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
        skymap: &maligog::ImageView,
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

        let mut cmd_buf = self.device.create_command_buffer(
            Some("render cmd buf"),
            self.device.graphics_queue_family_index(),
        );
        let mut geometry_infos = Vec::new();
        let mut geometry_info_offsets = vec![0];

        log::debug!("allocating as descriptor set");
        let as_descriptor_set = self.device.allocate_descriptor_set(
            Some("as descriptor set"),
            &self.descriptor_pool,
            &self.as_descriptor_set_layout,
        );
        log::debug!("allocating skymap descriptor set");
        let skymap_descriptor_set = self.device.allocate_descriptor_set(
            Some("skymap descriptor set"),
            &self.descriptor_pool,
            &self.skymap_descriptor_set_layout,
        );
        log::debug!("creating image descriptor set");
        let image_descriptor_set = self.device.create_descriptor_set(
            Some("image descriptor set"),
            &self.descriptor_pool,
            &self.image_descriptor_set_layout,
            btreemap! {
                0 => maligog::DescriptorUpdate::Image(vec![depth_image.create_view()]),
            },
        );

        for (i, mesh) in scene.mesh_infos().iter().enumerate() {
            let convert = mesh.primitive_infos.iter().map(|i| {
                GeometryInfo {
                    index_offset: i.index_offset,
                    vertex_offset: i.vertex_offset,
                    index_count: i.index_count,
                    vertex_count: i.vertex_count,
                    material_index: i.material_index,
                    color_offset: i.color_offset.unwrap_or_default(),
                    tex_coord_offset: i.tex_coord_offset.unwrap_or_default(),
                    has_color: match i.color_offset {
                        Some(_) => 1,
                        None => 0,
                    },
                    has_tex_coord: match i.tex_coord_offset {
                        Some(_) => 1,
                        None => 0,
                    },
                }
            });
            geometry_infos.extend(convert);

            // how many geometries in every mesh
            geometry_info_offsets
                .push(geometry_info_offsets.last().unwrap() + mesh.primitive_infos.len() as u32);
        }

        let material_infos = scene
            .material_infos()
            .iter()
            .map(|i| {
                let mut has_base_color_texture = 0;
                let mut base_color_sampler_index = 0;
                let mut base_color_image_index = 0;
                let mut has_metallic_roughness_texture = 0;
                let mut metallic_roughness_sampler_index = 0;
                let mut metallic_roughness_image_index = 0;
                if let Some(texture) = i.base_color_texture {
                    has_base_color_texture = 1;
                    base_color_sampler_index = texture.sampler_index;
                    base_color_image_index = texture.image_index;
                }
                if let Some(texture) = i.metallic_roughness_texture {
                    has_metallic_roughness_texture = 1;
                    metallic_roughness_sampler_index = texture.sampler_index;
                    metallic_roughness_image_index = texture.image_index;
                }

                MaterialInfo {
                    base_color_factor: i.base_color_factor,
                    has_base_color_texture,
                    base_color_sampler_index,
                    base_color_image_index,
                    has_metallic_roughness_texture,
                    metallic_roughness_sampler_index,
                    metallic_roughness_image_index,
                    padding: 0,
                }
            })
            .collect::<Vec<_>>();

        let geometry_infos_buffer = self.device.create_buffer_init(
            Some("geometry infos"),
            bytemuck::cast_slice(&geometry_infos),
            maligog::BufferUsageFlags::STORAGE_BUFFER,
            maligog::MemoryLocation::GpuOnly,
        );
        let geometry_info_offsets_buffer = self.device.create_buffer_init(
            Some("geometry info offsets"),
            bytemuck::cast_slice(&geometry_info_offsets),
            maligog::BufferUsageFlags::STORAGE_BUFFER,
            maligog::MemoryLocation::GpuOnly,
        );
        let material_info_buffer = self.device.create_buffer_init(
            Some("material info"),
            bytemuck::cast_slice(&material_infos),
            maligog::BufferUsageFlags::STORAGE_BUFFER,
            maligog::MemoryLocation::GpuOnly,
        );

        log::debug!("potential problematic update");
        as_descriptor_set.update(btreemap! {
            0 => maligog::DescriptorUpdate::AccelerationStructure(vec![scene.tlas().clone()]),
            1 => maligog::DescriptorUpdate::Buffer(vec![scene.index_buffer().clone()]),
            2 => maligog::DescriptorUpdate::Buffer(vec![scene.vertex_buffer().clone()]),
            3 => maligog::DescriptorUpdate::Buffer(vec![maligog::BufferView { buffer: geometry_infos_buffer.clone(), offset: 0}]),
            4 => maligog::DescriptorUpdate::Buffer(vec![maligog::BufferView { buffer: geometry_info_offsets_buffer.clone(), offset: 0}]),
            5 => maligog::DescriptorUpdate::Buffer(vec![scene.transform_buffer().clone()]),
            6 => maligog::DescriptorUpdate::Sampler(scene.samplers().to_vec()),
            8 => maligog::DescriptorUpdate::Buffer(vec![maligog::BufferView {buffer:material_info_buffer, offset:0}]),
        });
        log::debug!("update done");

        if scene.images().len() > 0 {
            as_descriptor_set.update(btreemap! {
                7 => maligog::DescriptorUpdate::Image(scene.images().iter().map(|i|i.create_view()).collect()),
            });
        }
        if let Some(b) = scene.color_buffer() {
            as_descriptor_set.update(btreemap! {
                9 => maligog::DescriptorUpdate::Buffer(vec![b]),
            });
        }
        if let Some(b) = scene.tex_coord_buffer() {
            as_descriptor_set.update(btreemap! {
                10 => maligog::DescriptorUpdate::Buffer(vec![b]),
            });
        }
        skymap_descriptor_set.update(btreemap! {
            0 => maligog::DescriptorUpdate::Image(vec![skymap.clone()]),
        });
        // cmd_buf.encode(|rec| {
        //     rec.bind_ray_tracing_pipeline(&self.depth_pipeline, |rec| {
        //         rec.bind_descriptor_sets();
        //     });
        // });

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
