//! Replays a browser parity run's scene natively and writes its plates.
//!
//! `native-client --run <parity run> --scene <map> --out <dir>
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
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use native_client::audio::{self, Audio, OutputSettings};
use native_client::bundle::{Library, RUNTIME};
use native_client::capture::Replay;
use native_client::effects::{EffectPass, Effects};
use native_client::environment::Environment;
use native_client::fog_mask::fog_mask;
use native_client::frame::{Passes, Targets, DISPLAY_FORMAT};
use native_client::gpu::{self, Gpu};
use native_client::models::Models;
use native_client::pipelines::Pipelines;
use native_client::post::Post;
use native_client::sky::Sky;
use native_client::world::{DepthPrepass, Ghosts, WorldColour};
use presentation::records::View;
use wgpu::{CommandEncoder, CurrentSurfaceTexture, PresentMode, Surface, SurfaceConfiguration};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

struct Args {
    scene: PathBuf,
    out: PathBuf,
    window: String,
    pace: usize,
    sound: Option<PathBuf>,
}

fn args() -> Result<Args, String> {
    let mut run = None;
    let (mut scene, mut out, mut window, mut pace) = (None, None, "offscreen".to_owned(), 120);
    let mut sound = None;
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
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if !["offscreen", "minimised", "hidden", "visible"].contains(&window.as_str()) {
        return Err(format!("--window {window}"));
    }
    Ok(Args {
        scene: run
            .ok_or("--run is required")?
            .join(scene.ok_or("--scene is required")?),
        out: out.ok_or("--out is required")?,
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
}

impl Run {
    fn start(event_loop: &ActiveEventLoop, args: &Args) -> Result<Self, String> {
        let game = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/game.json");
        let replay = Replay::open(&args.scene, &game)?;
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
        let environment = Environment::new(
            &gpu.device,
            &gpu.queue,
            &pipelines,
            &sky.lut,
            &light.sun(),
            &light.aerial(),
        );
        let installed = Instant::now();
        let library = Library::open(std::path::Path::new(RUNTIME))?;
        let models = Models::new(
            &gpu.device,
            &gpu.queue,
            &pipelines,
            &library,
            replay.presentation(),
        )?;
        println!(
            "models: {} drawables installed in {:.2} s",
            replay.presentation().world().appearances.len(),
            installed.elapsed().as_secs_f64()
        );
        let effects = Effects::new(&gpu.device, &gpu.queue, &pipelines, &environment, &targets)?;
        let (fog_mask, fog_rim) = fog_mask(&gpu.device, &pipelines, &targets, &replay.fog_look);
        std::fs::create_dir_all(&args.out).map_err(|e| e.to_string())?;
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
                environment,
                models,
                effects,
                passes: vec![
                    Box::new(sky),
                    Box::new(DepthPrepass::new(&pipelines)),
                    Box::new(WorldColour::new(&pipelines)),
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
        })
    }

    /// Copies the display target to the window and presents it, if the
    /// window can show it.
    fn present(&mut self, mut encoder: CommandEncoder) {
        let now = Instant::now();
        if let Some(last) = self.last.replace(now) {
            self.presents.intervals.push(now - last);
        }
        let texture = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(t) | CurrentSurfaceTexture::Suboptimal(t) => t,
            CurrentSurfaceTexture::Occluded => {
                self.presents.occluded += 1;
                self.gpu.queue.submit([encoder.finish()]);
                return;
            }
            other => {
                let status = format!("{other:?}");
                self.presents.other.push(status);
                self.gpu.queue.submit([encoder.finish()]);
                return;
            }
        };
        let targets = &self.passes.targets;
        encoder.copy_texture_to_texture(
            targets.display_texture.as_image_copy(),
            texture.texture.as_image_copy(),
            targets.display_texture.size(),
        );
        self.gpu.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.gpu.queue.present(texture);
        self.presents.shown += 1;
    }

    /// Frames records up to the next plate and draws it; then paces. False when done.
    fn step(&mut self, args: &Args) -> Result<bool, String> {
        while let Some(record) = self.replay.records.get(self.next).cloned() {
            self.next += 1;
            // The reel's wall time is both the sound's wall and its mixer clock.
            let now = record.reel_ms / 1000.0;
            let mut sound = self.sound.as_mut();
            self.replay.frame(&record, |p| {
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
            let frame = self.replay.last_frame();
            self.view = frame.view;
            let (device, queue) = (&self.gpu.device, &self.gpu.queue);
            self.passes.prepare(device, queue, frame, record.target);
            let (counts, wrap) = (frame.counts, frame.light_wrap);
            let presentation = self.replay.presentation();
            self.passes
                .prepare_effects(device, queue, presentation, wrap);
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
                        d.soldiers + d.vehicles + d.corpses + d.pieces + d.ghosts,
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
            println!(
                "{plate}: drawn {drawn:?}, culled soldiers {}, effects {} and {} lights",
                counts.culled,
                self.passes.effects.count(),
                counts.lights
            );
            let encoder = self.passes.encode(device, queue, &self.view);
            let rgba = self.passes.read_display(device, queue, encoder);
            write_png(&args.out.join(&plate), self.passes.targets.size, &rgba)?;
            self.plates += 1;
            let encoder = device.create_command_encoder(&Default::default());
            self.present(encoder);
            return Ok(true);
        }
        if self.paced < args.pace {
            self.paced += 1;
            let encoder = self
                .passes
                .encode(&self.gpu.device, &self.gpu.queue, &self.view);
            self.present(encoder);
            return Ok(true);
        }
        Ok(false)
    }

    fn report(&mut self, args: &Args) -> Result<(), String> {
        println!("plates: {} written to {}", self.plates, args.out.display());
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
        }
        if self.count_mismatches.is_empty() {
            println!("model and effect counts: equal to the browser's on every plate");
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
