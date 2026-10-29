//! The grass: grown on the GPU over the window of world tiles the
//! presentation crate fits to the view, whenever the view moves, then drawn
//! as two tiers of clumps by indexed indirect draws. The tables are the
//! ground's (`Ground::grass_params`, `grass_props`) and the blade shapes
//! packed from the grass appearances' bundles by the same crate.
use presentation::presentation::Presentation;
use wgpu::*;

use crate::bundle::{BundleSource, Shape};
use crate::pipelines::{Bound, BoundCompute, Pipelines};
use crate::{bind, buffer};
use presentation::static_world::ground::grass::{
    self as field, KindMesh, PackedShapes, TierMesh, Window, FIELD_SEGMENTS,
};

/// Two indexed-indirect draws' arguments (5 words each), then each tier's
/// raw count.
const ARGS_WORDS: usize = 12;

/// Pack authored grass bundles through the same decoder and blade layout on every platform.
pub fn pack_shapes(library: &impl BundleSource, names: &[String]) -> Result<PackedShapes, String> {
    let bundles = names
        .iter()
        .map(|name| library.bundle(name))
        .collect::<Result<Vec<_>, _>>()?;
    let kinds = names
        .iter()
        .zip(&bundles)
        .map(|(name, bundle)| {
            let Shape::Static(states) = &bundle.shape else {
                return Err(format!("grass {name} is not a static bundle"));
            };
            let state = states.first().ok_or(format!("grass {name} has no state"))?;
            Ok(KindMesh {
                name,
                tiers: state
                    .tiers
                    .iter()
                    .map(|m| TierMesh {
                        positions: &m.positions,
                        normals: &m.normals,
                        uvs: &m.uvs,
                        colors: &m.colors,
                    })
                    .collect(),
                top: state.top.ok_or(format!("grass {name} has no bounds"))?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    field::pack_shapes(&kinds)
}

/// World-load inputs resolved by the presentation from the authored blade facts.
pub struct GrassData<'a> {
    pub params: &'a [u32],
    pub props: &'a [u32],
    pub shapes: &'a [f32],
    pub max_blades: u32,
    pub capacity: [u32; 2],
}

pub struct Grass {
    build: BoundCompute,
    finish: BoundCompute,
    draw: Bound,
    build_group: BindGroup,
    draw_groups: Vec<BindGroup>,
    params: Buffer,
    props: Buffer,
    clumps: Buffer,
    shapes: Buffer,
    tiers: Vec<Buffer>,
    /// `GrassParams` as the ground laid it, its per-frame fields rewritten
    /// each regrow.
    words: Vec<u32>,
    args: Buffer,
    capacity: [u32; 2],
    clump_stride: u64,
    indices: Vec<(Buffer, u32)>,
    window: Window,
    /// The view the clumps were last grown for (view-projection, pixel
    /// scale): a still view draws the same clumps without regrowing.
    grown_for: Option<([f32; 16], f32)>,
    drawn: bool,
    regrow: bool,
}

impl Grass {
    pub fn from_data(
        device: &Device,
        queue: &Queue,
        pipelines: &Pipelines,
        heights: &Buffer,
        data: GrassData<'_>,
    ) -> Self {
        let words = data.params.to_vec();
        let storage = BufferUsages::STORAGE;
        let uniform = BufferUsages::UNIFORM | BufferUsages::COPY_DST;
        let params = buffer(
            device,
            queue,
            "grass-params",
            uniform,
            bytemuck::cast_slice(&words),
        );
        let props = buffer(
            device,
            queue,
            "grass-props",
            storage,
            bytemuck::cast_slice(data.props),
        );
        let [near, far] = [data.capacity[0] as u64, data.capacity[1] as u64];
        let clumps = device.create_buffer(&BufferDescriptor {
            label: Some("grass-clumps"),
            size: ((near + far) * pipelines.sizes.grass_clump).max(16),
            usage: storage | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let shapes = buffer(
            device,
            queue,
            "grass-shapes",
            storage,
            bytemuck::cast_slice(data.shapes),
        );
        let args = device.create_buffer(&BufferDescriptor {
            label: Some("grass-args"),
            size: (ARGS_WORDS * 4) as u64,
            usage: storage
                | BufferUsages::INDIRECT
                | BufferUsages::COPY_DST
                | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let build_group = bind(
            device,
            pipelines.layout("grassBuildLayout"),
            "grass-build",
            &[
                params.as_entire_binding(),
                heights.as_entire_binding(),
                props.as_entire_binding(),
                clumps.as_entire_binding(),
                args.as_entire_binding(),
            ],
        );
        let mut indices = Vec::new();
        let mut draw_groups = Vec::new();
        let mut tiers = Vec::new();
        for (t, segments) in FIELD_SEGMENTS.into_iter().enumerate() {
            let mut strip = field::strip_indices(data.max_blades, segments);
            let count = strip.len() as u32;
            strip.resize(strip.len().next_multiple_of(2), 0);
            indices.push((
                buffer(
                    device,
                    queue,
                    "grass-indices",
                    BufferUsages::INDEX,
                    bytemuck::cast_slice(&strip),
                ),
                count,
            ));
            let first = if t == 0 { 0 } else { near as u32 };
            let tier = [t as u32, first, field::blade_vertices(segments), 0];
            let tier = buffer(
                device,
                queue,
                "grass-tier",
                BufferUsages::UNIFORM,
                bytemuck::cast_slice(&tier),
            );
            draw_groups.push(bind(
                device,
                pipelines.layout("grassDrawLayout"),
                "grass-draw",
                &[
                    params.as_entire_binding(),
                    tier.as_entire_binding(),
                    clumps.as_entire_binding(),
                    shapes.as_entire_binding(),
                ],
            ));
            tiers.push(tier);
        }
        Self {
            build: pipelines.bound_compute("grassBuild"),
            finish: pipelines.bound_compute("grassFinish"),
            draw: pipelines.bound("grassDraw"),
            build_group,
            draw_groups,
            params,
            props,
            clumps,
            shapes,
            tiers,
            words,
            args,
            capacity: data.capacity,
            clump_stride: pipelines.sizes.grass_clump,
            indices,
            window: Window {
                x0: 0.0,
                y0: 0.0,
                tiles_x: 0,
                tiles_y: 0,
                reach: 0.0,
            },
            grown_for: None,
            drawn: false,
            regrow: false,
        }
    }

    pub fn probe_source(&self) -> Option<crate::probes::grass::GrassProbeSource<'_>> {
        self.drawn
            .then_some(crate::probes::grass::GrassProbeSource {
                args: &self.args,
                clumps: &self.clumps,
                capacity: self.capacity,
                stride: self.clump_stride,
            })
    }

    /// Fits the window to `presentation`'s last frame and, if its view
    /// moved, writes the parameters for a regrow.
    pub fn prepare(&mut self, queue: &Queue, presentation: &Presentation) {
        self.prepare_records(
            queue,
            presentation.grass_window(),
            &presentation.last_frame().camera,
            |words, window| presentation.grass_frame_params(words, window),
        );
    }

    /// Retain the native view-change gate while accepting copied presentation
    /// records. `write_params` supplies the presentation-owned frame fields
    /// only when a regrow is needed.
    pub fn prepare_records(
        &mut self,
        queue: &Queue,
        window: Option<(Window, f64)>,
        camera: &presentation::records::Camera,
        write_params: impl FnOnce(&mut [u32], &Window),
    ) {
        self.drawn = false;
        self.regrow = false;
        let Some((window, scale)) = window else {
            return;
        };
        self.window = window;
        if self.window.tiles_x * self.window.tiles_y == 0 {
            return;
        }
        self.drawn = true;
        let view_proj = camera.view_proj;
        let key = (*view_proj.first_chunk().unwrap(), scale as f32);
        if self.grown_for == Some(key) {
            return;
        }
        self.grown_for = Some(key);
        self.regrow = true;
        write_params(&mut self.words, &self.window);
        queue.write_buffer(&self.params, 0, bytemuck::cast_slice(&self.words));
        let mut args = [0u32; ARGS_WORDS];
        args[0] = self.indices[0].1;
        args[5] = self.indices[1].1;
        queue.write_buffer(&self.args, 0, bytemuck::cast_slice(&args));
    }

    /// Grow the clumps again on the next frame, though the view has not
    /// moved: the scars under them changed.
    pub fn regrow(&mut self) {
        self.grown_for = None;
    }

    /// The window this frame's clumps are grown over, when they regrow: the
    /// scars they read are loaded for it.
    pub fn scar_bounds(&self) -> Option<[f64; 4]> {
        let w = &self.window;
        self.regrow.then(|| {
            [
                w.x0,
                w.y0,
                w.x0 + f64::from(w.tiles_x) * field::TILE_M,
                w.y0 + f64::from(w.tiles_y) * field::TILE_M,
            ]
        })
    }

    /// Grows this frame's clumps, if the view moved (compute, before the
    /// colour pass); `terrain` is the ground's group.
    pub fn encode_build(&self, encoder: &mut CommandEncoder, terrain: (&str, &BindGroup)) {
        if !self.regrow {
            return;
        }
        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("grass-build"),
            timestamp_writes: None,
        });
        let own = ("grassBuildLayout", &self.build_group);
        self.build.set(&mut pass, &[own, terrain]);
        pass.dispatch_workgroups(self.window.tiles_x, self.window.tiles_y, 1);
        self.finish.set(&mut pass, &[own]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    /// Both tiers into the world's colour pass; `groups` the frame's.
    pub fn draw(&self, pass: &mut RenderPass, groups: &[(&str, &BindGroup)]) {
        if !self.drawn {
            return;
        }
        for (t, (group, (indices, _))) in self.draw_groups.iter().zip(&self.indices).enumerate() {
            let mut all = groups.to_vec();
            all.push(("grassDrawLayout", group));
            self.draw.set(pass, &all);
            pass.set_index_buffer(indices.slice(..), IndexFormat::Uint16);
            pass.draw_indexed_indirect(&self.args, (t * 20) as u64);
        }
    }
}

impl Grass {
    pub fn record_memory(&self, memory: &mut crate::memory::ResourceMemory) {
        for buffer in [
            &self.params,
            &self.props,
            &self.clumps,
            &self.shapes,
            &self.args,
        ] {
            memory.buffer(buffer);
        }
        for tier in &self.tiers {
            memory.buffer(tier);
        }
        for (buffer, _) in &self.indices {
            memory.buffer(buffer);
        }
    }
}

impl Grass {
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({
            "tiles": u64::from(self.window.tiles_x) * u64::from(self.window.tiles_y),
            "capacity": self.capacity,
            "clumpBytes": self.clumps.size(),
        })
    }
}
