use std::{
    io::{Cursor, Read},
    mem,
    path::Path,
};

use bitstream_io::{BitWrite, BitWriter};
use bytemuck::{self, Pod, Zeroable};
use fast_obj::ObjLoadError;
use meshopt::{DecodePosition, VertexDataAdapter};
use tort_math::{AABB, Vec2, Vec3};

#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
#[repr(C)]
struct Vertex {
    position: Vec3,
    tex_coord: Vec2,
    normal: Vec3,
}

impl Vertex {
    #[inline]
    pub fn new(position: Vec3, tex_coord: Vec2, normal: Vec3) -> Self {
        Self {
            position,
            tex_coord,
            normal,
        }
    }
}

impl DecodePosition for Vertex {
    #[inline]
    fn decode_position(&self) -> [f32; 3] {
        self.position.into()
    }
}

struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct SimpleMeshBuildSettings {}

fn load_mesh(path: impl AsRef<Path>) -> Result<Mesh, ObjLoadError> {
    let mesh = fast_obj::Mesh::new(path)?;

    let mut vertices = vec![Default::default(); mesh.indices().len()];

    let positions = mesh.positions();
    let tex_coords = mesh.texcoords();
    let normals = mesh.normals();
    let indices = mesh.indices();

    for (i, index) in indices.iter().enumerate() {
        let position_idx = 3 * index.p as usize;
        let tex_coord_idx = 2 * index.t as usize;
        let normal_idx = 3 * index.n as usize;

        vertices[i] = Vertex::new(
            Vec3::new(
                positions[position_idx],
                positions[position_idx + 1],
                positions[position_idx + 2],
            ),
            Vec2::new(tex_coords[tex_coord_idx], tex_coords[tex_coord_idx + 1]),
            Vec3::new(
                normals[normal_idx],
                normals[normal_idx + 1],
                normals[normal_idx + 2],
            ),
        );
    }

    let (vertex_count, remap) = meshopt::generate_vertex_remap(&vertices, None);
    vertices.shrink_to(vertex_count);

    Ok(Mesh {
        vertices: meshopt::remap_vertex_buffer(&vertices, vertex_count, &remap),
        indices: meshopt::remap_index_buffer(None, indices.len(), &remap),
    })
}

const MAX_VERTICES: usize = 64;
const MAX_TRIANGLES: usize = 124;
const CONE_WEIGHT: f32 = 0.0;

fn build_from_mesh(mesh: &Mesh, settings: &SimpleMeshBuildSettings) -> anyhow::Result<Vec<u8>> {
    let meshlets = meshopt::build_meshlets(
        &mesh.indices,
        &VertexDataAdapter::new(
            bytemuck::cast_slice(&mesh.vertices),
            mem::size_of::<Vertex>(),
            0,
        )
            ?,
        MAX_VERTICES,
        MAX_TRIANGLES,
        CONE_WEIGHT,
    );

    let mut bit_writer = BitWriter::<_, bitstream_io::LittleEndian>::new(Cursor::new(Vec::new()));

    let mut data_offset = meshlets.len() * 78;

    for meshlet in meshlets.iter() {
        let num_bits_x: u32 = 32;
        let num_bits_y: u32 = 32;
        let num_bits_z: u32 = 32;

        let num_bits_tex_x: u32 = 32;
        let num_bits_tex_y: u32 = 32;

        let num_bits_normal: u32 = 8;
        let num_bits_idx: u32 = 8;

        let aabb = AABB::from(meshlet.vertices.iter().map(|v| &mesh.vertices[*v as usize].position));

        bit_writer.write(32, aabb.min.x.to_bits())?;
        bit_writer.write(32, aabb.min.y.to_bits())?;
        bit_writer.write(32, aabb.min.z.to_bits())?;
        bit_writer.write(32, aabb.max.x.to_bits())?;
        bit_writer.write(32, aabb.max.y.to_bits())?;
        bit_writer.write(32, aabb.max.z.to_bits())?;

        bit_writer.write(5, num_bits_x - 1)?; //x
        bit_writer.write(5, num_bits_y - 1)?; //y
        bit_writer.write(5, num_bits_z - 1)?; //z

        bit_writer.write(5, num_bits_tex_x - 1)?; //uv_x
        bit_writer.write(5, num_bits_tex_y - 1)?; //uv_y

        bit_writer.write(3, num_bits_normal - 1)?; //normal

        bit_writer.write(5, 31)?; //index

        bit_writer
            .write(6, meshlet.vertices.len() as u32 - 1)?;
        bit_writer
            .write(7, (meshlet.triangles.len() / 3) as u32 - 1)?;

        bit_writer.write(32, data_offset as u32)?;


        data_offset += (num_bits_x
            + num_bits_y
            + num_bits_z
            + num_bits_tex_x
            + num_bits_tex_y
            + num_bits_normal * 3
            + num_bits_idx) as usize;
    }

    for meshlet in meshlets.iter() {
        let num_bits_x: u32 = 32;
        let num_bits_y: u32 = 32;
        let num_bits_z: u32 = 32;

        let num_bits_tex_x: u32 = 32;
        let num_bits_tex_y: u32 = 32;

        let num_bits_normal: u32 = 8;
        let num_bits_idx: u32 = 8;

        let aabb = AABB::from(meshlet.vertices.iter().map(|v| &mesh.vertices[*v as usize].position));
        //TODO: we dont want to compute the aabb twice

        for vertex_index in meshlet.vertices {
            let vertex = mesh.vertices[*vertex_index as usize];

            let x = (vertex.position.x - aabb.min.x) / (aabb.max.x - aabb.min.x);
            let y = (vertex.position.y - aabb.min.y) / (aabb.max.y - aabb.min.y);
            let z = (vertex.position.z - aabb.min.z) / (aabb.max.z - aabb.min.z);

            let quantized_x = meshopt::quantize_unorm(x, num_bits_x as _) as u32;
            let quantized_y = meshopt::quantize_unorm(y, num_bits_y as _) as u32;
            let quantized_z = meshopt::quantize_unorm(z, num_bits_z as _) as u32;

            bit_writer.write(num_bits_x, quantized_x)?;
            bit_writer.write(num_bits_y, quantized_y)?;
            bit_writer.write(num_bits_z, quantized_z)?;

            bit_writer.write(num_bits_tex_x, meshopt::quantize_unorm(vertex.tex_coord.x, num_bits_tex_x as _))?;
            bit_writer.write(num_bits_tex_y, meshopt::quantize_unorm(vertex.tex_coord.y, num_bits_tex_y as _))?;

            bit_writer.write(num_bits_normal, meshopt::quantize_unorm(vertex.normal.x, num_bits_normal as _))?;
            bit_writer.write(num_bits_normal, meshopt::quantize_unorm(vertex.normal.y, num_bits_normal as _))?;
            bit_writer.write(num_bits_normal, meshopt::quantize_unorm(vertex.normal.z, num_bits_normal as _))?;
        }

        for index in meshlet.triangles {
            bit_writer.write(num_bits_idx, *index)?;
        }
    }

    bit_writer.byte_align()?;
    Ok(bit_writer.bytewriter().unwrap().writer().get_mut().clone())
}
