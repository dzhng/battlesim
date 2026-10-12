//! The battle's sound, decided once for every client: what is heard, when,
//! where and how loud ([`frame`]), from the tables it reads ([`tables`]), and
//! the synthesized baseline sounds and recipe buffers every player prepares
//! the same way ([`synth`], [`bank`]). A player only carries out the
//! frame's commands on its own mixer.
pub mod bank;
pub mod frame;
pub mod synth;
pub mod tables;

/// The contact a glancing round's ricochet is chosen under.
pub const GLANCE: &str = "ricochet";
