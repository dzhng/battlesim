/** Completion belongs to submitted frames, including across scene replacement. */
export class ViewportFrames {
  private pending: { resolve: () => void; reject: (error: unknown) => void }[] = [];
  private error: unknown;

  constructor(
    signal: AbortSignal,
    private readonly redraw: () => void,
  ) {
    signal.addEventListener("abort", () => this.fail(new Error("Viewport departed")), {
      once: true,
    });
  }

  request(): Promise<void> {
    if (this.error !== undefined) return Promise.reject(this.error);
    this.redraw();
    return new Promise((resolve, reject) => this.pending.push({ resolve, reject }));
  }

  submitted(): void {
    for (const request of this.pending.splice(0)) request.resolve();
  }

  fail(error: unknown): void {
    this.error = error;
    for (const request of this.pending.splice(0)) request.reject(error);
  }
}
