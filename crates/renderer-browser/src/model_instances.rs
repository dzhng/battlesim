use super::*;

impl Renderer {
    /// Grow the shared presentation instance stream for a real capture or
    /// battle frame. The initial probe allocation is deliberately small; the
    /// shared renderer owns the buffer lifetime, so a larger publication must
    /// be admitted here rather than rejected by the upload seam.
    pub(super) fn ensure_production_model_instances(&mut self, bytes: usize) {
        if bytes as u64 <= self.production_model_instances.size() {
            return;
        }
        let mut size = self.production_model_instances.size().max(1);
        while size < bytes as u64 {
            size = size.saturating_mul(2);
        }
        self.production_model_instances = renderer_core::buffer(
            &self.device,
            "shared-production-model-instances",
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
            &vec![0; size as usize],
        );
    }
}
