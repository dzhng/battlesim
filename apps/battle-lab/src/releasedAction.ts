import type { PointerIntent, PointerPick } from "@web/battle/input/pointerIntent";
import type { SimClient } from "@web/battle/sim/client";
import type { CommandAck, Order } from "@web/battle/sim/protocol";

interface CapturedAction {
  pick: PointerPick;
  intent: PointerIntent;
  generation: number;
  client: SimClient | null;
}

interface ReleasedActionRecord {
  intent: PointerIntent | null;
  order: Order;
  queued: boolean;
  generation: number;
  client: SimClient | null;
  ack: CommandAck | null;
}

/** The session-local bridge from a released gesture to its delayed answer. */
export class ReleasedAction {
  private nextGeneration = 0;
  private captured: CapturedAction | null = null;
  private releasing = false;
  private current: ReleasedActionRecord | null = null;

  capture(pick: PointerPick, intent: PointerIntent, client: SimClient | null): void {
    const captured = {
      pick,
      intent,
      generation: ++this.nextGeneration,
      client,
    } satisfies CapturedAction;
    this.captured = captured;
  }

  capturedFor(client: SimClient | null): CapturedAction | null {
    return this.captured?.client === client ? this.captured : null;
  }

  /** Mark the held gesture as the one whose command may own release feedback. */
  beginRelease(): void {
    this.releasing = this.captured !== null;
  }

  cancelCapture(): void {
    this.releasing = false;
    this.captured = null;
  }

  /** Called synchronously by the command issue boundary. */
  issue(
    order: Order,
    queued: boolean,
    client: SimClient | null,
    releasedIntent: PointerIntent | null,
  ): void {
    const captured = this.releasing ? this.captured : null;
    this.current = {
      intent: releasedIntent,
      order,
      queued,
      generation: captured?.generation ?? 0,
      client: captured?.client ?? client,
      ack: null,
    };
    this.releasing = false;
  }

  acknowledge(order: Order, ack: CommandAck, queued: boolean, client: SimClient | null): void {
    const current = this.current;
    if (
      !current ||
      current.client !== client ||
      current.order !== order ||
      current.queued !== queued
    )
      return;
    current.ack = ack;
  }

  get released(): ReleasedActionRecord | null {
    return this.current;
  }

  retire(record: ReleasedActionRecord): void {
    if (this.current === record) this.current = null;
  }

  reset(): void {
    this.releasing = false;
    this.captured = null;
    this.current = null;
  }
}
