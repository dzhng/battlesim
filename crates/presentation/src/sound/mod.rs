//! The battle's sound, decided once for every client: what is heard, when,
//! where and how loud ([`frame`]), from the tables it reads ([`tables`]), and
//! the synthesized baseline sounds and recipe buffers every player prepares
//! the same way ([`synth`], [`bank`]), and which units' voice lines play
//! ([`voices`]), and the battle's score: its mood and next track
//! ([`music`]). A player only carries out their commands on its own mixer.
pub mod bank;
pub mod frame;
pub mod music;
pub mod synth;
pub mod tables;
pub mod voices;

/// The contact a glancing round's ricochet is chosen under.
pub const GLANCE: &str = "ricochet";

pub mod catalog;
mod mix;

mod setup;
