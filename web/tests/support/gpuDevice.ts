/** Device-edge stand-ins whose native lifetime is independent of renderer accounting. */
export function fakeGpuDevice() {
  const resources = new Set<object>();
  let destroyCalls = 0;
  const resource = (descriptor: { size: unknown; format?: string }) => {
    const value = {
      size: descriptor.size,
      format: descriptor.format,
      destroy() {
        destroyCalls++;
        resources.delete(value);
      },
      createView: () => ({}),
    };
    resources.add(value);
    return value;
  };
  return {
    device: { createBuffer: resource, createTexture: resource } as unknown as GPUDevice,
    resources,
    get destroyCalls() {
      return destroyCalls;
    },
  };
}
