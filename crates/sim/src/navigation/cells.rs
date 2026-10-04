//! The grid's cells. Open flat ground is implicit; every other cell lives in
//! a page, and pages are shared: a side's grid starts as the map's
//! ([`super::NavBase`]) and copies only the pages it comes to know
//! differently.
use std::ops::{Index, IndexMut};
use std::sync::Arc;

use super::NAV_CELL_M;
use crate::math::{v2, V2};

/// No known body that stops vehicles covers a cell.
pub(super) const NO_BODY: u8 = 0;
/// Sub-cells per cell side for infantry: 0.5 m.
pub(super) const SUB: usize = 4;
pub(super) const SUB_M: f64 = NAV_CELL_M / SUB as f64;
/// Every sub-cell of a cell free.
pub(super) const ALL_FREE: u16 = u16::MAX;
/// Sub-cell masks: a 4×4 cell's west and east columns, south and north rows.
const WEST_COLUMN: u16 = 0x1111;
const EAST_COLUMN: u16 = 0x8888;
const SOUTH_ROW: u16 = 0x000F;
const NORTH_ROW: u16 = 0xF000;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Cell {
    /// The ground under the centre is walkable.
    pub ground: bool,
    /// Infantry: the ground is walkable and the free sub-cells are one gap.
    pub infantry: bool,
    /// The heaviest known body stopping vehicles over the cell: its weight
    /// class's rank, or [`NO_BODY`].
    pub heaviest: u8,
    /// `Surface::road_factor`: 0 off any road.
    pub road_factor: f64,
    pub forest: bool,
    pub slope_deg: f64,
    /// Infantry's free sub-cells, bit `row * 4 + column` from the cell's
    /// south-west corner.
    pub free: u16,
    /// Infantry may cross into the east (bit 0) and north (bit 1) neighbour.
    pub open: u8,
}

const PAGE_SIDE: usize = 16;
type Page = [Cell; PAGE_SIDE * PAGE_SIDE];

#[derive(Clone)]
pub(super) struct Cells {
    /// Each page of cells that is not all open flat ground, row by row.
    pages: Vec<Option<Arc<Page>>>,
    pages_x: usize,
    /// Open flat ground, by whether the cell has an east and a north
    /// neighbour.
    pub implicit: [Cell; 4],
    pub nx: usize,
    pub ny: usize,
}

impl Cells {
    pub fn new(traversable: bool, nx: usize, ny: usize) -> Self {
        let pages_x = nx.div_ceil(PAGE_SIDE);
        Self {
            pages: vec![None; pages_x * ny.div_ceil(PAGE_SIDE)],
            pages_x,
            nx,
            ny,
            implicit: std::array::from_fn(|open| Cell {
                ground: traversable,
                infantry: traversable,
                heaviest: NO_BODY,
                road_factor: 0.0,
                forest: false,
                slope_deg: 0.0,
                free: if traversable { ALL_FREE } else { 0 },
                open: if traversable { open as u8 } else { 0 },
            }),
        }
    }

    /// Open flat ground at `at`.
    pub fn default_at(&self, at: usize) -> Cell {
        let (i, j) = (at % self.nx, at / self.nx);
        self.implicit[usize::from(i + 1 < self.nx) + 2 * usize::from(j + 1 < self.ny)]
    }

    /// The page `at` lies in, and its place there.
    fn place(&self, at: usize) -> (usize, usize) {
        let (i, j) = (at % self.nx, at / self.nx);
        (
            (j / PAGE_SIDE) * self.pages_x + i / PAGE_SIDE,
            (j % PAGE_SIDE) * PAGE_SIDE + i % PAGE_SIDE,
        )
    }

    /// Write `cell` at `at`, unless it is what stands there already (a
    /// write copies a shared page).
    pub fn set(&mut self, at: usize, cell: Cell) {
        if self[at] != cell {
            self[at] = cell;
        }
    }

    /// Every cell of a stored page that lies on the map, in order.
    pub fn stored(&self) -> impl Iterator<Item = (usize, &Cell)> {
        self.pages.iter().enumerate().flat_map(move |(p, page)| {
            let (i0, j0) = (p % self.pages_x * PAGE_SIDE, p / self.pages_x * PAGE_SIDE);
            page.iter().flat_map(move |page| {
                page.iter().enumerate().filter_map(move |(k, cell)| {
                    let (i, j) = (i0 + k % PAGE_SIDE, j0 + k / PAGE_SIDE);
                    (i < self.nx && j < self.ny).then_some((j * self.nx + i, cell))
                })
            })
        })
    }

