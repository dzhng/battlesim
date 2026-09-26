/** The world names the GPU work it owns. Whether anyone is measuring those names
 * is not the world's concern: a measurement owner outside this package may install
 * one observer per device, and an unobserved device pays only a WeakMap lookup. */
export interface GpuScopeObserver {
  withScope<T>(label: string, work: () => T): T;
}

const observers = new WeakMap<GPUDevice, GpuScopeObserver>();

/** Scope names describe owned work; runtime libraries still encode every pass. */
export function nativeGpuScope<T>(device: GPUDevice, label: string, work: () => T): T {
  const observer = observers.get(device);
  return observer ? observer.withScope(label, work) : work();
}

export function hasGpuScopeObserver(device: GPUDevice): boolean {
  return observers.has(device);
}

export function setGpuScopeObserver(device: GPUDevice, observer: GpuScopeObserver): void {
  observers.set(device, observer);
}

export function clearGpuScopeObserver(device: GPUDevice): void {
  observers.delete(device);
}
