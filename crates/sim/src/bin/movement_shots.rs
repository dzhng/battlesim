//! Movement shots: replays the scenario table of `tests/movement_scenarios.rs`
//! through the real `Battle` and draws it top-down, for the agent to review
//! pathfinding, cover and pushing (the user looks last). Never interactive.
//!
//! Run: `cargo run -p sim --release --features shots --bin movement_shots [name…]`
//! Writes `throwaway/movement/<scenario>/t###.png`, `<scenario>.gif` (needs
//! ffmpeg on PATH), `contact-sheet.png` and `report.txt`. Deterministic: the
//! same table and seeds give the same bytes.
//!
//! Legend: red dots are own soldiers, black crosses enemy soldiers, grey dots
//! the fallen; dark boxes are vehicle hulls with a pale nose line; grey boxes
//! are props, darker for heavier cover; brown circles are craters; dashed
//! lines are routes; a ring with a tick is a destination and its facing.
//! The canvas technique (a tiny RGB buffer, a fixed camera, PNG flip-books)
//! comes from `~/dev/game`'s weave harness (reuse manifest).

#[path = "../../tests/movement_scenarios.rs"]
mod scenarios;

use contract::ids::Side;
use contract::map::PropKind;
use scenarios::{Outcome, Scenario};
use sim::battle::Battle;
use sim::math::{v2, Obb2, V2};
use sim::world::{Prop, SurfaceKind};
use std::collections::BTreeSet;
use std::fs::{create_dir_all, remove_dir_all, File};
use std::io::{BufWriter, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// A frame every this many ticks (5 per second at 30 Hz); GIFs play at 10
/// frames per second, so twice real time.
const TICKS_PER_FRAME: u64 = 6;
const GIF_FPS: u32 = 10;
/// The canvas fits the map inside this box.
const MAX_W: f64 = 1280.0;
const MAX_H: f64 = 800.0;
/// Caption band above the map.
const BAND: u32 = 34;
/// Ground framed around a scenario's units, goals and props.
const MARGIN_M: f64 = 10.0;
/// A soldier is drawn at his body radius, never smaller than this.
const MIN_SOLDIER_PX: f64 = 2.5;

type Rgb = [u8; 3];
const PAPER: Rgb = [233, 231, 225];
const GRID: Rgb = [222, 220, 213];
const CONTOUR: Rgb = [205, 200, 188];
const ROAD: Rgb = [216, 206, 186];
const WATER: Rgb = [170, 196, 214];
const INK: Rgb = [40, 40, 40];
const MUTED: Rgb = [110, 110, 110];
const SOLDIER: Rgb = [205, 38, 38];
const ROUTE: Rgb = [226, 128, 128];
const FALLEN: Rgb = [175, 175, 170];
const HULL: Rgb = [58, 62, 72];
const NOSE: Rgb = [200, 204, 212];
const CRATER: Rgb = [196, 184, 170];
const CRATER_RIM: Rgb = [160, 146, 130];
/// Props by cover tier: light, medium, heavy.
const TIERS: [Rgb; 3] = [[186, 186, 180], [146, 146, 140], [98, 98, 94]];

// --- a tiny RGB canvas ----------------------------------------------------------

#[derive(Clone)]
struct Canvas {
    w: u32,
    h: u32,
    buf: Vec<u8>,
}

impl Canvas {
    fn new(w: u32, h: u32, c: Rgb) -> Self {
        Canvas {
            w,
            h,
            buf: c
                .iter()
                .copied()
                .cycle()
                .take((w * h * 3) as usize)
                .collect(),
        }
    }

    fn px(&mut self, x: i32, y: i32, c: Rgb) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = ((y as u32 * self.w + x as u32) * 3) as usize;
        self.buf[i..i + 3].copy_from_slice(&c);
    }

    fn get(&self, x: u32, y: u32) -> Rgb {
        let i = ((y * self.w + x) * 3) as usize;
        [self.buf[i], self.buf[i + 1], self.buf[i + 2]]
    }

    fn disc(&mut self, cx: f64, cy: f64, r: f64, c: Rgb) {
        let ri = r.ceil() as i32 + 1;
        let (x0, y0) = (cx.floor() as i32, cy.floor() as i32);
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let (px, py) = ((x0 + dx) as f64 + 0.5 - cx, (y0 + dy) as f64 + 0.5 - cy);
                if px * px + py * py <= r * r {
                    self.px(x0 + dx, y0 + dy, c);
                }
            }
        }
    }

    fn ring(&mut self, cx: f64, cy: f64, r: f64, width: f64, c: Rgb) {
        let ri = (r + width).ceil() as i32 + 1;
        let (x0, y0) = (cx.floor() as i32, cy.floor() as i32);
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let (px, py) = ((x0 + dx) as f64 + 0.5 - cx, (y0 + dy) as f64 + 0.5 - cy);
                if ((px * px + py * py).sqrt() - r).abs() <= width * 0.5 {
                    self.px(x0 + dx, y0 + dy, c);
                }
            }
        }
    }

    /// A line `width` px wide; `dash` = (on, off) px draws it dashed from
    /// `phase` px into the pattern, and returns the phase at its end.
    fn line(
        &mut self,
        a: V2,
        b: V2,
        width: f64,
        c: Rgb,
        dash: Option<(f64, f64)>,
        phase: f64,
    ) -> f64 {
        let len = (b - a).length();
        let n = (len * 2.0).ceil().max(1.0) as usize;
        for k in 0..=n {
            let t = k as f64 / n as f64;
            if let Some((on, off)) = dash {
                if (phase + t * len) % (on + off) >= on {
                    continue;
                }
            }
            let p = a + (b - a) * t;
            self.disc(p.x, p.y, width * 0.5, c);
        }
        phase + len
    }

    /// A filled rectangle in pixel space, rotated by `yaw` (screen frame).
    fn obb(&mut self, r: &Obb2, c: Rgb) {
        let reach = r.half.length().ceil() as i32 + 1;
        let (x0, y0) = (r.center.x.floor() as i32, r.center.y.floor() as i32);
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let p = v2((x0 + dx) as f64 + 0.5, (y0 + dy) as f64 + 0.5);
                if r.contains(p, 0.0) {
                    self.px(x0 + dx, y0 + dy, c);
                }
            }
        }
    }

    fn text(&mut self, x: i32, y: i32, s: &str, scale: i32, c: Rgb) {
        for (i, ch) in s.chars().enumerate() {
            let rows = glyph(ch);
            for (row, bits) in rows.iter().enumerate() {
                for col in 0..3 {
                    if bits & (0b100 >> col) != 0 {
                        for sy in 0..scale {
                            for sx in 0..scale {
                                self.px(
                                    x + (i as i32 * 4 + col) * scale + sx,
                                    y + row as i32 * scale + sy,
                                    c,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    /// Box-filtered downscale to width `w`.
    fn shrink(&self, w: u32) -> Canvas {
        let f = self.w as f64 / w as f64;
        let h = (self.h as f64 / f).round() as u32;
        let mut out = Canvas::new(w, h, PAPER);
        for y in 0..h {
            for x in 0..w {
                let (sx0, sy0) = ((x as f64 * f) as u32, (y as f64 * f) as u32);
                let sx1 = (((x + 1) as f64 * f) as u32).clamp(sx0 + 1, self.w);
                let sy1 = (((y + 1) as f64 * f) as u32).clamp(sy0 + 1, self.h);
                let mut sum = [0u32; 3];
                for yy in sy0..sy1 {
                    for xx in sx0..sx1 {
                        let c = self.get(xx, yy);
                        (0..3).for_each(|k| sum[k] += c[k] as u32);
                    }
                }
                let n = (sx1 - sx0) * (sy1 - sy0);
                out.px(x as i32, y as i32, sum.map(|v| (v / n) as u8));
            }
        }
        out
    }

    fn blit(&mut self, src: &Canvas, x: u32, y: u32) {
        for yy in 0..src.h {
            for xx in 0..src.w {
                self.px((x + xx) as i32, (y + yy) as i32, src.get(xx, yy));
            }
        }
    }

    fn write(&self, path: &Path) {
        let file = File::create(path).unwrap();
        let mut enc = png::Encoder::new(BufWriter::new(file), self.w, self.h);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Best);
        enc.write_header()
            .unwrap()
            .write_image_data(&self.buf)
            .unwrap();
    }
}

/// A 3×5 pixel font: five rows of three bits. Letters print as capitals.
fn glyph(c: char) -> [u8; 5] {
    match c.to_ascii_uppercase() {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b110, 0b101, 0b101, 0b101, 0b101],
        'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'Q' => [0b010, 0b101, 0b101, 0b110, 0b011],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b110, 0b001, 0b010, 0b100, 0b111],
        '3' => [0b110, 0b001, 0b010, 0b001, 0b110],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b110, 0b001, 0b110],
        '6' => [0b011, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b110],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        ',' => [0b000, 0b000, 0b000, 0b010, 0b100],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '=' => [0b000, 0b111, 0b000, 0b111, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '(' => [0b010, 0b100, 0b100, 0b100, 0b010],
        ')' => [0b010, 0b001, 0b001, 0b001, 0b010],
        '\'' => [0b010, 0b010, 0b000, 0b000, 0b000],
        _ => [0; 5],
    }
}

// --- the view ---------------------------------------------------------------------

/// World metres to canvas pixels: north up, under the caption band, with
/// the world point (`min_x`, `max_y`) at the band's lower-left corner.
#[derive(Clone, Copy)]
struct View {
    scale: f64,
    min_x: f64,
    max_y: f64,
}

impl View {
    /// One fixed view per scenario: the bounding box of its units' starts,
    /// its order goals, props and bursts, plus a margin, widened to the
    /// canvas's aspect and kept inside the map. Returns the canvas size too.
    fn frame(s: &Scenario) -> (View, u32, u32) {
        let num = |v: &serde_json::Value| v.as_f64().unwrap();
        let xy = |v: &serde_json::Value| v2(num(&v[0]), num(&v[1]));
        let mut points = Vec::new();
        for u in s.units.as_array().unwrap() {
            points.push(xy(&u["position"]));
        }
        for o in s.scripts.as_array().unwrap() {
            if o["order"]["goal"].is_array() {
                points.push(xy(&o["order"]["goal"]));
            }
        }
        for e in s.events.as_array().unwrap() {
            if e["burst"].is_object() {
                points.push(xy(&e["burst"]["point"]));
            }
        }
        for p in s.map["props"].as_array().into_iter().flatten() {
            let c = xy(&p["center"]);
            let r = num(&p["half_extents"][0]).hypot(num(&p["half_extents"][1]));
            points.extend([c - v2(r, r), c + v2(r, r)]);
        }
        let size = xy(&s.map["size"]);
        let mut lo = v2(f64::INFINITY, f64::INFINITY);
        let mut hi = v2(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in points {
            lo = v2(lo.x.min(p.x - MARGIN_M), lo.y.min(p.y - MARGIN_M));
            hi = v2(hi.x.max(p.x + MARGIN_M), hi.y.max(p.y + MARGIN_M));
        }
        let aspect = MAX_W / MAX_H;
        let (w, h) = (hi.x - lo.x, hi.y - lo.y);
        if w < h * aspect {
            let grow = (h * aspect - w) / 2.0;
            lo.x -= grow;
            hi.x += grow;
        } else {
            let grow = (w / aspect - h) / 2.0;
            lo.y -= grow;
            hi.y += grow;
        }
        // Slide the box back inside the map, or clip it to the map.
        let fit = |lo: f64, hi: f64, max: f64| {
            if hi - lo >= max {
                (0.0, max)
            } else if lo < 0.0 {
                (0.0, hi - lo)
            } else if hi > max {
                (max - (hi - lo), max)
            } else {
                (lo, hi)
            }
        };
        let (x0, x1) = fit(lo.x, hi.x, size.x);
        let (y0, y1) = fit(lo.y, hi.y, size.y);
        let scale = (MAX_W / (x1 - x0)).min(MAX_H / (y1 - y0));
        let view = View {
            scale,
            min_x: x0,
            max_y: y1,
        };
        let w = ((x1 - x0) * scale).round() as u32;
        let h = ((y1 - y0) * scale).round() as u32 + BAND;
        (view, w, h)
    }

    fn px(&self, p: V2) -> V2 {
        v2(
            (p.x - self.min_x) * self.scale,
            BAND as f64 + (self.max_y - p.y) * self.scale,
        )
    }

    fn world(&self, x: u32, y: u32) -> V2 {
        v2(
            self.min_x + (x as f64 + 0.5) / self.scale,
            self.max_y - (y as f64 + 0.5 - BAND as f64) / self.scale,
        )
    }

    /// A world rectangle in pixel space (the y flip turns yaw clockwise).
    fn obb(&self, r: &Obb2) -> Obb2 {
        Obb2 {
            center: self.px(r.center),
            yaw: -r.yaw,
            half: r.half * self.scale,
        }
    }
}

/// Paper, 10 m grid, 1 m height contours, roads and water: drawn once.
fn background(b: &Battle, view: View, w: u32, h: u32) -> Canvas {
    let mut cv = Canvas::new(w, h, PAPER);
    let world = b.world();
    let mut heights = vec![0.0; (w * h) as usize];
    for y in BAND..h {
        for x in 0..w {
            let p = view.world(x, y);
            let Some(s) = world.surface_at(p.x, p.y) else {
                continue;
            };
            heights[(y * w + x) as usize] = s.z;
            let px_m = 1.0 / view.scale;
            let grid = p.x.rem_euclid(10.0) < px_m || p.y.rem_euclid(10.0) < px_m;
            let c = match s.kind {
                SurfaceKind::Road | SurfaceKind::Bridge => ROAD,
                SurfaceKind::Water => WATER,
                SurfaceKind::Ground if grid => GRID,
                SurfaceKind::Ground => PAPER,
            };
            cv.px(x as i32, y as i32, c);
        }
    }
    for y in BAND + 1..h {
        for x in 1..w {
            let z = heights[(y * w + x) as usize];
            let left = heights[(y * w + x - 1) as usize];
            let up = heights[((y - 1) * w + x) as usize];
            if z.floor() != left.floor() || z.floor() != up.floor() {
                cv.px(x as i32, y as i32, CONTOUR);
            }
        }
    }
    cv
}

/// Provisional cover tier by today's prop kinds, for shading only: the body
/// table (slices 33–34) replaces it. 0 light, 1 medium, 2 heavy.
fn tier(p: &Prop) -> usize {
    match p.kind {
        PropKind::Crate => 0,
        PropKind::Trunk => 1,
        // A jeep-sized wreck is light, a tank's heavy (Q24).
        PropKind::Wreck if p.half.x * p.half.y < 3.0 => 0,
        PropKind::Wall if p.half.z < 0.6 => 1,
        _ => 2,
    }
}

/// Craters as circles: each 8-connected patch of cratered ground cells
/// becomes one circle of the patch's area, at its centroid.
fn craters(b: &Battle) -> Vec<(V2, f64)> {
    let cell = b.ground().cell_m();
    let mut left: BTreeSet<(i64, i64)> = b
        .ground()
        .cells()
        .filter(|(_, _, c)| c.crater > 0)
        .map(|(x, y, _)| ((x / cell).round() as i64, (y / cell).round() as i64))
        .collect();
    let mut out = Vec::new();
    while let Some(seed) = left.pop_first() {
        let (mut stack, mut patch) = (vec![seed], Vec::new());
        while let Some((i, j)) = stack.pop() {
            patch.push((i, j));
            for (di, dj) in [
                (-1, -1),
                (-1, 0),
                (-1, 1),
                (0, -1),
                (0, 1),
                (1, -1),
                (1, 0),
                (1, 1),
            ] {
                if left.remove(&(i + di, j + dj)) {
                    stack.push((i + di, j + dj));
                }
            }
        }
        let k = patch.len() as f64;
        let (sx, sy) = patch
            .iter()
            .fold((0.0, 0.0), |(x, y), &(i, j)| (x + i as f64, y + j as f64));
        let center = v2((sx / k + 0.5) * cell, (sy / k + 0.5) * cell);
        out.push((center, (k / std::f64::consts::PI).sqrt() * cell));
    }
    out
}

fn frame(s: &Scenario, tick_hz: u32, b: &Battle, view: View, bg: &Canvas) -> Canvas {
    let mut cv = bg.clone();
    let m = view.scale;
    for (c, r) in craters(b) {
        let p = view.px(c);
        cv.disc(p.x, p.y, r * m, CRATER);
        cv.ring(p.x, p.y, r * m, 1.5, CRATER_RIM);
    }
    // Props at true size, with a 1 px darker outline.
    for p in b.world().props() {
        if p.kind != PropKind::BridgeDeck {
            let r = view.obb(&p.footprint());
            let fill = TIERS[tier(p)];
            cv.obb(&r, fill.map(|c| (c as f64 * 0.6) as u8));
            let inner = Obb2 {
                half: v2((r.half.x - 1.0).max(0.5), (r.half.y - 1.0).max(0.5)),
                ..r
            };
            cv.obb(&inner, fill);
        }
    }
    let units: Vec<_> = scenarios::units(b).collect();
    // Routes and destinations under the bodies.
    for u in units.iter().filter(|u| u.alive() && u.side == Side::Blue) {
        let Some((goal, _)) = u.movement_goal() else {
            continue;
        };
        let mut from = u.position.xy();
        let mut last = from;
        let mut phase = 0.0;
        for &w in u.route.iter().flatten() {
            phase = cv.line(
                view.px(from),
                view.px(w),
                1.5,
                ROUTE,
                Some((6.0, 4.0)),
                phase,
            );
            last = from;
            from = w;
        }
        let g = view.px(goal);
        let r = 1.2 * m.max(4.0 / 1.2);
        cv.ring(g.x, g.y, r, 2.0, SOLDIER);
        let facing = (goal - last).normalized();
        if facing.length() > 0.5 {
            let tip = view.px(goal + facing * (r / m + 1.2));
            let base = view.px(goal + facing * (r / m));
            cv.line(base, tip, 2.0, SOLDIER, None, 0.0);
        }
    }
    let dot = (scenarios::SOLDIER_RADIUS_M * m).max(MIN_SOLDIER_PX);
    for u in &units {
        for s in &u.members {
            if let Some(f) = s.corpse {
                let p = view.px(f.at.xy());
                cv.disc(p.x, p.y, dot * 0.8, FALLEN);
            }
        }
    }
    for u in &units {
        if let Some(hull) = u.hull_box() {
            let r = view.obb(&hull);
            cv.obb(&r, if u.alive() { HULL } else { MUTED });
            let nose = hull.center + v2(hull.half.x * 0.9, 0.0).rotated(hull.yaw);
            cv.line(view.px(hull.center), view.px(nose), 2.0, NOSE, None, 0.0);
            continue;
        }
        for p in u.member_positions() {
            let p = view.px(p.xy());
            if u.side == Side::Blue {
                cv.disc(p.x, p.y, dot, SOLDIER);
            } else {
                let d = dot;
                cv.line(
                    v2(p.x - d, p.y - d),
                    v2(p.x + d, p.y + d),
                    2.0,
                    INK,
                    None,
                    0.0,
                );
                cv.line(
                    v2(p.x - d, p.y + d),
                    v2(p.x + d, p.y - d),
                    2.0,
                    INK,
                    None,
                    0.0,
                );
            }
        }
    }
    scale_bar(&mut cv, m);
    let secs = b.tick() as f64 / tick_hz as f64;
    cv.text(8, 6, &format!("{}   t={secs:.1}s", s.name), 2, INK);
    cv.text(8, 20, s.caption, 2, MUTED);
    cv
}

/// A 10 m bar in the lower-left corner.
fn scale_bar(cv: &mut Canvas, m: f64) {
    let (x0, y) = (12.0, cv.h as f64 - 12.0);
    let x1 = x0 + 10.0 * m;
    cv.line(v2(x0, y), v2(x1, y), 2.0, INK, None, 0.0);
    for x in [x0, x1] {
        cv.line(v2(x, y - 4.0), v2(x, y + 1.0), 2.0, INK, None, 0.0);
    }
    cv.text(x0 as i32, y as i32 - 16, "10 m", 2, INK);
}

/// Width of a contact-sheet cell.
const CELL_W: u32 = 560;

/// Render one scenario; returns its outcomes, its frame count and the
/// contact sheet's three moments (a third, two thirds, the end), shrunk.
fn shoot(s: &Scenario, root: &Path) -> (Vec<Outcome>, usize, Vec<Canvas>) {
    let dir = root.join(s.name);
    if let Err(e) = remove_dir_all(&dir) {
        assert_eq!(e.kind(), ErrorKind::NotFound, "clearing {dir:?}: {e}");
    }
    create_dir_all(&dir).unwrap();
    let (view, w, h) = View::frame(s);
    let mut bg = None;
    let hz = scenarios::tick_hz(s);
    let n = (s.seconds * hz as f64).round() as usize / TICKS_PER_FRAME as usize + 1;
    let moments = [n / 3, 2 * n / 3, n - 1];
    let (mut count, mut picks) = (0, Vec::new());
    let outcomes = scenarios::run(s, |b| {
        if b.tick() % TICKS_PER_FRAME != 0 {
            return;
        }
        let bg = bg.get_or_insert_with(|| background(b, view, w, h));
        let cv = frame(s, hz, b, view, bg);
        cv.write(&dir.join(format!("t{count:03}.png")));
        if moments.contains(&count) {
            picks.push(cv.shrink(CELL_W));
        }
        count += 1;
    });
    gif(&dir, &root.join(format!("{}.gif", s.name)));
    (outcomes, count, picks)
}

fn gif(frames: &Path, out: &Path) {
    let status = Command::new("ffmpeg")
        .args(["-loglevel", "error", "-y", "-framerate", &GIF_FPS.to_string(), "-i"])
        .arg(frames.join("t%03d.png"))
        .args([
            "-vf",
            "split[a][b];[a]palettegen=max_colors=64:stats_mode=full[p];[b][p]paletteuse=dither=none",
            "-bitexact",
        ])
        .arg(out)
        .status();
    match status {
        Ok(s) if s.success() => {}
        other => eprintln!("  no GIF for {out:?} (ffmpeg: {other:?}); the PNGs are there"),
    }
}

/// One row per scenario: three moments (a third, two thirds, the end).
fn contact_sheet(rows: &[(&Scenario, Vec<Canvas>)], path: &Path) {
    const GAP: u32 = 8;
    let cells: Vec<&Vec<Canvas>> = rows.iter().map(|(_, picks)| picks).collect();
    let width = GAP + 3 * (CELL_W + GAP);
    let height = GAP + cells.iter().map(|r| r[0].h + 16 + GAP).sum::<u32>();
    let mut sheet = Canvas::new(width, height, [250, 250, 248]);
    let mut y = GAP;
    for ((s, _), row) in rows.iter().zip(cells) {
        sheet.text(
            GAP as i32,
            y as i32 + 2,
            &format!("{}: {}", s.name, s.caption),
            2,
            INK,
        );
        y += 16;
        for (i, c) in row.iter().enumerate() {
            sheet.blit(c, GAP + i as u32 * (CELL_W + GAP), y);
        }
        y += row[0].h + GAP;
    }
    sheet.write(path);
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../throwaway/movement");
    let only: Vec<String> = std::env::args().skip(1).collect();
    let all = scenarios::scenarios();
    let chosen: Vec<&Scenario> = all
        .iter()
        .filter(|s| only.is_empty() || only.iter().any(|n| n == s.name))
        .collect();
    assert!(!chosen.is_empty(), "no scenario named {only:?}");
    create_dir_all(&root).unwrap();
    let mut report = String::new();
    let mut rows = Vec::new();
    for s in chosen {
        let (outcomes, count, picks) = shoot(s, &root);
        println!("{} → {count} frames", s.name);
        report += &format!("{}: {}\n", s.name, s.caption);
        for o in outcomes {
            let state = match (o.pending, o.passed) {
                (None, true) => "pass".to_string(),
                (None, false) => "FAIL".to_string(),
                (Some(why), passed) => {
                    format!(
                        "pending, {} ({why})",
                        if passed { "passes" } else { "fails" }
                    )
                }
            };
            report += &format!("  {}: {state}; {}\n", o.label, o.detail);
        }
        rows.push((s, picks));
    }
    print!("{report}");
    if only.is_empty() {
        contact_sheet(&rows, &root.join("contact-sheet.png"));
        File::create(root.join("report.txt"))
            .unwrap()
            .write_all(report.as_bytes())
            .unwrap();
    }
    println!("→ {}", root.canonicalize().unwrap().display());
}
