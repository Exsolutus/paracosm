use crate::resource::Buffer;

use bevy_reflect::TypeUuid;

#[derive(Debug)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 3],
}

// TODO: allow values to be unloaded after been submitting to the GPU to conserve memory
#[derive(Debug, TypeUuid)]
#[uuid = "8ecbac0f-f545-4473-ad43-e1f4243af51e"]
pub struct Mesh {
    _vertices: Vec<Vertex>,
    _buffer: Buffer,
}