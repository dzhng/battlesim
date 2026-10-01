//! What a map's grid is before any side learns anything: the terrain under
//! each cell and the bodies the map stands on it. Both sides know that
//! alike when the battle starts and it never changes, so it is built once
//! and every side's [`NavGrid`](super::NavGrid) shares it.
use std::collections::BTreeMap;

use contract::map::MoverClass;

use super::cells::{
    cell_center, cell_of, one_gap, sub_center, Cell, Cells, ALL_FREE, NO_BODY, SUB,
};
use super::regions::{BaseRegions, Rect};
use super::{SlowGround, NAV_CELL_M};
use crate::math::{v2, Obb2};
use crate::world::{Prop, PropId, WorldGeometry};

/// A known body as the grid lays it: what of a prop planning reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Body {
    footprint: Obb2,
    /// Its weight class's rank.
    weight: u8,
    stops_vehicles: bool,
    stops_infantry: bool,
}

impl Body {
    /// `prop` as a grid lays it, if it stops any ground mover.
    pub fn of(prop: &Prop) -> Option<Body> {
        let body = Body {
            footprint: prop.footprint(),
            weight: prop.body.weight_class.rank(),
            stops_vehicles: prop.blocks(MoverClass::Vehicle),
            stops_infantry: prop.blocks(MoverClass::Infantry),
        };
        (body.stops_vehicles || body.stops_infantry).then_some(body)
    }

    /// Radius of the footprint's bounding circle ([`Prop::footprint_radius`]).
    fn radius(&self) -> f64 {
        self.footprint.half.x.hypot(self.footprint.half.y)
    }

    /// The weight rank it adds to the ranks that stop vehicles.
    pub fn stopping(&self) -> Option<usize> {
        (self.stops_vehicles && u32::from(self.weight) < u8::BITS).then_some(self.weight.into())
    }

    /// The rectangle outside which it leaves the ground as it was.
    pub fn rect(&self, soldier_radius: f64) -> Rect {
        let radius = self.radius() + soldier_radius * std::f64::consts::SQRT_2;
        let c = self.footprint.center;
        [c.x - radius, c.y - radius, 2.0 * radius, 2.0 * radius]
    }

    /// How it lies on a grid of `nx` by `ny` cells.
    pub fn stamp(&self, soldier_radius: f64, nx: usize, ny: usize) -> Stamp {
        let r = self.radius();
        // Vehicles over the cells the footprint's circle spans; infantry's
        // sub-cells over the footprint grown by a soldier.
        let reach = r + soldier_radius * std::f64::consts::SQRT_2;
        let c = self.footprint.center;
        let span = |reach: f64| {
            let (i0, j0) = cell_of(c - v2(reach, reach));
            let (i1, j1) = cell_of(c + v2(reach, reach));
            [i0, j0, i1, j1]
        };
        let [i0, j0, i1, j1] = span(reach);
        Stamp {
            body: *self,
            soldier_radius,
            vehicles: span(r),
            cells: [
                i0.max(0),
                j0.max(0),
                i1.min(nx as isize - 1),
                j1.min(ny as isize - 1),
            ],
        }
    }
}

/// A body laid on the grid: the cells it can change, and how.
#[derive(Clone, Copy)]
pub(super) struct Stamp {
    body: Body,
    soldier_radius: f64,
    /// The cells whose vehicles it may stop: `[i0, j0, i1, j1]`.
    vehicles: [isize; 4],
    /// The cells it may change at all, on the map.
    cells: [isize; 4],
}

impl Stamp {
    /// Every cell it may change, as (i, j).
    pub fn cells(&self) -> impl Iterator<Item = (usize, usize)> {
        let [i0, j0, i1, j1] = self.cells;
        (j0..=j1).flat_map(move |j| (i0..=i1).map(move |i| (i as usize, j as usize)))
    }

    pub fn reaches(&self, i: usize, j: usize) -> bool {
        let [i0, j0, i1, j1] = self.cells;
        (i0..=i1).contains(&(i as isize)) && (j0..=j1).contains(&(j as isize))
    }

    /// Lay the body on `cell` at (i, j), one of its [`cells`](Self::cells):
    /// the heaviest body over a vehicle's cell, and the sub-cells a
    /// soldier's disc cannot stand in.
    pub fn lay(&self, cell: &mut Cell, i: usize, j: usize) {
        let Body {
            footprint,
            weight,
            stops_vehicles,
            stops_infantry,
        } = self.body;
        let [vi0, vj0, vi1, vj1] = self.vehicles;
        if stops_vehicles
            && (vi0..=vi1).contains(&(i as isize))
            && (vj0..=vj1).contains(&(j as isize))
            && footprint.contains(cell_center(i, j), NAV_CELL_M / 2.0)
        {
            cell.heaviest = cell.heaviest.max(weight);
        }
        if stops_infantry && cell.free != 0 {
            for bit in 0..SUB * SUB {
                if footprint.contains(sub_center(i, j, bit), self.soldier_radius) {
                    cell.free &= !(1 << bit);
                }
            }
        }
    }
}

