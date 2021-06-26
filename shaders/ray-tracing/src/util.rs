use spirv_std::glam::Vec3;

// [0, 1] float rng
pub fn rng(state: &mut u32) -> f32 {
    // Condensed version of pcg_output_rxs_m_xs_32_32, with simple conversion to floating-point [0,1].
    *state = *state * 747796405 + 1;
    let state = *state;
    let mut word = ((state >> (state >> 28) + 4) ^ state) * 277803737;
    word = (word >> 22) ^ word;
    return word as f32 / 4294967295.0;
}

pub fn facefoward(n: &Vec3, i: &Vec3) -> Vec3 {
    match n.dot(*i) < 0.0 {
        true => *n,
        false => -*n,
    }
}
