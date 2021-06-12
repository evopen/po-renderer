mod wireframe;
use async_trait::async_trait;

pub use wireframe::Wireframe;

#[async_trait]
pub trait ScenePass {
    fn execute(
        &self,
        recorder: &mut maligog::CommandRecorder,
        scene: &maligog_gltf::Scene,
        image_view: &maligog::ImageView,
        camera: &super::Camera,
        clear_color: Option<maligog::ClearColorValue>,
    );

    async fn update(&mut self);
}