/// Bodies are bucketed by the cells they may change, this many a side.
const BUCKET_CELLS: usize = 16;

/// The bucket of body cells that cell (i, j) lies in, on a grid `nx` wide.
pub(super) fn body_bucket(i: usize, j: usize, nx: usize) -> usize {
    (j / BUCKET_CELLS) * nx.div_ceil(BUCKET_CELLS) + i / BUCKET_CELLS
}

/// The buckets `stamp`'s cells lie in.
pub(super) fn body_buckets(stamp: &Stamp, nx: usize) -> impl Iterator<Item = usize> {
    let [i0, j0, i1, j1] = stamp.cells.map(|v| v.div_euclid(BUCKET_CELLS as isize));
    let across = nx.div_ceil(BUCKET_CELLS) as isize;
    (j0..=j1).flat_map(move |j| (i0..=i1).map(move |i| (j * across + i) as usize))
}

/// The map's grid: terrain, and the bodies authored on it where the map
/// put them.
pub struct NavBase {
    pub(super) cells: Cells,
    pub(super) terrain: super::terrain::Terrain,
    /// Infantry's free sub-cells on the bare ground, for the cells where
    /// they are neither all free nor none: those beside ground nobody
    /// crosses.
    shores: BTreeMap<usize, u16>,
    /// The bodies laid, by prop id.
    bodies: Vec<(PropId, Body)>,
    /// Body bucket `k` holds `held[starts[k]..starts[k + 1]]`, indices of
    /// `bodies`.
    starts: Vec<u32>,
    held: Vec<u32>,
    pub(super) regions: BaseRegions,
    pub(super) slow: SlowGround,
    /// How many laid bodies stop vehicles, by weight rank.
    pub(super) stopping: [u32; u8::BITS as usize],
    /// Sizes infantry's sub-cell gaps.
    pub(super) soldier_radius: f64,
}

impl NavBase {
    /// The ground covered by the grid, in metres.
    pub(crate) fn extent(&self) -> [f64; 2] {
        [
            self.cells.nx as f64 * NAV_CELL_M,
            self.cells.ny as f64 * NAV_CELL_M,
        ]
    }

