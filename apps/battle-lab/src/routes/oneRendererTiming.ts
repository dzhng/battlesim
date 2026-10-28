/** Separate throughput from single-frame latency; both controls use this loop. */
export async function measureFrames(
  frames: number,
  render: () => void,
  drain: () => Promise<unknown>,
  serial = new URLSearchParams(window.location.search).get("timing") === "1",
) {
  const cpuMs: number[] = [];
  const queueMs: number[] = [];
  if (serial) {
    for (let i = 0; i < frames; i++) {
      const start = performance.now();
      render();
      const encoded = performance.now();
      await drain();
      cpuMs.push(encoded - start);
      queueMs.push(performance.now() - encoded);
    }
  } else {
    const start = performance.now();
    for (let i = 0; i < frames; i++) render();
    const encoded = performance.now();
    await drain();
    cpuMs.push(encoded - start);
    queueMs.push(performance.now() - encoded);
  }
  return {
    encodeMs: cpuMs.reduce((sum, value) => sum + value, 0),
    submitMs: queueMs.reduce((sum, value) => sum + value, 0),
    timing: { protocol: serial ? "serial-queue-completion" : "batched", cpuMs, queueMs },
  };
}
