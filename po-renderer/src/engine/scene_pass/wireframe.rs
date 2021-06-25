use bytemuck::{Pod, Zeroable};
use maligog::vk;
use maligog::Device;

use crate::engine::util;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct Transform {
    model: glam::Mat4,
    view: glam::Mat4,
    projection: glam::Mat4,
}

pub struct Wireframe {
    pipeline: maligog::GraphicsPipeline,
    rx: crossbeam::channel::Receiver<Vec<u8>>,
    device: Device,
    pipeline_layout: maligog::PipelineLayout,
    render_pass: maligog::RenderPass,
    scene: Option<maligog_gltf::Scene>,
}

impl Wireframe {
    pub fn new(device: &Device) -> Self {
        let descriptor_set_layout = device.create_descriptor_set_layout(Some("wireframe"), &[]);
        let pipeline_layout = device.create_pipeline_layout(
            Some("wireframe"),
            &[&descriptor_set_layout],
            &[maligog::PushConstantRange::builder()
                .offset(0)
                .size(std::mem::size_of::<Transform>() as u32)
                .stage_flags(maligog::ShaderStageFlags::VERTEX)
                .build()],
        );
        let (tx, rx) = crossbeam::channel::bounded(1);
        let builder = util::spirv_builder("./shaders/wireframe");
        let tx1 = tx.clone();
        let result = builder
            .watch(move |result| {
                crate::engine::util::handle_shader_compile(result, &tx1);
            })
            .unwrap();
        crate::engine::util::handle_shader_compile(result, &tx);

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
            &render_pass,
            vec![
                maligog::ShaderStage::new(&module, maligog::ShaderStageFlags::VERTEX, "main_vs"),
                maligog::ShaderStage::new(&module, maligog::ShaderStageFlags::FRAGMENT, "main_fs"),
            ],
        );

        Self {
            pipeline,
            rx,
            device: device.clone(),
            pipeline_layout,
            render_pass,
            scene: None,
        }
    }

    fn build_pipeline(
        device: &Device,
        pipeline_layout: &maligog::PipelineLayout,
        render_pass: &maligog::RenderPass,
        shader_stages: Vec<maligog::ShaderStage>,
    ) -> maligog::GraphicsPipeline {
        let pipeline = device.create_graphics_pipeline(
            Some("wireframe"),
            pipeline_layout,
            shader_stages,
            render_pass,
            &vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_binding_descriptions(&[vk::VertexInputBindingDescription::builder()
                    .stride(3 * 4)
                    .input_rate(vk::VertexInputRate::VERTEX)
                    .binding(0)
                    .build()])
                .vertex_attribute_descriptions(&[vk::VertexInputAttributeDescription::builder()
                    .binding(0)
                    .location(0)
                    .format(vk::Format::R32G32B32_SFLOAT)
                    .offset(0)
                    .build()])
                .build(),
            &vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                .build(),
            &vk::PipelineRasterizationStateCreateInfo::builder()
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .polygon_mode(vk::PolygonMode::LINE)
                .line_width(1.0)
                .build(),
            &vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                .build(),
            &vk::PipelineDepthStencilStateCreateInfo::default(),
            &vk::PipelineColorBlendStateCreateInfo::builder()
                .attachments(&[vk::PipelineColorBlendAttachmentState::builder()
                    .blend_enable(false)
                    .color_write_mask(vk::ColorComponentFlags::all())
                    .build()])
                .build(),
            &vk::PipelineViewportStateCreateInfo::builder()
                .viewport_count(1)
                .scissor_count(1),
            &vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
                .build(),
        );
        pipeline
    }
}

impl super::ScenePass for Wireframe {
    fn execute(
        &self,
        recorder: &mut maligog::CommandRecorder,
        image_view: &maligog::ImageView,
        camera: &super::super::Camera,
        clear_color: Option<maligog::ClearColorValue>,
        skymap: &maligog::ImageView,
    ) {
        let scene = self.scene.as_ref().unwrap();
        let mut transform = Transform {
            model: glam::Mat4::IDENTITY,
            view: glam::Mat4::look_at_lh(
                camera.location,
                camera.location + camera.front,
                camera.up,
            ),
            projection: glam::Mat4::perspective_lh(camera.fov, camera.aspect_ratio, 0.001, 10000.0),
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
            rec.bind_graphics_pipeline(&self.pipeline, |rec| {
                let tlas = scene.tlas();
                for geometry in tlas.geometries() {
                    for instance in geometry.blas_instances() {
                        transform.model.clone_from(instance.transform());
                        rec.push_constants(
                            maligog::ShaderStageFlags::VERTEX,
                            &bytemuck::cast_slice(&[transform]),
                        );
                        for geometry in instance.blas().geometries() {
                            rec.bind_vertex_buffers(
                                &[&geometry.vertex_buffer_view().buffer_view.buffer],
                                &[geometry.vertex_buffer_view().buffer_view.offset],
                            );
                            rec.bind_index_buffer(
                                &geometry.index_buffer_view().buffer_view.buffer,
                                geometry.index_buffer_view().buffer_view.offset,
                                geometry.index_buffer_view().index_type,
                            );
                            rec.set_scissor(&[vk::Rect2D {
                                offset: vk::Offset2D { x: 0, y: 0 },
                                extent: vk::Extent2D {
                                    width: image_view.width(),
                                    height: image_view.height(),
                                },
                            }]);
                            rec.set_viewport(vk::Viewport {
                                x: 0.0,
                                y: image_view.height() as f32,
                                width: image_view.width() as f32,
                                height: -(image_view.height() as f32),
                                min_depth: 0.0,
                                max_depth: 1.0,
                            });
                            rec.draw_indexed(geometry.index_buffer_view().count, 1);
                        }
                    }
                }
            });
        });
    }

    fn update(&mut self) {
        if let Ok(spirv) = self.rx.try_recv() {
            log::info!("updating shader");
            let module = self.device.create_shader_module(spirv);

            self.pipeline = Self::build_pipeline(
                &self.device,
                &self.pipeline_layout,
                &self.render_pass,
                vec![
                    maligog::ShaderStage::new(
                        &module,
                        maligog::ShaderStageFlags::VERTEX,
                        "main_vs",
                    ),
                    maligog::ShaderStage::new(
                        &module,
                        maligog::ShaderStageFlags::FRAGMENT,
                        "main_fs",
                    ),
                ],
            );
        }
    }

    fn prepare_scene(&mut self, scene: &maligog_gltf::Scene) {
        let need_reload = self.scene.is_none() || self.scene.as_ref().unwrap() != scene;
        if need_reload {
            self.scene = Some(scene.clone());
        }
    }
}