    /// The grid of `world`'s surfaces with `bodies` standing on them.
    /// `soldier_radius` sizes infantry's sub-cell gaps.
    pub fn build<'a>(
        world: &WorldGeometry,
        bodies: impl Iterator<Item = &'a Prop>,
        soldier_radius: f64,
    ) -> Self {
        let nx = (world.width() / NAV_CELL_M).floor() as usize;
        let ny = (world.depth() / NAV_CELL_M).floor() as usize;
        let mut cells = Cells::new(world.slope_cutoff_deg() > 0.0, nx, ny);
        let mut regions = world.navigation_regions();
        // Regions overlap (a forest and its crowns' reach, a road's
        // segments): each cell is read once.
        let mut read = vec![0u64; (nx * ny).div_ceil(64)];
        for &[x, y, w, h] in &regions {
            let (i0, j0) = cell_of(v2(x - NAV_CELL_M, y - NAV_CELL_M));
            let (i1, j1) = cell_of(v2(x + w + NAV_CELL_M, y + h + NAV_CELL_M));
            for j in j0.max(0)..=j1.min(ny as isize - 1) {
                for i in i0.max(0)..=i1.min(nx as isize - 1) {
                    let (i, j) = (i as usize, j as usize);
                    let at = j * nx + i;
                    if read[at / 64] >> (at % 64) & 1 == 1 {
                        continue;
                    }
                    read[at / 64] |= 1 << (at % 64);
                    let c = cell_center(i, j);
                    if let Some(s) = world.surface_at(c.x, c.y) {
                        cells.set(
                            at,
                            Cell {
                                ground: s.traversable,
                                infantry: s.traversable,
                                heaviest: NO_BODY,
                                road_factor: s.road_factor,
                                forest: s.forest,
                                slope_deg: s.slope_deg,
                                free: if s.traversable { ALL_FREE } else { 0 },
                                open: if s.traversable {
                                    u8::from(i + 1 < nx) + 2 * u8::from(j + 1 < ny)
                                } else {
                                    0
                                },
                            },
                        );
                    }
                }
            }
        }
        drop(read);
        // Beside ground nobody crosses (water, a slope past the cutoff), each
        // sub-cell reads the ground under its own centre.
        let mut border = Vec::new();
        for (at, cell) in cells.stored() {
            if cell.free != 0 || *cell == cells.default_at(at) {
                continue;
            }
            let (i, j) = (at % nx, at / nx);
            for jj in j.saturating_sub(1)..=(j + 1).min(ny - 1) {
                for ii in i.saturating_sub(1)..=(i + 1).min(nx - 1) {
                    if cells[jj * nx + ii].free != 0 {
                        border.push(jj * nx + ii);
                    }
                }
            }
        }
        border.sort_unstable();
        border.dedup();
        let mut shores = BTreeMap::new();
        for at in border {
            let (i, j) = (at % nx, at / nx);
            let mut free = cells[at].free;
            for bit in 0..SUB * SUB {
                let c = sub_center(i, j, bit);
                if !world.traversable_at(c.x, c.y) {
                    free &= !(1 << bit);
                }
            }
            if free != ALL_FREE {
                shores.insert(at, free);
                cells[at].free = free;
            }
        }
        let terrain = super::terrain::Terrain::new(&cells);
        let mut bodies: Vec<(PropId, Body)> = bodies
            .filter_map(|prop| Some((prop.id, Body::of(prop)?)))
            .collect();
        bodies.sort_unstable_by_key(|(id, _)| *id);
        let mut stopping = [0; u8::BITS as usize];
        let buckets = nx.div_ceil(BUCKET_CELLS) * ny.div_ceil(BUCKET_CELLS);
        let mut starts = vec![0u32; buckets + 1];
        for (_, body) in &bodies {
            if let Some(rank) = body.stopping() {
                stopping[rank] += 1;
            }
            regions.push(body.rect(soldier_radius));
            let stamp = body.stamp(soldier_radius, nx, ny);
            for (i, j) in stamp.cells() {
                stamp.lay(&mut cells[j * nx + i], i, j);
            }
            for k in body_buckets(&stamp, nx) {
                starts[k + 1] += 1;
            }
        }
        for k in 0..buckets {
            starts[k + 1] += starts[k];
        }
        let mut held = vec![0u32; starts[buckets] as usize];
        let mut next = starts.clone();
        for (index, (_, body)) in bodies.iter().enumerate() {
            for k in body_buckets(&body.stamp(soldier_radius, nx, ny), nx) {
                held[next[k] as usize] = index as u32;
                next[k] += 1;
            }
        }
        // The ground layer wrote every cell as open to a squad wherever it
        // is walkable. Where open flat ground is walkable too, that stands
        // unless the cell or a neighbour is not wholly free.
        let walkable = cells.implicit[0].ground;
        let laid: Vec<usize> = cells
            .stored()
            .filter(|(at, cell)| match walkable {
                true => cell.free != ALL_FREE,
                false => **cell != cells.default_at(*at),
            })
            .map(|(at, _)| at)
            .collect();
        settle(&mut cells, &laid);
        NavBase {
            terrain,
            slow: SlowGround::new(&cells),
            cells,
            shores,
            bodies,
            starts,
            held,
            regions: BaseRegions::new(regions, world.width(), world.depth()),
            stopping,
            soldier_radius,
        }
    }

    /// The body the map lays for prop `id`.
    pub(super) fn body(&self, id: PropId) -> Option<Body> {
        let at = self.bodies.binary_search_by_key(&id, |(id, _)| *id).ok()?;
        Some(self.bodies[at].1)
    }

    /// The map's bodies that may change a cell of body bucket `bucket`.
    pub(super) fn bodies_in(&self, bucket: usize) -> impl Iterator<Item = &(PropId, Body)> {
        self.held[self.starts[bucket] as usize..self.starts[bucket + 1] as usize]
            .iter()
            .map(|&index| &self.bodies[index as usize])
    }

    /// Infantry's free sub-cells at `at` with no body laid, where its
    /// ground is walkable or not as `ground` says.
    pub(super) fn bare(&self, at: usize, ground: bool) -> u16 {
        let whole = if ground { ALL_FREE } else { 0 };
        self.shores.get(&at).copied().unwrap_or(whole)
    }
}

/// Once the bodies over `laid` cells have changed: whether each is open to
/// a squad, and which of its neighbours a squad may cross into from it and
/// from the cells west and south of it.
pub(super) fn settle(cells: &mut Cells, laid: &[usize]) {
    let nx = cells.nx;
    for &at in laid {
        let free = cells[at].free;
        let infantry = free != 0 && one_gap(free);
        if cells[at].infantry != infantry {
            cells[at].infantry = infantry;
        }
    }
    for &at in laid {
        let west = (at % nx > 0).then(|| at - 1);
        let south = (at / nx > 0).then(|| at - nx);
        for at in [Some(at), west, south].into_iter().flatten() {
            let open = cells.crossings(at);
            if cells[at].open != open {
                cells[at].open = open;
            }
        }
    }
}
