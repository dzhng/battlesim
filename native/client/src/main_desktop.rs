//! Replays a browser parity run's scene natively and writes its plates, or
//! times it.
//!
//! `native-client --run <parity run> --scene <map> (--out <dir> | --bench <dir>)
//!   [--window offscreen|minimised|hidden|visible] [--pace <frames>]
//!   [--sound <wav>]`
//! A held benchmark adds `--bench-freeze-index <record> --bench-frames <draws>`.
//!
//! Every frame record is framed through the presentation crate; each one
//! the browser wrote a plate of is drawn, presented and written to `--out`
//! under the control plate's name. Then the last frame is drawn and presented
//! `--pace` more times, and the report says how presenting went. The window
//! opens off-screen unless told otherwise. With `--sound`, the battle's
//! sound is heard through every record at the reel's own times and rendered
//! to that file (never played aloud).
//!
//! With `--bench`, every frame record is framed and drawn back to back without
//! window presentation. A frozen benchmark frames records through its chosen
//! zero-based position once, then draws that held publication. The timing
//! report goes to `native-<scene>-<time>.json` for `web/fps-compare.mjs`.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use native_client::audio::{self, Audio, OutputSettings};
use native_client::bench::{self, Bench};
use native_client::bundle::Library;
use native_client::capture::Replay;
use native_client::counts::Counted;
use native_client::gpu::{self, Gpu};
use presentation::assets::RUNTIME;
use presentation::records::View;
use renderer_core::frame::FrameStage;
use renderer_core::runtime::{Runtime, RuntimeConfig, RuntimeWorld, WorldGeometry};
use renderer_core::DISPLAY_FORMAT;
use wgpu::{
    CommandEncoder, CurrentSurfaceTexture, PresentMode, SubmissionIndex, Surface,
    SurfaceConfiguration,
};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

#[cfg(test)]
mod runtime_path_tests {
    use super::runtime_dir;
    use std::fs;

