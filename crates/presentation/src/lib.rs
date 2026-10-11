//! The per-frame presentation both clients share
//! (`specs/native-rendering-parity/slices/18b-presentation-api.md`): one
//! side's publications in, a [`Frame`] of flat records out. It interpolates
//! between ticks, poses every soldier and vehicle (joint palettes included),
//! makes the combat effects and their cast lights, and chooses by the camera
//! what is drawn and at which tier. It reads only the side's publication, so
//! it never sees past fog of war, and it never touches the simulation, so
//! tuning it never moves a digest or a replay.
pub mod effects;
pub mod presentation;
pub mod records;
pub mod seed;
pub mod units;
pub mod view;
pub mod world;
