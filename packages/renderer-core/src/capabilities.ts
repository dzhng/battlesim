interface GpuCapabilities {
  ok: boolean;
  adapter: string;
  format: string;
  reason: string;
}

export type GpuPowerPreference = "high-performance" | "low-power" | "default";

// The single source of truth for what the granted device can do and every
// deliberate downgrade we make from it. Computed once at device creation from
// the adapter limits + granted device features; passes read it, they never
// re-probe the adapter. The world depth format is NOT a capability: it is the
// fixed engine-wide contract (depthContract.ts GPU_DEPTH_FORMAT, depth32float,
// WebGPU-core mandatory).
export interface GpuDeviceCaps {
  maxStorageBufferBindingSize: number;
  maxBufferSize: number;
  msaaSampleCount: number;
  msaaSupported: boolean;
  timestampQuery: boolean;
  powerPreference: GpuPowerPreference;
}

// 4x is the sample count WebGPU core guarantees for every renderable format, so
// it is the one MSAA tier we rely on without a per-format query.
const GPU_MSAA_SAMPLE_COUNT = 4 as const;

interface ResolveDeviceCapsInput {
  adapterLimits: Record<string, number>;
  deviceFeatures: Iterable<string>;
  powerPreference: GpuPowerPreference;
}

export function resolveDeviceCaps(input: ResolveDeviceCapsInput): GpuDeviceCaps {
  const features = new Set(input.deviceFeatures);
  return {
    maxStorageBufferBindingSize: input.adapterLimits.maxStorageBufferBindingSize ?? 0,
    maxBufferSize: input.adapterLimits.maxBufferSize ?? 0,
    msaaSampleCount: GPU_MSAA_SAMPLE_COUNT,
    msaaSupported: true,
    timestampQuery: features.has("timestamp-query"),
    powerPreference: input.powerPreference,
  };
}

export interface GpuCapabilityOptions {
  forceUnsupported?: boolean;
}

export async function probeGpuCapabilities(
  options: GpuCapabilityOptions = {},
): Promise<GpuCapabilities> {
  try {
    if (options.forceUnsupported) {
      throw new Error("WebGPU support was disabled for this verification run.");
    }
    if (!navigator.gpu) {
      throw new Error("navigator.gpu is not available in this browser.");
    }
    const adapter = await navigator.gpu.requestAdapter();
    if (!adapter) {
      throw new Error("requestAdapter returned no WebGPU adapter.");
    }
    const info = adapter.info ?? {};
    return {
      ok: true,
      adapter:
        [info.vendor, info.architecture, info.description].filter(Boolean).join(" / ") ||
        "available adapter",
      format: navigator.gpu.getPreferredCanvasFormat(),
      reason: "",
    };
  } catch (error) {
    return {
      ok: false,
      adapter: "",
      format: "",
      reason: error instanceof Error ? error.message : String(error),
    };
  }
}
