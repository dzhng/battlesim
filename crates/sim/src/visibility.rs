//! The ground visibility field ("fog"): which ground cells a side can see
//! right now, by radial line-of-sight sweeps from each own unit's eye over the
//! true terrain, solid props and foliage. Identification is decided per target
//! by `sensing`; this field is what the player sees as clear versus fogged.
use contract::observation::VisibilityField;
use contract::scenario::SensorRules;

use crate::math::{v2, V3};
use crate::world::WorldGeometry;

/// Solid prop tops per cell: tall solids occlude sight across their cell.
/// Trunks and bridge decks are too thin to fill a cell and are left out.
pub struct OcclusionGrid {
    cell: f64,
    nx: usize,
    ny: usize,
    top: Vec<f64>,
    revision: u64,
}

impl OcclusionGrid {
    pub fn new(world: &WorldGeometry, cell: f64) -> Self {
        let nx = (world.width() / cell).ceil() as usize;
        let ny = (world.depth() / cell).ceil() as usize;
        let mut grid = OcclusionGrid {
            cell,
            nx,
            ny,
            top: Vec::new(),
            revision: u64::MAX,
        };
        grid.refresh(world);
        grid
    }

    /// Rebuild when the world's obstacles changed.
    pub fn refresh(&mut self, world: &WorldGeometry) {
        if self.revision == world.obstacle_revision() {
            return;
        }
        self.revision = world.obstacle_revision();
        self.top = vec![f64::NEG_INFINITY; self.nx * self.ny];
        for prop in world.props().filter(|p| p.kind.blocks_movement()) {
            let r = prop.footprint_radius();
            let i0 = ((prop.center.x - r) / self.cell).floor().max(0.0) as usize;
            let j0 = ((prop.center.y - r) / self.cell).floor().max(0.0) as usize;
            let i1 = (((prop.center.x + r) / self.cell).floor() as usize).min(self.nx - 1);
            let j1 = (((prop.center.y + r) / self.cell).floor() as usize).min(self.ny - 1);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let c = v2((i as f64 + 0.5) * self.cell, (j as f64 + 0.5) * self.cell);
                    if prop.footprint_contains(c, 0.0) {
                        let top = &mut self.top[j * self.nx + i];
                        *top = top.max(prop.top_z());
                    }
                }
            }
        }
    }

    pub fn field(&self) -> VisibilityField {
        VisibilityField {
            cell_m: self.cell,
            nx: self.nx as u32,
            ny: self.ny as u32,
            bits: vec![0; (self.nx * self.ny).div_ceil(32)],
        }
    }
}

/// Mark every cell visible from `eye` within `range` into `field`.
pub fn sweep(
    world: &WorldGeometry,
    grid: &OcclusionGrid,
    s: &SensorRules,
    eye: V3,
    range: f64,
    field: &mut VisibilityField,
) {
    let cell = grid.cell;
    // The ground an observer stands on is seen (rays start one cell out).
    let (ei, ej) = ((eye.x / cell).floor(), (eye.y / cell).floor());
    if ei >= 0.0 && ej >= 0.0 && (ei as usize) < grid.nx && (ej as usize) < grid.ny {
        let idx = ej as usize * grid.nx + ei as usize;
        field.bits[idx / 32] |= 1 << (idx % 32);
    }
    let rays = ((std::f64::consts::TAU * range / cell).ceil() as usize).max(64);
    let steps = (range / cell).ceil() as usize;
    for r in 0..rays {
        let angle = r as f64 / rays as f64 * std::f64::consts::TAU;
        let dir = v2(angle.cos(), angle.sin());
        let mut horizon = f64::NEG_INFINITY; // steepest occluding slope so far
        let mut foliage = 0.0;
        for k in 1..=steps {
            let dist = k as f64 * cell;
            let p = eye.xy() + dir * dist;
            let Some(ground) = world.height_at(p.x, p.y) else {
                break;
            };
            let i = (p.x / cell) as usize;
            let j = (p.y / cell) as usize;
            if i >= grid.nx || j >= grid.ny {
                break;
            }
            let target = ground + s.fog_target_height_m;
            let slope = (target - eye.z) / dist;
            let reach = range * (-foliage / s.forest_attenuation_m).exp();
            if slope >= horizon && foliage < s.forest_full_block_m && dist <= reach {
                let idx = j * grid.nx + i;
                field.bits[idx / 32] |= 1 << (idx % 32);
            }
            // Foliage between eye and later cells, below the canopy only.
            let sight_z = eye.z + horizon.max(slope) * dist;
            if world.forest_depth(p.x, p.y).is_some() {
                let canopy = world
                    .forests()
                    .iter()
                    .filter(|f| crate::world::in_forest(f, p.x, p.y))
                    .map(|f| f.canopy_height_m)
                    .fold(0.0, f64::max);
                if sight_z < ground + canopy {
                    foliage += cell;
                }
            }
            let occluder = ground.max(grid.top[j * grid.nx + i]);
            horizon = horizon.max((occluder - eye.z) / dist);
        }
    }
}
