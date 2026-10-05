//! Courts: the paved pieces of a dense district's block. Each built parcel
//! is paved as its own yard, the building standing on it; car parks are cut
//! from the ground the parcels leave beside the carriageways; the rest of
//! the district, off its carriageways, stays grass: its lawn, crossed by
//! paved paths from the streets it meets and the yard gates that open on it
//! to its middle, one of those from a street a lane a vehicle drives.
use super::lots::Ground;
use super::Pass;
use crate::layout::geometry::{add, distance, round_cm, scale, sub, Point};
use crate::layout::Lawn;
use crate::{CourtKind, CourtPlan, Diagnostic, DistrictPlan, MapPlan};
use contract::ground::{polygon_contains, GroundShape};
use contract::map::{SurfaceArea, SurfaceKind};
use std::collections::BTreeSet;

/// The courts of every district of `plan` whose presets pave them, with the
/// paving each is laid as. `ground` holds the parcels and buildings the pass
/// cut and stood; the car parks are kept on it as parcels are.
pub fn lay(
    pass: &Pass,
    plan: &MapPlan,
    ground: &mut Ground,
) -> Result<(Vec<CourtPlan>, Vec<SurfaceArea>), Vec<Diagnostic>> {
    let built: BTreeSet<String> = ground.buildings.iter().map(|b| b.id.clone()).collect();
    let mut courts = Vec::new();
    for district in plan.settlements.iter().flat_map(|s| &s.districts) {
        let rule = &pass.district(district)?.props.courts;
        if !rule.paved {
            continue;
        }
        let prefix = format!("{}/", district.id);
        let first = courts.len();
        for lot in &ground.plan_lots {
            if lot.id.starts_with(&prefix) && built.contains(&lot.id) {
                courts.push(CourtPlan {
                    id: format!("{}/yard", lot.id),
                    district: district.id.clone(),
                    kind: CourtKind::Yard,
                    ring: lot.ring.clone(),
                });
            }
        }
        if let Some(parking) = &rule.parking {
            for (n, ring) in ground
                .parking(pass, district, parking)
                .into_iter()
                .enumerate()
            {
                courts.push(CourtPlan {
                    id: format!("{}/parking-{n}", district.id),
                    district: district.id.clone(),
                    kind: CourtKind::Parking,
                    ring: ring.to_vec(),
                });
            }
        }
        if let Some(lawn) = &rule.lawn {
            let paths = paths(pass, ground, district, lawn, &courts[first..]);
            courts.extend(paths);
        }
    }
    let paving = courts
        .iter()
        .filter_map(|court| {
            Some(SurfaceArea {
                kind: SurfaceKind::Paving,
                shape: GroundShape::polygon(court.ring.clone()).ok()?,
            })
        })
        .collect();
    Ok((courts, paving))
}

/// The lawn is judged on a grid of cells this wide.
const CELL_M: f64 = 2.0;

#[derive(Clone, Copy, PartialEq)]
enum Cell {
    /// Off the district, on a yard, a car park or near the water.
    Other,
    /// A carriageway or the walk beside it.
    Street,
    /// Lawn, with the piece it belongs to.
    Lawn(u32),
}

/// A district's ground as cells.
struct Cells {
    origin: Point,
    columns: usize,
    rows: usize,
    cells: Vec<Cell>,
}

impl Cells {
    fn centre(&self, index: usize) -> Point {
        let (column, row) = (index % self.columns, index / self.columns);
        add(
            self.origin,
            [(column as f64 + 0.5) * CELL_M, (row as f64 + 0.5) * CELL_M],
        )
    }

    fn at(&self, p: Point) -> Cell {
        let d = scale(sub(p, self.origin), 1.0 / CELL_M);
        if d[0] < 0.0 || d[1] < 0.0 {
            return Cell::Other;
        }
        let (column, row) = (d[0] as usize, d[1] as usize);
        if column >= self.columns || row >= self.rows {
            return Cell::Other;
        }
        self.cells[row * self.columns + column]
    }

