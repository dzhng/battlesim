import type { Auditioner, LiveMix } from "./audition";

export type AuditionPhase = "stopped" | "loading" | "playing";

/** The workbench projection of the one library or movement audition it owns. */
export interface AuditionSnapshot {
  phase: AuditionPhase;
  key: string | null;
  mix: LiveMix | null;
  /** Monotonic stop notification consumed by the music tab. */
  stops: number;
}

type Listener = () => void;

/** Owns the workbench's keyed audition admission while SoundAuditioner owns resources. */
export class AuditionSession {
  private active: object | null = null;
  private snapshot: AuditionSnapshot = { phase: "stopped", key: null, mix: null, stops: 0 };
  private readonly listeners = new Set<Listener>();

  constructor(private readonly audition: Auditioner) {}

  getSnapshot = () => this.snapshot;

  subscribe = (listener: Listener) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  /** Stop the current owner and notify children even when no keyed run is active. */
  stop() {
    this.invalidate();
    this.publish({ phase: "stopped", key: null, mix: null, stops: this.snapshot.stops + 1 });
  }

  /**
   * Starts one keyed workbench audition. A stale completion resolves quietly and cannot
   * install its key or mix after a replacement, while the underlying auditioner still
   * owns cancellation and resource disposal.
   */
  async start<T>(
    key: string,
    open: () => Promise<T>,
    mixOf: (value: T) => LiveMix | null = () => null,
  ): Promise<T | null> {
    this.invalidate();
    const operation = {};
    this.active = operation;
    this.publish({ phase: "loading", key, mix: null, stops: this.snapshot.stops });
    try {
      const value = await open();
      if (this.active !== operation) return null;
      this.active = null;
      this.publish({ phase: "playing", key, mix: mixOf(value), stops: this.snapshot.stops });
      return value;
    } catch (error) {
      if (this.active !== operation) return null;
      this.invalidate();
      this.publish({ phase: "stopped", key: null, mix: null, stops: this.snapshot.stops });
      throw error;
    }
  }

  private invalidate() {
    this.active = null;
    this.audition.stop();
  }

  private publish(snapshot: AuditionSnapshot) {
    this.snapshot = snapshot;
    for (const listener of this.listeners) listener();
  }
}
