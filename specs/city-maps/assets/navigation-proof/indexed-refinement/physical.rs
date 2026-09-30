// Included inside the source-derived navigation module; proof only.
#[derive(Clone, Debug, PartialEq)]
pub enum RefinementStatus {
    Pending,
    Complete(Option<f64>),
}
pub struct Refinement {
    a: V2,
    b: V2,
    m: Mobility,
    policy: RoutePolicy,
    samples: usize,
    piece: f64,
    next: usize,
    total: f64,
    point: Option<PointCheck>,
    done: Option<Option<f64>>,
    pub work: usize,
    tile: Option<((u8, usize), Box<[f64; TILE_SAMPLES]>)>,
}
impl Refinement {
    pub fn new(a: V2, b: V2, m: Mobility, policy: RoutePolicy) -> Self {
        let length = (b - a).length();
        let spacing = if m.class == MoverClass::Infantry {
            SUB_M / 2.0
        } else {
            NAV_CELL_M / 4.0
        };
        let samples = ((length / spacing).ceil() as usize).max(1);
        Self {
            a,
            b,
            m,
            policy,
            samples,
            piece: length / samples as f64,
            next: 0,
            total: 0.0,
            point: None,
            done: None,
            work: 0,
            tile: None,
        }
    }
    pub fn poll(&mut self, grid: &NavGrid, budget: &mut usize) -> RefinementStatus {
        if let Some(v) = self.done {
            return RefinementStatus::Complete(v);
        }
        let before = *budget;
        while *budget > 0 {
            if self.next == self.samples {
                self.done = Some(Some(self.total));
                break;
            }
            if self.point.is_none() {
                *budget -= 1;
                let p =
                    self.a + (self.b - self.a) * ((self.next as f64 + 0.5) / self.samples as f64);
                self.point = Some(PointCheck::new(p));
            }
            let point = self.point.as_mut().unwrap();
            match point.poll(grid, &self.m, budget, &mut self.tile) {
                PointStatus::Pending => break,
                PointStatus::Blocked => {
                    self.done = Some(None);
                    break;
                }
                PointStatus::Clear(c) => {
                    if *budget == 0 {
                        break;
                    }
                    *budget -= 1;
                    self.total += NavGrid::cell_cost(&c, &self.m, self.policy, self.piece);
                    self.next += 1;
                    self.point = None;
                }
            }
        }
        self.work += before - *budget;
        self.done
            .map_or(RefinementStatus::Pending, RefinementStatus::Complete)
    }
}
enum PointStatus {
    Pending,
    Blocked,
    Clear(Cell),
}
struct PointCheck {
    p: V2,
    cell: Option<(usize, Cell)>,
    clearance: Option<ClearanceTask>,
    ready: bool,
    avoid: usize,
}
impl PointCheck {
    fn new(p: V2) -> Self {
        Self {
            p,
            cell: None,
            clearance: None,
            ready: false,
            avoid: 0,
        }
    }
    fn poll(
        &mut self,
        grid: &NavGrid,
        m: &Mobility,
        budget: &mut usize,
        tile: &mut Option<((u8, usize), Box<[f64; TILE_SAMPLES]>)>,
    ) -> PointStatus {
        if self.cell.is_none() {
            if *budget == 0 {
                return PointStatus::Pending;
            }
            *budget -= 1;
            let (i, j) = cell_of(self.p);
            let Some(at) = grid.index(i, j) else {
                return PointStatus::Blocked;
            };
            let c = grid.cells[at];
            let enters = match m.class {
                MoverClass::Infantry => c.infantry && c.free & (1 << sub_of(self.p)) != 0,
                MoverClass::Vehicle => NavGrid::vehicle_enters(&c, m.push),
            };
            if !enters {
                return PointStatus::Blocked;
            }
            self.cell = Some((at, c));
        }
        let (at, c) = self.cell.unwrap();
        if !self.ready {
            if m.class == MoverClass::Infantry {
                self.ready = true;
            } else {
                if self.clearance.is_none() {
                    if *budget == 0 {
                        return PointStatus::Pending;
                    }
                    *budget -= 1;
                    let fresh = ClearanceTask::new(grid, at, m.push);
                    if let Some((key, page)) = tile.as_ref() {
                        if *key == (fresh.stop, fresh.tile) {
                            if !(page[fresh.local] - NAV_CELL_M / 2.0 >= m.half_width_m) {
                                return PointStatus::Blocked;
                            }
                            self.ready = true;
                        }
                    }
                    if !self.ready {
                        self.clearance = Some(fresh);
                    }
                }
                if !self.ready {
                    let Some(d) = self.clearance.as_mut().unwrap().poll(grid, budget) else {
                        return PointStatus::Pending;
                    };
                    let task = self.clearance.as_mut().unwrap();
                    let page = task.packed.take().unwrap();
                    *tile = Some(((task.stop, task.tile), page));
                    if !(d - NAV_CELL_M / 2.0 >= m.half_width_m) {
                        return PointStatus::Blocked;
                    }
                    self.ready = true;
                }
            }
        }
        while self.avoid < grid.avoid.len() {
            if *budget == 0 {
                return PointStatus::Pending;
            }
            *budget -= 1;
            let b = &grid.avoid[self.avoid];
            self.avoid += 1;
            if b.contains(
                cell_center(at % grid.nx, at / grid.nx),
                m.half_width_m + NAV_CELL_M / 2.0,
            ) {
                return PointStatus::Blocked;
            }
        }
        PointStatus::Clear(c)
    }
}
struct ClearanceTask {
    stop: u8,
    tile: usize,
    local: usize,
    tx: usize,
    ty: usize,
    x0: usize,
    y0: usize,
    nx: usize,
    ny: usize,
    phase: u8,
    at: usize,
    out: Vec<f64>,
    values: Vec<f64>,
    done: Option<f64>,
    packed: Option<Box<[f64; TILE_SAMPLES]>>,
}
impl ClearanceTask {
    fn new(grid: &NavGrid, at: usize, push: PushClass) -> Self {
        let (i, j) = (at % grid.nx, at / grid.nx);
        let (tx, ty) = (i / TILE_SIDE * TILE_SIDE, j / TILE_SIDE * TILE_SIDE);
        let (x0, y0) = (
            tx.saturating_sub(CLEARANCE_HALO),
            ty.saturating_sub(CLEARANCE_HALO),
        );
        let (x1, y1) = (
            (tx + TILE_SIDE - 1 + CLEARANCE_HALO).min(grid.nx - 1),
            (ty + TILE_SIDE - 1 + CLEARANCE_HALO).min(grid.ny - 1),
        );
        Self {
            stop: grid.stopping(push),
            tile: (j / TILE_SIDE) * grid.nx.div_ceil(TILE_SIDE) + i / TILE_SIDE,
            local: (j % TILE_SIDE) * TILE_SIDE + i % TILE_SIDE,
            tx,
            ty,
            x0,
            y0,
            nx: x1 - x0 + 1,
            ny: y1 - y0 + 1,
            phase: 0,
            at: 0,
            out: Vec::new(),
            values: Vec::new(),
            done: None,
            packed: None,
        }
    }
    fn relax(&mut self, at: usize, from: usize, w: f64) {
        let d = self.out[from] + w;
        if d < self.out[at] {
            self.out[at] = d;
        }
    }
    fn poll(&mut self, grid: &NavGrid, budget: &mut usize) -> Option<f64> {
        if self.done.is_some() {
            return self.done;
        }
        while *budget > 0 {
            *budget -= 1;
            match self.phase {
                0 => {
                    self.out = Vec::with_capacity(self.nx * self.ny);
                    self.phase = 1;
                }
                1 => {
                    let (x, y) = (self.at % self.nx, self.at / self.nx);
                    let (ii, jj) = (x + self.x0, y + self.y0);
                    let edge = ii.min(grid.nx - 1 - ii).min(jj).min(grid.ny - 1 - jj) as f64
                        * NAV_CELL_M
                        + NAV_CELL_M;
                    let c = grid.cells[jj * grid.nx + ii];
                    self.out.push(if c.ground && c.heaviest < self.stop {
                        MAX_CLEARANCE_M.min(edge)
                    } else {
                        0.0
                    });
                    self.at += 1;
                    if self.at == self.nx * self.ny {
                        self.at = 0;
                        self.phase = 2;
                    }
                }
                2 => {
                    let k = self.at;
                    let (x, y) = (k % self.nx, k / self.nx);
                    let (a, b) = (NAV_CELL_M, NAV_CELL_M * std::f64::consts::SQRT_2);
                    if x > 0 {
                        self.relax(k, k - 1, a);
                    }
                    if y > 0 {
                        self.relax(k, k - self.nx, a);
                        if x > 0 {
                            self.relax(k, k - self.nx - 1, b);
                        }
                        if x + 1 < self.nx {
                            self.relax(k, k - self.nx + 1, b);
                        }
                    }
                    self.at += 1;
                    if self.at == self.nx * self.ny {
                        self.at = self.nx * self.ny;
                        self.phase = 3;
                    }
                }
                3 => {
                    self.at -= 1;
                    let k = self.at;
                    let (x, y) = (k % self.nx, k / self.nx);
                    let (a, b) = (NAV_CELL_M, NAV_CELL_M * std::f64::consts::SQRT_2);
                    if x + 1 < self.nx {
                        self.relax(k, k + 1, a);
                    }
                    if y + 1 < self.ny {
                        self.relax(k, k + self.nx, a);
                        if x > 0 {
                            self.relax(k, k + self.nx - 1, b);
                        }
                        if x + 1 < self.nx {
                            self.relax(k, k + self.nx + 1, b);
                        }
                    }
                    if self.at == 0 {
                        self.phase = 4;
                        self.values = Vec::with_capacity(TILE_SAMPLES);
                    }
                }
                4 => {
                    let (x, y) = (self.at % TILE_SIDE + self.tx, self.at / TILE_SIDE + self.ty);
                    self.values.push(if x < grid.nx && y < grid.ny {
                        self.out[(y - self.y0) * self.nx + x - self.x0]
                    } else {
                        0.0
                    });
                    self.at += 1;
                    if self.at == TILE_SAMPLES {
                        self.phase = 5;
                    }
                }
                5 => {
                    let values: Box<[f64; TILE_SAMPLES]> = std::mem::take(&mut self.values)
                        .into_boxed_slice()
                        .try_into()
                        .ok()
                        .unwrap();
                    let d = values[self.local];
                    self.packed = Some(values);
                    self.done = Some(d);
                    return self.done;
                }
                _ => unreachable!(),
            }
        }
        None
    }
}
