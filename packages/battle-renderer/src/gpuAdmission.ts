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

/** Overlap routine validation while preserving an explicit pre-submission barrier. */
export class GpuAdmissionBatch {
  private pending: Promise<void>[] = [];
  constructor(private readonly device: GPUDevice) {}

  run<T>(work: () => T): T {
    const close = beginGpuAdmission(this.device);
    try {
      return work();
    } finally {
      const accepted = close();
      // The batch owns rejection until settle(), including while later work awaits.
      void accepted.catch(() => {});
      this.pending.push(accepted);
    }
  }

  async settle(): Promise<void> {
    const results = await Promise.allSettled(this.pending.splice(0));
    const rejected = results.find((result) => result.status === "rejected");
    if (rejected?.status === "rejected") throw rejected.reason;
  }
}
