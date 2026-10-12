//! Replays a browser parity run's scene natively and writes its plates, or
//! times it.
//!
//! `native-client --run <parity run> --scene <map> (--out <dir> | --bench <dir>)
//!   [--window offscreen|minimised|hidden|visible] [--pace <frames>]
//!   [--sound <wav>]`
//!
//! Every frame record is framed through the presentation crate; each one
//! the browser wrote a plate of is drawn, presented and written to `--out`
//! under the control plate's name. Then the last frame is drawn and presented
//! `--pace` more times, and the report says how presenting went. The window
//! opens off-screen unless told otherwise. With `--sound`, the battle's
//! sound is heard through every record at the reel's own times and rendered
//! to that file (never played aloud).
//!
//! With `--bench`, no plate is written: every frame record is framed, drawn
//! and presented back to back, timed (`bench`), and the report is written to
//! that directory as `native-<scene>-<time>.json` for `web/fps-compare.mjs`.
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use native_client::audio::{self, Audio, OutputSettings};
use native_client::bench::{self, Bench};
use native_client::bundle::{Library, RUNTIME};
use native_client::capture::{Record, Replay};
use native_client::effects::{EffectPass, Effects};
use native_client::environment::Environment;
use native_client::fog::{Fog, FogSight};
use native_client::fog_mask::fog_mask;
use native_client::frame::{Passes, Targets, DISPLAY_FORMAT};
use native_client::gpu::{self, Gpu};
use native_client::grass::Grass;
use native_client::ground::Ground;
use native_client::models::Models;
use native_client::pipelines::Pipelines;
use native_client::post::Post;
use native_client::scenery::{Drawn, Scenery};
use native_client::shadow::{ShadowPass, SunShadow};
use native_client::sky::Sky;
use native_client::world::{DepthPrepass, Ghosts, WorldColour};
use presentation::records::View;
use presentation::shadow::SunShadowFrame;
use wgpu::{
    CommandBuffer, CommandEncoder, CurrentSurfaceTexture, PresentMode, SubmissionIndex, Surface,
    SurfaceConfiguration,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

struct Args {
    scene: PathBuf,
    /// Where plates are written; absent for a bench.
    out: Option<PathBuf>,
    /// Where a bench's report is written.
    bench: Option<PathBuf>,
    window: String,
    pace: usize,
    sound: Option<PathBuf>,
}

fn args() -> Result<Args, String> {
    let mut run = None;
    let (mut scene, mut out, mut window, mut pace) = (None, None, "offscreen".to_owned(), 120);
    let (mut sound, mut bench) = (None, None);
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--run" => run = Some(PathBuf::from(value()?)),
            "--scene" => scene = Some(value()?),
            "--out" => out = Some(PathBuf::from(value()?)),
            "--window" => window = value()?,
            "--pace" => pace = value()?.parse().map_err(|e| format!("--pace: {e}"))?,
            "--sound" => sound = Some(PathBuf::from(value()?)),
            "--bench" => bench = Some(PathBuf::from(value()?)),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if !["offscreen", "minimised", "hidden", "visible"].contains(&window.as_str()) {
        return Err(format!("--window {window}"));
    }
    if out.is_some() == bench.is_some() {
        return Err("one of --out and --bench is required".into());
    }
    if bench.is_some() && sound.is_some() {
        return Err("a bench renders no sound".into());
    }
    Ok(Args {
        scene: run
            .ok_or("--run is required")?
            .join(scene.ok_or("--scene is required")?),
        out,
        bench,
        window,
        pace,
        sound,
    })
}

#[derive(Default)]
struct Presents {
    shown: usize,
    occluded: usize,
    other: Vec<String>,
    /// Wall time from one presented frame's start to the next's.
    intervals: Vec<Duration>,
}

struct Run {
    window: Arc<Window>,
    surface: Surface<'static>,
    gpu: Gpu,
    passes: Passes,
    replay: Replay,
    next: usize,
    view: View,
    plates: usize,
    count_mismatches: Vec<String>,
    paced: usize,
    presents: Presents,
    last: Option<Instant>,
    /// The battle's sound, rendered to a buffer.
    sound: Option<Audio>,
    /// A bench's timing, and what its frames drew: the sum over its frames
    /// of the models' soldiers, vehicles, corpses, pieces and triangles, the
    /// scenery in view, the effects, the instance bytes uploaded, and the
    /// props.
    bench: Option<Bench>,
    drew: [f64; 9],
}

impl Run {
    fn start(event_loop: &ActiveEventLoop, args: &Args) -> Result<Self, String> {
        let game = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/game.json");
        let library = Library::open(std::path::Path::new(RUNTIME))?;
        let opened = Instant::now();
        let mut replay = Replay::open(&args.scene, &game, &library)?;
        println!(
            "world: ground set, scenery placed and the battle loaded in {:.2} s",
            opened.elapsed().as_secs_f64()
        );
        let size = replay.records.first().ok_or("no frame records")?.canvas;
        let mut attributes = Window::default_attributes()
            .with_title("battlegame native")
            .with_inner_size(PhysicalSize::new(size[0], size[1]))
            .with_resizable(false)
            .with_visible(args.window != "hidden");
        if args.window == "offscreen" {
            // AppKit pulls a titled window back onto the screen; a borderless
            // one stays where it is put.
            attributes = attributes
                .with_decorations(false)
                .with_position(PhysicalPosition::new(10000, 10000));
        }
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(|e| e.to_string())?,
        );
        if args.window == "minimised" {
            window.set_minimized(true);
        }
        println!("window at {:?}", window.outer_position());
        let instance = gpu::instance(Box::new(window.clone()));
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let gpu = gpu::open(&instance, Some(&surface))?;
        let caps = surface.get_capabilities(&gpu.adapter);
        if !caps.formats.contains(&DISPLAY_FORMAT)
            || !caps.usages.contains(wgpu::TextureUsages::COPY_DST)
        {
            return Err(format!(
                "the surface can't take {DISPLAY_FORMAT:?} copies: {caps:?}"
            ));
        }
        // The window's backing may not be the canvas size; the copy needs it.
        surface.configure(
            &gpu.device,
            &SurfaceConfiguration {
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: DISPLAY_FORMAT,
                color_space: Default::default(),
                width: size[0],
                height: size[1],
                present_mode: PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![],
            },
        );

        let built = Instant::now();
        let pipelines = Pipelines::build(&gpu.device, pipelines::DIR)?;
        println!(
            "pipelines: {} built in {:.2} s on {}",
            pipelines.names().count(),
            built.elapsed().as_secs_f64(),
            gpu.adapter.get_info().name
        );
        let targets = Targets::new(&gpu.device, size);
        let light = &replay.light;
        let sky = Sky::new(&gpu.device, &gpu.queue, &pipelines, &light.sky());
        let post = Post::new(
            &gpu.device,
            &pipelines,
            &targets,
            &light.bloom(),
            &light.grade(),
        );
        let shadow = SunShadow::new(
            &gpu.device,
            &pipelines,
            light.cascades.map_size,
            &SunShadowFrame::cold(light),
        );
        let environment = Environment::new(
            &gpu.device,
            &gpu.queue,
            &pipelines,
            &sky.lut,
            &light.sun(),
            &light.aerial(),
            &shadow,
        );
        let installed = Instant::now();
        let presentation = replay.presentation();
        let resolved = presentation.ground().ok_or("the replay set no ground")?;
        let (positions, indices) = &replay.terrain;
        let ground = Ground::new(
            &gpu.device,
            &gpu.queue,
            &pipelines,
            resolved,
            (positions, indices),
            light.backdrop.reach_m,
        )?;
        let grass = Grass::new(&gpu.device, &pipelines, &library, resolved, &ground.heights)?;
        let models = Models::new(&gpu.device, &gpu.queue, &pipelines, &library, presentation)?;
        let scenery = Scenery::new(&gpu.device, &pipelines, &library, presentation)?;
        println!(
            "{}: ground, grass, {} drawables and the scenery installed in {:.2} s",
            replay.map,
            presentation.world().appearances.len(),
            installed.elapsed().as_secs_f64()
        );
        // A fresh renderer holds no scars: the first upload writes them all.
        replay.presentation_mut().resend_scars();
        let effects = Effects::new(&gpu.device, &gpu.queue, &pipelines, &environment, &targets)?;
        // Fog of war over the ground's height pages; a fresh renderer holds none.
        let fog = Fog::new(&gpu.device, &pipelines, &targets, &ground.heights);
        replay.presentation_mut().resend_fog();
        let (fog_mask, fog_rim) = fog_mask(&gpu.device, &pipelines, &targets, &replay.fog_look);
        if let Some(dir) = args.out.as_ref().or(args.bench.as_ref()) {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let bench = args
            .bench
            .is_some()
            .then(|| Bench::new(&gpu.device, &gpu.queue));
        let sound = match &args.sound {
            None => None,
            Some(_) => {
                let read = |p: &str| -> Result<serde_json::Value, String> {
                    let text = std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"))?;
                    serde_json::from_str(&text).map_err(|e| format!("{p}: {e}"))
                };
                let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/");
                let setup = audio::setup(
                    &read(&format!("{root}game.json"))?,
                    &read(&format!("{root}sounds.json"))?,
                );
                let output = OutputSettings::Buffer(audio::SAMPLE_RATE);
                Some(Audio::new(&setup, std::path::Path::new(RUNTIME), output)?)
            }
        };
        Ok(Self {
            window,
            surface,
            gpu,
            passes: Passes {
                targets,
                light: replay.light.clone(),
                environment,
                shadow,
                ground,
                grass,
                models,
                scenery,
                effects,
                fog,
                passes: vec![
                    Box::new(ShadowPass::new(&pipelines)),
                    Box::new(DepthPrepass::new(&pipelines)),
                    Box::new(FogSight),
                    Box::new(WorldColour::new(&pipelines, sky)),
                    Box::new(EffectPass),
                    Box::new(fog_mask),
                    Box::new(Ghosts::new(&pipelines)),
                    Box::new(post),
                    Box::new(fog_rim),
                ],
            },
            replay,
            next: 0,
            view: View::default(),
            plates: 0,
            count_mismatches: Vec::new(),
            paced: 0,
            presents: Presents::default(),
            last: None,
            sound,
            bench,
            drew: [0.0; 9],
        })
    }

    /// Submits `encoder` after `first`, copying the display target to the
    /// window and presenting it if the window can show it.
    fn present(
        &mut self,
        first: Option<CommandBuffer>,
        mut encoder: CommandEncoder,
    ) -> SubmissionIndex {
        let now = Instant::now();
        if let Some(last) = self.last.replace(now) {
            self.presents.intervals.push(now - last);
        }
        let texture = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(t) | CurrentSurfaceTexture::Suboptimal(t) => t,
            CurrentSurfaceTexture::Occluded => {
                self.presents.occluded += 1;
                return self
                    .gpu
                    .queue
                    .submit(first.into_iter().chain([encoder.finish()]));
            }
            other => {
                let status = format!("{other:?}");
                self.presents.other.push(status);
                return self
                    .gpu
                    .queue
                    .submit(first.into_iter().chain([encoder.finish()]));
            }
        };
        let targets = &self.passes.targets;
        encoder.copy_texture_to_texture(
            targets.display_texture.as_image_copy(),
            texture.texture.as_image_copy(),
            targets.display_texture.size(),
        );
        let index = self
            .gpu
            .queue
            .submit(first.into_iter().chain([encoder.finish()]));
        self.window.pre_present_notify();
        self.gpu.queue.present(texture);
        self.presents.shown += 1;
        index
    }

    /// Prepares the layers to draw `record`'s frame, the last framed.
    fn prepare(&mut self, record: &Record) -> Result<(), String> {
        // The scars changed under the grass: it regrows over them.
        if self.replay.presentation_mut().scars_changed() {
            self.passes.grass.regrow();
        }
        let camera = self.replay.camera(record);
        let frame = self.replay.last_frame();
        self.view = frame.view;
        let wrap = frame.light_wrap;
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        self.passes
            .prepare(device, queue, self.replay.presentation(), &camera)?;
        self.passes
            .prepare_scars(device, self.replay.presentation_mut());
        self.passes
            .prepare_fog(device, queue, self.replay.presentation_mut())?;
        let presentation = self.replay.presentation();
        self.passes
            .prepare_effects(device, queue, presentation, wrap);
        Ok(())
    }

    /// Frames, draws and presents the next record, timed. False when done.
    fn bench_step(&mut self) -> Result<bool, String> {
        let Some(record) = self.replay.records.get(self.next).cloned() else {
            return Ok(false);
        };
        self.next += 1;
        let bench = self.bench.as_mut().ok_or("not a bench")?;
        bench.start();
        // Every frame is drawn: the static batches it names unchanged are
        // held from the frame before.
        self.replay.frame(&record, true, |_| {})?;
        self.bench.as_mut().ok_or("not a bench")?.mark();
        self.prepare(&record)?;
        let d = self.passes.models.drawn();
        let scenery = self.passes.scenery.drawn();
        let trees: u32 = scenery.populations.iter().flatten().sum();
        let bytes = self.replay.last_frame().instances.len() * 4;
        let frame = [
            f64::from(d.soldiers),
            f64::from(d.vehicles),
            f64::from(d.corpses),
            f64::from(d.pieces),
            d.triangles as f64,
            f64::from(trees),
            f64::from(self.passes.effects.count()),
            bytes as f64,
            f64::from(d.props),
        ];
        for (sum, v) in self.drew.iter_mut().zip(frame) {
            *sum += v;
        }
        let bench = self.bench.as_mut().ok_or("not a bench")?;
        bench.mark();
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        let first = bench.begin(device);
        let mut encoder = self.passes.encode(device, queue, &self.view);
        bench.end(&mut encoder);
        bench.mark();
        let index = self.present(first, encoder);
        let bench = self.bench.as_mut().ok_or("not a bench")?;
        bench.submitted(&self.gpu.device, index)?;
        Ok(true)
    }

    /// Frames records up to the next plate and draws it; then paces. False when done.
    fn step(&mut self, args: &Args) -> Result<bool, String> {
        if self.bench.is_some() {
            return self.bench_step();
        }
        let out = args.out.as_ref().ok_or("--out is required")?;
        while let Some(record) = self.replay.records.get(self.next).cloned() {
            self.next += 1;
            // The reel's wall time is both the sound's wall and its mixer clock.
            let now = record.reel_ms / 1000.0;
            let mut sound = self.sound.as_mut();
            let drawn = record.plate.is_some();
            self.replay.frame(&record, drawn, |p| {
                if let Some(audio) = sound.as_mut() {
                    audio.note(p, now);
                }
            })?;
            if let Some(audio) = self.sound.as_mut() {
                audio.hear(self.replay.presentation(), now, now);
            }
            let Some(plate) = record.plate.clone() else {
                continue;
            };
            self.prepare(&record)?;
            let counts = self.replay.last_frame().counts;
            let (device, queue) = (&self.gpu.device, &self.gpu.queue);
            let presentation = self.replay.presentation();
            self.count_mismatches.extend(effect_mismatches(
                &plate,
                &self.replay.effect_counts,
                &counts,
                presentation.effects(),
                self.passes.effects.count(),
            ));
            let drawn = self.passes.models.drawn();
            let want = self
                .replay
                .model_counts
                .iter()
                .find(|c| c["plate"] == plate.as_str());
            if let Some(want) = want {
                let n = |k: &str| want[k].as_u64().unwrap_or(u64::MAX) as u32;
                // What the frame poses, and what the models draw of it: a
                // soldier off screen is counted, not drawn.
                let d = drawn;
                let checks = [
                    ("soldiers", counts.soldiers, n("soldiers")),
                    ("vehicles", counts.vehicles, n("vehicles")),
                    ("aircraft", counts.aircraft, n("aircraft")),
                    ("corpses", counts.corpses, n("corpses")),
                    ("fading", counts.fading, n("fading")),
                    (
                        "instances drawn of those listed",
                        d.soldiers + d.vehicles + d.corpses + d.pieces + d.props + d.ghosts,
                        d.listed,
                    ),
                ];
                for (what, got, wanted) in checks {
                    if got != wanted {
                        self.count_mismatches
                            .push(format!("{plate} {what}: {got} against {wanted}"));
                    }
                }
            }
            self.count_mismatches.extend(scenery_mismatches(
                &plate,
                &self.replay.instance_counts,
                &self.passes.scenery.drawn(),
            ));
            self.count_mismatches.extend(prop_mismatches(
                &plate,
                &self.replay.instance_counts,
                self.passes.models.drawn_props(),
            ));
            let scars = self.replay.presentation().scar_stats();
            println!(
                "{plate}: drawn {drawn:?}, culled soldiers {}, effects {} and {} lights, scar pages {} ({})",
                counts.culled,
                self.passes.effects.count(),
                counts.lights,
                scars.pages,
                if self.passes.ground.scars.regional() { "by region" } else { "at once" },
            );
            let encoder = self.passes.encode(device, queue, &self.view);
            let rgba = self.passes.read_display(device, queue, encoder);
            write_png(&out.join(&plate), self.passes.targets.size, &rgba)?;
            self.plates += 1;
            let encoder = device.create_command_encoder(&Default::default());
            self.present(None, encoder);
            return Ok(true);
        }
        if self.paced < args.pace {
            self.paced += 1;
            let encoder = self
                .passes
                .encode(&self.gpu.device, &self.gpu.queue, &self.view);
            self.present(None, encoder);
            return Ok(true);
        }
        Ok(false)
    }

    fn report(&mut self, args: &Args) -> Result<(), String> {
        if let Some(dir) = &args.bench {
            return self.bench_report(args, dir);
        }
        if let Some(out) = &args.out {
            println!("plates: {} written to {}", self.plates, out.display());
        }
        if let (Some(audio), Some(path)) = (&mut self.sound, &args.sound) {
            let samples = audio.samples();
            write_wav(path, samples)?;
            let seconds = samples.len() as f64 / 2.0 / f64::from(audio::SAMPLE_RATE);
            let (played, stats) = (audio.played(), audio.stats());
            println!(
                "sound: {:.1} s to {}; {} voices started, {} stopped, {} sounding; the frame started {}, dropped {}, stole {}, {} cues",
                seconds,
                path.display(),
                played.started,
                played.stopped,
                played.live,
                stats.started,
                stats.dropped,
                stats.stolen,
                stats.cues
            );
            println!(
                "voice lines {}, music tracks {}; not played: {:?}",
                played.lines,
                played.tracks,
                audio.errors()
            );
        }
        if self.count_mismatches.is_empty() {
            println!("model, scenery and effect counts: equal to the browser's on every plate");
        } else {
            println!("counts differ:\n  {}", self.count_mismatches.join("\n  "));
        }
        let p = &self.presents;
        let mut ms: Vec<f64> = p.intervals.iter().map(|d| d.as_secs_f64() * 1e3).collect();
        ms.sort_by(f64::total_cmp);
        let at = |q: f64| {
            ms.get(((ms.len() as f64 - 1.0) * q) as usize)
                .copied()
                .unwrap_or(0.0)
        };
        println!(
            "window {}: {} presented, {} occluded, other {:?}; frame interval p50 {:.2} ms, p95 {:.2} ms over the last {} frames",
            args.window,
            p.shown,
            p.occluded,
            p.other,
            at(0.5),
            at(0.95),
            ms.len()
        );
        Ok(())
    }

    /// Writes the bench's report beside the browser's, and prints its summary.
    fn bench_report(&mut self, args: &Args, dir: &std::path::Path) -> Result<(), String> {
        let frames = self.next.max(1) as f64;
        let mean = |i: usize| (self.drew[i] / frames).round();
        let passes: Vec<&str> = self.passes.passes.iter().map(|p| p.name()).collect();
        let p = &self.presents;
        let conditions = serde_json::json!({
            "power": bench::power_source(),
            "build": if cfg!(debug_assertions) { "native debug" } else { "native release" },
            "adapter": self.gpu.adapter.get_info().name,
            "pace": format!(
                "unpaced, at most {} frames in flight; window {}: {} presented, {} occluded",
                bench::IN_FLIGHT, args.window, p.shown, p.occluded
            ),
            "passes": passes,
            "meanDrawn": {
                "soldiers": mean(0),
                "vehicles": mean(1),
                "corpses": mean(2),
                "pieces": mean(3),
                "triangles": mean(4),
                "scenery": mean(5),
                "effects": mean(6),
                "instanceBytes": mean(7),
                "props": mean(8),
            },
        });
        let scene = self.replay.map.clone();
        let report = self.bench.as_mut().ok_or("not a bench")?.report(
            &self.gpu.device,
            &scene,
            self.passes.targets.size,
            conditions,
        )?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();
        let path = dir.join(format!("native-{scene}-{stamp}.json"));
        std::fs::write(&path, format!("{report}\n"))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        println!("{}", bench::summary(&report));
        println!("bench: → {}", path.display());
        Ok(())
    }
}

