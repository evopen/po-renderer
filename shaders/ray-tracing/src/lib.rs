#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr),
    register_attr(spirv)
)]
#![allow(dead_code, unused_imports, unused)]

mod util;

use spirv_std::glam::uvec3;
use spirv_std::num_traits::float::Float;
use spirv_std::RuntimeArray;

use spirv_std::glam;
use spirv_std::glam::{
    vec2, vec3, vec4, Mat4, UVec2, UVec3, Vec2, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles,
};
use spirv_std::Image;
use spirv_std::{image, Sampler};

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

        let xy = UVec2::new(pixel.x, launch_size.y - pixel.y);

        color_image.write(xy, payload.extend(1.0));
    }
}

pub struct ShaderRecordData {
    index_offset: u32,
    vertex_offset: u32,
}

pub struct GeometryInfo {
    index_offset: u64,
    vertex_offset: u64,
    index_count: u32,
    vertex_count: u32,
}

#[spirv(closest_hit)]
pub fn closest_hit(
    #[spirv(incoming_ray_payload)] payload: &mut Vec3,
    #[spirv(hit_attribute)] hit_attr: &mut Vec2,
    #[spirv(instance_id)] instance_id: i32, // index of instance in tlas
    #[spirv(ray_geometry_index)] geometry_index: usize, // index of geometry in instance
    #[spirv(primitive_id)] primitive_id: usize, // index of triangle in geometry
    #[spirv(instance_custom_index)] instance_custom_index: usize, // blas id
    #[spirv(ray_tmax)] ray_tmax: f32,
    #[spirv(world_ray_origin)] world_ray_origin: Vec3,
    #[spirv(world_ray_direction)] world_ray_direction: Vec3,
    #[spirv(object_ray_origin)] object_ray_origin: Vec3,
    #[spirv(object_ray_direction)] object_ray_direction: Vec3,
    #[spirv(shader_record_buffer)] shader_record_buffer: &mut ShaderRecordData,
    // #[spirv(world_to_object)] world_to_object: glam::Affine3A,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] index_buffer: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] vertex_buffer: &mut [f32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] geometry_infos: &mut [GeometryInfo],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 4)] geometry_info_offsets: &mut [u32],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 5)] transform_buffer: &mut [Mat4],
    #[spirv(push_constant)] camera_info: &CameraInfo,
) {
    let barycentrics = vec3(1.0 - hit_attr.x - hit_attr.y, hit_attr.x, hit_attr.y);

    let geometry_info =
        &geometry_infos[geometry_info_offsets[instance_custom_index] as usize + geometry_index];
    let index_offset = (geometry_info.index_offset / 4) as usize; // by index
    let vertex_offset = (geometry_info.vertex_offset / 4) as usize; // by index

    let v0_index = index_buffer[index_offset + primitive_id * 3] as usize;
    let v0 = vec3(
        vertex_buffer[vertex_offset + v0_index * 3],
        vertex_buffer[vertex_offset + v0_index * 3 + 1],
        vertex_buffer[vertex_offset + v0_index * 3 + 2],
    );
    let v1_index = index_buffer[index_offset + primitive_id * 3 + 1] as usize;
    let v1 = vec3(
        vertex_buffer[vertex_offset + v1_index * 3],
        vertex_buffer[vertex_offset + v1_index * 3 + 1],
        vertex_buffer[vertex_offset + v1_index * 3 + 2],
    );
    let v2_index = index_buffer[index_offset + primitive_id * 3 + 2] as usize;
    let v2 = vec3(
        vertex_buffer[vertex_offset + v2_index * 3],
        vertex_buffer[vertex_offset + v2_index * 3 + 1],
        vertex_buffer[vertex_offset + v2_index * 3 + 2],
    );

    let object_to_world = transform_buffer[instance_id as usize];

    let object_position = v0 * barycentrics.x + v1 * barycentrics.y + v2 * barycentrics.z;
    // let object_normal = (v1 - v0).cross(v2 - v0).normalize();
    let world_position = object_to_world.transform_vector3(object_position);
    let world_v0 = object_to_world.transform_point3(v0);
    let world_v1 = object_to_world.transform_point3(v1);
    let world_v2 = object_to_world.transform_point3(v2);
    let mut world_normal = (world_v1 - world_v0).cross(world_v2 - world_v0).normalize();
    world_normal = util::facefoward(&world_normal, &world_ray_direction);

    *payload = world_normal;
}

#[spirv(miss)]
pub fn miss(
    #[spirv(incoming_ray_payload)] payload: &mut Vec3,
    #[spirv(world_ray_direction)] world_ray_direction: Vec3,
    #[spirv(descriptor_set = 1, binding = 2)] sampler: &Sampler,
    #[spirv(descriptor_set = 2, binding = 0)] sky_texture: &image::Image<
        f32,
        { image::Dimensionality::TwoD },
        { image::ImageDepth::False },
        { image::Arrayed::False },
        { image::Multisampled::False },
        { image::Sampled::Yes },
        { image::ImageFormat::Rgba8 },
        { None },
    >,
) {
    // *payload = vec3(1.0, 0.5, 0.23);
    let coord = sample_sphereical_map(&world_ray_direction);
    let color: Vec4 = sky_texture.sample_by_lod(*sampler, coord, 0.0);
    *payload = color.xyz();
}

pub fn sample_sphereical_map(direction: &Vec3) -> Vec2 {
    let inv_atan = vec2(0.1591, 0.3183);
    let mut uv = vec2(direction.z.atan2(direction.x), direction.y.asin());
    uv *= inv_atan;
    uv += Vec2::splat(0.5);
    return uv;
}
