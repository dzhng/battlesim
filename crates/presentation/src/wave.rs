//! The wave field's clock (slice 16 of water and wetlands): the renderer
//! runs a height field over a square of water around what the camera looks
//! at, stepped at a fixed rate on the presentation clock, so its rings and
//! V-wakes spread alike at any frame rate. [`WaveClock`] says how many steps
//! a frame runs, when the field starts over (a new epoch) and how far it
//! moved; [`area`] where it lies, in the camera every shader binds
//! (`Camera.waves`). The shaders read its numbers ([`WAVE`]) by name
//! (`WAVE_*`, prepended to every built-in shader).
use std::sync::atomic::{AtomicU32, Ordering};

use crate::view::Camera;

/// The field's tuning, the renderer's and the clock's.
pub struct Wave {
    /// Steps a second, on the presentation clock.
    pub hz: f64,
    /// The most steps one frame runs: past it a jump drops the excess.
    pub max_steps: u32,
    /// Texels along each side, and metres a texel.
    pub texels: u32,
    pub texel_m: f64,
    /// RedGPU's wave speed and damping a step, and how steeply the field's
    /// slope tilts the water's normal.
    pub speed: f64,
    pub damping: f64,
    pub normal: f64,
    /// Waves show whole for a camera this near what it looks at and are gone
    /// past this far (metres): beyond, a wake is under a pixel.
    pub fade_m: [f64; 2],
}

/// Splats one step takes at most; excess splats are dropped per step so
/// batching steps into frames never changes which impulses survive.
pub const STEP_SPLATS: usize = 256;

pub const WAVE: Wave = Wave {
    hz: 30.0,
    max_steps: 6,
    texels: 1024,
    texel_m: 0.125,
    speed: 0.2,
    damping: 0.08,
    normal: 0.15,
    fade_m: [180.0, 320.0],
};

/// The field's south-west texel for `camera`: centred on what it looks at,
/// on the texel grid.
fn origin(camera: &Camera) -> [i32; 2] {
    let half = i64::from(WAVE.texels / 2);
    let at = |v: f64| ((v / WAVE.texel_m).floor() as i64 - half) as i32;
    [at(camera.target[0]), at(camera.target[1])]
}

/// The field as `camera`'s record carries it (`Camera.waves`): its
/// south-west corner and side (metres), and how much of its waves show.
pub fn area(camera: &Camera) -> [f32; 4] {
    let [x, y] = origin(camera).map(|t| f64::from(t) * WAVE.texel_m);
    let d = camera
        .eye
        .iter()
        .zip(camera.target)
        .map(|(e, t)| (e - t).powi(2))
        .sum::<f64>()
        .sqrt();
    let [near, far] = WAVE.fade_m;
    let shown = 1.0 - ((d - near) / (far - near)).clamp(0.0, 1.0);
    let side = f64::from(WAVE.texels) * WAVE.texel_m;
    [x as f32, y as f32, side as f32, shown as f32]
}

/// Epochs handed out so far: every clock's first is new.
static EPOCHS: AtomicU32 = AtomicU32::new(0);

/// What a frame runs: `steps` steps of the field, which starts over
/// whenever `epoch` changes, laid with its south-west texel at
/// `origin_texel`, `shift` texels on from where the last frame laid it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WaveStep {
    pub steps: u32,
    pub epoch: u32,
    pub origin_texel: [i32; 2],
    pub shift: [i32; 2],
}

/// The field's clock: the steps it has run on the presentation clock (as
/// `floor(clock · hz)`, so any split into frames runs the same steps), and
/// where it lay.
#[derive(Debug)]
pub struct WaveClock {
    done: Option<i64>,
    epoch: u32,
    origin: Option<[i32; 2]>,
}

impl Default for WaveClock {
    fn default() -> Self {
        Self::new()
    }
}

impl WaveClock {
    /// A clock in an epoch of its own.
    pub fn new() -> Self {
        Self {
            done: None,
            epoch: EPOCHS.fetch_add(1, Ordering::Relaxed) + 1,
            origin: None,
        }
    }

    /// Start over, in a new epoch: a reset, a load or a side switch.
    pub fn restart(&mut self) {
        *self = Self::new();
    }

    /// The step index the field has run up to (exclusive).
    pub fn done(&self) -> Option<i64> {
        self.done
    }

    /// The frame at presentation time `clock` for `camera`: the steps since
    /// the last, at most `max_steps`. The clock running backwards starts
    /// over; standing still runs none.
    pub fn advance(&mut self, clock: f64, camera: &Camera) -> WaveStep {
        let target = (clock * WAVE.hz).floor() as i64;
        if self.done.is_some_and(|d| target < d) {
            self.restart();
        }
        let done = self.done.unwrap_or(target);
        let steps = (target - done).min(i64::from(WAVE.max_steps)) as u32;
        self.done = Some(target);
        let origin = origin(camera);
        let was = self.origin.replace(origin).unwrap_or(origin);
        WaveStep {
            steps,
            epoch: self.epoch,
            origin_texel: origin,
            shift: [origin[0] - was[0], origin[1] - was[1]],
        }
    }
}
