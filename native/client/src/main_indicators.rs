//! The recorded indicators on the GPU: a frame's overlay uploaded as the
//! browser uploads its player meshes (ten floats a vertex, drawn through one
//! identity instance), and handed to each encode as player geometry.

use native_client::overlays::{RecordedOverlays, VERTEX_FLOATS};
use renderer_core::overlay::{Geometry, MeshDraw};
use renderer_core::paint::PaintGeometry;
use renderer_core::runtime::PlayerGeometry;
use wgpu::{Buffer, BufferUsages, Device, Queue};

/// A frame's player geometry: the uploaded indicators, or none when the
/// replay draws none.
pub(super) fn geometry(meshes: Option<&IndicatorMeshes>) -> PlayerGeometry<'_> {
    meshes.map_or_else(PlayerGeometry::default, IndicatorMeshes::geometry)
}

struct Mesh {
    buffer: Buffer,
    vertices: u32,
}

pub(super) struct IndicatorMeshes {
    /// Placement (xyz, yaw), tint (rgb) and highlight: in place, untinted,
    /// as the browser's `overlay_instances`.
    instances: Buffer,
    /// Opaque, translucent, painted and painted-marching, as `Channels`.
    meshes: [Option<Mesh>; 4],
    /// The recorded parts the uploaded overlay concatenates.
    shown: Option<Vec<u32>>,
}

impl IndicatorMeshes {
    pub fn new(device: &Device, queue: &Queue) -> Self {
        Self {
            instances: renderer_core::buffer(
                device,
                queue,
                "overlay-instance",
                BufferUsages::VERTEX,
                bytemuck::cast_slice(&[0.0f32, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0]),
            ),
            meshes: Default::default(),
            shown: None,
        }
    }

    /// Uploads the overlay the page drew with record `index`, unless it is
    /// the one already uploaded.
    pub fn show(
        &mut self,
        device: &Device,
        queue: &Queue,
        overlays: &RecordedOverlays,
        index: usize,
    ) {
        let parts = overlays.parts_of(index);
        if self.shown.as_deref() == Some(parts) {
            return;
        }
        self.shown = Some(parts.to_vec());
        self.meshes = overlays.channels(index).map(|floats| {
            (!floats.is_empty()).then(|| Mesh {
                vertices: (floats.len() / VERTEX_FLOATS) as u32,
                buffer: renderer_core::buffer(
                    device,
                    queue,
                    "recorded-overlay",
                    BufferUsages::VERTEX,
                    bytemuck::cast_slice(&floats),
                ),
            })
        });
    }

    /// The uploaded overlay as the frame's player geometry.
    fn geometry(&self) -> PlayerGeometry<'_> {
        let draw = |channel: usize| {
            self.meshes[channel].as_ref().map(|mesh| MeshDraw {
                vertices: &mesh.buffer,
                instances: &self.instances,
                vertex_count: mesh.vertices,
            })
        };
        PlayerGeometry {
            overlay: Geometry {
                opaque: draw(0),
                translucent: draw(1),
            },
            paint: PaintGeometry {
                still: draw(2),
                marching: draw(3),
                pointer: None,
            },
        }
    }
}
