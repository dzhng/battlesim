//! Document-lifetime GPU admission shared by independent canvas renderers.
use super::*;
use futures_channel::oneshot;
use std::rc::Rc;
use std::sync::Mutex;

struct Failure {
    sender: Option<oneshot::Sender<Option<String>>>,
    message: Option<String>,
    disposed: bool,
}

struct FailureState(Mutex<Failure>);

impl FailureState {
    fn report(&self, message: String) {
        let sender = {
            let mut state = self.0.lock().expect("GPU failure lock");
            if state.disposed {
                return;
            }
            if state.message.is_some() {
                return;
            }
            state.message = Some(message.clone());
            state.sender.take()
        };
        if let Some(sender) = sender {
            let _ = sender.send(Some(message));
        }
    }

    fn dispose(&self) {
        let sender = {
            let mut state = self.0.lock().expect("GPU failure lock");
            state.disposed = true;
            state.sender.take()
        };
        if let Some(sender) = sender {
            let _ = sender.send(None);
        }
    }
}

pub(super) struct GpuInner {
    pub(super) instance: Instance,
    pub(super) adapter: Adapter,
    pub(super) device: Device,
    pub(super) queue: Queue,
    failure: Arc<FailureState>,
}

impl GpuInner {
    pub(super) fn ensure_live(&self) -> Result<(), JsValue> {
        let state = self.failure.0.lock().expect("GPU failure lock");
        if state.disposed {
            return Err(js_error("Page GPU resources have been released"));
        }
        match &state.message {
            Some(message) => Err(js_error(message)),
            None => Ok(()),
        }
    }
}

/// The page owns this handle; Renderer owns only its canvas and scene resources.
#[wasm_bindgen]
pub struct BrowserGpu {
    pub(super) inner: Rc<GpuInner>,
    failure: js_sys::Promise,
    info: String,
}

#[wasm_bindgen]
impl BrowserGpu {
    pub async fn new() -> Result<BrowserGpu, JsValue> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::BROWSER_WEBGPU,
            ..InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .map_err(|error| {
                js_error(format!(
                    "WebGPU is required: no compatible graphics adapter. {error}"
                ))
            })?;
        let (device, queue) = adapter
            .request_device(&renderer_core::device_descriptor(&adapter))
            .await
            .map_err(|error| js_error(format!("WebGPU device admission failed: {error}")))?;
        let (sender, receiver) = oneshot::channel();
        let state = Arc::new(FailureState(Mutex::new(Failure {
            sender: Some(sender),
            message: None,
            disposed: false,
        })));
        let errors = state.clone();
        device.on_uncaptured_error(Arc::new(move |error| {
            errors.report(format!("WebGPU error: {error}"));
        }));
        let losses = state.clone();
        device.set_device_lost_callback(move |reason, message| {
            losses.report(format!("WebGPU device lost ({reason:?}): {message}"));
        });
        let failure = wasm_bindgen_futures::future_to_promise(async move {
            Ok(receiver
                .await
                .ok()
                .flatten()
                .map_or(JsValue::UNDEFINED, |message| JsValue::from_str(&message)))
        });
        let info = adapter_info(&adapter, &device);
        Ok(Self {
            inner: Rc::new(GpuInner {
                instance,
                adapter,
                device,
                queue,
                failure: state,
            }),
            failure,
            info,
        })
    }

    /// Resolves once: an unexpected failure message, or undefined on disposal.
    #[wasm_bindgen(unchecked_return_type = "Promise<string | undefined>")]
    pub fn failure(&self) -> js_sys::Promise {
        self.failure.clone()
    }

    /// Owned diagnostics; no WebGPU adapter or device escapes into JavaScript.
    pub fn info(&self) -> String {
        self.info.clone()
    }

    /// Only the document owner disposes this device, after releasing its scenes.
    pub fn dispose(&self) {
        self.inner.failure.dispose();
        self.inner.device.destroy();
    }
}

fn adapter_info(adapter: &Adapter, device: &Device) -> String {
    let info = adapter.get_info();
    let limits = device.limits();
    let features: Vec<_> = device
        .features()
        .iter_names()
        .map(|(name, _)| name.to_ascii_lowercase().replace('_', "-"))
        .collect();
    // BrowserWebGpu currently exposes description as name, but not vendor or
    // architecture. Preserve available facts rather than inventing those fields.
    serde_json::json!({
        "name": info.name,
        "backend": "browser-webgpu",
        "fallback": info.device_type == DeviceType::Cpu,
        "features": features,
        "limits": {
            "maxTextureDimension1D": limits.max_texture_dimension_1d,
            "maxTextureDimension2D": limits.max_texture_dimension_2d,
            "maxTextureDimension3D": limits.max_texture_dimension_3d,
            "maxTextureArrayLayers": limits.max_texture_array_layers,
            "maxBindGroups": limits.max_bind_groups,
            "maxBindingsPerBindGroup": limits.max_bindings_per_bind_group,
            "maxDynamicUniformBuffersPerPipelineLayout": limits.max_dynamic_uniform_buffers_per_pipeline_layout,
            "maxDynamicStorageBuffersPerPipelineLayout": limits.max_dynamic_storage_buffers_per_pipeline_layout,
            "maxSampledTexturesPerShaderStage": limits.max_sampled_textures_per_shader_stage,
            "maxSamplersPerShaderStage": limits.max_samplers_per_shader_stage,
            "maxStorageBuffersPerShaderStage": limits.max_storage_buffers_per_shader_stage,
            "maxStorageTexturesPerShaderStage": limits.max_storage_textures_per_shader_stage,
            "maxUniformBuffersPerShaderStage": limits.max_uniform_buffers_per_shader_stage,
            "maxUniformBufferBindingSize": limits.max_uniform_buffer_binding_size,
            "maxStorageBufferBindingSize": limits.max_storage_buffer_binding_size,
            "minUniformBufferOffsetAlignment": limits.min_uniform_buffer_offset_alignment,
            "minStorageBufferOffsetAlignment": limits.min_storage_buffer_offset_alignment,
            "maxVertexBuffers": limits.max_vertex_buffers,
            "maxBufferSize": limits.max_buffer_size,
            "maxVertexAttributes": limits.max_vertex_attributes,
            "maxVertexBufferArrayStride": limits.max_vertex_buffer_array_stride,
            "maxInterStageShaderVariables": limits.max_inter_stage_shader_variables,
            "maxColorAttachments": limits.max_color_attachments,
            "maxColorAttachmentBytesPerSample": limits.max_color_attachment_bytes_per_sample,
            "maxComputeWorkgroupStorageSize": limits.max_compute_workgroup_storage_size,
            "maxComputeInvocationsPerWorkgroup": limits.max_compute_invocations_per_workgroup,
            "maxComputeWorkgroupSizeX": limits.max_compute_workgroup_size_x,
            "maxComputeWorkgroupSizeY": limits.max_compute_workgroup_size_y,
            "maxComputeWorkgroupSizeZ": limits.max_compute_workgroup_size_z,
            "maxComputeWorkgroupsPerDimension": limits.max_compute_workgroups_per_dimension,
        },
    }).to_string()
}
