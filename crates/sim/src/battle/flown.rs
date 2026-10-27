//! What each round flew this tick, and the stretches of it each side may
//! draw.

use super::*;

/// Enemy round flight is shown only over seen ground, sampled this finely.
const SEGMENT_SAMPLES: usize = 8;

/// One round's flight this tick, for the animation feed: presentation data
/// rebuilt every tick from the flight events, never carried state.
#[derive(Clone, Debug)]
pub(super) struct Flown {
    pub(super) side: Side,
    pub(super) from: V3,
    /// Where it glanced off hulls on the way, in order, with the outward
    /// normal there.
    pub(super) ricochets: Vec<(V3, V3)>,
    pub(super) to: V3,
    pub(super) weapon: usize,
    /// The soldier who fired it; `None` for a vehicle's gun.
    pub(super) shooter: Option<u32>,
    /// What it struck at `to`, and the outward normal there.
    pub(super) hit: Option<(SegmentHit, V3)>,
}

/// What a round struck, as its segment shows it: a body's id says whether
/// it is a hull, a prop's row whether it is metal.
pub(super) fn hit(world: &WorldGeometry, struck: Struck) -> SegmentHit {
    match struck {
        Struck::Terrain => SegmentHit::Ground,
        Struck::Prop(id) if world.prop(id).is_some_and(|p| p.body.metal) => SegmentHit::MetalProp,
        Struck::Prop(_) => SegmentHit::Prop,
        Struck::Body(b) if b.0 >= VEHICLE_BODY_BASE => SegmentHit::Hull,
        Struck::Body(_) => SegmentHit::Soldier,
    }
}

impl Flown {
    /// The path's corners: the start, each ricochet, the end.
    fn corners(&self) -> impl Iterator<Item = V3> + '_ {
        std::iter::once(self.from)
            .chain(self.ricochets.iter().map(|&(p, _)| p))
            .chain(std::iter::once(self.to))
    }

    fn segment(&self, piece: Piece, own: bool, hit: Option<(SegmentHit, V3)>) -> VisibleSegment {
        VisibleSegment {
            path: piece.path.into_iter().map(xyz).collect(),
            ricochets: piece
                .ricochets
                .into_iter()
                .map(|(point, n)| SegmentRicochet {
                    point,
                    normal: xyz(n),
                })
                .collect(),
            own,
            kind: self.weapon,
            shooter_member: self.shooter,
            hit: hit.map_or(SegmentHit::None, |(h, _)| h),
            impact_normal: hit.map(|(_, n)| xyz(n)),
        }
    }

    /// The whole flight, for the side that fired it.
    pub(super) fn whole(&self) -> VisibleSegment {
        let piece = Piece {
            path: self.corners().collect(),
            ricochets: (1..).zip(self.ricochets.iter().map(|&(_, n)| n)).collect(),
        };
        self.segment(piece, true, self.hit)
    }
}

/// A drawn stretch of a round's path: its points, and the ricochets among
/// them by index.
#[derive(Default)]
struct Piece {
    path: Vec<V3>,
    pub(super) ricochets: Vec<(usize, V3)>,
}

/// The parts of a round's flight over ground `fog` shows as seen, leg by leg
/// between its ricochets.
pub(super) fn clip_to_seen(fog: &VisibilityField, round: &Flown) -> Vec<VisibleSegment> {
    let corners: Vec<V3> = round.corners().collect();
    let legs = corners.len() - 1;
    // Runs of seen samples, each ending at its last seen sample: a drawn
    // stretch never reaches over unseen ground. A run keeps each ricochet it
    // passes as a corner; along a leg only its latest sample (`tail`)
    // matters. An impact shows only if the struck point itself is seen.
    let mut out = Vec::new();
    let mut run: Option<(Piece, Option<V3>)> = None;
    for leg in 0..legs {
        let (a, b) = (corners[leg], corners[leg + 1]);
        for k in (leg.min(1))..=SEGMENT_SAMPLES {
            let p = a + (b - a) * (k as f64 / SEGMENT_SAMPLES as f64);
            let end = leg + 1 == legs && k == SEGMENT_SAMPLES;
            let seen = fog.visible(p.x, p.y);
            if seen {
                let (piece, tail) = run.get_or_insert_with(Default::default);
                let ricochet = (k == SEGMENT_SAMPLES && !end).then(|| round.ricochets[leg].1);
                if piece.path.is_empty() || ricochet.is_some() {
                    piece.path.push(p);
                    *tail = None;
                    if let Some(n) = ricochet {
                        piece.ricochets.push((piece.path.len() - 1, n));
                    }
                } else {
                    *tail = Some(p);
                }
                if !end {
                    continue;
                }
            }
            if let Some((mut piece, tail)) = run.take() {
                piece.path.extend(tail);
                if piece.path.len() >= 2 {
                    let hit = round.hit.filter(|_| seen && end);
                    out.push(round.segment(piece, false, hit));
                }
            }
        }
    }
    out
}