    #[test]
    fn runtime_dir_finds_assets_next_to_executable_tree() {
        let root = std::env::temp_dir().join(format!("native-runtime-{}", std::process::id()));
        let executable = root.join("target/debug/native-client.exe");
        let runtime = root.join("assets/runtime");
        fs::create_dir_all(&runtime).unwrap();
        fs::write(runtime.join("catalog.json"), b"{}").unwrap();

        assert_eq!(runtime_dir(&executable, None), runtime);

        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod count_verdict_tests {
    use super::count_verdict;

    #[test]
    fn a_run_whose_counts_differ_fails_naming_each_difference() {
        let differ = [
            "plate-3.png soldiers: 4 against 5".to_owned(),
            "plate-7.png lights: 2 against 1".to_owned(),
        ];
        let e = count_verdict(&differ).expect_err("a differing run passed");
        for line in &differ {
            assert!(e.contains(line), "{e}");
        }
    }

    #[test]
    fn a_run_equal_to_the_browser_passes() {
        assert_eq!(count_verdict(&[]), Ok(()));
    }
}

struct Args {
    scene: PathBuf,
    /// Where plates are written; absent for a bench.
    out: Option<PathBuf>,
    /// Where a bench's report is written.
    bench: Option<PathBuf>,
    /// One canonical record held across a fixed number of GPU draws.
    frozen_bench: Option<(usize, usize)>,
    window: String,
    pace: usize,
    sound: Option<PathBuf>,
}

/// Resolve native runtime files without relying on the build machine's path.
/// Packaged builds set `BATTLEGAME_RUNTIME`; development builds find the
/// repository's `assets/runtime` beside the executable's target directory.
fn runtime_dir(executable: &Path, configured: Option<&Path>) -> PathBuf {
    if let Some(path) = configured {
        return path.to_owned();
    }
    for ancestor in executable.ancestors() {
        let candidate = ancestor.join("assets/runtime");
        if candidate.join("catalog.json").is_file() {
            return candidate;
        }
    }
    PathBuf::from(RUNTIME)
}

fn repository_root(executable: &Path) -> PathBuf {
    for ancestor in executable.ancestors() {
        if ancestor.join("fixtures/rules.json").is_file() {
            return ancestor.to_owned();
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .to_owned()
}

fn args() -> Result<Args, String> {
    let mut run = None;
    let (mut scene, mut out, mut window, mut pace) = (None, None, "offscreen".to_owned(), 120);
    let (mut sound, mut bench) = (None, None);
    let (mut freeze_index, mut bench_frames) = (None, None);
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
            "--bench-freeze-index" => {
                freeze_index = Some(
                    value()?
                        .parse::<usize>()
                        .map_err(|e| format!("{flag}: {e}"))?,
                );
            }
            "--bench-frames" => {
                bench_frames = Some(
                    value()?
                        .parse::<usize>()
                        .map_err(|e| format!("{flag}: {e}"))?,
                );
            }
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
    if freeze_index.is_some() != bench_frames.is_some() {
        return Err("--bench-freeze-index and --bench-frames must be supplied together".into());
    }
    if freeze_index.is_some() && bench.is_none() {
        return Err("--bench-freeze-index and --bench-frames require --bench".into());
    }
    if let Some(frames) = bench_frames {
        bench::reported_draws(frames).map_err(|e| format!("--bench-frames: {e}"))?;
    }
    Ok(Args {
        scene: run
            .ok_or("--run is required")?
            .join(scene.ok_or("--scene is required")?),
        out,
        bench,
        frozen_bench: freeze_index.zip(bench_frames),
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
    scene: Runtime,
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
        let executable = std::env::current_exe().ok();
        let configured = std::env::var_os("BATTLEGAME_RUNTIME");
        let runtime = runtime_dir(
            executable.as_deref().unwrap_or_else(|| Path::new("")),
            configured.as_deref().map(Path::new),
        );
        let library = Library::open(&runtime)?;
        let root = executable
            .as_deref()
            .map(repository_root)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
        std::env::set_var("BATTLEGAME_ROOT", &root);
        let opened = Instant::now();
        let mut replay = Replay::open_with_root(&args.scene, &library, &root)?;
        if let Some((index, _)) = args.frozen_bench {
            if index >= replay.records.len() {
                return Err(format!(
                    "--bench-freeze-index {index}: capture has {} records",
                    replay.records.len()
                ));
            }
        }
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
        let sizes = renderer_core::pipelines::builtin_sizes();
        let effect_sheets = native_client::effects::load_sheets(
            &root.join("assets/third-party/effects"),
            sizes.effect_sheet,
        )?;
        let light = replay.light.clone();
        let resolved = replay
            .presentation()
            .ground()
            .ok_or("the replay set no ground")?;
        let geometry = Arc::new(WorldGeometry::from_triangles(
            &replay.terrain.0,
            &replay.terrain.1,
            &resolved.water_cells(),
            replay.backdrop_reach_m,
            None,
            &[],
            true,
        ));
        let config = RuntimeConfig {
            size,
            light,
            fog_look: replay.fog_look,
            grass_density: Some(1.0),
            xray_min_hidden_fraction: None,
            paint: None,
            separate_high_air: false,
            water_detail: Default::default(),
            controls: Default::default(),
        };
        let mut scene = Runtime::new(
            &gpu.device,
            &gpu.queue,
            config,
            RuntimeWorld {
                presentation: replay.presentation_mut(),
                assets: &library,
                geometry,
                effect_sheets,
            },
        )?;
        pollster::block_on(scene.prepare_assets(&gpu.device, &gpu.queue))?;
        if let Some((index, _)) = args.frozen_bench {
            for at in 0..=index {
                let record = replay.records[at].clone();
                replay.frame(&record, scene.static_generation(), |_| {})?;
            }
        }
        println!(
            "pipelines: {} built and world installed in {:.2} s on {}",
            scene.pipelines.names().count(),
            built.elapsed().as_secs_f64(),
            gpu.adapter.get_info().name
        );
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
                let fixtures = root.join("fixtures");
                let setup = audio::setup(
                    &replay.rules,
                    &read(&fixtures.join("presentation.json").to_string_lossy())?,
                    &read(&fixtures.join("audio.json").to_string_lossy())?,
                    &read(&fixtures.join("sounds.json").to_string_lossy())?,
                )?;
                let output = OutputSettings::Buffer(audio::SAMPLE_RATE);
                Some(Audio::new(&setup, &runtime, output)?)
            }
        };
        Ok(Self {
            window,
            surface,
            gpu,
            scene,
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

    /// Submits `encoder`, copying the display target to the window and
    /// presenting it if the window can show it.
    fn present(&mut self, mut encoder: CommandEncoder) -> SubmissionIndex {
        let now = Instant::now();
        if let Some(last) = self.last.replace(now) {
            self.presents.intervals.push(now - last);
        }
        // A bench measures rendering, not the platform's swapchain pacing.
        // Windows can block an off-screen surface even when it is never shown.
        if self.bench.is_some() {
            return self.gpu.queue.submit([encoder.finish()]);
        }
        let texture = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(t) | CurrentSurfaceTexture::Suboptimal(t) => t,
            CurrentSurfaceTexture::Occluded => {
                self.presents.occluded += 1;
                return self.gpu.queue.submit([encoder.finish()]);
            }
            other => {
                let status = format!("{other:?}");
                self.presents.other.push(status);
                return self.gpu.queue.submit([encoder.finish()]);
            }
        };
        let targets = &self.scene.targets;
        encoder.copy_texture_to_texture(
            targets.display_texture.as_image_copy(),
            texture.texture.as_image_copy(),
            targets.display_texture.size(),
        );
        let index = self.gpu.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        self.gpu.queue.present(texture);
        self.presents.shown += 1;
        index
    }

    /// Prepares the layers to draw the last frame framed.
    fn prepare(&mut self) -> Result<(), String> {
        self.view = self.replay.last_frame().view;
        self.scene.prepare(
            &self.gpu.device,
            &self.gpu.queue,
            self.replay.presentation_mut(),
        )?;
        Ok(())
    }

    /// Draws the next benchmark sample, advancing only changing publications.
    fn bench_step(&mut self, args: &Args) -> Result<bool, String> {
        let draws = args
            .frozen_bench
            .map_or(self.replay.records.len(), |(_, draws)| draws);
        if self.next >= draws {
            return Ok(false);
        }
        let record = args
            .frozen_bench
            .is_none()
            .then(|| self.replay.records[self.next].clone());
        self.next += 1;
        let bench = self.bench.as_mut().ok_or("not a bench")?;
        bench.start();
        if let Some(record) = record {
            self.replay
                .frame(&record, self.scene.static_generation(), |_| {})?;
        }
        self.bench.as_mut().ok_or("not a bench")?.mark();
        self.prepare()?;
        let bench = self.bench.as_mut().ok_or("not a bench")?;
        bench.mark();
        let (device, queue) = (&self.gpu.device, &self.gpu.queue);
        let mut encoder = device.create_command_encoder(&Default::default());
        bench.begin(&mut encoder);
        self.scene
            .encode(device, queue, &self.view, &mut encoder, Default::default());
        bench.end(&mut encoder);
        bench.mark();
        let index = self.present(encoder);
        let bench = self.bench.as_mut().ok_or("not a bench")?;
        bench.submitted(&self.gpu.device, index)?;
        // What the frame drew, tallied outside its timed phases.
        let d = self.scene.models.drawn();
        let scenery = Counted::of(self.replay.presentation()).scenery;
        let trees: u32 = scenery.iter().flatten().sum();
        let bytes = std::mem::size_of_val(&self.replay.last_frame().instances[..]);
        let frame = [
            f64::from(d.soldiers),
            f64::from(d.vehicles),
            f64::from(d.corpses),
            f64::from(d.pieces),
            d.triangles as f64,
            f64::from(trees),
            f64::from(self.scene.effects.count()),
            bytes as f64,
            f64::from(d.props),
        ];
        for (sum, v) in self.drew.iter_mut().zip(frame) {
            *sum += v;
        }
        Ok(true)
    }

    /// Frames records up to the next plate and draws it; then paces. False when done.
    fn step(&mut self, args: &Args) -> Result<bool, String> {
        if self.bench.is_some() {
            return self.bench_step(args);
        }
        let out = args.out.as_ref().ok_or("--out is required")?;
        while let Some(record) = self.replay.records.get(self.next).cloned() {
            self.next += 1;
            // The reel's wall time is both the sound's wall and its mixer clock.
            let now = record.reel_ms / 1000.0;
            let mut sound = self.sound.as_mut();
            self.replay
                .frame(&record, self.scene.static_generation(), |p| {
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
            self.prepare()?;
            let counts = self.replay.last_frame().counts;
            let presentation = self.replay.presentation();
            let row = |rows: &[serde_json::Value]| {
                rows.iter()
                    .find(|c| c["plate"] == plate.as_str())
                    .cloned()
                    .unwrap_or_default()
            };
            self.count_mismatches
                .extend(Counted::of(presentation).mismatches(
                    &plate,
                    &row(&self.replay.model_counts),
                    &row(&self.replay.effect_counts),
                    &row(&self.replay.instance_counts),
                ));
            // What the models and effects draw of what the frame lists: a
            // soldier off screen is counted, not drawn.
            let drawn = self.scene.models.drawn();
            let d = &drawn;
            for (what, got, listed) in [
                (
                    "instances drawn of those listed",
                    d.soldiers + d.vehicles + d.corpses + d.pieces + d.props + d.ghosts,
                    d.listed,
                ),
                (
                    "effect instances drawn of those listed",
                    self.scene.effects.count(),
                    presentation.effects().len() as u32,
                ),
            ] {
                if got != listed {
                    self.count_mismatches
                        .push(format!("{plate} {what}: {got} against {listed}"));
                }
            }
            let (device, queue) = (&self.gpu.device, &self.gpu.queue);
            let scars = self.replay.presentation().scar_stats();
            println!(
                "{plate}: drawn {drawn:?}, culled soldiers {}, effects {} and {} lights, scar pages {} ({})",
                counts.culled,
                self.scene.effects.count(),
                counts.lights,
                scars.pages,
                if self.scene.ground.material.scars.regional() { "by region" } else { "at once" },
            );
            let mut encoder = device.create_command_encoder(&Default::default());
            self.scene
                .encode(device, queue, &self.view, &mut encoder, Default::default());
            let read = renderer_core::readback::RgbaReadback::encode(
                device,
                &mut encoder,
                &self.scene.targets.display_texture,
            )?;
            queue.submit([encoder.finish()]);
            let mut rgba = pollster::block_on(read.read(device))?;
            // Native plates are opaque exports, independent of stored display alpha.
            for pixel in rgba.as_chunks_mut::<4>().0 {
                pixel[3] = 255;
            }
            output::write_png(&out.join(&plate), self.scene.targets.size, &rgba)?;
            self.plates += 1;
            let encoder = device.create_command_encoder(&Default::default());
            self.present(encoder);
            return Ok(true);
        }
        if self.paced < args.pace {
            self.paced += 1;
            let mut encoder = self.gpu.device.create_command_encoder(&Default::default());
            self.scene.encode(
                &self.gpu.device,
                &self.gpu.queue,
                &self.view,
                &mut encoder,
                Default::default(),
            );
            self.present(encoder);
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
            output::write_wav(path, samples)?;
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
        count_verdict(&self.count_mismatches)
    }

    /// Writes the bench's report beside the browser's, and prints its summary.
    fn bench_report(&mut self, args: &Args, dir: &std::path::Path) -> Result<(), String> {
        let frames = self.next.max(1) as f64;
        let retained = bench::reported_draws(self.next)?;
        let source = |draw: usize| args.frozen_bench.map_or(draw, |(index, _)| index);
        let first = &self.replay.records[source(retained.start)];
        let last = &self.replay.records[source(retained.end - 1)];
        let mean = |i: usize| (self.drew[i] / frames).round();
        let passes = FrameStage::ORDER.map(FrameStage::label);
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
            "records": {
                "mode": if args.frozen_bench.is_some() { "frozen" } else { "changing" },
                "draws": self.next,
                "initializedThrough": args.frozen_bench.map(|(index, _)| index),
                "reportedDrawFirst": retained.start,
                "reportedDrawLast": retained.end - 1,
                "reportedSourceFirst": source(retained.start),
                "reportedSourceLast": source(retained.end - 1),
                "firstSimTime": first.sim_time,
                "lastSimTime": last.sim_time,
                "firstTick": first.tick,
                "lastTick": last.tick,
            },
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
            self.scene.targets.size,
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

/// A plate run passes only when every plate's counts equal the browser's:
/// otherwise it fails listing each difference, after the rest of the report.
fn count_verdict(mismatches: &[String]) -> Result<(), String> {
    if mismatches.is_empty() {
        println!("model, building, prop, scenery and effect counts: equal to the browser's on every plate");
        return Ok(());
    }
    Err(format!(
        "counts differ from the browser's:\n  {}",
        mismatches.join("\n  ")
    ))
}

#[path = "main_app.rs"]
mod app;
#[path = "main_output.rs"]
mod output;
use app::App;

pub fn run() {
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
