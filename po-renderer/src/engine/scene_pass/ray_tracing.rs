use bytemuck::{Pod, Zeroable};
use maligog::vk;
use maligog::Device;

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
    render_pass: maligog::RenderPass,
    color_image: maligog::Image,
    ao_image: maligog::Image,
}

impl RayTracing {
    pub fn new(device: &Device, width: u32, height: u32) -> Self {
        let descriptor_set_layout = device.create_descriptor_set_layout(
            Some("ray tracing"),
            &[
                maligog::DescriptorSetLayoutBinding {
                    binding: 0,
                    descriptor_type: maligog::DescriptorType::AccelerationStructure,
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
        let pipeline_layout = device.create_pipeline_layout(
            Some("ray tracing"),
            &[&descriptor_set_layout],
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

        let render_pass = device.create_render_pass(
            &vk::RenderPassCreateInfo::builder()
                .attachments(&[vk::AttachmentDescription::builder()
                    .format(vk::Format::B8G8R8A8_UNORM)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .load_op(vk::AttachmentLoadOp::LOAD)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .initial_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL_KHR)
                    .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .build()])
                .subpasses(&[vk::SubpassDescription::builder()
                    .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                    .color_attachments(&[vk::AttachmentReference::builder()
                        .layout(vk::ImageLayout::ATTACHMENT_OPTIMAL_KHR)
                        .attachment(0)
                        .build()])
                    .build()])
                .build(),
        );

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
            maligog::ImageUsageFlags::STORAGE,
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

        Self {
            pipeline,
            rx,
            device: device.clone(),
            pipeline_layout,
            render_pass,
            color_image,
            ao_image,
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
        let mut camera_info = CameraInfo {
            origin: camera.location,
            direction: camera.front,
            vfov: camera.fov,
        };
        let framebuffer = self.device.create_framebuffer(
            self.render_pass.clone(),
            image_view.width(),
            image_view.height(),
            vec![&image_view],
        );
        recorder.begin_render_pass(&self.render_pass, &&framebuffer, |rec| {
            if let Some(color) = clear_color {
                rec.clear_attachments(
                    &[vk::ClearAttachment::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .color_attachment(0)
                        .clear_value(vk::ClearValue { color })
                        .build()],
                    &[vk::ClearRect::builder()
                        .base_array_layer(0)
                        .layer_count(1)
                        .rect(
                            vk::Rect2D::builder()
                                .offset(vk::Offset2D::default())
                                .extent(
                                    vk::Extent2D::builder()
                                        .width(image_view.width())
                                        .height(image_view.height())
                                        .build(),
                                )
                                .build(),
                        )
                        .build()],
                );
            }
            rec.bind_ray_tracing_pipeline(&self.pipeline, |rec| {
                let tlas = scene.tlas();
                // transform.model.clone_from(instance.transform());
                rec.push_constants(
                    maligog::ShaderStageFlags::RAYGEN_KHR,
                    &bytemuck::cast_slice(&[camera_info]),
                );
            });
        });
    }

    fn update(&mut self) {
        if let Ok(spirv) = self.rx.try_recv() {
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
}
