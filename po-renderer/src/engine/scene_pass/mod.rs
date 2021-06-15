mod ray_tracing;
mod wireframe;

pub use ray_tracing::RayTracing;
pub use wireframe::Wireframe;

pub trait ScenePass {
    fn execute(
        &self,
        recorder: &mut maligog::CommandRecorder,
        scene: &maligog_gltf::Scene,
        image_view: &maligog::ImageView,
        camera: &super::Camera,
        clear_color: Option<maligog::ClearColorValue>,
        skymap: &maligog::ImageView,
    );

    fn update(&mut self);
}
