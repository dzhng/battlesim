// Live-resource accounting for a caller-owned device. Wraps the device's
// buffer/texture factories so every allocation and destroy is counted and
// sized, including those TypeGPU makes internally. The lab installs it on its
// device to prove repeated resize/reset/dispose returns to baseline, and the
// battle frame's resource registry reports its numbers.

export interface GpuAllocationCounts {
  buffers: number;
  textures: number;
  /** Bytes held by the live buffers. */
  bufferBytes: number;
  /** Bytes held by the live textures: every sample, layer and mip level. */
  textureBytes: number;
}

/** Bytes per texel of every format the renderer allocates. A format missing
 *  here is an error at creation, so telemetry never silently reads zero. */
const TEXEL_BYTES: Record<string, number> = {
  r8unorm: 1,
  r16float: 2,
  rg8unorm: 2,
  r32float: 4,
  rg16float: 4,
  rgba8unorm: 4,
  "rgba8unorm-srgb": 4,
  bgra8unorm: 4,
  "bgra8unorm-srgb": 4,
  rgb10a2unorm: 4,
  rg11b10ufloat: 4,
  depth32float: 4,
  depth24plus: 4,
  "depth24plus-stencil8": 4,
  rg32float: 8,
  rgba16float: 8,
  rgba32float: 16,
};

function extent(size: GPUExtent3D): [number, number, number] {
  if (Symbol.iterator in Object(size)) {
    const [w = 1, h = 1, d = 1] = size as Iterable<number>;
    return [w, h, d];
  }
  const dict = size as GPUExtent3DDict;
  return [dict.width, dict.height ?? 1, dict.depthOrArrayLayers ?? 1];
}

/** Bytes a texture holds: texels × samples, summed over its mip chain. A 3D
 *  texture halves its depth with each level; array layers do not. */
export function textureByteSize(descriptor: GPUTextureDescriptor): number {
  const texel = TEXEL_BYTES[descriptor.format];
  if (texel === undefined) throw new Error(`No texel size for format ${descriptor.format}`);
  const [w, h, d] = extent(descriptor.size);
  const is3d = descriptor.dimension === "3d";
  let texels = 0;
  for (let level = 0; level < (descriptor.mipLevelCount ?? 1); level++) {
    const lw = Math.max(1, w >> level);
    const lh = Math.max(1, h >> level);
    const ld = is3d ? Math.max(1, d >> level) : d;
    texels += lw * lh * ld;
  }
  return texels * texel * (descriptor.sampleCount ?? 1);
}

const trackers = new WeakMap<GPUDevice, () => GpuAllocationCounts>();

/** Count this device's live allocations. Installing twice returns the one
 *  tracker, so the factories are wrapped once however many owners ask. */
export function trackGpuAllocations(device: GPUDevice): () => GpuAllocationCounts {
  const existing = trackers.get(device);
  if (existing) return existing;
  const buffers = new Set<GPUBuffer>();
  const textures = new Map<GPUTexture, number>();
  const createBuffer = device.createBuffer.bind(device);
  const createTexture = device.createTexture.bind(device);
  device.createBuffer = (descriptor) => {
    const buffer = createBuffer(descriptor);
    buffers.add(buffer);
    const destroy = buffer.destroy.bind(buffer);
    buffer.destroy = () => {
      buffers.delete(buffer);
      destroy();
    };
    return buffer;
  };
  device.createTexture = (descriptor) => {
    const bytes = textureByteSize(descriptor);
    const texture = createTexture(descriptor);
    textures.set(texture, bytes);
    const destroy = texture.destroy.bind(texture);
    texture.destroy = () => {
      textures.delete(texture);
      destroy();
    };
    return texture;
  };
  const counts = () => {
    let textureBytes = 0;
    for (const bytes of textures.values()) textureBytes += bytes;
    return {
      buffers: buffers.size,
      textures: textures.size,
      bufferBytes: [...buffers].reduce((n, b) => n + b.size, 0),
      textureBytes,
    };
  };
  trackers.set(device, counts);
  return counts;
}
