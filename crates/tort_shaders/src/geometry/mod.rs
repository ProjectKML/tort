use spirv_std::{arch::set_mesh_outputs_ext, spirv};
use tort_math::{dequantize_unorm, Mat4, UVec3, Vec3, Vec4, AABB};

use crate::utils::BitReader;

fn murmur_hash_11(mut src: u32) -> u32 {
    const M: u32 = 0x5bd1e995;
    let mut h = 1190494759;
    src *= M;
    src ^= src >> 24;
    src *= M;
    h *= M;
    h ^= src;
    h ^= h >> 13;
    h *= M;
    h ^= h >> 15;

    h
}

fn murmur_hash_11_color(src: u32) -> Vec3 {
    let hash = murmur_hash_11(src);
    Vec3::new(
        ((hash >> 16) & 0xFF) as f32,
        ((hash >> 8) & 0xFF) as f32,
        (hash & 0xFF) as f32,
    ) / 256.0
}

unsafe fn read_vec3(reader: &mut BitReader) -> Vec3 {
    Vec3 {
        x: f32::from_bits(reader.read_bits_unchecked(32)),
        y: f32::from_bits(reader.read_bits_unchecked(32)),
        z: f32::from_bits(reader.read_bits_unchecked(32)),
    }
}

struct Meshlet {
    aabb: AABB,

    num_bits_x: u32,
    num_bits_y: u32,
    num_bits_z: u32,

    num_bits_tex_x: u32,
    num_bits_tex_y: u32,

    num_bits_normal: u32,
    num_bits_idx: u32,

    num_vertices: u32,
    num_primitives: u32,

    data_offset: u32,
}

impl Meshlet {
    unsafe fn from_bits(reader: &mut BitReader) -> Self {
        Self {
            aabb: AABB {
                min: read_vec3(reader),
                max: read_vec3(reader),
            },

            num_bits_x: reader.read_bits_unchecked(5) + 1,
            num_bits_y: reader.read_bits_unchecked(5) + 1,
            num_bits_z: reader.read_bits_unchecked(5) + 1,

            num_bits_tex_x: reader.read_bits_unchecked(5) + 1,
            num_bits_tex_y: reader.read_bits_unchecked(5) + 1,

            num_bits_normal: reader.read_bits_unchecked(3) + 1,
            num_bits_idx: reader.read_bits_unchecked(5) + 1,

            num_vertices: reader.read_bits_unchecked(6) + 1,
            num_primitives: reader.read_bits_unchecked(7) + 1,

            data_offset: reader.read_bits_unchecked(32),
        }
    }
}

#[spirv(mesh_ext(
    threads(32),
    output_vertices = 64,
    output_primitives_ext = 124,
    output_triangles_ext
))]
pub fn pass_mesh(
    #[spirv(global_invocation_id)] giid: UVec3,
    #[spirv(workgroup_id)] wgid: UVec3,
    #[spirv(position)] positions: &mut [Vec4; 64],
    colors: &mut [Vec3; 64],
    #[spirv(primitive_triangle_indices_ext)] indices: &mut [UVec3; 124],
    #[spirv(push_constant)] view_projecton_matrix: &Mat4,
    #[spirv(descriptor_set = 0, binding = 0, storage_buffer)] mesh_data: &[u32],
) {
    let mut reader = BitReader::new(mesh_data, wgid.x as usize * 270);
    let meshlet = unsafe { Meshlet::from_bits(&mut reader) };

    unsafe {
        set_mesh_outputs_ext(meshlet.num_vertices, meshlet.num_primitives);
    }

    let num_bits_vertex = (meshlet.num_bits_x
        + meshlet.num_bits_y
        + meshlet.num_bits_z
        + meshlet.num_bits_tex_x
        + meshlet.num_bits_tex_y
        + meshlet.num_bits_normal * 3) as usize;

    let meshlet_color = murmur_hash_11_color(wgid.x);

    let mut i = 0;
    while i < meshlet.num_vertices as usize {
        let mut reader = BitReader::new(
            mesh_data,
            (meshlet.data_offset as usize) + i * num_bits_vertex,
        );

        unsafe {
            let quantized_x = reader.read_bits_unchecked(meshlet.num_bits_x);
            let quantized_y = reader.read_bits_unchecked(meshlet.num_bits_y);
            let quantized_z = reader.read_bits_unchecked(meshlet.num_bits_z);

            let x = meshlet.aabb.min.x
                + dequantize_unorm(quantized_x, meshlet.num_bits_x)
                    * (meshlet.aabb.max.x - meshlet.aabb.min.x);
            let y = meshlet.aabb.min.y
                + dequantize_unorm(quantized_y, meshlet.num_bits_y)
                    * (meshlet.aabb.max.y - meshlet.aabb.min.y);
            let z = meshlet.aabb.min.z
                + dequantize_unorm(quantized_z, meshlet.num_bits_z)
                    * (meshlet.aabb.max.z - meshlet.aabb.min.z);

            let tex_x = dequantize_unorm(
                reader.read_bits_unchecked(meshlet.num_bits_tex_x),
                meshlet.num_bits_tex_x,
            );
            let tex_y = dequantize_unorm(
                reader.read_bits_unchecked(meshlet.num_bits_tex_y),
                meshlet.num_bits_tex_x,
            );

            let normal_x = dequantize_unorm(
                reader.read_bits_unchecked(meshlet.num_bits_normal),
                meshlet.num_bits_normal,
            );
            let normal_y = dequantize_unorm(
                reader.read_bits_unchecked(meshlet.num_bits_normal),
                meshlet.num_bits_normal,
            );
            let normal_z = dequantize_unorm(
                reader.read_bits_unchecked(meshlet.num_bits_normal),
                meshlet.num_bits_normal,
            );

            positions[i] = *view_projecton_matrix * Vec4::new(x, y, z, 1.0);
            colors[i] = meshlet_color;
        }

        i += 32;
    }

    let index_offset =
        (meshlet.data_offset as usize) + (meshlet.num_vertices as usize) * num_bits_vertex;
    let num_bits_primitive = 3 * (meshlet.num_bits_idx as usize);

    i = 0;
    while i < meshlet.num_primitives as usize {
        let mut reader = BitReader::new(mesh_data, index_offset + i * num_bits_primitive);

        unsafe {
            let a = reader.read_bits_unchecked(meshlet.num_bits_idx);
            let b = reader.read_bits_unchecked(meshlet.num_bits_idx);
            let c = reader.read_bits_unchecked(meshlet.num_bits_idx);

            indices[i] = UVec3::new(a, b, c);
        }

        i += 32;
    }
}

#[spirv(fragment)]
pub fn pass_frag(color: Vec3, out_color: &mut Vec4) {
    *out_color = Vec4::new(color.x, color.y, color.z, 1.0);
}
