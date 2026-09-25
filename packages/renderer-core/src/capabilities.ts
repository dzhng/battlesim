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