    /// The cells beside `index`, four ways, then the four corners when
    /// `corners` asks.
    fn beside(&self, index: usize, corners: bool) -> impl Iterator<Item = usize> + '_ {
        let (column, row) = (
            (index % self.columns) as isize,
            (index / self.columns) as isize,
        );
        let steps: &[(isize, isize)] = if corners {
            &[
                (1, 0),
                (-1, 0),
                (0, 1),
                (0, -1),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ]
        } else {
            &[(1, 0), (-1, 0), (0, 1), (0, -1)]
        };
        steps.iter().filter_map(move |(dx, dy)| {
            let (c, r) = (column + dx, row + dy);
            (c >= 0 && r >= 0 && (c as usize) < self.columns && (r as usize) < self.rows)
                .then(|| r as usize * self.columns + c as usize)
        })
    }
}

/// The paved courts of a district, each with its bounds: what the lawn is
/// not.
struct Paved<'a>(Vec<([f64; 4], &'a [Point])>);

impl Paved<'_> {
    fn covers(&self, p: Point) -> bool {
        self.0.iter().any(|([x0, y0, x1, y1], ring)| {
            p[0] >= *x0 && p[0] <= *x1 && p[1] >= *y0 && p[1] <= *y1 && polygon_contains(ring, p)
        })
    }
}

/// The district's ground as cells, its lawn (an open parcel's included)
/// cut into pieces that meet along a cell's side: the answer, and how many
/// pieces.
fn cells(pass: &Pass, ground: &Ground, district: &DistrictPlan, paved: &Paved) -> (Cells, u32) {
    let [x0, y0, x1, y1] = contract::ground::limits(&district.ring, 0.0);
    let columns = libm::ceil((x1 - x0) / CELL_M) as usize;
    let rows = libm::ceil((y1 - y0) / CELL_M) as usize;
    let verge = pass.presets.parcels.verge_m;
    let bank = pass.presets.rivers.bank_m();
    let mut found = Cells {
        origin: [x0, y0],
        columns,
        rows,
        cells: vec![Cell::Other; columns * rows],
    };
    // Lawn not yet sorted into pieces.
    const OPEN: Cell = Cell::Lawn(u32::MAX);
    for index in 0..columns * rows {
        let p = found.centre(index);
        found.cells[index] = if !polygon_contains(&district.ring, p) {
            Cell::Other
        } else if ground.network.edge_gap(p, verge + CELL_M, || true) < verge {
            Cell::Street
        } else if paved.covers(p) || ground.network.near_water(p, bank) {
            Cell::Other
        } else {
            OPEN
        };
    }
    let mut pieces = 0;
    for start in 0..columns * rows {
        if found.cells[start] != OPEN {
            continue;
        }
        found.cells[start] = Cell::Lawn(pieces);
        let mut open = vec![start];
        while let Some(index) = open.pop() {
            let next: Vec<usize> = found
                .beside(index, false)
                .filter(|n| found.cells[*n] == OPEN)
                .collect();
            for n in next {
                found.cells[n] = Cell::Lawn(pieces);
                open.push(n);
            }
        }
        pieces += 1;
    }
    (found, pieces)
}

/// Where a path starts: the end it is paved from, the lawn it is checked
/// from, and whether it comes from a street.
struct Entry {
    start: Point,
    from: Point,
    street: bool,
}

