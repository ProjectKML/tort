use spirv_std::{arch::set_mesh_outputs_ext, spirv};
use tort_math::{UVec3, Vec4};

#[spirv(mesh_ext(
    threads(1),
    output_vertices = 3,
    output_primitives_ext = 1,
    output_triangles_ext
))]
pub fn pass_mesh(
    #[spirv(position)] positions: &mut [Vec4; 3],
    #[spirv(primitive_triangle_indices_ext)] indices: &mut [UVec3; 1],
) {
    unsafe {
        set_mesh_outputs_ext(3, 1);
    }

    positions[0] = Vec4::new(-0.5, 0.5, 0.0, 1.0);
    positions[1] = Vec4::new(0.5, 0.5, 0.0, 1.0);
    positions[2] = Vec4::new(0.0, -0.5, 0.0, 1.0);

    indices[0] = UVec3::new(0, 1, 2);
}

#[spirv(fragment)]
pub fn pass_frag(out_color: &mut Vec4) {
    *out_color = Vec4::new(0.1, 1.0, 0.1, 1.0);
}
