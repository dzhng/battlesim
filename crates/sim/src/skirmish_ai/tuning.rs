//! The doctrine's playbook numbers: timings and distances it reads from
//! `fixtures/skirmish-ai.json`.

use contract::ids::Tick;

use super::team;

/// The doctrine's timings and distances, authored in
/// `fixtures/skirmish-ai.json`: its playbook's numbers, not battle rules.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Tuning {
    /// Seconds between decisions.
    pub(super) decision_s: f64,
    /// The longest delay before it answers air evidence; each AI draws its
    /// own once, up to this.
    pub(super) reaction_max_s: f64,
    /// Seconds air evidence keeps it ready to answer after the last sighting.
    pub(super) air_memory_s: f64,
    /// A doctrine holds at least `min_dwell_s` before a threat turns it, and
    /// moves on after `max_dwell_s`.
    pub(super) min_dwell_s: f64,
    pub(super) max_dwell_s: f64,
    /// Seconds a sighting or contact stays fresh enough for its artillery.
    pub(super) sighting_horizon_s: f64,
    /// Its supply trucks wait this share of the way from its entry to its
    /// combat units, and move only when farther than `truck_slack_m` off.
    pub(super) supply_share: f64,
    pub(super) truck_slack_m: f64,
    /// Its artillery aims this share of a contact's radius off the contact's
    /// centre, and this share of an objective's radius past the objective's
    /// centre, each radius capped at `aim_cap_m`.
    pub(super) contact_aim_share: f64,
    pub(super) objective_aim_share: f64,
    pub(super) aim_cap_m: f64,
    /// Its recon stands this share of its sight short of an objective, and
    /// at most this share of the way back along its approach.
    pub(super) recon_sight_share: f64,
    pub(super) recon_approach_share: f64,
    /// How long it lets a unit of its own stand held by its own cover
    /// (`BlockedTrajectory`, from any weapon) before it moves it, and how
    /// far: farther than the cover a unit is likely to stand under.
    pub(super) relocate_after_s: f64,
    pub(super) relocate_m: f64,
    /// With no identified enemy in reach, its carriers let squads out this
    /// share of the way from its entry to an objective short of it.
    pub(super) unload_share: f64,
    /// A squad rides only when the ride sets it down at least this much
    /// nearer its objective; nearer than that, it walks.
    pub(super) ride_min_m: f64,
    /// A loaded carrier keeps its drop point until a new one lies farther
    /// than this from it, and lets its squad out within this of it.
    pub(super) reaim_m: f64,
    /// Ground purchases stop this far up their assigned lane, clear of the
    /// entry footprint and before the staging point.
    pub(super) purchase_advance_m: f64,
    /// A unit that loses this share of its strength within this window
    /// pops smoke and reverses this far (`smoke.rs`).
    pub(super) smoke_loss_share: f64,
    pub(super) smoke_window_s: f64,
    pub(super) smoke_reverse_m: f64,
    /// A group below either share triggers an emergency airlift when no
    /// supplier can reach it in this many seconds.
    pub(super) emergency_ammo_share: f64,
    pub(super) emergency_health_share: f64,
    pub(super) emergency_reach_s: f64,
    #[serde(flatten)]
    pub(super) team: team::TeamTuning,
}

impl Tuning {
    pub(super) fn shipped() -> Self {
        serde_json::from_str(include_str!("../../../../fixtures/skirmish-ai.json"))
            .expect("authored skirmish AI tuning")
    }

    /// `s` seconds in ticks at `hz`.
    pub(super) fn ticks(s: f64, hz: u32) -> Tick {
        (s * f64::from(hz)) as Tick
    }
}
