import { resolveDeviceCaps, type GpuDeviceCaps, type GpuPowerPreference } from "./capabilities";

export interface GpuDeviceInfo {
  adapter: GPUAdapter;
  device: GPUDevice;
  format: GPUTextureFormat;
  vendor: string;
  architecture: string;
  description: string;
  features: string[];
  limits: Record<string, number>;
  caps: GpuDeviceCaps;
}

export interface DeviceLostReport {
  reason: "destroyed" | "unknown";
  message: string;
}

export interface UncapturedErrorReport {
  message: string;
}

interface GpuDeviceCallbacks {
  /** Fires when the GPU device is lost (driver reset, eviction, or destroy()). */
  onDeviceLost?: (report: DeviceLostReport) => void;
  /** Fires on an uncaptured validation/out-of-memory error the renderer did not guard. */
  onUncapturedError?: (report: UncapturedErrorReport) => void;
}

interface RequestGpuDeviceOptions {
  callbacks?: GpuDeviceCallbacks;
  /** Defaults to 'high-performance' — we always want the discrete GPU when present. */
  powerPreference?: GpuPowerPreference;
}

// Bindings the renderer relies on above the spec's conservative device
// defaults. We request them up to the adapter's ceiling so the granted device
// has the headroom (e.g. animation and pose storage buffers); caps reports what landed.
const REQUESTED_LIMIT_KEYS = ["maxStorageBufferBindingSize", "maxBufferSize"] as const;

export async function requestGpuDevice(
  options: RequestGpuDeviceOptions = {},
): Promise<GpuDeviceInfo> {
  if (!navigator.gpu) {
    throw new Error("WebGPU is required: navigator.gpu is not available in this browser.");
  }
  const { callbacks } = options;
  const powerPreference: GpuPowerPreference = options.powerPreference ?? "high-performance";
  let adapter: GPUAdapter | null;
  try {
    adapter = await navigator.gpu.requestAdapter(
      powerPreference === "default" ? {} : { powerPreference },
    );
  } catch (error) {
    throw new Error(`WebGPU adapter request failed: ${messageOf(error)}`);
  }
  if (!adapter) {
    throw new Error("WebGPU is required: requestAdapter returned no adapter.");
  }
  const requiredFeatures: GPUFeatureName[] = adapter.features.has("timestamp-query")
    ? ["timestamp-query"]
    : [];
  const adapterLimits = numericLimits(adapter.limits);
  const requiredLimits: Record<string, number> = {};
  for (const key of REQUESTED_LIMIT_KEYS) {
    const value = adapterLimits[key];
    if (typeof value === "number") requiredLimits[key] = value;
  }
  let device: GPUDevice;
  try {
    device = await adapter.requestDevice({ requiredFeatures, requiredLimits });
  } catch (error) {
    throw new Error(`WebGPU device request failed: ${messageOf(error)}`);
  }
  attachDeviceErrorHandlers(device, callbacks);
  const info = adapter.info ?? {};
  return {
    adapter,
    device,
    format: navigator.gpu.getPreferredCanvasFormat(),
    vendor: info.vendor ?? "unknown",
    architecture: info.architecture ?? "unknown",
    description: info.description ?? "",
    features: Array.from(device.features).sort(),
    limits: numericLimits(device.limits),
    caps: resolveDeviceCaps({
      adapterLimits,
      deviceFeatures: device.features,
      powerPreference,
    }),
  };
}

// GPUSupportedLimits exposes its values as prototype getters, so Object.entries
// returns nothing — we read the standard limit names by direct property access.
const GPU_LIMIT_NAMES = [
  "maxTextureDimension1D",
  "maxTextureDimension2D",
  "maxTextureDimension3D",
  "maxTextureArrayLayers",
  "maxBindGroups",
  "maxBindGroupsPlusVertexBuffers",
  "maxBindingsPerBindGroup",
  "maxDynamicUniformBuffersPerPipelineLayout",
  "maxDynamicStorageBuffersPerPipelineLayout",
  "maxSampledTexturesPerShaderStage",
  "maxSamplersPerShaderStage",
  "maxStorageBuffersPerShaderStage",
  "maxStorageTexturesPerShaderStage",
  "maxUniformBuffersPerShaderStage",
  "maxUniformBufferBindingSize",
  "maxStorageBufferBindingSize",
  "minUniformBufferOffsetAlignment",
  "minStorageBufferOffsetAlignment",
  "maxVertexBuffers",
  "maxBufferSize",
  "maxVertexAttributes",
  "maxVertexBufferArrayStride",
  "maxInterStageShaderVariables",
  "maxColorAttachments",
  "maxColorAttachmentBytesPerSample",
  "maxComputeWorkgroupStorageSize",
  "maxComputeInvocationsPerWorkgroup",
  "maxComputeWorkgroupSizeX",
  "maxComputeWorkgroupSizeY",
  "maxComputeWorkgroupSizeZ",
  "maxComputeWorkgroupsPerDimension",
] as const;

function numericLimits(limits: GPUSupportedLimits): Record<string, number> {
  const indexed = limits as unknown as Record<string, number | undefined>;
  const out: Record<string, number> = {};
  for (const name of GPU_LIMIT_NAMES) {
    const value = indexed[name];
    if (typeof value === "number") out[name] = value;
  }
  return out;
}

/** Wire `device.lost` and `onuncapturederror` so no GPU fault is silent. */
function attachDeviceErrorHandlers(device: GPUDevice, callbacks?: GpuDeviceCallbacks): void {
  const lost = (device as { lost?: Promise<GPUDeviceLostInfo> }).lost;
  if (lost && typeof lost.then === "function") {
    void lost.then((info) => {
      // Our own destroy() on teardown is not a fault.
      if (info.reason === "destroyed") return;
      const report: DeviceLostReport = {
        reason: info.reason ?? "unknown",
        message: info.message ?? "",
      };
      console.error(`WebGPU device lost (${report.reason}): ${report.message}`);
      callbacks?.onDeviceLost?.(report);
    });
  }
  const errorCapable = device as {
    onuncapturederror?: ((event: GPUUncapturedErrorEvent) => void) | null;
  };
  if ("onuncapturederror" in errorCapable) {
    errorCapable.onuncapturederror = (event) => {
      const report: UncapturedErrorReport = {
        message: event.error?.message ?? "unknown WebGPU error",
      };
      console.error(`WebGPU uncaptured error: ${report.message}`);
      callbacks?.onUncapturedError?.(report);
    };
  }
}

export function gpuFailureMessage(error: unknown): string {
  const msg = messageOf(error);
  return `${msg} Enable WebGPU or use a browser/GPU combination that exposes navigator.gpu.`;
}

function messageOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
