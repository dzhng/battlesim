// Live-resource accounting for a caller-owned device. Wraps the device's
// buffer/texture factories so every allocation and destroy is counted,
// including those TypeGPU makes internally. Diagnostic: the lab installs it on
// its own device to prove repeated resize/reset/dispose returns to baseline.

export interface GpuAllocationCounts {
  buffers: number;
  textures: number;
}

export function trackGpuAllocations(device: GPUDevice): () => GpuAllocationCounts {
  const live = { buffers: new Set<GPUBuffer>(), textures: new Set<GPUTexture>() };
  const createBuffer = device.createBuffer.bind(device);
  const createTexture = device.createTexture.bind(device);
  device.createBuffer = (descriptor) => {
    const buffer = createBuffer(descriptor);
    live.buffers.add(buffer);
    const destroy = buffer.destroy.bind(buffer);
    buffer.destroy = () => {
      live.buffers.delete(buffer);
      destroy();
    };
    return buffer;
  };
  device.createTexture = (descriptor) => {
    const texture = createTexture(descriptor);
    live.textures.add(texture);
    const destroy = texture.destroy.bind(texture);
    texture.destroy = () => {
      live.textures.delete(texture);
      destroy();
    };
    return texture;
  };
  return () => ({ buffers: live.buffers.size, textures: live.textures.size });
}
