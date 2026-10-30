//! Desktop adapter admission around the shared renderer's device policy.
use wgpu::*;

pub struct Gpu {
    pub adapter: Adapter,
    pub device: Device,
    pub queue: Queue,
}

/// An instance that can present to a window on this display.
pub fn instance(display: Box<dyn wgpu::wgt::WgpuHasDisplayHandle>) -> Instance {
    Instance::new(InstanceDescriptor::new_with_display_handle(display))
}

pub use renderer_core::{bind, buffer};

pub fn headless_instance() -> Instance {
    Instance::new(InstanceDescriptor::new_without_display_handle())
}

pub fn open(instance: &Instance, surface: Option<&Surface>) -> Result<Gpu, String> {
    let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
        power_preference: PowerPreference::HighPerformance,
        compatible_surface: surface,
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .map_err(|e| format!("no adapter: {e}"))?;
    let (device, queue) =
        pollster::block_on(adapter.request_device(&renderer_core::device_descriptor(&adapter)))
            .map_err(|e| format!("no device: {e}"))?;
    Ok(Gpu {
        adapter,
        device,
        queue,
    })
}
