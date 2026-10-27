//! The names a publication's enums travel by: each is the index of its
//! name in one of these lists, which the layout publishes in order.
use contract::catalog::AltitudeLayer;
use contract::command::{Engagement, MoveDirection, RoutePolicy};
use contract::observation::{
    ActionReason, AirTransitDirection, ContactSource, EncounterResult, HoldPhase, Intensity,
    LeanSide, MoveState, Posture, SegmentHit, ServiceStatus, SoundBand, SoundCategory,
    SuppressionTier,
};
use contract::scenario::CoverTier;

pub(super) const MOVE_STATES: [MoveState; 7] = [
    MoveState::Idle,
    MoveState::Moving,
    MoveState::Waiting,
    MoveState::RouteBlocked,
    MoveState::Halted,
    MoveState::Packing,
    MoveState::Planning,
];
pub(super) const COVER_TIERS: [CoverTier; 3] =
    [CoverTier::Light, CoverTier::Medium, CoverTier::Heavy];
pub(super) const LEAN_SIDES: [LeanSide; 2] = [LeanSide::Left, LeanSide::Right];
pub(super) const POSTURES: [Posture; 2] = [Posture::Packed, Posture::Deployed];
pub(super) const POLICIES: [RoutePolicy; 2] = [RoutePolicy::Shortest, RoutePolicy::Fastest];
pub(super) const DIRECTIONS: [MoveDirection; 2] = [MoveDirection::Forward, MoveDirection::Reverse];
pub(super) const CONTACT_SOURCES: [ContactSource; 3] = [
    ContactSource::Firing,
    ContactSource::LastSeen,
    ContactSource::Radar,
];
pub(super) const LAYERS: [AltitudeLayer; 3] = [
    AltitudeLayer::Ground,
    AltitudeLayer::LowAir,
    AltitudeLayer::HighAir,
];
pub(super) const AIR_TRANSIT_DIRECTIONS: [AirTransitDirection; 2] =
    [AirTransitDirection::In, AirTransitDirection::Out];
pub(super) const SOUND_CATEGORIES: [SoundCategory; 4] = [
    SoundCategory::Infantry,
    SoundCategory::Vehicle,
    SoundCategory::Shot,
    SoundCategory::Shout,
];
pub(super) const INTENSITIES: [Intensity; 2] = [Intensity::Steady, Intensity::Stressed];
pub(super) const SOUND_BANDS: [SoundBand; 2] = [SoundBand::Near, SoundBand::Far];
pub(super) const GROUP_ENCODINGS: [&str; 4] = ["replacement", "snapshot", "copies", "packed"];
pub(super) const SEGMENT_HITS: [SegmentHit; 6] = [
    SegmentHit::None,
    SegmentHit::Ground,
    SegmentHit::Hull,
    SegmentHit::Prop,
    SegmentHit::Soldier,
    SegmentHit::MetalProp,
];
pub(super) const ENGAGEMENTS: [Engagement; 2] =
    [Engagement::FireAtWill, Engagement::ReturnFireOnly];
pub(super) const ENCOUNTER_RESULTS: [EncounterResult; 4] = [
    EncounterResult::Running,
    EncounterResult::Captured,
    EncounterResult::Defeated,
    EncounterResult::Inconclusive,
];
pub(super) const SERVICE_STATUSES: [ServiceStatus; 7] = [
    ServiceStatus::OutOfRange,
    ServiceStatus::SourceNotDeployed,
    ServiceStatus::Moving,
    ServiceStatus::Firing,
    ServiceStatus::Serving,
    ServiceStatus::NoStock,
    ServiceStatus::Full,
];
pub(super) const SUPPRESSION_TIERS: [SuppressionTier; 3] = [
    SuppressionTier::None,
    SuppressionTier::Suppressed,
    SuppressionTier::Pinned,
];
pub(super) const HOLD_PHASES: [HoldPhase; 3] =
    [HoldPhase::Entering, HoldPhase::Inside, HoldPhase::Exiting];
pub(super) const HOST_KINDS: [&str; 2] = ["building", "carrier"];
/// Every refusal the contract names, in its order; a new reason appends here
/// and to the codec vectors.
pub(super) const ORDER_ERROR_NAMES: [&str; 29] = [
    "passive_side",
    "not_skirmish",
    "match_finished",
    "unit_unavailable",
    "wrong_faction",
    "insufficient_credits",
    "unit_limit_reached",
    "out_of_sequence",
    "no_units",
    "unknown_unit",
    "not_own_unit",
    "wrong_player",
    "missing_player",
    "out_of_bounds",
    "no_valid_destination",
    "replay_in_progress",
    "rate_limited",
    "unknown_target",
    "air_contact",
    "destroyed",
    "not_infantry",
    "not_a_building",
    "capacity_full",
    "one_squad_per_building",
    "building_occupied",
    "not_a_carrier",
    "no_room_to_unload",
    "aboard",
    "ai_controlled",
];
pub(super) const REASONS: [ActionReason; 16] = [
    ActionReason::Firing,
    ActionReason::NoCompatibleTarget,
    ActionReason::HoldingFire,
    ActionReason::OutOfRange,
    ActionReason::BlockedTrajectory,
    ActionReason::FriendlyInLine,
    ActionReason::Aiming,
    ActionReason::Reloading,
    ActionReason::TurretTraversing,
    ActionReason::MovingStationaryWeapon,
    ActionReason::OutOfAmmo,
    ActionReason::TrackingLastSighting,
    ActionReason::Guiding,
    ActionReason::NoOwnSight,
    ActionReason::NoFacingSlot,
    ActionReason::ChangingPosition,
];
pub(super) const TARGET_KINDS: [&str; 4] = ["none", "identified", "contact", "ground"];

pub(super) fn tag<T: PartialEq>(all: &[T], v: &T) -> f32 {
    all.iter().position(|k| k == v).unwrap() as f32
}

pub(super) fn names<T: std::fmt::Debug>(all: &[T]) -> Vec<String> {
    all.iter()
        .map(|k| {
            let s = format!("{k:?}");
            // CamelCase → snake_case, matching the serde names.
            s.chars()
                .enumerate()
                .fold(String::new(), |mut out, (i, c)| {
                    if c.is_uppercase() && i > 0 {
                        out.push('_');
                    }
                    out.push(c.to_ascii_lowercase());
                    out
                })
        })
        .collect()
}
