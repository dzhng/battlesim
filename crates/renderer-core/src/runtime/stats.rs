//! Facts captured with canonical preparation, serialized only on inspection.
use super::*;
use serde_json::{json, Value};

#[derive(Default)]
pub(super) struct FrameMetadata {
    clock: f64,
    palette_matrices: usize,
    culled_bodies: u32,
    building_population: Option<(usize, usize)>,
    building_walk: Option<[usize; 12]>,
    fallen: usize,
    scenery_walk: Option<[usize; 8]>,
    scenery_pending: bool,
    scenery_falling: bool,
    fog: Option<presentation::fog::sight::FogStats>,
    scars: presentation::scars::ScarStats,
    receiver_range: [f32; 2],
    grass_kinds: Vec<String>,
}
impl FrameMetadata {
    pub(super) fn new(presentation: &Presentation) -> Self {
        let mut facts = Self {
            grass_kinds: presentation
                .ground()
                .map_or_else(Vec::new, |ground| ground.grass_kinds()),
            ..Default::default()
        };
        facts.update(presentation);
        facts
    }
    pub(super) fn update(&mut self, presentation: &Presentation) {
        let frame = presentation.last_frame();
        self.clock = frame.clock;
        self.palette_matrices = frame.palettes.len();
        self.culled_bodies = frame.counts.culled;
        self.building_population = presentation.building_population();
        self.building_walk = presentation
            .building_stats()
            .map(|s| s.try_into().expect("canonical building walk"));
        self.fallen = presentation.fallen().len();
        self.scenery_walk = presentation.scenery_stats();
        self.scenery_pending = presentation.scenery_pending();
        self.scenery_falling = presentation.scenery_falling();
        self.fog = presentation.fog_stats();
        self.scars = presentation.scar_stats();
        self.receiver_range = frame.shadow.as_ref().map_or([0.; 2], |fit| {
            [fit.receiver.control[2], fit.receiver.control[0]]
        });
    }
}

fn extend(target: &mut Value, other: Value) {
    target
        .as_object_mut()
        .expect("diagnostic object")
        .extend(other.as_object().expect("diagnostic fields").clone());
}

impl Runtime {
    pub fn drawn_instances(&self) -> Value {
        let mut drawn = self.models.drawn_instances();
        drawn["scenery"] = self.scenery.drawn_instances();
        drawn
    }

    pub fn submitted(&self) -> crate::submitted::SubmittedFrame {
        let mut work = self.ground.submissions.snapshot();
        work.add(self.models.submissions.snapshot());
        work.add(self.scenery.submissions.snapshot());
        work
    }

    /// Scene facts and issued work; GPU timer and platform resources are added
    /// by their lifetime owners. Inspection never waits for the GPU.
    pub fn frame_stats(&self) -> Value {
        let facts = &self.frame_metadata;
        let mut models = self.models.diagnostics();
        extend(
            &mut models,
            json!({
                "paletteMatrices": facts.palette_matrices,
                "culledBodies": facts.culled_bodies,
            }),
        );
        let mut buildings = self.models.building_diagnostics();
        let walk = facts.building_walk.unwrap_or_default();
        let (population, capacity) = facts.building_population.unwrap_or_default();
        extend(
            &mut buildings,
            json!({
                "buildings": population, "fallen": facts.fallen,
                "chunks": walk[0], "coarse": walk[1], "residentChunks": walk[2],
                "pool": {"capacity": capacity, "used": walk[3]},
                "expandedRows": walk[4], "refused": walk[5], "ruins": walk[6],
                "ruinTiers": &walk[7..11], "visitedBounds": walk[11],
            }),
        );
        let mut fog = facts.fog.as_ref().map_or_else(
            || {
                json!({
                    "enabled": false, "eyes": 0, "slots": 0, "occluders": 0,
                    "rebuilt": 0, "pending": 0, "rebuiltTotal": 0,
                })
            },
            |stats| serde_json::to_value(stats).expect("canonical fog stats"),
        );
        extend(&mut fog, self.fog.allocation_diagnostics());
        let mut scars = serde_json::to_value(&facts.scars).expect("canonical scar stats");
        extend(
            &mut scars,
            self.ground.material.scars.allocation_diagnostics(),
        );
        let grass = self.grass.as_ref().map_or_else(
            || {
                json!({
                    "enabled": false, "tiles": 0, "capacity": [0, 0], "clumpBytes": 0, "kinds": [],
                })
            },
            |grass| {
                let mut stats = grass.diagnostics();
                stats["enabled"] = self.config.controls.grass.into();
                stats["kinds"] = json!(facts.grass_kinds);
                stats
            },
        );
        let look = self.fog_mask.look();
        let mask = self.fog_mask.mask_view();
        let mut memory = crate::memory::ResourceMemory::default();
        self.record_memory(&mut memory);
        let mut depth = self.pipelines.depth_state("worldPrepass");
        depth["clearValue"] = 0.into();
        let mut stats = json!({
            "width": self.targets.size[0], "height": self.targets.size[1],
            "frames": self.frame.frames(), "clock": facts.clock, "gpu": null,
            "instances": self.models.drawn().listed,
            "view": self.config.controls.view,
            "models": models, "buildings": buildings, "submitted": self.submitted(),
            "depth": depth,
            "memory": memory.counts(),
            "shadow": {"cascades": self.shadow.active(), "receiverRange": facts.receiver_range},
            "fog": fog,
            "fogEdge": {
                "edgeSoftnessPx": look.edge_softness_px, "rimWidthPx": look.rim_width_px,
                "reachPx": if look.has_edge() && mask == crate::fog_mask::FogMaskView::None {look.reach_px} else {0.},
                "maskView": match mask {crate::fog_mask::FogMaskView::None => "none", crate::fog_mask::FogMaskView::Fog => "fog", crate::fog_mask::FogMaskView::Ground => "ground", crate::fog_mask::FogMaskView::Classes => "classes"},
            },
            "overlay": {"glowRadiusPx": self.overlay_glow[0], "glowStrength": self.overlay_glow[2]},
            "scenery": self.scenery.diagnostics(facts.scenery_walk.unwrap_or_default(), facts.scenery_pending, facts.scenery_falling),
            "grass": grass, "scars": scars,
            "effects": {"instances": self.effects.count(), "capacity": self.effects.capacity(), "lights": self.environment.cast_light_count()},
        });
        extend(&mut stats, self.ground.mesh_diagnostics());
        stats
    }
}
