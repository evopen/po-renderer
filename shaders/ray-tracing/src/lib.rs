#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr),
    register_attr(spirv)
)]

use spirv_std::glam::{
    vec2, vec3, vec4, Mat4, UVec2, UVec3, Vec2, Vec3, Vec3Swizzles, Vec4Swizzles,
};
use spirv_std::image;
use spirv_std::Image;

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

pub struct CameraInfo {
    view_inv: Mat4,
    projection_inv: Mat4,
}

#[spirv(ray_generation)]
pub fn main(
    #[spirv(push_constant)] camera_info: &CameraInfo,
    #[spirv(launch_id)] pixel: UVec3,
    #[spirv(launch_size)] launch_size: UVec3,
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
        let origin = camera_info.view_inv * Vec3::splat(0.0).extend(1.0);

        let pixel_center = Vec2::new(pixel.x as f32, pixel.y as f32) + Vec2::splat(0.5);

        // map to (0, 1)
        let uv = pixel_center / Vec2::new(launch_size.x as f32, launch_size.y as f32);

        // map to (-1, 1) square
        let d = uv * 2.0 - Vec2::splat(1.0);

        let target = camera_info.projection_inv * d.extend(1.0).extend(1.0);
        let target_norm = (target.xyz() / target.w).normalize();
        let direction = (camera_info.view_inv * target_norm.extend(0.0)).normalize();
        tlas.trace_ray(
            spirv_std::ray_tracing::RayFlags::OPAQUE,
            0xFF,
            0,
            0,
            0,
            origin.xyz(),
            tmin,
            direction.xyz(),
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
