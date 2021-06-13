mod wireframe;

pub use wireframe::Wireframe;

pub trait ScenePass {
    fn execute(
        &self,
        recorder: &mut maligog::CommandRecorder,
        scene: &maligog_gltf::Scene,
        image_view: &maligog::ImageView,
        camera: &super::Camera,
        clear_color: Option<maligog::ClearColorValue>,
    );

    fn update(&mut self);
}
