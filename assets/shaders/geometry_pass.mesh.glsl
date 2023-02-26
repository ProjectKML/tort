#version 460

#extension GL_EXT_mesh_shader : require

layout(local_size_x = 1) in;
layout(max_vertices = 3, max_primitives = 1, triangles) out;

layout(location = 0) out vec3[] outColors;

void main() {
    SetMeshOutputsEXT(3, 1);

    gl_PrimitiveTriangleIndicesEXT[0] = uvec3(0, 1, 2);

    gl_MeshVerticesEXT[0].gl_Position = vec4(-0.5, 0.5, 0.0, 1.0);
    gl_MeshVerticesEXT[1].gl_Position = vec4(0.5, 0.5, 0.0, 1.0);
    gl_MeshVerticesEXT[2].gl_Position = vec4(0.0, -0.5, 0.0, 1.0);

    outColors[0] = vec3(1.0, 0.0, 0.0);
    outColors[1] = vec3(0.0, 1.0, 0.0);
    outColors[2] = vec3(0.0, 0.0, 1.0);
}