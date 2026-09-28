/** Close all scopes synchronously before awaiting their results, so other resource owners may proceed. */
export function beginGpuAdmission(device: GPUDevice) {
  device.pushErrorScope("out-of-memory");
  device.pushErrorScope("internal");
  device.pushErrorScope("validation");
  let pending: Promise<void> | undefined;
  return () =>
    (pending ??= (async () => {
      const errors = await Promise.all([
        device.popErrorScope(),
        device.popErrorScope(),
        device.popErrorScope(),
      ]);
      const error = errors.find(Boolean);
      if (error) throw new Error(error.message);
    })());
}
