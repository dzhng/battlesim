//! The static world resolved once per map for every renderer (slice 20b):
//! the buildings resolved against the template art library, and the walk that
//! chooses what each view draws of them. The simulation never reads art.
pub mod buildings;
#[cfg(feature = "files")]
pub mod files;
pub mod library;
pub mod walk;
