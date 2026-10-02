//! The ground visibility field ("fog"): which ground cells a side can see
//! right now, by radial line-of-sight sweeps from each own unit's eye over the
//! true terrain, solid props and foliage. Identification is decided per target
//! by `sensing`; this field is what the player sees as clear versus fogged.
use contract::observation::VisibilityField;
use contract::scenario::SensorRules;

use crate::math::{v2, V3};
use crate::sight::Sight;
use crate::world::WorldGeometry;

/// Solid prop tops per cell: tall solids occlude sight across their cell.
/// Trunks and bridge decks are too thin to fill a cell and are left out.
pub struct OcclusionGrid {
    cell: f64,
    nx: usize,
    ny: usize,
    top: Vec<f64>,
    /// Four-by-four fog cells per tile; entries are rebuilt only on demand.
    tile_revisions: Vec<TileRevision>,
}

#[derive(Clone, Copy)]
struct TileRevision {
    world: u64,
    buckets: u64,
}

impl OcclusionGrid {
    pub fn new(world: &WorldGeometry, cell: f64) -> Self {
        let nx = (world.width() / cell).ceil() as usize;
        let ny = (world.depth() / cell).ceil() as usize;
        OcclusionGrid {
            cell,
            nx,
            ny,
            top: vec![f64::NEG_INFINITY; nx * ny],
            tile_revisions: vec![
                TileRevision {
                    world: u64::MAX,
                    buckets: u64::MAX
                };
                nx.div_ceil(4) * ny.div_ceil(4)
            ],
        }
    }

    fn top(&mut self, world: &WorldGeometry, i: usize, j: usize) -> f64 {
        let tile = (j / 4) * self.nx.div_ceil(4) + i / 4;
        if self.tile_revisions[tile].world != world.obstacle_revision() {
            let (i0, j0) = (i / 4 * 4, j / 4 * 4);
            let (i1, j1) = ((i0 + 4).min(self.nx), (j0 + 4).min(self.ny));
            let center = v2(
                (i0 + i1) as f64 * 0.5 * self.cell,
                (j0 + j1) as f64 * 0.5 * self.cell,
            );
            let radius = ((i1 - i0) as f64).hypot((j1 - j0) as f64) * 0.5 * self.cell;
            let revision = world.obstacle_revision_near(center, radius);
            self.tile_revisions[tile].world = world.obstacle_revision();
            if self.tile_revisions[tile].buckets == revision {
                return self.top[j * self.nx + i];
            }
            for y in j0..j1 {
                self.top[y * self.nx + i0..y * self.nx + i1].fill(f64::NEG_INFINITY);
            }
            // The world's footprint index already contains every body touching
            // this tile. Test the same fog-cell centres as the solid raster.
            for prop in world
                .props_near(center, radius)
                .into_iter()
                .filter(|p| p.body.occludes)
            {
                let r = prop.footprint_radius();
                let x0 = (((prop.center.x - r) / self.cell).floor().max(0.0) as usize).max(i0);
                let y0 = (((prop.center.y - r) / self.cell).floor().max(0.0) as usize).max(j0);
                let x1 = (((prop.center.x + r) / self.cell).floor() as usize).min(i1 - 1);
                let y1 = (((prop.center.y + r) / self.cell).floor() as usize).min(j1 - 1);
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let c = v2((x as f64 + 0.5) * self.cell, (y as f64 + 0.5) * self.cell);
                        if prop.footprint().contains(c, 0.0) {
                            let top = &mut self.top[y * self.nx + x];
                            *top = top.max(prop.top_z());
                        }
                    }
                }
            }
            self.tile_revisions[tile].buckets = revision;
        }
        self.top[j * self.nx + i]
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

/// Mark every cell visible from `eye` within `sight`'s reach into `field`.
/// Each ray runs only as far as the shape reaches along it.
pub fn sweep(
    world: &WorldGeometry,
    grid: &mut OcclusionGrid,
    s: &SensorRules,
    eye: V3,
    sight: &Sight,
    field: &mut VisibilityField,
) {
    let cell = grid.cell;
    // The ground an observer stands on is seen (rays start one cell out).
    let (ei, ej) = ((eye.x / cell).floor(), (eye.y / cell).floor());
    if ei >= 0.0 && ej >= 0.0 && (ei as usize) < grid.nx && (ej as usize) < grid.ny {
        let idx = ej as usize * grid.nx + ei as usize;
        field.bits[idx / 32] |= 1 << (idx % 32);
    }
    // Rays one cell apart at the farthest reach.
    let rays = ((std::f64::consts::TAU * sight.max_range() / cell).ceil() as usize).max(64);
    // The most a cell's target can rise above the eye: from the highest
    // ground on the map, with a metre's margin over the surface's rounding.
    let rise = world.max_height() + 1.0 + s.fog_target_height_m - eye.z;
    for r in 0..rays {
        let angle = r as f64 / rays as f64 * std::f64::consts::TAU;
        let dir = v2(angle.cos(), angle.sin());
        let range = sight.range_at(angle);
        let steps = (range / cell).ceil() as usize;
        // A side's field is the union of every eye. When earlier eyes marked
        // every cell this ray could reach, its terrain and foliage work cannot
        // contribute another bit, whatever would block it.
        // Check the tail first: blocked rays often have an unseen far cell,
        // so this proof can fail before walking the already-visible prefix.
        if (1..=steps).rev().all(|k| {
            let p = eye.xy() + dir * (k as f64 * cell);
            field.visible(p.x, p.y)
        }) {
            continue;
        }
        // Once the horizon is steeper than any later cell's target could be,
        // the rest of the ray marks nothing: stop it there. (Only the result
        // is the same; a slope bound, not a rule.)
        let far = steps as f64 * cell;
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
            // Foliage only accumulates, so the reach only shrinks: once this
            // cell is past it, or the foliage blocks fully, no later cell on
            // the ray can be seen.
            match crate::sensing::foliage_reach(range, foliage, s) {
                Some(reach) if dist <= reach => {}
                _ => break,
            }
            if slope >= horizon {
                let idx = j * grid.nx + i;
                field.bits[idx / 32] |= 1 << (idx % 32);
            }
            // Foliage between eye and later cells, below the canopy only;
            // cleared ground has none (Q16).
            let sight_z = eye.z + horizon.max(slope) * dist;
            let f = world.foliage_at(p.x, p.y);
            if !f.is_open() && sight_z < ground + f.canopy_m {
                foliage += f.depth_per_m * cell;
            }
            let occluder = ground.max(grid.top(world, i, j));
            horizon = horizon.max((occluder - eye.z) / dist);
            let steepest_later = rise / if rise >= 0.0 { dist + cell } else { far };
            if horizon > steepest_later {
                break;
            }
        }
    }
}
