use std::ops::Range;

use meshopt::Meshlet;
use tort_math::{dequantize_unorm, AABB};

use crate::mesh::{SimpleMeshBuildSettings, Vertex};

pub(crate) struct VertexSizeDesc {
    pub num_bits_x: u32,
    pub num_bits_y: u32,
    pub num_bits_z: u32,
}

pub(crate) fn get_bits_per_vertex(
    vertices: &[Vertex],
    meshlet: &Meshlet,
    settings: &SimpleMeshBuildSettings,
) -> VertexSizeDesc {
    let aabb = AABB::from(
        meshlet
            .vertices
            .iter()
            .map(|v| &vertices[*v as usize].position),
    );
    let cluster_range = aabb.range();

    const BITS_RANGE: Range<u32> = 4..31;

    let mut num_bits_x: u32 = 32;
    let mut num_bits_y: u32 = 32;
    let mut num_bits_z: u32 = 32;

    for bits in BITS_RANGE {
        let mut current_error_x: f32 = 0.;
        let mut current_error_y: f32 = 0.;
        let mut current_error_z: f32 = 0.;

        for vertex_index in meshlet.vertices {
            let vertex = &vertices[*vertex_index as usize];

            let x = (vertex.position.x - aabb.min.x) / (aabb.max.x - aabb.min.x);
            let y = (vertex.position.y - aabb.min.y) / (aabb.max.y - aabb.min.y);
            let z = (vertex.position.z - aabb.min.z) / (aabb.max.z - aabb.min.z);

            let quantized_x = meshopt::quantize_unorm(x, bits as _) as u32;
            let quantized_y = meshopt::quantize_unorm(y, bits as _) as u32;
            let quantized_z = meshopt::quantize_unorm(z, bits as _) as u32;

            let dequantized_x = dequantize_unorm(quantized_x, bits as _);
            let dequantized_y = dequantize_unorm(quantized_y, bits as _);
            let dequantized_z = dequantize_unorm(quantized_z, bits as _);

            let error_x = (dequantized_x - x).abs() / cluster_range;
            let error_y = (dequantized_y - y).abs() / cluster_range;
            let error_z = (dequantized_z - z).abs() / cluster_range;

            current_error_x = current_error_x.max(error_x);
            current_error_y = current_error_y.max(error_y);
            current_error_z = current_error_z.max(error_z);
        }

        if current_error_x < settings.error && num_bits_x == 32 {
            num_bits_x = bits;
        }

        if current_error_y < settings.error && num_bits_y == 32 {
            num_bits_y = bits;
        }

        if current_error_z < settings.error && num_bits_z == 32 {
            num_bits_z = bits;
        }

        if num_bits_x != 32 && num_bits_y != 32 && num_bits_z != 32 {
            break
        }
    }

    VertexSizeDesc {
        num_bits_x,
        num_bits_y,
        num_bits_z,
    }
}

#[inline]
pub(crate) fn get_bits_per_index(num_vertices: usize) -> u32 {
    ((num_vertices as f32).log2().ceil()) as u32
}

#[cfg(test)]
mod tests {
    #[test]
    fn dequantize_unorm() {
        fn test_with(_v: f32) {
            let quantized = meshopt::quantize_unorm(0.5, 30) as u32;
            let dequantized = super::dequantize_unorm(quantized, 30);
            assert_eq!(dequantized, 0.5);
        }

        for i in 0..10 {
            let v = 1.0 / (i as f32);
            test_with(v);
        }
    }
}
