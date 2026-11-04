//! External player meshes retained by the browser for device reconstruction.
use super::*;
use renderer_core::overlay::{Geometry, MeshDraw};
use renderer_core::paint::PaintGeometry;
use renderer_core::runtime::PlayerGeometry;

pub(super) struct Mesh {
    vertices: Vec<f32>,
    buffer: Buffer,
}

impl Mesh {
    fn new(device: &Device, queue: &Queue, vertices: Vec<f32>) -> Self {
        let bytes = if vertices.is_empty() {
            &[0; 40]
        } else {
            bytemuck::cast_slice(&vertices)
        };
        Self {
            buffer: renderer_core::buffer(
                device,
                queue,
                "player-geometry",
                BufferUsages::VERTEX,
                bytes,
            ),
            vertices,
        }
    }

    fn draw<'a>(&'a self, instances: &'a Buffer) -> Option<MeshDraw<'a>> {
        (!self.vertices.is_empty()).then_some(MeshDraw {
            vertices: &self.buffer,
            instances,
            vertex_count: (self.vertices.len() / 10) as u32,
        })
    }
}

pub(super) struct PlayerMeshes {
    opaque: Mesh,
    translucent: Mesh,
    paint: Mesh,
    marching: Mesh,
    pointer: Mesh,
}

impl PlayerMeshes {
    pub(super) fn new(device: &Device, queue: &Queue) -> Self {
        Self {
            opaque: Mesh::new(device, queue, Vec::new()),
            translucent: Mesh::new(device, queue, Vec::new()),
            paint: Mesh::new(device, queue, Vec::new()),
            marching: Mesh::new(device, queue, Vec::new()),
            pointer: Mesh::new(device, queue, Vec::new()),
        }
    }

    pub(super) fn draw<'a>(&'a self, instances: &'a Buffer) -> PlayerGeometry<'a> {
        PlayerGeometry {
            overlay: Geometry {
                opaque: self.opaque.draw(instances),
                translucent: self.translucent.draw(instances),
            },
            paint: PaintGeometry {
                still: self.paint.draw(instances),
                marching: self.marching.draw(instances),
                pointer: self.pointer.draw(instances),
            },
        }
    }

    pub(super) fn recreate(&self, device: &Device, queue: &Queue) -> Self {
        let copy = |mesh: &Mesh| Mesh::new(device, queue, mesh.vertices.clone());
        Self {
            opaque: copy(&self.opaque),
            translucent: copy(&self.translucent),
            paint: copy(&self.paint),
            marching: copy(&self.marching),
            pointer: copy(&self.pointer),
        }
    }

    pub(super) fn record_memory(
        &self,
        memory: &mut renderer_core::memory::ResourceMemory,
    ) -> usize {
        [
            &self.opaque,
            &self.translucent,
            &self.paint,
            &self.marching,
            &self.pointer,
        ]
        .into_iter()
        .map(|mesh| {
            memory.buffer(&mesh.buffer);
            size_of_val(mesh.vertices.as_slice())
        })
        .sum()
    }
}

fn validate(meshes: &[&[f32]]) -> Result<(), JsValue> {
    if meshes.iter().any(|mesh| !mesh.len().is_multiple_of(10)) {
        return Err(js_error("player geometry needs ten floats per vertex"));
    }
    Ok(())
}

#[wasm_bindgen]
impl Renderer {
    pub fn set_overlays(
        &mut self,
        opaque: &[f32],
        translucent: &[f32],
        paint: &[f32],
        marching: &[f32],
    ) -> Result<(), JsValue> {
        validate(&[opaque, translucent, paint, marching])?;
        self.player_meshes.opaque = Mesh::new(&self.gpu.device, &self.gpu.queue, opaque.to_vec());
        self.player_meshes.translucent =
            Mesh::new(&self.gpu.device, &self.gpu.queue, translucent.to_vec());
        self.player_meshes.paint = Mesh::new(&self.gpu.device, &self.gpu.queue, paint.to_vec());
        self.player_meshes.marching =
            Mesh::new(&self.gpu.device, &self.gpu.queue, marching.to_vec());
        Ok(())
    }

    pub fn set_pointer_marks(&mut self, pointer: &[f32]) -> Result<(), JsValue> {
        validate(&[pointer])?;
        self.player_meshes.pointer = Mesh::new(&self.gpu.device, &self.gpu.queue, pointer.to_vec());
        Ok(())
    }
}
