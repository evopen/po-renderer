#![cfg_attr(
    target_arch = "spirv",
    no_std,
    feature(register_attr),
    register_attr(spirv)
)]
#![allow(dead_code, unused_imports, unused)]

use spirv_std::glam::{vec2, vec3, vec4, Mat4, UVec2, UVec3, Vec3, Vec3Swizzles, Vec4};
use spirv_std::image;

#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

pub struct Transform {
    model: Mat4,
    view: Mat4,
    projection: Mat4,
}

#[spirv(vertex)]
pub fn main_vs(
    pos: Vec3,
    #[spirv(push_constant)] transform: &Transform,
    #[spirv(position)] out_pos: &mut Vec4,
) {
    *out_pos = transform.projection * transform.view * transform.model * pos.extend(1.0);
}

#[spirv(fragment)]
pub fn main_fs(output: &mut Vec4) {
    *output = vec4(0.0, 1.0, 0.0, 1.0);
}
