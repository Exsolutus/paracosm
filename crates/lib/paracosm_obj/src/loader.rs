use anyhow::Result;
use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_log::prelude::*;
use bevy_math::prelude::*;
use bevy_utils::{BoxedFuture, HashMap};
use obj::raw::{object::Polygon, RawObj};
use paracosm_render::{
    mesh::{Mesh, Vertex},
    //render_resource::PrimitiveTopology,
};
use thiserror::Error;

#[derive(Default)]
pub struct ObjLoader;

impl AssetLoader for ObjLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy_asset::LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move { Ok(load_obj(bytes, load_context).await?) })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["obj"];
        EXTENSIONS
    }
}

#[derive(Error, Debug)]
pub enum ObjError {
    #[error("Invalid OBJ file: {0}")]
    Gltf(#[from] obj::ObjError),
    #[error("Mesh is not triangulated.")]
    NonTriangulatedMesh,
}

async fn load_obj<'a, 'b>(
    bytes: &'a [u8],
    load_context: &'a mut LoadContext<'b>,
) -> Result<(), ObjError> {
    let mesh = load_obj_from_bytes(bytes)?;
    load_context.set_default_asset(LoadedAsset::new(mesh));
    
    Ok(())
}

type VertexKey = (usize, usize, usize);

struct MeshIndices {
    indices: Vec<u32>,
    saved: HashMap<VertexKey, u32>,
    next: u32,
}

impl MeshIndices {
    pub fn new(capacity: usize) -> Self {
        Self {
            indices: Vec::with_capacity(capacity),
            saved: HashMap::with_capacity(capacity),
            next: 0,
        }
    }

    pub fn insert<F: FnOnce()>(&mut self, key: VertexKey, create_vertex: F) {
        // Check if the vertex is already saved
        match self.saved.get(&key) {
            Some(index) => self.indices.push(*index), // If saved, just use the existing index
            None => {
                // Save the index to both the indices and saved
                self.indices.push(self.next);
                self.saved.insert(key, self.next);
                // Increment next index
                self.next += 1;
                // Create a vertex externally
                create_vertex()
            }
        }
    }
}

impl From<MeshIndices> for Vec<u32> {
    fn from(val: MeshIndices) -> Self {
        val.indices
    }
}

pub fn load_obj_from_bytes(bytes: &[u8]) -> Result<Mesh, ObjError> {
    let raw = obj::raw::parse_obj(bytes)?;
    let vertcount = raw.polygons.len() * 3;

    let mut indices = MeshIndices::new(vertcount);

    let mut vertices = Vec::with_capacity(vertcount);

    for polygon in &raw.polygons {
        match polygon {
            Polygon::P(poly) if poly.len() == 3 => {
                let normal = calculate_normal(&raw, poly);

                for ipos in poly {
                    indices.insert((*ipos, 0, 0), || {
                        let position = convert_position(&raw, *ipos);
                        vertices.push(Vertex::new(
                            position, 
                            normal, 
                            position
                        ));
                    });
                }
            }
            Polygon::PT(poly) if poly.len() == 3 => {
                let triangle: Vec<usize> = poly.iter().map(|(ipos, _)| *ipos).collect();
                let normal = calculate_normal(&raw, &triangle);

                for (ipos, itex) in poly {
                    indices.insert((*ipos, 0, *itex), || {
                        let position = convert_position(&raw, *ipos);
                        vertices.push(Vertex::new(
                            position, 
                            normal, 
                            position
                        ));
                    });
                }
            }
            Polygon::PN(poly) if poly.len() == 3 => {
                for (ipos, inorm) in poly {
                    indices.insert((*ipos, *inorm, 0), || {
                        let position = convert_position(&raw, *ipos);
                        vertices.push(Vertex::new(
                            position, 
                            convert_normal(&raw, *inorm), 
                            position
                        ));
                    });
                }
            }
            Polygon::PTN(poly) if poly.len() == 3 => {
                for (ipos, itex, inorm) in poly {
                    indices.insert((*ipos, *inorm, *itex), || {
                        let position = convert_position(&raw, *ipos);
                        vertices.push(Vertex::new(
                            position, 
                            convert_normal(&raw, *inorm), 
                            position
                        ));
                    });
                }
            }
            _ => return Err(ObjError::NonTriangulatedMesh),
        }
    }

    debug!("\npoly count: {} \nvertex count: {} \nindex count: {}", raw.polygons.len(), vertices.len(), indices.indices.len());
    let mesh = Mesh::with_geometry(vertices, indices.indices);

    Ok(mesh)
}

fn convert_position(raw: &RawObj, index: usize) -> Vec3 {
    let position = raw.positions[index];
    Vec3::new(position.0, position.1, position.2)
}

fn convert_normal(raw: &RawObj, index: usize) -> Vec3 {
    let normal = raw.normals[index];
    Vec3::new(normal.0, normal.1, normal.2)
}

fn convert_texture(raw: &RawObj, index: usize) -> Vec3 {
    let tex_coord = raw.tex_coords[index];
    // Flip UV for correct values
    Vec3::new(tex_coord.0, 1.0 - tex_coord.1, 0.0)
}

/// Simple and inaccurate normal calculation
fn calculate_normal(raw: &RawObj, polygon: &[usize]) -> Vec3 {
    // Extract triangle
    let triangle: Vec<Vec3> = polygon
        .iter()
        .map(|index| convert_position(raw, *index) )
        .collect();

    // Calculate normal
    let v1 = triangle[1] - triangle[0];
    let v2 = triangle[2] - triangle[0];
    let n = v1.cross(v2);

    n
}