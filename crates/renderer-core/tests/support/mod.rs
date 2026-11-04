//! Headless GPU admission using the shared renderer's device policy.
use wgpu::*;

pub struct Gpu {
    pub device: Device,
    pub queue: Queue,
}

pub fn gpu() -> Result<Gpu, String> {
    let instance = Instance::new(InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
        power_preference: PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .map_err(|e| format!("no adapter: {e}"))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&renderer_core::device_descriptor(&adapter)))
            .map_err(|e| format!("no device: {e}"))?;
    Ok(Gpu { device, queue })
}