/// The paths across `district`'s lawn, `courts` its yards and car parks:
/// on each piece of lawn at least `lawn.least_m2` large, from the middle of
/// each stretch it meets a street along (the longest first, at most
/// `lawn.streets`) and from each yard gate that opens on it (the nearest
/// first, at most `lawn.gates`), straight to the cell farthest from its
/// edges, where the strip stays on that lawn or the street, and where at
/// least two meet there or one is a lane. The first
/// from a street whose strip `lawn.lane_m` wide stays so is a lane.
fn paths(
    pass: &Pass,
    ground: &Ground,
    district: &DistrictPlan,
    lawn: &Lawn,
    courts: &[CourtPlan],
) -> Vec<CourtPlan> {
    let paved = Paved(
        courts
            .iter()
            .map(|c| (contract::ground::limits(&c.ring, 0.0), c.ring.as_slice()))
            .collect(),
    );
    let (cells, pieces) = cells(pass, ground, district, &paved);
    let count = cells.cells.len();
    // How many cells each cell is from ground that is not lawn.
    let mut depth: Vec<u32> = cells
        .cells
        .iter()
        .map(|cell| {
            if matches!(cell, Cell::Lawn(_)) {
                u32::MAX
            } else {
                0
            }
        })
        .collect();
    let mut open: std::collections::VecDeque<usize> =
        (0..count).filter(|index| depth[*index] == 0).collect();
    while let Some(index) = open.pop_front() {
        let next: Vec<usize> = cells
            .beside(index, true)
            .filter(|n| depth[*n] == u32::MAX)
            .collect();
        for n in next {
            depth[n] = depth[index] + 1;
            open.push_back(n);
        }
    }
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); pieces as usize];
    for index in 0..count {
        if let Cell::Lawn(piece) = cells.cells[index] {
            members[piece as usize].push(index);
        }
    }
    let mut found = Vec::new();
    for (piece, own) in members.iter().enumerate() {
        let piece = piece as u32;
        if (own.len() as f64) * CELL_M * CELL_M < lawn.least_m2 {
            continue;
        }
        let hub = own
            .iter()
            .copied()
            .max_by(|a, b| depth[*a].cmp(&depth[*b]).then(b.cmp(a)))
            .expect("a piece has cells");
        let middle = cells.centre(hub);
        // A strip from `from` to the middle, `half` either side, stays on
        // this lawn or the street, and off every court to the metre.
        let fits = |from: Point, half: f64| {
            let length = distance(from, middle);
            let along = scale(sub(middle, from), 1.0 / length);
            let across = [-along[1], along[0]];
            let steps = libm::ceil(length) as usize;
            let shares = [-1.0, -0.5, 0.0, 0.5, 1.0];
            (0..=steps).all(|k| {
                let p = add(from, scale(along, length * k as f64 / steps as f64));
                shares.iter().all(|share| {
                    let q = add(p, scale(across, share * half));
                    let cell = cells.at(q);
                    (cell == Cell::Street || cell == Cell::Lawn(piece)) && !paved.covers(q)
                })
            })
        };
        // Each stretch of the piece's edge along a street: its cells, and
        // the way out to the street.
        let edge: BTreeSet<usize> = own
            .iter()
            .copied()
            .filter(|index| {
                cells
                    .beside(*index, false)
                    .any(|n| cells.cells[n] == Cell::Street)
            })
            .collect();
        let mut stretches: Vec<Vec<usize>> = Vec::new();
        let mut seen = BTreeSet::new();
        for &start in &edge {
            if !seen.insert(start) {
                continue;
            }
            let (mut stretch, mut open) = (Vec::new(), vec![start]);
            while let Some(index) = open.pop() {
                stretch.push(index);
                for n in cells.beside(index, true) {
                    if edge.contains(&n) && seen.insert(n) {
                        open.push(n);
                    }
                }
            }
            stretches.push(stretch);
        }
        stretches.sort_by(|a, b| b.len().cmp(&a.len()).then(a[0].cmp(&b[0])));
        let mut streets = Vec::new();
        for stretch in &stretches {
            let mean = scale(
                stretch
                    .iter()
                    .fold([0.0, 0.0], |sum, index| add(sum, cells.centre(*index))),
                1.0 / stretch.len() as f64,
            );
            let at = *stretch
                .iter()
                .min_by(|a, b| {
                    distance(cells.centre(**a), mean)
                        .total_cmp(&distance(cells.centre(**b), mean))
                        .then(a.cmp(b))
                })
                .expect("a stretch has cells");
            let from = cells.centre(at);
            if distance(from, middle) < 2.0 * CELL_M {
                continue;
            }
            // Back away from the middle, over the walk to the carriageway's
            // edge, which draws over it.
            let back = scale(sub(from, middle), 0.5 / distance(from, middle));
            let mut start = from;
            // At most 8 m, a walk's width and the slant of a path across
            // it: a path that does not meet the street by then runs along
            // it, and is not laid.
            let mut steps = 0;
            while ground.network.edge_gap(start, 1.0, || true) > 0.0 {
                if steps == 16 {
                    break;
                }
                start = add(start, back);
                steps += 1;
            }
            if steps == 16 {
                continue;
            }
            streets.push(Entry {
                start,
                from,
                street: true,
            });
        }
        let mut gates = Vec::new();
        for yard in courts.iter().filter(|c| c.kind == CourtKind::Yard) {
            if yard.ring.len() != 4 {
                continue;
            }
            // The middle of its rear edge, and the way in from its street
            // edge.
            let r = &yard.ring;
            let rear = scale(add(r[2], r[3]), 0.5);
            let inward = scale(sub(r[3], r[0]), 1.0 / distance(r[0], r[3]));
            let from = add(rear, scale(inward, CELL_M));
            // Out of the gate square to its rear, give or take an eighth
            // of a turn, so the path crosses the boundary rather than runs
            // along it.
            let out = scale(sub(middle, from), 1.0 / distance(from, middle));
            if cells.at(from) != Cell::Lawn(piece)
                || distance(from, middle) < 2.0 * CELL_M
                || out[0] * inward[0] + out[1] * inward[1] < libm::sqrt(0.5)
            {
                continue;
            }
            // In through the boundary, which stands inside the parcel.
            gates.push(Entry {
                start: sub(rear, inward),
                from,
                street: false,
            });
        }
        gates.sort_by(|a, b| distance(a.from, middle).total_cmp(&distance(b.from, middle)));
        let lane = streets
            .iter()
            .position(|entry| fits(entry.from, lawn.lane_m / 2.0));
        let half = lawn.path_m / 2.0;
        let mut chosen: Vec<(&Entry, bool)> = Vec::new();
        if let Some(lane) = lane {
            chosen.push((&streets[lane], true));
        }
        let mut street_paths = chosen.len();
        for (n, entry) in streets.iter().enumerate() {
            if street_paths as u32 >= lawn.streets {
                break;
            }
            if Some(n) != lane && fits(entry.from, half) {
                chosen.push((entry, false));
                street_paths += 1;
            }
        }
        let mut gate_paths = 0;
        for entry in &gates {
            if gate_paths >= lawn.gates {
                break;
            }
            if fits(entry.from, half) {
                chosen.push((entry, false));
                gate_paths += 1;
            }
        }
        // A footpath leads somewhere: a middle only one reaches is no
        // meeting of paths, and it would end in the grass. A lane alone is
        // still a vehicle's way in.
        if chosen.len() < 2 && !chosen.iter().any(|(_, lane)| *lane) {
            continue;
        }
        for (entry, lane) in chosen {
            let length = distance(entry.start, middle);
            let along = scale(sub(middle, entry.start), 1.0 / length);
            let left = scale([-along[1], along[0]], half);
            // On past the middle by half its width, so the paths meeting
            // there overlap.
            let end = add(middle, scale(along, half));
            let ring = [
                add(entry.start, left),
                sub(entry.start, left),
                sub(end, left),
                add(end, left),
            ]
            .map(round_cm);
            debug_assert!(entry.street || !lane);
            found.push(CourtPlan {
                id: format!("{}/path-{}", district.id, found.len()),
                district: district.id.clone(),
                kind: if lane {
                    CourtKind::Lane
                } else {
                    CourtKind::Path
                },
                ring: ring.to_vec(),
            });
        }
    }
    found
}