/// Interleaved stereo float samples as a 32-bit float WAV.
fn write_wav(path: &std::path::Path, samples: &[f32]) -> Result<(), String> {
    let rate = audio::SAMPLE_RATE;
    let data = (samples.len() * 4) as u32;
    let mut bytes = Vec::with_capacity(44 + data as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    for (v, n) in [
        (16u32, 4),
        (3, 2),
        (2, 2),
        (rate, 4),
        (rate * 8, 4),
        (8, 2),
        (32, 2),
    ] {
        bytes.extend_from_slice(&v.to_le_bytes()[..n]);
    }
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Where `plate`'s effects differ from the browser's (`effectCounts`): those
/// running by kind, the cast lights offered, the instances by shape, and the
/// instances drawn of those the frame lists.
fn effect_mismatches(
    plate: &str,
    effect_counts: &[serde_json::Value],
    counts: &presentation::records::Counts,
    effects: &[presentation::records::Effect],
    drawn: u32,
) -> Vec<String> {
    const KINDS: [&str; 8] = [
        "tracer", "flash", "impact", "sparks", "blast", "puff", "flame", "trail",
    ];
    const SHAPES: [&str; 4] = ["streak", "glow", "flipbook", "rotor"];
    let Some(want) = effect_counts.iter().find(|c| c["plate"] == plate) else {
        return vec![format!("{plate}: no effect counts in the manifest")];
    };
    let n = |v: &serde_json::Value| v.as_u64().unwrap_or(u64::MAX) as u32;
    let mut checks: Vec<(String, u32, u32)> = KINDS
        .iter()
        .zip(counts.running)
        .map(|(k, got)| (format!("running {k}"), got, n(&want["running"][k])))
        .collect();
    checks.push(("lights".into(), counts.lights, n(&want["lights"])));
    for (shape, name) in SHAPES.iter().enumerate() {
        let got = effects
            .iter()
            .filter(|e| e.misc[0] as usize == shape)
            .count() as u32;
        checks.push((
            format!("{name} instances"),
            got,
            n(&want["instances"][name]),
        ));
    }
    checks.push((
        "effect instances drawn of those listed".into(),
        drawn,
        effects.len() as u32,
    ));
    checks
        .into_iter()
        .filter(|(_, got, wanted)| got != wanted)
        .map(|(what, got, wanted)| format!("{plate} {what}: {got} against {wanted}"))
        .collect()
}

/// Where `plate`'s scenery differs from the browser's (`instanceCounts`):
/// the instances in view per population, kind and tier, and the stumps.
fn scenery_mismatches(
    plate: &str,
    instance_counts: &[serde_json::Value],
    drawn: &Drawn,
) -> Vec<String> {
    const POPULATIONS: [&str; 5] = ["forest", "understorey", "backdrop", "dressing", "felled"];
    let Some(want) = instance_counts.iter().find(|c| c["plate"] == plate) else {
        return vec![format!("{plate}: no instance counts in the manifest")];
    };
    let want = &want["scenery"];
    let list = |v: &serde_json::Value| -> Vec<u32> {
        v.as_array()
            .map(|a| {
                a.iter()
                    .map(|n| n.as_u64().unwrap_or(u64::MAX) as u32)
                    .collect()
            })
            .unwrap_or_default()
    };
    let total = |v: &[u32]| v.iter().sum::<u32>();
    POPULATIONS
        .iter()
        .zip(&drawn.populations)
        .map(|(name, got)| (*name, got.clone(), list(&want[name])))
        .chain([("stumps", drawn.stumps.clone(), list(&want["stumps"]))])
        .filter(|(_, got, wanted)| got != wanted)
        .map(|(name, got, wanted)| {
            format!(
                "{plate} scenery {name}: {} against {} ({got:?} against {wanted:?})",
                total(&got),
                total(&wanted)
            )
        })
        .collect()
}

/// Where the props `drawn` (by appearance, per tier) differ from what the
/// browser drew of `plate` (`instanceCounts[].props`).
fn prop_mismatches(
    plate: &str,
    instance_counts: &[serde_json::Value],
    drawn: &BTreeMap<String, [u32; 4]>,
) -> Vec<String> {
    let Some(want) = instance_counts.iter().find(|c| c["plate"] == plate) else {
        return vec![];
    };
    let want: BTreeMap<String, [u32; 4]> = want["props"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            let levels = &p["levels"];
            let level = |t: usize| levels[t].as_u64().unwrap_or(u64::MAX) as u32;
            let name = p["appearance"].as_str().unwrap_or("").to_owned();
            (name, std::array::from_fn(level))
        })
        .collect();
    let total = |m: &BTreeMap<String, [u32; 4]>| -> u32 { m.values().flatten().sum() };
    let names: BTreeSet<&String> = want.keys().chain(drawn.keys()).collect();
    names
        .into_iter()
        .filter(|n| want.get(*n) != drawn.get(*n))
        .map(|n| {
            format!(
                "{plate} props {n}: {:?} against {:?} (all props {} against {})",
                drawn.get(n),
                want.get(n),
                total(drawn),
                total(&want)
            )
        })
        .collect()
}

fn write_png(path: &std::path::Path, size: [u32; 2], rgba: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), size[0], size[1]);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())
}

struct App {
    args: Args,
    run: Option<Run>,
    error: Option<String>,
}

impl App {
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: String) {
        self.error = Some(error);
        event_loop.exit();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.run.is_some() {
            return;
        }
        match Run::start(event_loop, &self.args) {
            Ok(run) => self.run = Some(run),
            Err(e) => self.fail(event_loop, e),
        }
        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Occluded(occluded) => println!("window occluded: {occluded}"),
            _ => {}
        }
    }

    /// The frames run from the loop itself, not from redraw requests, which
    /// macOS doesn't deliver to a window it isn't showing.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(run) = self.run.as_mut() else { return };
        match run.step(&self.args) {
            Ok(true) => {}
            Ok(false) => {
                if let Err(e) = run.report(&self.args) {
                    self.error = Some(e);
                }
                self.run = None;
                event_loop.exit();
            }
            Err(e) => self.fail(event_loop, e),
        }
    }
}

fn main() {
    let result = args().and_then(|args| {
        let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
        let mut app = App {
            args,
            run: None,
            error: None,
        };
        event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
        app.error.map_or(Ok(()), Err)
    });
    if let Err(e) = result {
        eprintln!("native-client: {e}");
        std::process::exit(1);
    }
}
