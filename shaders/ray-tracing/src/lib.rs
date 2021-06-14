#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr),
    register_attr(spirv)
)]

use spirv_std::glam::{vec2, vec3, vec4, UVec2, UVec3, Vec3, Vec3Swizzles};
use spirv_std::image;
use spirv_std::Image;

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

pub struct CameraInfo {
    origin: Vec3,
    direction: Vec3,
    vfov: f32,
}

#[spirv(ray_generation)]
pub fn main(
    #[spirv(push_constant)] camera_info: &CameraInfo,
    #[spirv(launch_id)] pixel: UVec3,
    #[spirv(ray_payload)] payload: &mut Vec3,
    #[spirv(descriptor_set = 0, binding = 0)] tlas: &spirv_std::ray_tracing::AccelerationStructure,
    // #[spirv(descriptor_set = 0, binding = 1)] img: &Image!(2D, type=f32, sampled=false),
    #[spirv(descriptor_set = 1, binding = 0)] color_image: &mut image::Image<
        f32,
        { image::Dimensionality::TwoD },
        { image::ImageDepth::False },
        { image::Arrayed::False },
        { image::Multisampled::False },
        { image::Sampled::No },
        { image::ImageFormat::Rgba32f },
        { None },
    >,
    #[spirv(descriptor_set = 1, binding = 1)] ao_image: &mut image::Image<
        f32,
        { image::Dimensionality::TwoD },
        { image::ImageDepth::False },
        { image::Arrayed::False },
        { image::Multisampled::False },
        { image::Sampled::No },
        { image::ImageFormat::R32f },
        { None },
    >,
    // #[spirv(uniform, descriptor_set = 0, binding = 2)] camera_pos: &mut Vec2,
) {
    unsafe {
        let tmin = 0.001;
        let tmax = 10000.0;
        let origin = vec3(-0.001, 0.0, 10.0);
        let resolution = color_image.query_size::<UVec2>();
        let fov_vertical_slope = 1.0 / 5.0;

        // normalize width to [-1, 1] and height to [-aspect_ratio, aspect_ratio]
        let screen_uv = vec2(
            2.0 * (pixel.x as f32 + 0.5 - 0.5 * resolution.x as f32) / resolution.y as f32,
            -(2.0 * (pixel.y as f32 + 0.5 - 0.5 * resolution.y as f32) / resolution.y as f32),
        );
        let direction = vec3(
            fov_vertical_slope * screen_uv.x,
            fov_vertical_slope * screen_uv.y,
            -1.0,
        )
        .normalize();
        tlas.trace_ray(
            spirv_std::ray_tracing::RayFlags::OPAQUE,
            0xFF,
            0,
            0,
            0,
            origin,
            tmin,
            direction,
            tmax,
            payload,
        );

        color_image.write(pixel.xy(), vec4(payload.x, payload.y, payload.z, 1.0));
    }
}

#[spirv(closest_hit)]
pub fn closest_hit(#[spirv(incoming_ray_payload)] payload: &mut Vec3) {
    *payload = vec3(0.0, 1.0, 1.0);
}

#[spirv(miss)]
pub fn miss(#[spirv(incoming_ray_payload)] payload: &mut Vec3) {
    *payload = vec3(1.0, 0.5, 0.23);
}
