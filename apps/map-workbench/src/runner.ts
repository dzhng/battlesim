interface RunnerOptions<Input, Output> {
  delayMs: number;
  run(input: Input, signal: AbortSignal): Promise<Output>;
  publish(output: Output, input: Input): void;
  fail(error: unknown, input: Input): void;
}

/** One running job and one latest draft; an older result never becomes current. */
export class LatestRunner<Input, Output> {
  private version = 0;
  private pending: { value: Input; version: number; settled: boolean } | null = null;
  private active: AbortController | null = null;
  private completion: Promise<void> = Promise.resolve();
  private timer: ReturnType<typeof setTimeout> | undefined;
  private disposed = false;

  constructor(private readonly options: RunnerOptions<Input, Output>) {}

  request(value: Input): void {
    if (this.disposed) return;
    const pending = { value, version: ++this.version, settled: false };
    this.pending = pending;
    clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      pending.settled = true;
      this.start();
    }, this.options.delayMs);
  }

  invalidate(): void {
    ++this.version;
    this.pending = null;
    clearTimeout(this.timer);
  }

  cancel(): Promise<void> {
    this.invalidate();
    this.active?.abort();
    return this.completion;
  }

  dispose(): void {
    this.disposed = true;
    void this.cancel();
  }

  private start(): void {
    if (this.disposed || this.active || !this.pending?.settled) return;
    const job = this.pending;
    this.pending = null;
    const controller = new AbortController();
    this.active = controller;
    this.completion = Promise.resolve()
      .then(() => this.options.run(job.value, controller.signal))
      .then(
        (result) => {
          if (!this.disposed && !controller.signal.aborted && job.version === this.version)
            this.options.publish(result, job.value);
        },
        (error: unknown) => {
          if (!this.disposed && !controller.signal.aborted && job.version === this.version)
            this.options.fail(error, job.value);
        },
      )
      .finally(() => {
        this.active = null;
        this.start();
      });
  }
}