    /// Pages stored, and how many of them no other grid shares.
    pub fn pages(&self) -> (usize, usize) {
        let stored = self.pages.iter().flatten();
        (
            stored.clone().count(),
            stored.filter(|page| Arc::strong_count(page) == 1).count(),
        )
    }

    /// Whether infantry may cross from `at` into its east (bit 0) and north
    /// (bit 1) neighbour: both open to a squad, and free sub-cells meeting
    /// across the shared edge.
    pub fn crossings(&self, at: usize) -> u8 {
        let (i, j) = (at % self.nx, at / self.nx);
        let cell = &self[at];
        let mut open = 0;
        if cell.infantry {
            if i + 1 < self.nx && self[at + 1].infantry {
                let east = (cell.free & EAST_COLUMN) >> (SUB - 1);
                open |= u8::from(east & self[at + 1].free & WEST_COLUMN != 0);
            }
            if j + 1 < self.ny && self[at + self.nx].infantry {
                let north = (cell.free & NORTH_ROW) >> (SUB * (SUB - 1));
                open |= u8::from(north & self[at + self.nx].free & SOUTH_ROW != 0) << 1;
            }
        }
        open
    }
}

impl Index<usize> for Cells {
    type Output = Cell;
    fn index(&self, at: usize) -> &Cell {
        let (i, j) = (at % self.nx, at / self.nx);
        match &self.pages[(j / PAGE_SIDE) * self.pages_x + i / PAGE_SIDE] {
            Some(page) => &page[(j % PAGE_SIDE) * PAGE_SIDE + i % PAGE_SIDE],
            None => &self.implicit[usize::from(i + 1 < self.nx) + 2 * usize::from(j + 1 < self.ny)],
        }
    }
}

impl IndexMut<usize> for Cells {
    fn index_mut(&mut self, at: usize) -> &mut Cell {
        let (page, local) = self.place(at);
        if self.pages[page].is_none() {
            let (i0, j0) = (
                page % self.pages_x * PAGE_SIDE,
                page / self.pages_x * PAGE_SIDE,
            );
            let (nx, ny, implicit) = (self.nx, self.ny, self.implicit);
            self.pages[page] = Some(Arc::new(std::array::from_fn(|k| {
                let (i, j) = (i0 + k % PAGE_SIDE, j0 + k / PAGE_SIDE);
                implicit[usize::from(i + 1 < nx) + 2 * usize::from(j + 1 < ny)]
            })));
        }
        &mut Arc::make_mut(self.pages[page].as_mut().expect("the page is stored"))[local]
    }
}

pub(super) fn cell_center(i: usize, j: usize) -> V2 {
    v2((i as f64 + 0.5) * NAV_CELL_M, (j as f64 + 0.5) * NAV_CELL_M)
}

pub(super) fn cell_of(p: V2) -> (isize, isize) {
    (
        (p.x / NAV_CELL_M).floor() as isize,
        (p.y / NAV_CELL_M).floor() as isize,
    )
}

/// Centre of sub-cell `bit` of cell (i, j).
pub(super) fn sub_center(i: usize, j: usize, bit: usize) -> V2 {
    let (c, r) = (bit % SUB, bit / SUB);
    v2(
        i as f64 * NAV_CELL_M + (c as f64 + 0.5) * SUB_M,
        j as f64 * NAV_CELL_M + (r as f64 + 0.5) * SUB_M,
    )
}

/// The sub-cell of its cell that `p` lies in.
pub(super) fn sub_of(p: V2) -> usize {
    let c = (p.x.rem_euclid(NAV_CELL_M) / SUB_M) as usize;
    let r = (p.y.rem_euclid(NAV_CELL_M) / SUB_M) as usize;
    r.min(SUB - 1) * SUB + c.min(SUB - 1)
}

/// Whether a cell's free sub-cells are one gap, connected edge to edge.
pub(super) fn one_gap(free: u16) -> bool {
    let mut gap = free.isolate_lowest_one();
    loop {
        let grown = (gap
            | ((gap << 1) & !WEST_COLUMN)
            | ((gap >> 1) & !EAST_COLUMN)
            | (gap << SUB)
            | (gap >> SUB))
            & free;
        if grown == gap {
            return gap == free;
        }
        gap = grown;
    }
}
