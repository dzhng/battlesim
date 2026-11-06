//! Command-line arguments for the native client: the scene to replay, where
//! to write plates or a bench, the clip window, and which recorded layers
//! (`Playback`) the replay draws.
use super::*;
use native_client::capture::Playback;
use presentation::assets::RUNTIME;
use std::path::{Path, PathBuf};

pub(super) struct Args {
    pub(super) scene: PathBuf,
    /// Where plates or clip frames are written; absent for a bench.
    pub(super) out: Option<PathBuf>,
    /// Where a bench's report is written.
    pub(super) bench: Option<PathBuf>,
    /// One canonical record held across a fixed number of GPU draws.
    pub(super) frozen_bench: Option<(usize, usize)>,
    pub(super) window: String,
    pub(super) pace: usize,
    pub(super) sound: Option<PathBuf>,
    /// Write the HDR look's HLG signal as 16-bit PNGs rather than the display.
    pub(super) hdr: bool,
    /// Leave the music out of the sound.
    pub(super) no_music: bool,
    /// Frames at a fixed rate over a window of the reel, instead of plates.
    pub(super) clip: Option<Clip>,
    /// The layers the capture drew that the replay draws.
    pub(super) playback: Playback,
}

/// Resolve native runtime files without relying on the build machine's path.
/// Packaged builds set `BATTLEGAME_RUNTIME`; development builds find the
/// repository's `assets/runtime` beside the executable's target directory.
pub(super) fn runtime_dir(executable: &Path, configured: Option<&Path>) -> PathBuf {
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

pub(super) fn repository_root(executable: &Path) -> PathBuf {
    for ancestor in executable.ancestors() {
        if ancestor.join("fixtures/rules.json").is_file() {
            return ancestor.to_owned();
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .to_owned()
}

pub(super) fn args() -> Result<Args, String> {
    let mut run = None;
    let (mut scene, mut out, mut window, mut pace) = (None, None, "offscreen".to_owned(), 120);
    let (mut sound, mut bench) = (None, None);
    let (mut freeze_index, mut bench_frames) = (None, None);
    let (mut hdr, mut no_music, mut fps, mut from_ms, mut to_ms) = (false, false, None, None, None);
    let mut playback = Playback::default();
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
            "--hdr" => hdr = true,
            "--no-music" => no_music = true,
            "--no-indicators" => playback.indicators = false,
            "--no-cards" => playback.cards = false,
            "--no-fog" => playback.fog = false,
            "--fps" => fps = Some(value()?.parse().map_err(|e| format!("{flag}: {e}"))?),
            "--from-ms" => from_ms = Some(value()?.parse().map_err(|e| format!("{flag}: {e}"))?),
            "--to-ms" => to_ms = Some(value()?.parse().map_err(|e| format!("{flag}: {e}"))?),
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
    if bench.is_some() && (hdr || fps.is_some()) {
        return Err("a bench writes no frames: --hdr and --fps need --out".into());
    }
    let clip = Clip::new(fps, from_ms, to_ms)?;
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
        hdr,
        no_music,
        clip,
        playback,
    })
}
