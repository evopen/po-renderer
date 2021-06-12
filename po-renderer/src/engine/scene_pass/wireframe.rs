use maligog::vk;
use maligog::Device;

use crate::engine::util;

#[derive(Debug)]
struct Transform {
    model: glam::Mat4,
    view: glam::Mat4,
    projection: glam::Mat4,
}

pub struct Wireframe {
    pipeline: maligog::GraphicsPipeline,
    rx: tokio::sync::watch::Receiver<Vec<u8>>,
    device: Device,
    pipeline_layout: maligog::PipelineLayout,
    render_pass: maligog::RenderPass,
}

impl super::ScenePass for Wireframe {}

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
        let compile_result = util::spirv_builder("./shaders/wireframe").build().unwrap();
        let spirv = std::fs::read(compile_result.module.unwrap_single()).unwrap();
        let (tx, rx) = tokio::sync::watch::channel(spirv);
        let watcher = util::spirv_builder("./shaders/wireframe");
        let h = std::thread::spawn(|| {
            log::info!("watching wireframe shader");
            watcher
                .watch(move |result| {
                    crate::engine::util::handle_shader_compile(result, &tx);
                })
                .unwrap();
            log::info!("watch has ended");
        });
        std::mem::forget(h);

        let module = device.create_shader_module(&*rx.borrow());

        let render_pass = device.create_render_pass(
            &vk::RenderPassCreateInfo::builder()
                .attachments(&[vk::AttachmentDescription::builder()
                    .format(vk::Format::B8G8R8A8_UNORM)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
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
        }
    }

    pub async fn refresh_pipeline(&mut self) {
        self.rx.changed().await.unwrap();

        let module = self.device.create_shader_module(&*self.rx.borrow());

        self.pipeline = Self::build_pipeline(
            &self.device,
            &self.pipeline_layout,
            &self.render_pass,
            vec![
                maligog::ShaderStage::new(&module, maligog::ShaderStageFlags::VERTEX, "main_vs"),
                maligog::ShaderStage::new(&module, maligog::ShaderStageFlags::FRAGMENT, "main_fs"),
            ],
        );
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
