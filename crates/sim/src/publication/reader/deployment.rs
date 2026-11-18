//! Shared interpretation of the fixed deployment cells.
use super::{Deployment, Reader, Result};

impl Reader {
    pub(super) fn deployment(&self, progress: f64, target: f64) -> Result<Option<Deployment>> {
        Ok(self
            .optional("postures", target)?
            .map(|target| Deployment { progress, target }))
    }
}
