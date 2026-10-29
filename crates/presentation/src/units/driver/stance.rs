//! How a soldier holds himself: his postures (never one that puts his chest
//! under the water he stands in), the stance he takes under fire (his own
//! for life, from his id), and the fall he plays from each.
use serde::Deserialize;

use crate::static_world::ground::water::Wet;

/// `presentation.pose.stance`: what share of men fire lying down or
/// standing, kneel when pinned, and fall back onto their backs, and how
/// high each posture (standing, kneeling, lying) holds a man's chest over
/// the ground he stands on, metres.
#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct StanceFeel {
    pub fire_prone: f64,
    pub fire_stand: f64,
    pub pinned_kneel: f64,
    pub fall_back: f64,
    pub chest_m: [f64; 3],
}

/// Highest first: each holds his chest lower than the one before.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Posture {
    Stand,
    Kneel,
    Prone,
}

impl Posture {
    /// The posture he takes for `self` standing in water `depth` deep (its
    /// metres, asked only if he is down): the first from `self` upward that
    /// holds his chest (`chest_m`, by posture) above the water. He stands in
    /// any water, so a wading man never lies down in it.
    fn above_water(self, depth: impl FnOnce() -> f64, chest_m: &[f64; 3]) -> Posture {
        if self == Posture::Stand {
            return self;
        }
        let depth = depth();
        [Posture::Prone, Posture::Kneel]
            .into_iter()
            .skip_while(|&p| p != self)
            .find(|&p| chest_m[p as usize] > depth)
            .unwrap_or(Posture::Stand)
    }
}

/// The fall a soldier plays dying in each posture (`INFANTRY_DEATHS`), and
/// standing, back onto his back.
const DEATHS: [&str; 3] = ["death", "death_kneel", "death_prone"];
const FALL_BACK: &str = "death_back";

/// The corpse (an `INFANTRY_CORPSES` index: face down, face up) a fall ends in.
pub(super) fn lies_after(fall: &str) -> usize {
    usize::from(fall == FALL_BACK)
}

/// How a soldier holds himself under fire, his own for life (from his id).
#[derive(Clone, Copy, Debug)]
pub(super) struct Stance {
    pub firing: Posture,
    pub pinned: Posture,
    pub falls_back: bool,
}

impl Stance {
    /// His posture this moment. Out on his `lean` he kneels to fire, pinned
    /// or not; otherwise, `pinned` or `firing`, he takes his own stance.
    /// Lying down he can only crawl so fast: `fast`er, he goes crouched. In
    /// the water he stands `wet` in, he rises until his chest is clear of it.
    pub(super) fn posture(
        self,
        lean: bool,
        pinned: bool,
        firing: bool,
        fast: bool,
        wet: impl FnOnce() -> Option<Wet>,
        feel: &StanceFeel,
    ) -> Posture {
        let held = if lean {
            Posture::Kneel
        } else if pinned {
            self.pinned
        } else if firing {
            self.firing
        } else {
            Posture::Stand
        };
        let posture = if held == Posture::Prone && fast {
            Posture::Kneel
        } else {
            held
        };
        posture.above_water(|| wet().map_or(0.0, |w| w.depth_m), &feel.chest_m)
    }
}

/// One place along a low-discrepancy sequence decides both postures, so
/// neighbouring ids spread across the stances and the men who fire lying
/// down are the ones who stay flat when pinned.
pub(super) fn stance_manner(soldier: f64, s: &StanceFeel) -> Stance {
    let u = (soldier * 0.414_213_562_373_095_15) % 1.0;
    let firing = if u < s.fire_prone {
        Posture::Prone
    } else if u >= 1.0 - s.fire_stand {
        Posture::Stand
    } else {
        Posture::Kneel
    };
    let pinned = if u >= 1.0 - s.pinned_kneel {
        Posture::Kneel
    } else {
        Posture::Prone
    };
    // A second sequence, so how he falls does not follow how he stands.
    let falls_back = (soldier * 0.754_877_666_246_692_7) % 1.0 < s.fall_back;
    Stance {
        firing,
        pinned,
        falls_back,
    }
}

/// The fall he plays dying in `posture`.
pub(super) fn fall_of(posture: Posture, stance: Stance) -> &'static str {
    if posture == Posture::Stand && stance.falls_back {
        FALL_BACK
    } else {
        DEATHS[posture as usize]
    }
}
