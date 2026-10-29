//! RGB raster drawing, bitmap text, PNG frames and deterministic ffmpeg GIF encoding.
use sim::math::{v2, Obb2, V2};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::process::Command;

pub type Rgb = [u8; 3];

#[derive(Clone)]
pub struct Canvas {
    pub w: u32,
    pub h: u32,
    buf: Vec<u8>,
}

impl Canvas {
    pub fn new(w: u32, h: u32, c: Rgb) -> Self {
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

    pub fn px(&mut self, x: i32, y: i32, c: Rgb) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let i = ((y as u32 * self.w + x as u32) * 3) as usize;
        self.buf[i..i + 3].copy_from_slice(&c);
    }

    pub fn get(&self, x: u32, y: u32) -> Rgb {
        let i = ((y * self.w + x) * 3) as usize;
        [self.buf[i], self.buf[i + 1], self.buf[i + 2]]
    }

    pub fn disc(&mut self, cx: f64, cy: f64, r: f64, c: Rgb) {
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

    pub fn ring(&mut self, cx: f64, cy: f64, r: f64, width: f64, c: Rgb) {
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
    pub fn line(
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
    pub fn obb(&mut self, r: &Obb2, c: Rgb) {
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

    pub fn text(&mut self, x: i32, y: i32, s: &str, scale: i32, c: Rgb) {
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
    pub fn shrink(&self, w: u32) -> Canvas {
        let f = self.w as f64 / w as f64;
        let h = (self.h as f64 / f).round() as u32;
        let mut out = Canvas::new(w, h, [233, 231, 225]);
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

    pub fn blit(&mut self, src: &Canvas, x: u32, y: u32) {
        for yy in 0..src.h {
            for xx in 0..src.w {
                self.px((x + xx) as i32, (y + yy) as i32, src.get(xx, yy));
            }
        }
    }

    pub fn write(&self, path: &Path) {
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

pub fn encode_gif(frames: &Path, out: &Path, fps: u32) -> bool {
    let status = Command::new("ffmpeg")
        .args(["-loglevel", "error", "-y", "-framerate", &fps.to_string(), "-i"])
        .arg(frames.join("t%03d.png"))
        .args([
            "-vf",
            "split[a][b];[a]palettegen=max_colors=64:stats_mode=full[p];[b][p]paletteuse=dither=none",
            "-bitexact",
        ])
        .arg(out)
        .status();
    match status {
        Ok(s) if s.success() => true,
        other => {
            eprintln!("  no GIF for {out:?} (ffmpeg: {other:?}); the PNGs are there");
            false
        }
    }
}
